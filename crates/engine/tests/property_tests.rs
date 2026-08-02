//! Property & fuzz tests (§18.3). The round-trip law is the app's core
//! safety promise: a clean preview applies fully, and reverting restores a
//! byte-identical tree.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use nameshift_engine::{
    apply_plan, apply_rule, build_plan, compute_preview, compute_revert_preview, expand_tokens,
    format_date, format_size, host_profile, intrinsic_problem, revert_through, split_name,
    trimmed_name, CaseStyle, CoreState, DirectoryNameCache, FileItem, FileSortKey,
    FsDirectoryLister, NoMetadata, NumberPosition, RenameRule, RevertStatus, RuleKind,
    LINUX_PROFILE, MACOS_PROFILE, TEMP_PREFIX, WINDOWS_PROFILE,
};
use proptest::prelude::*;
use tempfile::TempDir;

static NO_METADATA: NoMetadata = NoMetadata;

fn listing(dir: &Path) -> HashSet<String> {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn assert_no_temps(dir: &Path) {
    for name in listing(dir) {
        prop_assert_no_temp(&name);
    }
}

fn prop_assert_no_temp(name: &str) {
    assert!(!name.starts_with(TEMP_PREFIX), "stranded temp: {name}");
}

// ---- generators ------------------------------------------------------------

fn file_names() -> impl Strategy<Value = Vec<String>> {
    // Lowercase ASCII stems are fold-stable on every filesystem the CI runs.
    proptest::collection::hash_set("[a-z]{1,8}", 2..7).prop_map(|stems| {
        stems
            .into_iter()
            .enumerate()
            .map(|(i, stem)| {
                if i % 3 == 0 {
                    format!("{stem}{i}")
                } else {
                    format!("{stem}{i}.txt")
                }
            })
            .collect()
    })
}

fn arbitrary_rule() -> impl Strategy<Value = RenameRule> {
    prop_oneof![
        "[a-z0-9_-]{1,4}".prop_map(|text| {
            let mut r = RenameRule::new(RuleKind::AddPrefix);
            r.text = text;
            r
        }),
        "[a-z0-9_-]{1,4}".prop_map(|text| {
            let mut r = RenameRule::new(RuleKind::AddSuffix);
            r.text = text;
            r
        }),
        ("[a-z]", "[a-z0-9_]{0,3}").prop_map(|(needle, replacement)| {
            let mut r = RenameRule::new(RuleKind::ReplaceText);
            r.text = needle;
            r.replacement = replacement;
            r
        }),
        "[a-z]{1,2}".prop_map(|needle| {
            let mut r = RenameRule::new(RuleKind::RemoveText);
            r.text = needle;
            r
        }),
        proptest::sample::select(vec![
            CaseStyle::Lowercase,
            CaseStyle::Uppercase,
            CaseStyle::TitleCase
        ])
        .prop_map(|style| {
            let mut r = RenameRule::new(RuleKind::ChangeCase);
            r.case_style = style;
            r
        }),
        (
            proptest::sample::select(vec![
                NumberPosition::Before,
                NumberPosition::After,
                NumberPosition::ReplaceName
            ]),
            0i64..50,
            1i64..4
        )
            .prop_map(|(position, start, padding)| {
                let mut r = RenameRule::new(RuleKind::NumberSequentially);
                r.number_position = position;
                r.number_start = start;
                r.number_padding = padding;
                r.text = "-".into();
                r
            }),
        Just({
            let mut r = RenameRule::new(RuleKind::Template);
            r.text = "{name}-{n:2}".into();
            r
        }),
        Just(RenameRule::new(RuleKind::Sanitize)),
    ]
}

fn rule_stack() -> impl Strategy<Value = Vec<RenameRule>> {
    proptest::collection::vec(arbitrary_rule(), 1..4)
}

fn build_state(dir: &Path, names: &[String], rules: Vec<RenameRule>) -> CoreState {
    let files = names
        .iter()
        .map(|name| {
            let path = dir.join(name);
            fs::write(&path, name.as_bytes()).expect("fixture");
            FileItem::new(&path, false)
        })
        .collect();
    CoreState {
        files,
        rules,
        sort_key: FileSortKey::Name,
        keep_rules_after_apply: false,
        ..CoreState::default()
    }
}

// ---- round-trip law --------------------------------------------------------

proptest! {
    /// ∀ tree + rule stack: a problem-free preview applies fully, and the
    /// revert restores the original names byte-identically.
    #[test]
    fn round_trip_law(names in file_names(), rules in rule_stack()) {
        let tmp = TempDir::new().unwrap();
        let mut state = build_state(tmp.path(), &names, rules);
        let original: HashSet<String> = listing(tmp.path());

        let mut disk = DirectoryNameCache::new(Box::new(FsDirectoryLister));
        let preview = compute_preview(&state, host_profile(), &mut disk, &NO_METADATA);
        prop_assume!(preview.counts.conflict_count == 0);
        if preview.counts.change_count == 0 {
            return Ok(());
        }
        let promised: HashSet<String> =
            preview.entries.iter().map(|e| e.new_name.clone()).collect();

        let plan = build_plan(&state, &preview).expect("gating held");
        let outcome = apply_plan(&mut state, &plan, &|| false, &mut |_| {});
        prop_assert!(outcome.errors.is_empty(), "apply errors: {:?}", outcome.errors);
        prop_assert_eq!(outcome.renamed, preview.counts.change_count);
        prop_assert_eq!(listing(tmp.path()), promised);

        let snapshot_id = outcome.snapshot_id.expect("changes were recorded");
        let revert = revert_through(&mut state, snapshot_id, &|| false, &mut |_, _| {});
        prop_assert!(revert.errors.is_empty(), "revert errors: {:?}", revert.errors);
        prop_assert_eq!(listing(tmp.path()), original);
        assert_no_temps(tmp.path());
    }
}

// ---- no-temp law -----------------------------------------------------------

proptest! {
    /// After any apply / cancel-at-a-random-point / revert sequence, no
    /// `.fne-tmp-*` file remains.
    #[test]
    fn no_temp_law(names in file_names(), rules in rule_stack(), cancel_after in 0usize..25) {
        let tmp = TempDir::new().unwrap();
        let mut state = build_state(tmp.path(), &names, rules);

        let mut disk = DirectoryNameCache::new(Box::new(FsDirectoryLister));
        let preview = compute_preview(&state, host_profile(), &mut disk, &NO_METADATA);
        prop_assume!(preview.counts.conflict_count == 0);
        if preview.counts.change_count == 0 {
            return Ok(());
        }
        let plan = build_plan(&state, &preview).expect("gating held");
        let calls = AtomicUsize::new(0);
        let outcome = apply_plan(
            &mut state,
            &plan,
            &|| calls.fetch_add(1, Ordering::SeqCst) >= cancel_after,
            &mut |_| {},
        );
        assert_no_temps(tmp.path());

        if let Some(snapshot_id) = outcome.snapshot_id {
            revert_through(&mut state, snapshot_id, &|| false, &mut |_, _| {});
            assert_no_temps(tmp.path());
        }
    }
}

// ---- simulation law --------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Sabotage {
    Keep,
    DeleteRenamed,
    OccupyOriginal,
}

proptest! {
    /// Revert-preview statuses equal the actual revert outcomes, with
    /// injected missing/taken conditions.
    #[test]
    fn simulation_law(
        names in file_names(),
        sabotage in proptest::collection::vec(
            proptest::sample::select(vec![Sabotage::Keep, Sabotage::DeleteRenamed, Sabotage::OccupyOriginal]),
            2..7,
        ),
    ) {
        let tmp = TempDir::new().unwrap();
        let mut prefix = RenameRule::new(RuleKind::AddPrefix);
        prefix.text = "x-".into();
        let mut state = build_state(tmp.path(), &names, vec![prefix]);

        let mut disk = DirectoryNameCache::new(Box::new(FsDirectoryLister));
        let preview = compute_preview(&state, host_profile(), &mut disk, &NO_METADATA);
        prop_assume!(preview.counts.can_apply);
        let plan = build_plan(&state, &preview).expect("gating held");
        let outcome = apply_plan(&mut state, &plan, &|| false, &mut |_| {});
        prop_assert!(outcome.errors.is_empty());
        let snapshot_id = outcome.snapshot_id.expect("recorded");

        // Sabotage renamed files per the generated actions.
        let entries = state.history[0].entries.clone();
        for (entry, action) in entries.iter().zip(sabotage.iter().cycle()) {
            match action {
                Sabotage::Keep => {}
                Sabotage::DeleteRenamed => fs::remove_file(&entry.to).unwrap(),
                Sabotage::OccupyOriginal => fs::write(&entry.from, b"foreign").unwrap(),
            }
        }

        let simulated = compute_revert_preview(&state.history, snapshot_id, &|p: &Path| {
            p.symlink_metadata().is_ok()
        });
        let revert = revert_through(&mut state, snapshot_id, &|| false, &mut |_, _| {});

        prop_assert_eq!(revert.restored, simulated.restorable_rename_count,
            "simulation predicted the restore count");
        for entry in &simulated.entries {
            let current = entry.directory_path.join(&entry.current_name);
            let restored = entry.directory_path.join(&entry.restored_name);
            match entry.status {
                RevertStatus::Ok => {
                    prop_assert!(restored.symlink_metadata().is_ok(),
                        "predicted Ok: {restored:?} restored");
                    prop_assert!(current.symlink_metadata().is_err(),
                        "predicted Ok: {current:?} vacated");
                }
                RevertStatus::Missing => {
                    prop_assert!(current.symlink_metadata().is_err(),
                        "predicted Missing: {current:?} absent");
                }
                RevertStatus::NameTaken => {
                    prop_assert!(current.symlink_metadata().is_ok(),
                        "predicted NameTaken: {current:?} kept as is");
                }
            }
        }
        assert_no_temps(tmp.path());
    }
}

// ---- fuzz: no panics, ever -------------------------------------------------

proptest! {
    #[test]
    fn fuzz_rules_never_panic(name in ".{0,40}", text in ".{0,20}", replacement in ".{0,12}") {
        for kind in [
            RuleKind::RemoveText,
            RuleKind::ReplaceText,
            RuleKind::RegexReplace,
            RuleKind::AddPrefix,
            RuleKind::AddSuffix,
            RuleKind::ChangeCase,
            RuleKind::ChangeExtension,
            RuleKind::Sanitize,
            RuleKind::NumberSequentially,
            RuleKind::Template,
        ] {
            let mut rule = RenameRule::new(kind);
            rule.text = text.clone();
            rule.replacement = replacement.clone();
            rule.case_sensitive = false;
            let _ = apply_rule(&rule, &name, None, false);
            let _ = apply_rule(&rule, &name, None, true);
        }
        let _ = expand_tokens(&text, &name, "txt", None);
        let _ = split_name(&name);
        let _ = trimmed_name(&name, false);
        for profile in [&MACOS_PROFILE, &WINDOWS_PROFILE, &LINUX_PROFILE] {
            let _ = intrinsic_problem(profile, &name);
        }
    }

    #[test]
    fn fuzz_formatters_never_panic(bytes in any::<u64>(), pattern in ".{0,24}") {
        let _ = format_size(bytes);
        let date = chrono::Local::now();
        let _ = format_date(&date, &pattern);
    }
}

#[cfg(unix)]
proptest! {
    #[test]
    fn fuzz_non_utf8_paths_never_panic(bytes in proptest::collection::vec(any::<u8>(), 1..24)) {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::path::PathBuf;
        // Forbid separators/NUL so the bytes stay one path component.
        let cleaned: Vec<u8> =
            bytes.into_iter().map(|b| if b == b'/' || b == 0 { b'_' } else { b }).collect();
        let path = PathBuf::from("/tmp/fuzz").join(OsStr::from_bytes(&cleaned));
        let item = FileItem::new(&path, false);
        let _ = item.name();
        let _ = item.directory();
        let _ = nameshift_engine::item::standardized(&path);
    }
}
