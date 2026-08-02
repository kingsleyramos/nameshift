//! On-disk integration tests (§18.2): real tempdir trees through the real
//! engine, asserting both the resulting disk layout and the recorded
//! snapshot.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use nameshift_engine::{
    apply_plan, build_plan, compute_preview, compute_revert_preview, host_profile,
    perform_moves_hierarchical, recover_orphaned_temp_files, revert_through, ApplyOutcome,
    CoreState, DirectoryNameCache, FileItem, FileSortKey, FsDirectoryLister, NoMetadata,
    RenameRule, RevertStatus, RuleKind, WatchedFolder, TEMP_PREFIX,
};
use tempfile::TempDir;

static NO_METADATA: NoMetadata = NoMetadata;

fn make_file(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, name.as_bytes()).expect("create fixture file");
    path
}

fn listing(dir: &Path) -> HashSet<String> {
    fs::read_dir(dir)
        .expect("readable dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect()
}

fn assert_no_temps(dir: &Path) {
    for entry in walkdir(dir) {
        let name = entry
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        assert!(!name.starts_with(TEMP_PREFIX), "stranded temp: {entry:?}");
    }
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walkdir(&path));
        }
        out.push(path);
    }
    out
}

fn state_with_files(paths: &[&Path]) -> CoreState {
    CoreState {
        files: paths.iter().map(|p| FileItem::new(p, p.is_dir())).collect(),
        sort_key: FileSortKey::Name,
        ..CoreState::default()
    }
}

fn prefix_rule(text: &str) -> RenameRule {
    let mut r = RenameRule::new(RuleKind::AddPrefix);
    r.text = text.into();
    r
}

/// Run the full preview→plan→apply pipeline against the real filesystem.
fn apply(state: &mut CoreState) -> ApplyOutcome {
    let mut disk = DirectoryNameCache::new(Box::new(FsDirectoryLister));
    let preview = compute_preview(state, host_profile(), &mut disk, &NO_METADATA);
    let plan = build_plan(state, &preview).expect("can_apply must hold");
    // The preview promised these names; remember them to compare after.
    apply_plan(state, &plan, &|| false, &mut |_| {})
}

// ---- 1. plain batch: preview == outcome -----------------------------------

#[test]
fn plain_batch_rename_matches_preview() {
    let tmp = TempDir::new().unwrap();
    let a = make_file(tmp.path(), "one.txt");
    let b = make_file(tmp.path(), "two.txt");
    let mut state = state_with_files(&[&a, &b]);
    state.rules = vec![prefix_rule("x-")];

    let mut disk = DirectoryNameCache::new(Box::new(FsDirectoryLister));
    let preview = compute_preview(&state, host_profile(), &mut disk, &NO_METADATA);
    let promised: HashSet<String> = preview.entries.iter().map(|e| e.new_name.clone()).collect();

    let outcome = apply(&mut state);
    assert_eq!(outcome.renamed, 2);
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert_eq!(listing(tmp.path()), promised);
    assert_no_temps(tmp.path());

    // The snapshot records exactly what happened.
    assert_eq!(state.history.len(), 1);
    assert_eq!(state.history[0].entries.len(), 2);
    assert_eq!(state.history[0].summary, "Prefix “x-”");
    // Tracked paths moved with the files.
    let tracked: HashSet<String> = state.files.iter().map(|f| f.name()).collect();
    assert_eq!(tracked, promised);
}

// ---- 2. sibling swap (two-phase proof) ------------------------------------

#[test]
fn sibling_swap_two_phase() {
    let tmp = TempDir::new().unwrap();
    let a = make_file(tmp.path(), "a.txt");
    let b = make_file(tmp.path(), "b.txt");
    let mut state = state_with_files(&[&a, &b]);
    state.files[0].override_name = Some("b.txt".into());
    state.files[1].override_name = Some("a.txt".into());

    let outcome = apply(&mut state);
    assert_eq!(outcome.renamed, 2);
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    // Contents prove the swap actually happened.
    assert_eq!(
        fs::read_to_string(tmp.path().join("b.txt")).unwrap(),
        "a.txt"
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join("a.txt")).unwrap(),
        "b.txt"
    );
    assert_no_temps(tmp.path());
}

// ---- 3. case-only rename ---------------------------------------------------

#[test]
fn case_only_rename() {
    let tmp = TempDir::new().unwrap();
    let readme = make_file(tmp.path(), "readme.txt");
    let mut state = state_with_files(&[&readme]);
    state.files[0].override_name = Some("README.txt".into());

    let outcome = apply(&mut state);
    assert_eq!(outcome.renamed, 1, "{:?}", outcome.errors);
    let names = listing(tmp.path());
    assert!(names.contains("README.txt"), "{names:?}");
    assert!(!names.contains("readme.txt"), "{names:?}");
    assert_no_temps(tmp.path());
}

// ---- 4. nested folder batch + revert restores the tree ---------------------

#[test]
fn nested_folder_batch_and_revert() {
    let tmp = TempDir::new().unwrap();
    let parent = tmp.path().join("Parent");
    let child = parent.join("Child");
    fs::create_dir_all(&child).unwrap();
    let grandchild = make_file(&child, "deep.txt");

    let mut state = CoreState {
        files: vec![
            FileItem::new(&parent, true),
            FileItem::new(&child, true),
            FileItem::new(&grandchild, false),
        ],
        sort_key: FileSortKey::Name,
        ..CoreState::default()
    };
    // Rename all three in one Apply: prefix every FOLDER name and the file.
    state.files[0].override_name = Some("NewParent".into());
    state.files[1].override_name = Some("NewChild".into());
    state.files[2].override_name = Some("renamed.txt".into());

    // Folders mode applies the folders; files mode applies the file. Run the
    // folder moves and the file move as ONE hierarchical batch, the way the
    // apply worker does when the plan mixes depths.
    let moves = vec![
        (parent.clone(), tmp.path().join("NewParent")),
        (child.clone(), parent.join("NewChild")),
        (grandchild.clone(), child.join("renamed.txt")),
    ];
    let outcome = perform_moves_hierarchical(&moves, true, &|| false, &mut |_| {});
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert_eq!(outcome.succeeded.len(), 3);

    // Parents-first rewriting: recorded entries match the FINAL layout.
    let expected_file = tmp.path().join("NewParent/NewChild/renamed.txt");
    assert!(expected_file.is_file(), "final layout holds");
    let recorded_file_to = &outcome
        .succeeded
        .iter()
        .find(|(from, _)| from.ends_with("deep.txt"))
        .unwrap()
        .1;
    assert_eq!(
        recorded_file_to, &expected_file,
        "recorded entries match final disk layout"
    );

    // Fold into state + snapshot, then revert restores the exact tree.
    nameshift_engine::rewrite_live_paths(&mut state, &outcome.succeeded);
    let snapshot_id =
        nameshift_engine::record_snapshot(&mut state, "test".into(), &outcome.succeeded);
    let revert = revert_through(&mut state, snapshot_id, &|| false, &mut |_, _| {});
    assert!(revert.errors.is_empty(), "{:?}", revert.errors);
    assert_eq!(revert.restored, 3);
    assert!(grandchild.is_file(), "original tree restored");
    assert!(state.history.is_empty(), "fully-reverted snapshot removed");
    assert_no_temps(tmp.path());
}

// ---- 5. rewrite_live_paths tracks everything -------------------------------

#[test]
fn rewrite_live_paths_tracks_items_exclusions_and_watched_roots() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("Photos");
    fs::create_dir(&dir).unwrap();
    let inside = make_file(&dir, "img.jpg");
    let excluded = dir.join("skip.jpg");

    let mut state = CoreState {
        files: vec![FileItem::new(&inside, false)],
        watched_folders: vec![WatchedFolder::new(&dir)],
        ..CoreState::default()
    };
    state.excluded_paths.insert(excluded.clone());
    let folder_id = state.watched_folders[0].id;

    let renamed_dir = tmp.path().join("Shoots");
    let moves = vec![(dir.clone(), renamed_dir.clone())];
    fs::rename(&dir, &renamed_dir).unwrap();
    let rearmed = nameshift_engine::rewrite_live_paths(&mut state, &moves);

    assert_eq!(state.files[0].path, renamed_dir.join("img.jpg"));
    assert!(state.excluded_paths.contains(&renamed_dir.join("skip.jpg")));
    assert_eq!(state.watched_folders[0].path, renamed_dir);
    assert_eq!(state.watched_folders[0].id, folder_id, "same folder id");
    assert_eq!(rearmed, vec![folder_id], "the shell re-arms this watcher");
}

// ---- 6. cancellation -------------------------------------------------------

#[test]
fn cancel_in_phase_one_renames_nothing() {
    let tmp = TempDir::new().unwrap();
    let a = make_file(tmp.path(), "a.txt");
    let b = make_file(tmp.path(), "b.txt");
    let moves = vec![
        (a.clone(), tmp.path().join("x-a.txt")),
        (b.clone(), tmp.path().join("x-b.txt")),
    ];

    // Call sites: hierarchical group check (1), then phase 1 per item (2, 3).
    // Cancelling on the 3rd call lands mid-phase-1 with `a` already staged.
    let calls = AtomicUsize::new(0);
    let outcome = perform_moves_hierarchical(
        &moves,
        true,
        &|| calls.fetch_add(1, Ordering::SeqCst) + 1 >= 3,
        &mut |_| {},
    );
    assert!(
        outcome.succeeded.is_empty(),
        "a phase-1 cancel renames nothing"
    );
    assert_eq!(
        listing(tmp.path()),
        HashSet::from(["a.txt".into(), "b.txt".into()])
    );
    assert_no_temps(tmp.path());
}

#[test]
fn cancel_in_phase_two_mid_swap_completes_the_partner_forward() {
    let tmp = TempDir::new().unwrap();
    let a = make_file(tmp.path(), "a.txt");
    let b = make_file(tmp.path(), "b.txt");
    let moves = vec![(a.clone(), b.clone()), (b.clone(), a.clone())];

    // Cancel as soon as the first commit lands (progress fires per commit
    // here): the second temp can't roll back to b.txt — a.txt already
    // committed there — so it must complete forward and be recorded.
    let cancel = AtomicBool::new(false);
    let outcome = perform_moves_hierarchical(
        &moves,
        true,
        &|| cancel.load(Ordering::SeqCst),
        &mut |done| {
            if done >= 1 {
                cancel.store(true, Ordering::SeqCst);
            }
        },
    );
    assert_eq!(
        outcome.succeeded.len(),
        2,
        "partner completed forward: {:?}",
        outcome.errors
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join("b.txt")).unwrap(),
        "a.txt"
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join("a.txt")).unwrap(),
        "b.txt"
    );
    assert_no_temps(tmp.path());
}

// ---- 7. orphan temp recovery ----------------------------------------------

#[test]
fn orphan_recovery_restores_only_valid_free_temps() {
    let tmp = TempDir::new().unwrap();
    let valid_uuid = "7B4C2A10-53E5-4D2A-9C6F-2E8B1F0A9D11";
    make_file(
        tmp.path(),
        &format!("{TEMP_PREFIX}{valid_uuid}-restored.txt"),
    );
    make_file(
        tmp.path(),
        &format!("{TEMP_PREFIX}not-a-uuid-here-xxxx-lost.txt"),
    );
    make_file(
        tmp.path(),
        &format!("{TEMP_PREFIX}{valid_uuid}-occupied.txt"),
    );
    make_file(tmp.path(), "occupied.txt");

    let recovered = recover_orphaned_temp_files(tmp.path());
    assert_eq!(recovered, 1, "only the valid+free temp restores");
    let names = listing(tmp.path());
    assert!(names.contains("restored.txt"));
    assert!(
        names.contains("occupied.txt"),
        "existing files are never clobbered"
    );
    assert!(
        names
            .iter()
            .any(|n| n.starts_with(TEMP_PREFIX) && n.ends_with("-occupied.txt")),
        "the occupied orphan stays for manual handling"
    );
    assert!(
        names.iter().any(|n| n.contains("not-a-uuid")),
        "invalid-uuid names are not touched"
    );
}

// ---- 8. mid-batch failures -------------------------------------------------

#[test]
fn vanished_source_errors_and_remainder_proceeds() {
    let tmp = TempDir::new().unwrap();
    let gone = tmp.path().join("gone.txt");
    let stays = make_file(tmp.path(), "stays.txt");
    let moves = vec![
        (gone.clone(), tmp.path().join("x-gone.txt")),
        (stays.clone(), tmp.path().join("x-stays.txt")),
    ];
    let outcome = perform_moves_hierarchical(&moves, true, &|| false, &mut |_| {});
    assert_eq!(outcome.succeeded.len(), 1);
    assert_eq!(outcome.errors.len(), 1);
    assert!(
        outcome.errors[0].starts_with("Couldn’t rename “gone.txt”:"),
        "{:?}",
        outcome.errors
    );
    assert!(tmp.path().join("x-stays.txt").is_file());
    assert_no_temps(tmp.path());
}

#[cfg(unix)]
#[test]
fn readonly_directory_errors_and_remainder_proceeds() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = TempDir::new().unwrap();
    let locked_dir = tmp.path().join("locked");
    let open_dir = tmp.path().join("open");
    fs::create_dir_all(&locked_dir).unwrap();
    fs::create_dir_all(&open_dir).unwrap();
    let trapped = make_file(&locked_dir, "trapped.txt");
    let free = make_file(&open_dir, "free.txt");
    fs::set_permissions(&locked_dir, fs::Permissions::from_mode(0o555)).unwrap();
    // Root ignores permission bits (some CI containers); skip there.
    if fs::write(locked_dir.join("probe"), b"x").is_ok() {
        fs::set_permissions(&locked_dir, fs::Permissions::from_mode(0o755)).unwrap();
        eprintln!("skipping: running with permissions that bypass 0o555");
        return;
    }

    let moves = vec![
        (trapped.clone(), locked_dir.join("renamed.txt")),
        (free.clone(), open_dir.join("renamed.txt")),
    ];
    let outcome = perform_moves_hierarchical(&moves, true, &|| false, &mut |_| {});
    fs::set_permissions(&locked_dir, fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(outcome.succeeded.len(), 1);
    assert_eq!(outcome.errors.len(), 1);
    assert!(outcome.errors[0].starts_with("Couldn’t rename “trapped.txt”:"));
    assert!(trapped.is_file(), "failed item rolls back / stays put");
    assert!(open_dir.join("renamed.txt").is_file());
    assert_no_temps(tmp.path());
}

// ---- 9. revert with missing + name-taken matches the simulation ------------

#[test]
fn revert_missing_and_name_taken_match_simulation() {
    let tmp = TempDir::new().unwrap();
    let a = make_file(tmp.path(), "a.txt");
    let b = make_file(tmp.path(), "b.txt");
    let mut state = state_with_files(&[&a, &b]);
    state.rules = vec![prefix_rule("x-")];
    let outcome = apply(&mut state);
    assert_eq!(outcome.renamed, 2);
    let snapshot_id = outcome.snapshot_id.unwrap();

    // Sabotage: delete one renamed file; occupy the other's original name.
    fs::remove_file(tmp.path().join("x-b.txt")).unwrap();
    make_file(tmp.path(), "a.txt");

    let simulated = compute_revert_preview(&state.history, snapshot_id, &|p: &Path| {
        p.symlink_metadata().is_ok()
    });
    let statuses: HashSet<(String, RevertStatus)> = simulated
        .entries
        .iter()
        .map(|e| (e.current_name.clone(), e.status))
        .collect();
    assert!(
        statuses.contains(&("x-a.txt".into(), RevertStatus::NameTaken)),
        "{statuses:?}"
    );
    assert!(
        statuses.contains(&("x-b.txt".into(), RevertStatus::Missing)),
        "{statuses:?}"
    );
    assert_eq!(simulated.restorable_rename_count, 0);
    assert_eq!(simulated.name_taken_count, 1);

    // Execute and assert the simulation predicted the disk outcome.
    let revert = revert_through(&mut state, snapshot_id, &|| false, &mut |_, _| {});
    assert_eq!(revert.restored, 0, "nothing restorable");
    assert_eq!(revert.errors.len(), 2, "{:?}", revert.errors);
    let names = listing(tmp.path());
    assert!(names.contains("x-a.txt"), "name-taken file kept as is");
    assert!(names.contains("a.txt"), "foreign file untouched");
    assert!(!names.contains("b.txt"), "missing file stays missing");
    assert_no_temps(tmp.path());
}

// ---- 10. history bookkeeping -----------------------------------------------

#[test]
fn history_caps_at_fifty() {
    let mut state = CoreState::default();
    for i in 0..55 {
        nameshift_engine::record_snapshot(
            &mut state,
            format!("batch {i}"),
            &[(PathBuf::from("/a"), PathBuf::from("/b"))],
        );
    }
    assert_eq!(state.history.len(), 50);
    assert_eq!(state.history[0].summary, "batch 54", "newest first");
}

#[test]
fn revert_through_removes_reverted_snapshots_and_cancel_keeps_partial() {
    let tmp = TempDir::new().unwrap();
    let doc = make_file(tmp.path(), "doc.txt");
    let mut state = state_with_files(&[&doc]);
    // Keep-rules would deselect the file after each apply (§8.5 step 5);
    // this test re-applies to the same file three times.
    state.keep_rules_after_apply = false;

    state.rules = vec![prefix_rule("a-")];
    apply(&mut state);
    state.rules = vec![prefix_rule("b-")];
    apply(&mut state);
    state.rules = vec![prefix_rule("c-")];
    apply(&mut state);
    assert_eq!(state.history.len(), 3);
    assert!(tmp.path().join("c-b-a-doc.txt").is_file());

    // Reverting the middle snapshot also reverts the newer one above it.
    let middle = state.history[1].id;
    let revert = revert_through(&mut state, middle, &|| false, &mut |_, _| {});
    assert_eq!(revert.restored, 2, "{:?}", revert.errors);
    assert!(tmp.path().join("a-doc.txt").is_file());
    assert_eq!(state.history.len(), 1, "the oldest snapshot survives");
    assert_eq!(state.history[0].summary, "Prefix “a-”");

    // Cancel before anything happens → the snapshot stays.
    let oldest = state.history[0].id;
    let cancelled = revert_through(&mut state, oldest, &|| true, &mut |_, _| {});
    assert_eq!(cancelled.restored, 0);
    assert_eq!(
        state.history.len(),
        1,
        "cancelled revert keeps the snapshot"
    );
    assert_no_temps(tmp.path());
}

// ---- 13. platform-specific names -------------------------------------------

#[cfg(target_os = "macos")]
#[test]
fn nfc_nfd_rename_pair() {
    let tmp = TempDir::new().unwrap();
    let nfc = make_file(tmp.path(), "caf\u{E9}.txt");
    let mut state = state_with_files(&[&nfc]);
    state.files[0].override_name = Some("cafe\u{301}.txt".into());

    let outcome = apply(&mut state);
    assert_eq!(outcome.renamed, 1, "{:?}", outcome.errors);
    assert!(tmp
        .path()
        .join("cafe\u{301}.txt")
        .symlink_metadata()
        .is_ok());
    assert_no_temps(tmp.path());
}

#[cfg(windows)]
#[test]
fn long_path_rename() {
    // > 300 chars of nested directories, then rename a file inside (§16.1).
    let tmp = TempDir::new().unwrap();
    let mut deep = tmp.path().to_path_buf();
    for _ in 0..12 {
        deep = deep.join("a-fairly-long-directory-name");
    }
    fs::create_dir_all(&deep).expect("create deep tree");
    assert!(deep.to_string_lossy().len() > 300);
    let file = deep.join("long-path-file.txt");
    fs::write(nameshift_engine::platform::win_long_path(&file), b"x").expect("create deep file");

    let moves = vec![(file.clone(), deep.join("renamed.txt"))];
    let outcome = perform_moves_hierarchical(&moves, true, &|| false, &mut |_| {});
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert!(
        nameshift_engine::platform::win_long_path(&deep.join("renamed.txt"))
            .symlink_metadata()
            .is_ok()
    );
}

// ---- keep-rules deselect + override clearing (§8.5 steps 5–6) --------------

#[test]
fn keep_rules_deselects_applied_items_and_clears_overrides() {
    let tmp = TempDir::new().unwrap();
    let a = make_file(tmp.path(), "a.txt");
    let mut state = state_with_files(&[&a]);
    state.keep_rules_after_apply = true;
    state.files[0].override_name = Some("manual.txt".into());

    let outcome = apply(&mut state);
    assert_eq!(outcome.renamed, 1);
    assert!(
        !outcome.kept_rules && !outcome.cleared_rules,
        "no rules were involved"
    );
    assert!(!state.files[0].is_selected, "applied item deselected");
    assert_eq!(
        state.files[0].override_name, None,
        "override cleared for the applied mode"
    );
    assert_eq!(state.files[0].name(), "manual.txt");
}
