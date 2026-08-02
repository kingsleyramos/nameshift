//! Workspace state (§4.1): the single `CoreState` owned by the shell, plus
//! snapshots and rule presets.

use std::collections::HashSet;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::item::{FileItem, FileSortKey, ListMode, WatchedFolder};
use crate::rule::RenameRule;

/// History is capped at this many snapshots, newest first (§4.3).
pub const MAX_HISTORY_COUNT: usize = 50;

/// One recorded rename in a snapshot: absolute old and new paths.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export)]
pub struct SnapshotEntry {
    /// Absolute path before the rename.
    pub from: PathBuf,
    /// Absolute path after the rename.
    pub to: PathBuf,
}

/// One applied batch: every rename it performed, revertible later (§4.3).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Snapshot {
    /// Stable identity.
    #[serde(with = "crate::serde_util::uuid_upper")]
    #[ts(as = "String")]
    pub id: Uuid,
    /// When the batch was applied (UTC).
    #[serde(with = "crate::serde_util::utc_seconds")]
    #[ts(as = "String")]
    pub date: DateTime<Utc>,
    /// ` · `-joined description of what the batch did (§8.1).
    pub summary: String,
    /// The renames, in execution order.
    pub entries: Vec<SnapshotEntry>,
}

/// A named, reusable rule stack (§4.3 `presets.json`).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RulePreset {
    /// Stable identity.
    #[serde(with = "crate::serde_util::uuid_upper")]
    #[ts(as = "String")]
    pub id: Uuid,
    /// User-chosen display name.
    pub name: String,
    /// The saved rule stack.
    pub rules: Vec<RenameRule>,
    /// The saved global trim-spaces toggle.
    #[serde(default)]
    pub trims_whitespace: bool,
}

/// The whole workspace: every mutation goes through the shell's
/// `AppState::mutate` funnel, which bumps `version` (§13.2 invariant —
/// never mutate this outside the funnel).
#[derive(Clone, Debug, PartialEq)]
pub struct CoreState {
    /// Every tracked file and folder, in insertion order.
    pub files: Vec<FileItem>,
    /// The rule stack, top to bottom.
    pub rules: Vec<RenameRule>,
    /// Applied snapshots, newest first, capped at [`MAX_HISTORY_COUNT`].
    pub history: Vec<Snapshot>,
    /// Saved rule presets.
    pub presets: Vec<RulePreset>,
    /// Watched folder roots.
    pub watched_folders: Vec<WatchedFolder>,
    /// Paths the user removed from the list; rescans skip these.
    pub excluded_paths: HashSet<PathBuf>,
    /// Global trim-spaces toggle (§5.4).
    pub trims_whitespace: bool,
    /// Append ` 2`, ` 3`… to colliding names instead of blocking (§7.4).
    pub auto_resolves_conflicts: bool,
    /// Keep the rule stack after a successful Apply (§4.3 default).
    pub keep_rules_after_apply: bool,
    /// Global subfolder toggle for watched folders (§9).
    pub include_subfolders: bool,
    /// Active sort key.
    pub sort_key: FileSortKey,
    /// Sort direction; `false` reverses the whole ordering.
    pub sort_ascending: bool,
    /// Files or Folders mode (§7.1).
    pub list_mode: ListMode,
    /// `Some` → the UI is in revert-preview mode for this snapshot.
    pub selected_snapshot_id: Option<Uuid>,
    /// Drives the rules panel's post-apply empty state (§14.3).
    pub rules_cleared_by_apply: bool,
    /// Bumped by EVERY mutation; preview caches key on it (§13.1).
    pub version: u64,
}

impl Default for CoreState {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            rules: Vec::new(),
            history: Vec::new(),
            presets: Vec::new(),
            watched_folders: Vec::new(),
            excluded_paths: HashSet::new(),
            trims_whitespace: false,
            auto_resolves_conflicts: false,
            // Fresh-install default is ON (§4.3, §24 Q26); legacy sessions
            // lacking the key decode to false in the store crate.
            keep_rules_after_apply: true,
            include_subfolders: false,
            sort_key: FileSortKey::OrderAdded,
            sort_ascending: true,
            list_mode: ListMode::Files,
            selected_snapshot_id: None,
            rules_cleared_by_apply: false,
            version: 0,
        }
    }
}

impl CoreState {
    /// Items whose kind matches the current list mode — everything
    /// user-facing is scoped to these (§7.1).
    pub fn active_items(&self) -> impl Iterator<Item = &FileItem> {
        let wants_directories = self.list_mode == ListMode::Folders;
        self.files
            .iter()
            .filter(move |item| item.is_directory == wants_directories)
    }
}
