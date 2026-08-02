//! Store tests (§18.1): fixture decoding, legacy migration, corruption
//! tolerance, atomic writes, and full round-trips.

use std::path::PathBuf;

use nameshift_engine::{CaseStyle, FileSortKey, ListMode, RuleKind, RulePreset, Snapshot};
use nameshift_store::{
    atomic_write_json, load_history, load_presets, load_session, save_history, save_presets,
    save_session, SessionFileEntry, SessionState, StorePaths,
};
use tempfile::TempDir;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/legacy")
        .join(name);
    std::fs::read_to_string(path).expect("fixture readable")
}

fn store_in(tmp: &TempDir) -> StorePaths {
    StorePaths::with_base(tmp.path().join("NameShift"))
}

// ---- §B fixtures decode byte-exactly into expected structs ----------------

#[test]
fn legacy_presets_fixture_decodes() {
    let presets: Vec<RulePreset> = serde_json::from_str(&fixture("presets.json")).expect("decodes");
    assert_eq!(presets.len(), 1);
    let preset = &presets[0];
    assert_eq!(
        preset.id.to_string().to_uppercase(),
        "7B4C2A10-53E5-4D2A-9C6F-2E8B1F0A9D11"
    );
    assert_eq!(preset.name, "Photo import");
    assert!(preset.trims_whitespace);
    assert_eq!(preset.rules.len(), 1);
    let rule = &preset.rules[0];
    assert_eq!(rule.kind, RuleKind::Template);
    assert_eq!(rule.text, "{created} {name} {n:3}");
    assert!(rule.is_enabled);
    assert_eq!(rule.number_padding, 3);
    assert!(
        !rule.restart_per_folder,
        "field absent in legacy → default false"
    );

    // Re-encoding preserves semantics (decode → encode → decode → equal).
    let encoded = serde_json::to_string_pretty(&presets).unwrap();
    let again: Vec<RulePreset> = serde_json::from_str(&encoded).unwrap();
    assert_eq!(presets, again);
    assert!(
        encoded.contains("7B4C2A10-53E5-4D2A-9C6F-2E8B1F0A9D11"),
        "uppercase uuid on write"
    );
}

#[test]
fn legacy_presets_minimal_rule_gets_all_defaults_and_ignores_unknown_fields() {
    let presets: Vec<RulePreset> =
        serde_json::from_str(&fixture("presets-minimal.json")).expect("decodes");
    let rule = &presets[0].rules[0];
    assert_eq!(rule.kind, RuleKind::AddPrefix);
    assert_eq!(rule.text, "x-");
    assert!(rule.is_enabled);
    assert!(rule.case_sensitive);
    assert!(!rule.includes_extension);
    assert_eq!(rule.case_style, CaseStyle::Lowercase);
    assert_eq!(rule.number_start, 1);
    assert_eq!(rule.number_padding, 3);
    assert!(!rule.restart_per_folder);
    assert!(
        !presets[0].trims_whitespace,
        "missing trimsWhitespace defaults false"
    );
}

#[test]
fn legacy_history_fixture_decodes() {
    let history: Vec<Snapshot> = serde_json::from_str(&fixture("history.json")).expect("decodes");
    assert_eq!(history.len(), 1);
    let snapshot = &history[0];
    assert_eq!(snapshot.summary, "Folders · Suffix “ 2026” · Manual edits");
    assert_eq!(snapshot.entries.len(), 2);
    // The second entry lives beneath the first's renamed folder (§8.7
    // revert-order exercise).
    assert!(snapshot.entries[1]
        .from
        .starts_with(&snapshot.entries[0].to));
    assert_eq!(snapshot.date.to_rfc3339(), "2026-05-11T09:30:00+00:00");
}

#[test]
fn legacy_session_fixture_decodes_every_field() {
    let session: SessionState = serde_json::from_str(&fixture("session.json")).expect("decodes");
    assert!(session.trims_whitespace);
    assert!(session.auto_resolves_conflicts);
    assert!(session.keep_rules_after_apply);
    assert!(session.include_subfolders);
    assert_eq!(session.resolved_sort(), (FileSortKey::Name, false));
    assert_eq!(session.resolved_list_mode(), ListMode::Files);
    assert_eq!(session.files.len(), 2);
    assert_eq!(
        session.files[0].override_name.as_deref(),
        Some("cover art.jpg")
    );
    assert!(!session.files[0].is_from_folder);
    assert!(session.files[1].is_from_folder);
    assert!(!session.files[1].is_selected);
    assert_eq!(
        session.watched_folder_paths,
        vec![PathBuf::from("/Users/demo/Photos")]
    );
    assert_eq!(session.watched_folder_subfolders, Some(vec![None]));
    assert_eq!(
        session.excluded_paths,
        vec![PathBuf::from("/Users/demo/Photos/skip.jpg")]
    );
    assert_eq!(session.rules.len(), 1);
    assert_eq!(session.rules[0].kind, RuleKind::ChangeCase);
    assert_eq!(session.rules[0].case_style, CaseStyle::TitleCase);
    assert!(!session.rules[0].case_sensitive);
}

#[test]
fn legacy_session_pre_split_sort_migrates() {
    let session: SessionState =
        serde_json::from_str(&fixture("session-legacy-sort.json")).expect("decodes");
    assert_eq!(
        session.resolved_sort(),
        (FileSortKey::Name, false),
        "Name (Z–A) → (Name, desc)"
    );
    assert!(
        !session.keep_rules_after_apply,
        "missing key decodes false (§4.3)"
    );
    assert_eq!(
        session.watched_folder_subfolders, None,
        "absent in legacy files"
    );
}

#[test]
fn sort_order_raw_migration_table() {
    let cases = [
        ("Name (A–Z)", FileSortKey::Name, true),
        ("Name (Z–A)", FileSortKey::Name, false),
        ("Extension", FileSortKey::FileExtension, true),
        ("Folder", FileSortKey::Folder, true),
        ("Date Created", FileSortKey::DateCreated, true),
        ("Date Modified", FileSortKey::DateModified, true),
        ("Something Unknown", FileSortKey::OrderAdded, true),
    ];
    for (raw, key, ascending) in cases {
        let session = SessionState {
            sort_order_raw: Some(raw.to_string()),
            ..SessionState::default()
        };
        assert_eq!(
            session.resolved_sort(),
            (key, ascending),
            "migrating {raw:?}"
        );
    }
}

#[test]
fn split_sort_fields_win_over_legacy() {
    let mut session = SessionState {
        sort_order_raw: Some("Name (Z–A)".to_string()),
        ..SessionState::default()
    };
    session.set_sort(FileSortKey::Folder, true);
    assert_eq!(session.resolved_sort(), (FileSortKey::Folder, true));
    let encoded = serde_json::to_string(&session).unwrap();
    assert!(
        !encoded.contains("sortOrderRaw"),
        "legacy field is never written: {encoded}"
    );
    assert!(encoded.contains("\"sortKeyRaw\":\"Folder\""));
}

// ---- corruption tolerance & atomic writes ---------------------------------

#[test]
fn corrupt_files_fall_back_without_panic() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    std::fs::create_dir_all(&paths.base).unwrap();
    std::fs::write(paths.history_file(), b"{ not json").unwrap();
    std::fs::write(paths.presets_file(), b"[[[[").unwrap();
    std::fs::write(paths.session_file(), b"").unwrap();
    assert!(load_history(&paths).is_empty());
    assert!(load_presets(&paths).is_empty());
    assert!(load_session(&paths).is_none());
}

#[test]
fn missing_files_load_as_defaults() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    assert!(load_history(&paths).is_empty());
    assert!(load_presets(&paths).is_empty());
    assert!(
        load_session(&paths).is_none(),
        "fresh install: caller applies keep-rules-on default"
    );
}

#[test]
fn atomic_write_leaves_no_tmp_and_replaces_content() {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path().join("deep/dir/data.json");
    atomic_write_json(&target, &vec![1, 2, 3], false).unwrap();
    atomic_write_json(&target, &vec![4, 5], false).unwrap();
    let names: Vec<String> = std::fs::read_dir(target.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["data.json"], "no .tmp left behind");
    let content: Vec<i32> = serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
    assert_eq!(content, vec![4, 5]);
}

// ---- round-trip every field ------------------------------------------------

#[test]
fn session_round_trips_every_field() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let mut session = SessionState {
        rules: vec![nameshift_engine::RenameRule::new(RuleKind::AddSuffix)],
        trims_whitespace: true,
        auto_resolves_conflicts: true,
        keep_rules_after_apply: true,
        include_subfolders: true,
        files: vec![
            SessionFileEntry {
                path: PathBuf::from("/a/direct.txt"),
                is_selected: true,
                override_name: Some("renamed.txt".into()),
                is_from_folder: false,
                bookmark: None,
            },
            SessionFileEntry {
                path: PathBuf::from("/w/found.txt"),
                is_selected: false,
                override_name: None,
                is_from_folder: true,
                bookmark: Some("Ym9va21hcms=".into()),
            },
        ],
        watched_folder_paths: vec![PathBuf::from("/w")],
        watched_folder_bookmarks: Some(vec![Some("cm9vdA==".into())]),
        watched_folder_subfolders: Some(vec![Some(false)]),
        excluded_paths: vec![PathBuf::from("/w/skip.txt")],
        ..SessionState::default()
    };
    session.set_sort(FileSortKey::DateModified, false);
    session.set_list_mode(ListMode::Folders);

    save_session(&paths, &session).unwrap();
    let loaded = load_session(&paths).expect("session loads");
    assert_eq!(loaded, session);
    assert_eq!(loaded.resolved_sort(), (FileSortKey::DateModified, false));
    assert_eq!(loaded.resolved_list_mode(), ListMode::Folders);

    // overrideName is omitted (not null) when absent — legacy shape.
    let raw = std::fs::read_to_string(paths.session_file()).unwrap();
    assert!(!raw.contains("\"overrideName\":null"), "{raw}");
}

#[test]
fn history_and_presets_round_trip() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let history: Vec<Snapshot> = serde_json::from_str(&fixture("history.json")).unwrap();
    save_history(&paths, &history).unwrap();
    assert_eq!(load_history(&paths), history);

    let presets: Vec<RulePreset> = serde_json::from_str(&fixture("presets.json")).unwrap();
    save_presets(&paths, &presets).unwrap();
    assert_eq!(load_presets(&paths), presets);
}

// ---- macOS legacy directory migration --------------------------------------

#[cfg(target_os = "macos")]
#[test]
fn legacy_data_directory_migrates_once() {
    let tmp = TempDir::new().unwrap();
    let legacy = tmp.path().join("File Name Editor");
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(legacy.join("history.json"), b"[]").unwrap();

    let paths = StorePaths::resolve_in(tmp.path());
    assert_eq!(paths.base, tmp.path().join("NameShift"));
    assert!(
        paths.base.join("history.json").is_file(),
        "contents carried over"
    );
    assert!(!legacy.exists(), "old directory renamed away");

    // Second resolve is a no-op.
    let again = StorePaths::resolve_in(tmp.path());
    assert_eq!(again.base, paths.base);
}
