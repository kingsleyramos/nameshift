//! Preview pipeline tests (§18.1): ordering, numbering, overrides, trim,
//! auto-resolve, every §7.6 problem, counts, rule impact, restart-per-folder.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nameshift_engine::{
    compute_preview, rule_derived_name, CoreState, DirectoryLister, DirectoryNameCache, FileItem,
    FileSortKey, ListMode, MetadataSource, NoMetadata, NumberPosition, Preview, Problem,
    RenameRule, RuleKind, MACOS_PROFILE, WINDOWS_PROFILE,
};
use uuid::Uuid;

static NO_METADATA: NoMetadata = NoMetadata;

/// Lists directories from a fixed map — no filesystem involved.
struct MapLister(HashMap<PathBuf, Vec<String>>);

impl DirectoryLister for MapLister {
    fn list(&self, directory: &Path) -> Vec<String> {
        self.0.get(directory).cloned().unwrap_or_default()
    }
}

fn empty_disk() -> DirectoryNameCache {
    DirectoryNameCache::new(Box::new(MapLister(HashMap::new())))
}

fn disk_with(directory: &str, names: &[&str]) -> DirectoryNameCache {
    let mut map = HashMap::new();
    map.insert(
        PathBuf::from(directory),
        names.iter().map(|n| n.to_string()).collect(),
    );
    DirectoryNameCache::new(Box::new(MapLister(map)))
}

fn file(path: &str) -> FileItem {
    FileItem::new(Path::new(path), false)
}

fn folder(path: &str) -> FileItem {
    FileItem::new(Path::new(path), true)
}

fn state(files: Vec<FileItem>, rules: Vec<RenameRule>) -> CoreState {
    CoreState {
        files,
        rules,
        ..CoreState::default()
    }
}

fn preview(state: &CoreState, disk: &mut DirectoryNameCache) -> Preview {
    compute_preview(state, &MACOS_PROFILE, disk, &NO_METADATA)
}

fn names(preview: &Preview) -> Vec<(String, String)> {
    preview
        .entries
        .iter()
        .map(|e| (e.current_name.clone(), e.new_name.clone()))
        .collect()
}

fn rule(kind: RuleKind) -> RenameRule {
    RenameRule::new(kind)
}

fn prefix_rule(text: &str) -> RenameRule {
    let mut r = rule(RuleKind::AddPrefix);
    r.text = text.into();
    r
}

// ---- ordering (§7.2) ------------------------------------------------------

#[test]
fn name_sort_is_natural_and_case_insensitive() {
    let mut s = state(
        vec![
            file("/d/file10.txt"),
            file("/d/File2.txt"),
            file("/d/apple.txt"),
        ],
        vec![],
    );
    s.sort_key = FileSortKey::Name;
    let p = preview(&s, &mut empty_disk());
    let order: Vec<&str> = p.entries.iter().map(|e| e.current_name.as_str()).collect();
    assert_eq!(order, ["apple.txt", "File2.txt", "file10.txt"]);
}

#[test]
fn descending_reverses_every_key() {
    let mut s = state(
        vec![file("/d/a.txt"), file("/d/b.txt"), file("/d/c.txt")],
        vec![],
    );
    s.sort_key = FileSortKey::Name;
    s.sort_ascending = false;
    let p = preview(&s, &mut empty_disk());
    let order: Vec<&str> = p.entries.iter().map(|e| e.current_name.as_str()).collect();
    assert_eq!(order, ["c.txt", "b.txt", "a.txt"]);
}

#[test]
fn extension_sort_ties_by_name() {
    let mut s = state(
        vec![
            file("/d/b.PNG"),
            file("/d/z.jpg"),
            file("/d/a.png"),
            file("/d/y.jpg"),
        ],
        vec![],
    );
    s.sort_key = FileSortKey::FileExtension;
    let p = preview(&s, &mut empty_disk());
    let order: Vec<&str> = p.entries.iter().map(|e| e.current_name.as_str()).collect();
    assert_eq!(order, ["y.jpg", "z.jpg", "a.png", "b.PNG"]);
}

#[test]
fn folder_sort_ties_by_name() {
    let mut s = state(
        vec![file("/b/x.txt"), file("/a/z.txt"), file("/a/y.txt")],
        vec![],
    );
    s.sort_key = FileSortKey::Folder;
    let p = preview(&s, &mut empty_disk());
    let order: Vec<&str> = p.entries.iter().map(|e| e.current_name.as_str()).collect();
    assert_eq!(order, ["y.txt", "z.txt", "x.txt"]);
}

#[test]
fn order_added_is_insertion_order() {
    let s = state(
        vec![file("/d/c.txt"), file("/d/a.txt"), file("/d/b.txt")],
        vec![],
    );
    let p = preview(&s, &mut empty_disk());
    let order: Vec<&str> = p.entries.iter().map(|e| e.current_name.as_str()).collect();
    assert_eq!(order, ["c.txt", "a.txt", "b.txt"]);
}

/// Counts date reads to pin the read-once-up-front rule (§7.2).
#[derive(Default)]
struct DateCountingMetadata {
    reads: AtomicUsize,
}

impl MetadataSource for DateCountingMetadata {
    fn created(&self, path: &Path) -> Option<SystemTime> {
        self.reads.fetch_add(1, AtomicOrdering::SeqCst);
        // Deterministic distinct dates derived from the name.
        let seed = path.file_name()?.to_string_lossy().len() as u64;
        Some(UNIX_EPOCH + Duration::from_secs(seed * 1000))
    }
    fn modified(&self, _: &Path) -> Option<SystemTime> {
        None
    }
    fn size(&self, _: &Path) -> Option<u64> {
        None
    }
    fn kind(&self, _: &Path) -> Option<String> {
        None
    }
    fn attribute(&self, _: &Path, _: &str) -> Option<String> {
        None
    }
}

#[test]
fn date_sort_reads_each_date_once() {
    let mut s = state(
        vec![
            file("/d/loooong.txt"),
            file("/d/ab.txt"),
            file("/d/mid.txt"),
        ],
        vec![],
    );
    s.sort_key = FileSortKey::DateCreated;
    let metadata = DateCountingMetadata::default();
    let p = compute_preview(&s, &MACOS_PROFILE, &mut empty_disk(), &metadata);
    // One read per item — never one per comparison.
    assert_eq!(metadata.reads.load(AtomicOrdering::SeqCst), 3);
    let order: Vec<&str> = p.entries.iter().map(|e| e.current_name.as_str()).collect();
    assert_eq!(order, ["ab.txt", "mid.txt", "loooong.txt"]);
}

#[test]
fn missing_dates_sort_as_distant_past() {
    let mut s = state(vec![file("/d/b.txt"), file("/d/a.txt")], vec![]);
    s.sort_key = FileSortKey::DateModified;
    // NoMetadata returns None for every date → ties broken by name.
    let p = preview(&s, &mut empty_disk());
    let order: Vec<&str> = p.entries.iter().map(|e| e.current_name.as_str()).collect();
    assert_eq!(order, ["a.txt", "b.txt"]);
}

// ---- numbering (§6.3, §7.3) ----------------------------------------------

fn numbering_rule() -> RenameRule {
    let mut r = rule(RuleKind::NumberSequentially);
    r.number_position = NumberPosition::Before;
    r.text = "-".into();
    r.number_padding = 2;
    r
}

#[test]
fn numbering_follows_view_order_and_skips_deselected() {
    let mut s = state(
        vec![
            file("/d/cherry.txt"),
            file("/d/apple.txt"),
            file("/d/banana.txt"),
        ],
        vec![numbering_rule()],
    );
    s.sort_key = FileSortKey::Name;
    let banana = s
        .files
        .iter()
        .position(|f| f.name() == "banana.txt")
        .unwrap();
    s.files[banana].is_selected = false;
    let p = preview(&s, &mut empty_disk());
    let changed: Vec<&str> = p
        .entries
        .iter()
        .filter(|e| e.is_changed)
        .map(|e| e.new_name.as_str())
        .collect();
    assert_eq!(changed, ["01-apple.txt", "02-cherry.txt"]);
}

#[test]
fn descending_sort_reverses_numbering() {
    let mut s = state(
        vec![
            file("/d/apple.txt"),
            file("/d/banana.txt"),
            file("/d/cherry.txt"),
        ],
        vec![{
            let mut r = numbering_rule();
            r.number_padding = 3;
            r
        }],
    );
    s.sort_key = FileSortKey::Name;
    s.sort_ascending = false;
    let p = preview(&s, &mut empty_disk());
    let by_name: HashMap<String, String> = names(&p).into_iter().collect();
    assert_eq!(by_name["cherry.txt"], "001-cherry.txt");
    assert_eq!(by_name["banana.txt"], "002-banana.txt");
    assert_eq!(by_name["apple.txt"], "003-apple.txt");
}

#[test]
fn overrides_consume_numbers() {
    let mut s = state(
        vec![file("/d/a.txt"), file("/d/b.txt"), file("/d/c.txt")],
        vec![numbering_rule()],
    );
    s.sort_key = FileSortKey::Name;
    let b = s.files.iter().position(|f| f.name() == "b.txt").unwrap();
    s.files[b].override_name = Some("custom.txt".into());
    let p = preview(&s, &mut empty_disk());
    let by_name: HashMap<String, String> = names(&p).into_iter().collect();
    assert_eq!(by_name["a.txt"], "01-a.txt");
    assert_eq!(by_name["b.txt"], "custom.txt", "override wins");
    assert_eq!(
        by_name["c.txt"], "03-c.txt",
        "the override consumed number 2"
    );
}

#[test]
fn restart_per_folder_numbering() {
    let mut r = rule(RuleKind::NumberSequentially);
    r.number_position = NumberPosition::Before;
    r.text = "-".into();
    r.restart_per_folder = true;
    let mut s = state(
        vec![
            file("/one/a.txt"),
            file("/one/b.txt"),
            file("/one/c.txt"),
            file("/two/d.txt"),
            file("/two/e.txt"),
            file("/two/f.txt"),
        ],
        vec![r],
    );
    s.sort_key = FileSortKey::Folder;
    let p = preview(&s, &mut empty_disk());
    let new_names: Vec<&str> = p.entries.iter().map(|e| e.new_name.as_str()).collect();
    assert_eq!(
        new_names,
        [
            "001-a.txt",
            "002-b.txt",
            "003-c.txt",
            "001-d.txt",
            "002-e.txt",
            "003-f.txt"
        ]
    );
}

#[test]
fn restart_per_folder_skips_deselected_and_missing_field_decodes_false() {
    let mut r = rule(RuleKind::NumberSequentially);
    r.number_position = NumberPosition::Before;
    r.text = "-".into();
    r.restart_per_folder = true;
    let mut s = state(
        vec![file("/one/a.txt"), file("/one/b.txt"), file("/one/c.txt")],
        vec![r],
    );
    s.sort_key = FileSortKey::Name;
    s.files[1].is_selected = false;
    let p = preview(&s, &mut empty_disk());
    let by_name: HashMap<String, String> = names(&p).into_iter().collect();
    assert_eq!(by_name["a.txt"], "001-a.txt");
    assert_eq!(by_name["b.txt"], "b.txt");
    assert_eq!(
        by_name["c.txt"], "002-c.txt",
        "deselected files skip folder numbers too"
    );

    let decoded: RenameRule =
        serde_json::from_str(r#"{ "kind": "numberSequentially" }"#).expect("decodes");
    assert!(!decoded.restart_per_folder, "missing field decodes false");
}

// ---- overrides & trim ------------------------------------------------------

#[test]
fn override_wins_and_sets_flag() {
    let mut s = state(vec![file("/d/a.txt")], vec![prefix_rule("x-")]);
    s.files[0].override_name = Some("mine.txt".into());
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.entries[0].new_name, "mine.txt");
    assert!(p.entries[0].has_override);
    assert!(p.entries[0].is_changed);
}

#[test]
fn trim_pass_applies_after_rules() {
    let mut suffix = rule(RuleKind::AddSuffix);
    suffix.text = " ".into();
    let mut s = state(vec![file("/d/a.txt")], vec![suffix]);
    s.trims_whitespace = true;
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.entries[0].new_name, "a.txt");
    assert!(!p.entries[0].is_changed);
}

// ---- auto-resolve (§7.4) ---------------------------------------------------

#[test]
fn auto_resolve_keeper_wins() {
    // a.txt renames to b.txt while b.txt keeps its name → a gets “b 2.txt”.
    let mut replace = rule(RuleKind::ReplaceText);
    replace.text = "a".into();
    replace.replacement = "b".into();
    let mut s = state(vec![file("/d/a.txt"), file("/d/b.txt")], vec![replace]);
    s.sort_key = FileSortKey::Name;
    s.auto_resolves_conflicts = true;
    let mut disk = disk_with("/d", &["a.txt", "b.txt"]);
    let p = preview(&s, &mut disk);
    let by_name: HashMap<String, String> = names(&p).into_iter().collect();
    assert_eq!(by_name["a.txt"], "b 2.txt");
    assert_eq!(by_name["b.txt"], "b.txt");
    assert_eq!(p.counts.conflict_count, 0);
    assert!(p.counts.can_apply);
}

#[test]
fn auto_resolve_reuses_vacated_names() {
    // a→b while b→c: b's old name is vacated in the same batch, so a takes
    // b.txt with no suffix even though b.txt exists on disk.
    let mut s = state(vec![file("/d/a.txt"), file("/d/b.txt")], vec![]);
    s.sort_key = FileSortKey::Name;
    s.auto_resolves_conflicts = true;
    s.files[0].override_name = Some("b.txt".into());
    s.files[1].override_name = Some("c.txt".into());
    let mut disk = disk_with("/d", &["a.txt", "b.txt"]);
    let p = preview(&s, &mut disk);
    let by_name: HashMap<String, String> = names(&p).into_iter().collect();
    assert_eq!(
        by_name["a.txt"], "b.txt",
        "vacated name reused without suffix"
    );
    assert_eq!(by_name["b.txt"], "c.txt");
}

#[test]
fn auto_resolve_blocks_on_disk_names() {
    // Renaming onto an untracked on-disk name suffixes instead.
    let mut s = state(vec![file("/d/a.txt")], vec![]);
    s.auto_resolves_conflicts = true;
    s.files[0].override_name = Some("taken.txt".into());
    let mut disk = disk_with("/d", &["a.txt", "taken.txt"]);
    let p = preview(&s, &mut disk);
    assert_eq!(p.entries[0].new_name, "taken 2.txt");
}

#[test]
fn auto_resolve_folder_suffix_goes_at_the_end() {
    let mut s = CoreState {
        files: vec![folder("/d/Photos"), folder("/d/Shoots")],
        ..CoreState::default()
    };
    s.list_mode = ListMode::Folders;
    s.sort_key = FileSortKey::Name;
    s.auto_resolves_conflicts = true;
    s.files[1].override_name = Some("Photos".into());
    let mut disk = disk_with("/d", &["Photos", "Shoots"]);
    let p = compute_preview(&s, &MACOS_PROFILE, &mut disk, &NO_METADATA);
    let by_name: HashMap<String, String> = names(&p).into_iter().collect();
    assert_eq!(
        by_name["Shoots"], "Photos 2",
        "whole-name suffix for folders"
    );
}

#[test]
fn auto_resolve_gives_up_below_one_thousand() {
    let mut on_disk: Vec<String> = vec!["a.txt".into(), "x.txt".into()];
    on_disk.extend((2..1000).map(|i| format!("x {i}.txt")));
    let mut map = HashMap::new();
    map.insert(PathBuf::from("/d"), on_disk);
    let mut disk = DirectoryNameCache::new(Box::new(MapLister(map)));
    let mut s = state(vec![file("/d/a.txt")], vec![]);
    s.auto_resolves_conflicts = true;
    s.files[0].override_name = Some("x.txt".into());
    let p = preview(&s, &mut disk);
    // Every candidate through “x 999.txt” is taken; the still-colliding name
    // falls through to §7.6 flagging.
    assert_eq!(p.entries[0].problem, Some(Problem::ExistingFileCollision));
    assert!(!p.counts.can_apply);
}

// ---- §7.6 problems ---------------------------------------------------------

#[test]
fn empty_name_problem() {
    let mut remove = rule(RuleKind::RemoveText);
    remove.text = "abc".into();
    let s = state(vec![file("/d/abc")], vec![remove]);
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.entries[0].problem, Some(Problem::EmptyName));
}

#[test]
fn invalid_characters_problem() {
    let mut replace = rule(RuleKind::ReplaceText);
    replace.text = "a".into();
    replace.replacement = ":".into();
    let s = state(vec![file("/d/a.txt")], vec![replace]);
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.entries[0].problem, Some(Problem::InvalidCharacters));
}

#[test]
fn name_too_long_problem() {
    let mut template = rule(RuleKind::Template);
    template.text = "x".repeat(300);
    let s = state(vec![file("/d/a.txt")], vec![template]);
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.entries[0].problem, Some(Problem::NameTooLong));
}

#[test]
fn windows_reserved_and_trailing_dot_problems_via_injected_profile() {
    let mut template = rule(RuleKind::Template);
    template.text = "CON".into();
    let s = state(vec![file("/d/a.txt")], vec![template]);
    let p = compute_preview(&s, &WINDOWS_PROFILE, &mut empty_disk(), &NO_METADATA);
    assert_eq!(p.entries[0].problem, Some(Problem::ReservedName));

    let mut suffix = rule(RuleKind::AddSuffix);
    suffix.text = ".".into();
    let s = state(vec![file("/d/name")], vec![suffix]);
    let p = compute_preview(&s, &WINDOWS_PROFILE, &mut empty_disk(), &NO_METADATA);
    assert_eq!(p.entries[0].problem, Some(Problem::EndsWithDotOrSpace));
}

#[test]
fn duplicate_target_flags_both_sides() {
    // b.txt renames onto selected-but-unchanged a.txt: both rows flag.
    let mut s = state(vec![file("/d/a.txt"), file("/d/b.txt")], vec![]);
    s.sort_key = FileSortKey::Name;
    s.files[1].override_name = Some("a.txt".into());
    let p = preview(&s, &mut empty_disk());
    assert_eq!(
        p.entries[0].problem,
        Some(Problem::DuplicateTarget),
        "unchanged keeper flags too"
    );
    assert_eq!(p.entries[1].problem, Some(Problem::DuplicateTarget));
    assert_eq!(p.counts.conflict_count, 2);
}

#[test]
fn duplicate_target_respects_diff_key_case_folding() {
    let mut s = state(vec![file("/d/a.txt"), file("/d/b.txt")], vec![]);
    s.files[0].override_name = Some("Same.txt".into());
    s.files[1].override_name = Some("same.TXT".into());
    let p = preview(&s, &mut empty_disk());
    assert!(p
        .entries
        .iter()
        .all(|e| e.problem == Some(Problem::DuplicateTarget)));
}

#[test]
fn existing_file_collision_problem() {
    let mut s = state(vec![file("/d/a.txt")], vec![]);
    s.files[0].override_name = Some("taken.txt".into());
    let mut disk = disk_with("/d", &["a.txt", "taken.txt"]);
    let p = preview(&s, &mut disk);
    assert_eq!(p.entries[0].problem, Some(Problem::ExistingFileCollision));
}

#[test]
fn swap_vacancy_is_not_a_collision() {
    // a↔b: each target is being vacated by the other in the same batch —
    // two-phase makes swaps safe (§24 Q5).
    let mut s = state(vec![file("/d/a.txt"), file("/d/b.txt")], vec![]);
    s.files[0].override_name = Some("b.txt".into());
    s.files[1].override_name = Some("a.txt".into());
    let mut disk = disk_with("/d", &["a.txt", "b.txt"]);
    let p = preview(&s, &mut disk);
    assert!(
        p.entries.iter().all(|e| e.problem.is_none()),
        "{:?}",
        p.entries
    );
    assert!(p.counts.can_apply);
}

#[test]
fn case_only_rename_is_a_change_without_self_collision() {
    let mut s = state(vec![file("/d/readme.txt")], vec![]);
    s.files[0].override_name = Some("README.txt".into());
    let mut disk = disk_with("/d", &["readme.txt"]);
    let p = preview(&s, &mut disk);
    assert!(p.entries[0].is_changed);
    assert_eq!(
        p.entries[0].problem, None,
        "its target key equals its vacated source key"
    );
    assert!(p.counts.can_apply);
}

#[test]
fn unchanged_names_skip_intrinsic_checks() {
    // A stale “:” in an existing name must not be flagged (§7.6).
    let s = state(vec![file("/d/has:colon.txt")], vec![]);
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.entries[0].problem, None);
    assert!(!p.entries[0].is_changed);
}

#[test]
fn deselected_items_never_carry_problems() {
    let mut replace = rule(RuleKind::ReplaceText);
    replace.text = "a".into();
    replace.replacement = ":".into();
    let mut s = state(vec![file("/d/a.txt")], vec![replace]);
    s.files[0].is_selected = false;
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.entries[0].problem, None);
    assert_eq!(
        p.entries[0].new_name, "a.txt",
        "deselected items keep their names"
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_names_are_unrenamable_but_do_not_block() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let bad = PathBuf::from("/d").join(OsStr::from_bytes(b"bad\xFF.txt"));
    let mut items = vec![file("/d/good.txt")];
    items.push(FileItem::new(&bad, false));
    let s = state(items, vec![prefix_rule("x-")]);
    let p = preview(&s, &mut empty_disk());
    let bad_entry = p
        .entries
        .iter()
        .find(|e| e.current_name.contains('\u{FFFD}'))
        .unwrap();
    assert_eq!(bad_entry.problem, Some(Problem::UnrenamableName));
    assert!(!bad_entry.is_changed, "excluded from the plan");
    let good = p
        .entries
        .iter()
        .find(|e| e.current_name == "good.txt")
        .unwrap();
    assert_eq!(good.new_name, "x-good.txt");
    assert_eq!(
        p.counts.conflict_count, 0,
        "environmental badges don't block Apply"
    );
    assert!(p.counts.can_apply);
}

// ---- counts & gating (§7.7) ------------------------------------------------

#[test]
fn counts_and_gating() {
    let mut s = state(
        vec![file("/d/a.txt"), file("/d/b.txt"), file("/d/c.txt")],
        vec![prefix_rule("x-")],
    );
    s.files[2].is_selected = false;
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.counts.selected_count, 2);
    assert_eq!(p.counts.change_count, 2);
    assert_eq!(p.counts.conflict_count, 0);
    assert!(p.counts.can_apply);

    // No rules → nothing changes → can't apply.
    let s = state(vec![file("/d/a.txt")], vec![]);
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.counts.change_count, 0);
    assert!(!p.counts.can_apply);
}

// ---- rule impact (§7.9) ----------------------------------------------------

#[test]
fn rule_impact_hand_verified() {
    let mut remove = rule(RuleKind::RemoveText);
    remove.text = "IMG_".into();
    let prefix = prefix_rule("x-");
    let mut change_ext = rule(RuleKind::ChangeExtension);
    change_ext.text = "jpg".into();
    let s = state(
        vec![
            file("/d/IMG_1.jpg"),
            file("/d/IMG_2.png"),
            file("/d/other.jpg"),
        ],
        vec![remove.clone(), prefix.clone(), change_ext.clone()],
    );
    let p = preview(&s, &mut empty_disk());
    let impact: HashMap<Uuid, (u32, u32)> = p
        .rule_impact
        .iter()
        .map(|i| (i.rule_id, (i.affected, i.eligible)))
        .collect();
    assert_eq!(impact[&remove.id], (2, 3), "IMG_ appears in two names");
    assert_eq!(impact[&prefix.id], (3, 3), "prefix changes every name");
    assert_eq!(
        impact[&change_ext.id],
        (1, 3),
        "only the .png actually changes extension"
    );
}

#[test]
fn disabling_a_rule_zeroes_its_impact() {
    let mut prefix = prefix_rule("x-");
    prefix.is_enabled = false;
    let s = state(vec![file("/d/a.txt")], vec![prefix.clone()]);
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.rule_impact[0].rule_id, prefix.id);
    assert_eq!(p.rule_impact[0].affected, 0);
}

#[test]
fn overridden_items_are_not_eligible() {
    let mut s = state(
        vec![file("/d/a.txt"), file("/d/b.txt")],
        vec![prefix_rule("x-")],
    );
    s.files[0].override_name = Some("z.txt".into());
    let p = preview(&s, &mut empty_disk());
    assert_eq!(p.rule_impact[0].affected, 1);
    assert_eq!(p.rule_impact[0].eligible, 1);
}

// ---- mode scoping (§7.1) ---------------------------------------------------

#[test]
fn preview_is_scoped_to_the_active_mode() {
    let mut s = CoreState {
        files: vec![file("/d/a.txt"), folder("/d/Photos")],
        rules: vec![prefix_rule("x-")],
        ..CoreState::default()
    };
    let p = compute_preview(&s, &MACOS_PROFILE, &mut empty_disk(), &NO_METADATA);
    assert_eq!(p.entries.len(), 1);
    assert_eq!(p.entries[0].current_name, "a.txt");

    s.list_mode = ListMode::Folders;
    let p = compute_preview(&s, &MACOS_PROFILE, &mut empty_disk(), &NO_METADATA);
    assert_eq!(p.entries.len(), 1);
    assert_eq!(p.entries[0].current_name, "Photos");
    assert_eq!(p.entries[0].new_name, "x-Photos");
}

// ---- rule_derived_name (§7.3) ----------------------------------------------

#[test]
fn rule_derived_name_ignores_override() {
    let mut s = state(vec![file("/d/a.txt")], vec![prefix_rule("x-")]);
    s.files[0].override_name = Some("mine.txt".into());
    let id = s.files[0].id;
    let derived = rule_derived_name(&s, &MACOS_PROFILE, &NO_METADATA, id);
    assert_eq!(derived, "x-a.txt");
}

#[test]
fn rule_derived_name_for_deselected_or_unknown() {
    let mut s = state(vec![file("/d/a.txt")], vec![prefix_rule("x-")]);
    s.files[0].is_selected = false;
    let id = s.files[0].id;
    assert_eq!(
        rule_derived_name(&s, &MACOS_PROFILE, &NO_METADATA, id),
        "a.txt"
    );
    assert_eq!(
        rule_derived_name(&s, &MACOS_PROFILE, &NO_METADATA, Uuid::new_v4()),
        ""
    );
}

#[test]
fn rule_derived_name_uses_the_same_counter() {
    let mut s = state(
        vec![file("/d/a.txt"), file("/d/b.txt"), file("/d/c.txt")],
        vec![numbering_rule()],
    );
    s.sort_key = FileSortKey::Name;
    s.files[1].override_name = Some("custom.txt".into());
    let b = s.files[1].id;
    assert_eq!(
        rule_derived_name(&s, &MACOS_PROFILE, &NO_METADATA, b),
        "02-b.txt",
        "derived name uses the position the override consumed"
    );
}

// ---- laziness at the pipeline level (§6.6) ---------------------------------

#[test]
fn text_only_preview_never_touches_metadata() {
    let metadata = DateCountingMetadata::default();
    let s = state(
        vec![file("/nonexistent/a.txt"), file("/nonexistent/b.txt")],
        vec![prefix_rule("x-")],
    );
    let p = compute_preview(&s, &MACOS_PROFILE, &mut empty_disk(), &metadata);
    assert_eq!(p.counts.change_count, 2);
    assert_eq!(
        metadata.reads.load(AtomicOrdering::SeqCst),
        0,
        "no stat for text-only rules"
    );
}
