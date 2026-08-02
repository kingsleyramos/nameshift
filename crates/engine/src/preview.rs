//! The preview pipeline (§7): pure given `CoreState` + an injected
//! `DirectoryLister` + `PlatformProfile` + `MetadataSource`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::diffkey::{diff_key, fold_name};
use crate::item::FileItem;
use crate::platform::PlatformProfile;
use crate::rule::{apply_rule, split_name, trimmed_name};
use crate::sort::sort_items;
use crate::state::CoreState;
use crate::tokens::{MetadataSource, TokenContext};
use crate::validate::{intrinsic_problem, Problem};

/// One row of the live preview (§7).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PreviewEntry {
    /// Matches `FileItem.id`.
    #[serde(with = "crate::serde_util::uuid_upper")]
    #[ts(as = "String")]
    pub id: Uuid,
    /// The item's current on-disk name.
    pub current_name: String,
    /// The proposed name after rules, override, trim, and auto-resolve.
    pub new_name: String,
    /// The inclusion checkbox state.
    pub is_selected: bool,
    /// Whether a manual override produced `new_name`.
    pub has_override: bool,
    /// `new_name != current_name`.
    pub is_changed: bool,
    /// Why this item can't be renamed, if anything (§7.6).
    pub problem: Option<Problem>,
}

/// How many items one rule actually changed (§7.9); drives the rule-card
/// `affects 14 of 20` header.
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RuleImpact {
    /// The rule this impact belongs to.
    #[serde(with = "crate::serde_util::uuid_upper")]
    #[ts(as = "String")]
    pub rule_id: Uuid,
    /// Items whose name this rule changed at its stage.
    pub affected: u32,
    /// Selected, non-overridden active items.
    pub eligible: u32,
}

/// Derived counts for the action bar and Apply gating (§7.7).
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PreviewCounts {
    /// Included (checked) active items.
    pub selected_count: u32,
    /// Entries whose name will change.
    pub change_count: u32,
    /// Entries with an Apply-blocking problem.
    pub conflict_count: u32,
    /// `change_count > 0 && conflict_count == 0`.
    pub can_apply: bool,
}

/// The full preview payload.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Preview {
    /// One entry per active item, in view order.
    pub entries: Vec<PreviewEntry>,
    /// Per-rule impact counts, in stack order (disabled rules count 0).
    pub rule_impact: Vec<RuleImpact>,
    /// Derived counts (§7.7).
    pub counts: PreviewCounts,
}

/// Lists a directory's entry names. Injectable so preview tests never touch
/// the real filesystem.
pub trait DirectoryLister: Send + Sync {
    /// The plain entry names present in `directory` (empty if unreadable).
    fn list(&self, directory: &Path) -> Vec<String>;
}

/// The production lister: `std::fs::read_dir`.
pub struct FsDirectoryLister;

impl DirectoryLister for FsDirectoryLister {
    fn list(&self, directory: &Path) -> Vec<String> {
        std::fs::read_dir(crate::platform::win_long_path(directory))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Per-directory on-disk name sets, folded with `diff_key` semantics (§7.5).
///
/// Each directory is listed ONCE and cached until the file list itself
/// mutates (imports, rescans, apply) — never per keystroke; editing a rule
/// must cost zero stat calls (re-listing per keystroke beachballed the app
/// historically on network volumes). Known, accepted staleness: external
/// changes to *unwatched* directories can make a collision verdict stale
/// until the next list mutation. Apply's two-phase executor still fails
/// safely on a real collision, so this is a preview-accuracy gap, not a
/// data-loss path (§24 Q6).
pub struct DirectoryNameCache {
    lister: Box<dyn DirectoryLister>,
    folded: HashMap<PathBuf, HashSet<String>>,
}

impl DirectoryNameCache {
    /// A cache over the given lister.
    pub fn new(lister: Box<dyn DirectoryLister>) -> Self {
        Self {
            lister,
            folded: HashMap::new(),
        }
    }

    /// Drop every cached listing. Call on every file-list mutation.
    pub fn invalidate(&mut self) {
        self.folded.clear();
    }

    /// Whether `name` exists on disk in `directory`, by folded comparison.
    pub fn contains(&mut self, profile: &PlatformProfile, directory: &Path, name: &str) -> bool {
        let folded_name = fold_name(profile, name);
        self.names(profile, directory).contains(&folded_name)
    }

    fn names(&mut self, profile: &PlatformProfile, directory: &Path) -> &HashSet<String> {
        self.folded
            .entry(directory.to_path_buf())
            .or_insert_with(|| {
                self.lister
                    .list(directory)
                    .iter()
                    .map(|name| fold_name(profile, name))
                    .collect()
            })
    }
}

/// Compute the full preview (§7.3–7.9).
pub fn compute_preview(
    state: &CoreState,
    profile: &PlatformProfile,
    disk: &mut DirectoryNameCache,
    metadata: &dyn MetadataSource,
) -> Preview {
    let active: Vec<&FileItem> = state.active_items().collect();
    let sorted = sort_items(active, state.sort_key, state.sort_ascending, metadata);

    let enabled: Vec<&crate::rule::RenameRule> = state
        .rules
        .iter()
        .filter(|rule| rule.is_enabled && rule.is_effective())
        .collect();
    let mut affected: HashMap<Uuid, u32> = HashMap::new();
    let mut eligible: u32 = 0;

    // §7.3: one pass in view order; deselected items keep their names and
    // consume nothing; overrides consume a number, then short-circuit.
    let mut counter: u32 = 0;
    let mut folder_counters: HashMap<PathBuf, u32> = HashMap::new();
    let mut proposed: Vec<(&FileItem, String, bool)> = Vec::with_capacity(sorted.len());
    for item in sorted {
        let current = item.name();
        if !item.is_selected {
            proposed.push((item, current, false));
            continue;
        }
        counter += 1;
        let folder_counter = folder_counters.entry(item.directory()).or_insert(0);
        *folder_counter += 1;
        if let Some(override_name) = &item.override_name {
            proposed.push((item, override_name.clone(), true));
            continue;
        }
        if is_unrenamable(item) {
            // The real name isn't valid UTF-8; rules can't run on the lossy
            // display form safely. Keep the name, badge the row (§16.5).
            proposed.push((item, current, false));
            continue;
        }
        eligible += 1;
        let ctx = TokenContext {
            index: counter,
            folder_index: *folder_counter,
            path: &item.path,
            metadata,
        };
        let mut name = current;
        for rule in &enabled {
            let after = apply_rule(rule, &name, Some(&ctx), item.is_directory);
            if after != name {
                *affected.entry(rule.id).or_insert(0) += 1;
            }
            name = after;
        }
        if state.trims_whitespace {
            name = trimmed_name(&name, item.is_directory);
        }
        proposed.push((item, name, false));
    }

    if state.auto_resolves_conflicts {
        auto_resolve(&mut proposed, profile, disk);
    }

    // §7.6 checks 6–7 need the whole batch's target and source keys.
    let mut target_counts: HashMap<String, u32> = HashMap::new();
    for (item, new_name, _) in proposed.iter().filter(|(item, _, _)| item.is_selected) {
        *target_counts
            .entry(diff_key(profile, &item.directory(), new_name))
            .or_insert(0) += 1;
    }
    let source_keys: HashSet<String> = proposed
        .iter()
        .filter(|(item, _, _)| item.is_selected)
        .map(|(item, _, _)| diff_key(profile, &item.directory(), &item.name()))
        .collect();

    let mut entries = Vec::with_capacity(proposed.len());
    let mut counts = PreviewCounts::default();
    for (item, new_name, has_override) in proposed {
        let current_name = item.name();
        let is_changed = new_name != current_name;
        let problem = if item.is_selected {
            problem_for(
                item,
                &current_name,
                &new_name,
                is_changed,
                profile,
                disk,
                &target_counts,
                &source_keys,
            )
        } else {
            // Deselected items never carry problems (§7.6).
            None
        };
        if item.is_selected {
            counts.selected_count += 1;
        }
        if is_changed {
            counts.change_count += 1;
        }
        if problem.is_some_and(Problem::blocks_apply) {
            counts.conflict_count += 1;
        }
        entries.push(PreviewEntry {
            id: item.id,
            current_name,
            new_name,
            is_selected: item.is_selected,
            has_override,
            is_changed,
            problem,
        });
    }
    counts.can_apply = counts.change_count > 0 && counts.conflict_count == 0;

    let rule_impact = state
        .rules
        .iter()
        .map(|rule| RuleImpact {
            rule_id: rule.id,
            affected: affected.get(&rule.id).copied().unwrap_or(0),
            eligible,
        })
        .collect();

    Preview {
        entries,
        rule_impact,
        counts,
    }
}

/// The name the rules alone would produce for `id`, ignoring any override —
/// same counters, same trim (§7.3). Deselected or unknown ids return the
/// current name. Used by the UI to refuse to pin an override that merely
/// restates the rules' output (§14.4).
pub fn rule_derived_name(
    state: &CoreState,
    profile: &PlatformProfile,
    metadata: &dyn MetadataSource,
    id: Uuid,
) -> String {
    let _ = profile;
    let active: Vec<&FileItem> = state.active_items().collect();
    let sorted = sort_items(active, state.sort_key, state.sort_ascending, metadata);
    let enabled: Vec<&crate::rule::RenameRule> = state
        .rules
        .iter()
        .filter(|rule| rule.is_enabled && rule.is_effective())
        .collect();
    let mut counter: u32 = 0;
    let mut folder_counters: HashMap<PathBuf, u32> = HashMap::new();
    for item in sorted {
        if !item.is_selected {
            continue;
        }
        counter += 1;
        let folder_counter = folder_counters.entry(item.directory()).or_insert(0);
        *folder_counter += 1;
        if item.id != id {
            continue;
        }
        if is_unrenamable(item) {
            return item.name();
        }
        let ctx = TokenContext {
            index: counter,
            folder_index: *folder_counter,
            path: &item.path,
            metadata,
        };
        let mut name = item.name();
        for rule in &enabled {
            name = apply_rule(rule, &name, Some(&ctx), item.is_directory);
        }
        if state.trims_whitespace {
            name = trimmed_name(&name, item.is_directory);
        }
        return name;
    }
    state
        .files
        .iter()
        .find(|item| item.id == id)
        .map(FileItem::name)
        .unwrap_or_default()
}

fn is_unrenamable(item: &FileItem) -> bool {
    item.path
        .file_name()
        .is_some_and(|name| name.to_str().is_none())
}

/// §7.4: append ` 2`, ` 3`… to colliding proposed names instead of blocking.
/// First-come-first-served in view order; items keeping their names always
/// win.
fn auto_resolve(
    proposed: &mut [(&FileItem, String, bool)],
    profile: &PlatformProfile,
    disk: &mut DirectoryNameCache,
) {
    let mut taken: HashSet<String> = HashSet::new();
    let mut vacated: HashSet<String> = HashSet::new();
    for (item, new_name, _) in proposed.iter().filter(|(item, _, _)| item.is_selected) {
        if *new_name == item.name() {
            taken.insert(diff_key(profile, &item.directory(), new_name));
        } else {
            vacated.insert(diff_key(profile, &item.directory(), &item.name()));
        }
    }
    for key in &taken {
        vacated.remove(key);
    }

    for (item, new_name, _) in proposed.iter_mut() {
        if !item.is_selected || *new_name == item.name() || new_name.is_empty() {
            continue;
        }
        let directory = item.directory();
        let mut is_free = |candidate: &str| {
            let key = diff_key(profile, &directory, candidate);
            if taken.contains(&key) {
                return false;
            }
            if !vacated.contains(&key) && disk.contains(profile, &directory, candidate) {
                return false;
            }
            true
        };
        if !is_free(new_name) {
            // Folder names are whole strings — the suffix lands at the end.
            let (base, ext) = if item.is_directory {
                (new_name.as_str(), "")
            } else {
                split_name(new_name)
            };
            let mut suffix = 2;
            let mut resolved = new_name.clone();
            while !is_free(&resolved) && suffix < 1000 {
                resolved = if ext.is_empty() {
                    format!("{base} {suffix}")
                } else {
                    format!("{base} {suffix}.{ext}")
                };
                suffix += 1;
            }
            *new_name = resolved;
        }
        taken.insert(diff_key(profile, &directory, new_name));
    }
}

/// §7.6 problem detection for one selected item; first match wins.
#[allow(clippy::too_many_arguments)]
fn problem_for(
    item: &FileItem,
    current_name: &str,
    new_name: &str,
    is_changed: bool,
    profile: &PlatformProfile,
    disk: &mut DirectoryNameCache,
    target_counts: &HashMap<String, u32>,
    source_keys: &HashSet<String>,
) -> Option<Problem> {
    // Environmental badge first: the CURRENT name can't be edited safely.
    if is_unrenamable(item) {
        return Some(Problem::UnrenamableName);
    }
    // An unchanged item's name is valid by definition (it already exists on
    // disk), so intrinsic checks apply only when changed — a stale `:` in an
    // existing name must not be flagged (real false-positive fixed
    // historically).
    if is_changed {
        if let Some(problem) = intrinsic_problem(profile, new_name) {
            return Some(problem);
        }
    }
    let directory = item.directory();
    let target_key = diff_key(profile, &directory, new_name);
    // Unchanged items claim their current key here, so renaming *onto* a
    // selected-but-unchanged file is caught.
    if target_counts.get(&target_key).copied().unwrap_or(0) > 1 {
        return Some(Problem::DuplicateTarget);
    }
    // A name being vacated in the same batch is fine — two-phase makes swaps
    // safe (§24 Q5).
    if is_changed
        && !source_keys.contains(&target_key)
        && disk.contains(profile, &directory, new_name)
    {
        return Some(Problem::ExistingFileCollision);
    }
    let _ = current_name;
    None
}
