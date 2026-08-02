//! Watcher tests (§18.2.11): debounced rescans, reconciliation preserving
//! id/selection/override, teardown on drop, temp/hidden skipping, bundle
//! pruning.

use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nameshift_engine::{CoreState, FileItem};
use nameshift_watcher::{reconcile_folder, scan_folder, FolderWatcher};
use tempfile::TempDir;
use uuid::Uuid;

/// Wait (bounded) for the counter to reach at least `expected`.
fn wait_for(counter: &AtomicUsize, expected: usize) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if counter.load(Ordering::SeqCst) >= expected {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn events_fire_debounced_and_stop_after_drop() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("watched");
    fs::create_dir(&root).unwrap();
    let fired = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&fired);
    let watcher = FolderWatcher::watch(&root, move || {
        counter.fetch_add(1, Ordering::SeqCst);
    })
    .expect("watcher arms");

    fs::write(root.join("new.txt"), b"x").unwrap();
    assert!(wait_for(&fired, 1), "creation triggers a debounced rescan");

    // Deallocation invariant (§9): dropping the handle tears the OS watcher
    // down; later events must not fire.
    drop(watcher);
    let count_after_drop = fired.load(Ordering::SeqCst);
    fs::write(root.join("after-drop.txt"), b"x").unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    assert_eq!(
        fired.load(Ordering::SeqCst),
        count_after_drop,
        "no watcher survives removal"
    );
}

#[test]
fn scan_lists_files_and_directories_but_never_the_root() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join("a.txt"), b"x").unwrap();
    fs::write(root.join("sub/deep.txt"), b"x").unwrap();

    let shallow = scan_folder(&root, false);
    let names: Vec<String> = shallow
        .iter()
        .map(|e| e.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        vec!["a.txt", "sub"],
        "immediate children only; root absent"
    );
    assert!(
        shallow.iter().any(|e| e.is_directory),
        "subfolders are Folders-mode targets"
    );

    let deep = scan_folder(&root, true);
    assert!(
        deep.iter().any(|e| e.path.ends_with("sub/deep.txt")),
        "recursive walk finds nested files: {deep:?}"
    );
}

#[test]
fn scan_skips_hidden_and_temp_entries() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("visible.txt"), b"x").unwrap();
    fs::write(root.join(".hidden"), b"x").unwrap();
    fs::write(
        root.join(".fne-tmp-7B4C2A10-53E5-4D2A-9C6F-2E8B1F0A9D11-caught.txt"),
        b"x",
    )
    .unwrap();

    let entries = scan_folder(&root, false);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].path.ends_with("visible.txt"));
}

#[cfg(target_os = "macos")]
#[test]
fn scan_prunes_bundle_directories() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    let bundle = root.join("Demo.app");
    fs::create_dir_all(bundle.join("Contents")).unwrap();
    fs::write(bundle.join("Contents/Info.plist"), b"x").unwrap();

    let entries = scan_folder(&root, true);
    assert!(
        entries
            .iter()
            .any(|e| e.path.ends_with("Demo.app") && e.is_directory),
        "the bundle itself is a Folders-mode target"
    );
    assert!(
        !entries
            .iter()
            .any(|e| e.path.to_string_lossy().contains("Contents")),
        "never descend into bundles: {entries:?}"
    );
}

#[test]
fn reconcile_preserves_identity_and_skips_excluded() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("keep.txt"), b"x").unwrap();
    fs::write(root.join("gone.txt"), b"x").unwrap();

    let folder_id = Uuid::new_v4();
    let mut state = CoreState::default();
    let scanned = scan_folder(&root, false);
    assert!(reconcile_folder(&mut state, folder_id, &scanned));
    assert_eq!(state.files.len(), 2);

    // The user tweaks one item; a rescan must not disturb it.
    let keep_index = state
        .files
        .iter()
        .position(|f| f.name() == "keep.txt")
        .unwrap();
    let keep_id = state.files[keep_index].id;
    state.files[keep_index].is_selected = false;
    state.files[keep_index].override_name = Some("custom.txt".into());

    fs::remove_file(root.join("gone.txt")).unwrap();
    fs::write(root.join("added.txt"), b"x").unwrap();
    state
        .excluded_paths
        .insert(nameshift_engine::item::standardized(
            &root.join("banned.txt"),
        ));
    fs::write(root.join("banned.txt"), b"x").unwrap();

    let scanned = scan_folder(&root, false);
    assert!(reconcile_folder(&mut state, folder_id, &scanned));
    let names: Vec<String> = state.files.iter().map(|f| f.name()).collect();
    assert!(names.contains(&"keep.txt".to_string()));
    assert!(names.contains(&"added.txt".to_string()));
    assert!(
        !names.contains(&"gone.txt".to_string()),
        "vanished items drop"
    );
    assert!(
        !names.contains(&"banned.txt".to_string()),
        "excluded paths skipped"
    );

    let kept = state.files.iter().find(|f| f.name() == "keep.txt").unwrap();
    assert_eq!(kept.id, keep_id, "existing items keep their id");
    assert!(!kept.is_selected, "…and their selection");
    assert_eq!(
        kept.override_name.as_deref(),
        Some("custom.txt"),
        "…and their override"
    );
    assert_eq!(kept.folder_id, Some(folder_id));
}

#[test]
fn reconcile_does_not_duplicate_directly_imported_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("direct.txt"), b"x").unwrap();

    let mut state = CoreState {
        files: vec![FileItem::new(&root.join("direct.txt"), false)],
        ..CoreState::default()
    };
    let folder_id = Uuid::new_v4();
    let scanned = scan_folder(&root, false);
    reconcile_folder(&mut state, folder_id, &scanned);
    assert_eq!(state.files.len(), 1, "already-tracked path is not re-added");
    assert_eq!(
        state.files[0].folder_id, None,
        "the direct import keeps its identity"
    );
}

#[test]
fn watcher_fires_for_rename_and_delete_under_root() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("watched");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("a.txt"), b"x").unwrap();

    let fired = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&fired);
    let _watcher = FolderWatcher::watch(&root, move || {
        counter.fetch_add(1, Ordering::SeqCst);
    })
    .expect("watcher arms");

    fs::rename(root.join("a.txt"), root.join("b.txt")).unwrap();
    assert!(wait_for(&fired, 1), "rename triggers a rescan");
    let seen = fired.load(Ordering::SeqCst);
    fs::remove_file(root.join("b.txt")).unwrap();
    assert!(wait_for(&fired, seen + 1), "delete triggers a rescan");
}
