//! `session.json` — the restorable workspace (§4.3), with legacy-tolerant
//! decoding.

use std::path::PathBuf;

use nameshift_engine::{FileSortKey, ListMode, RenameRule};
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

/// One tracked file in the session (§4.3 `files` entries).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionFileEntry {
    /// Absolute path.
    pub path: PathBuf,
    /// Inclusion checkbox state.
    #[serde(default = "default_true")]
    pub is_selected: bool,
    /// Manual edit — omitted when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_name: Option<String>,
    /// Whether a watched folder discovered this item.
    #[serde(default)]
    pub is_from_folder: bool,
    /// Security-scoped bookmark, base64 — MAS builds only (§10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
}

/// The whole persisted workspace. Every field decodes with a default so
/// sessions written by any earlier version keep loading (§4.2 serde policy).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    /// The rule stack.
    #[serde(default)]
    pub rules: Vec<RenameRule>,
    /// Global trim-spaces toggle.
    #[serde(default)]
    pub trims_whitespace: bool,
    /// Auto-resolve naming conflicts toggle.
    #[serde(default)]
    pub auto_resolves_conflicts: bool,
    /// Keep rules after applying. A session file LACKING this key decodes to
    /// `false` — it was written under the old default; preserve what that
    /// user experienced. Only fresh installs (no session.json at all) get
    /// the new `true` default (§4.3, §24 Q26).
    #[serde(default)]
    pub keep_rules_after_apply: bool,
    /// Global include-subfolders toggle for watched folders.
    #[serde(default)]
    pub include_subfolders: bool,
    /// Current sort key, legacy raw value (`"Order Added"`, `"Name"`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_key_raw: Option<String>,
    /// Sort direction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_ascending: Option<bool>,
    /// Pre-split legacy sort field — read for migration, never written.
    #[serde(default, skip_serializing)]
    pub sort_order_raw: Option<String>,
    /// List mode raw value (`"Files"` / `"Folders"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_mode_raw: Option<String>,
    /// Every tracked file.
    #[serde(default)]
    pub files: Vec<SessionFileEntry>,
    /// Watched folder roots.
    #[serde(default)]
    pub watched_folder_paths: Vec<PathBuf>,
    /// Parallel array of scoped bookmarks — MAS builds only (§10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watched_folder_bookmarks: Option<Vec<Option<String>>>,
    /// Parallel array of per-folder subfolder overrides; `null` follows the
    /// global toggle. Additive — absent in legacy files (§4.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watched_folder_subfolders: Option<Vec<Option<bool>>>,
    /// User-removed paths that rescans skip.
    #[serde(default)]
    pub excluded_paths: Vec<PathBuf>,
}

impl SessionState {
    /// The sort this session should restore, migrating the legacy pre-split
    /// `sortOrderRaw` when the split fields are absent (§4.3).
    pub fn resolved_sort(&self) -> (FileSortKey, bool) {
        if let Some(raw) = &self.sort_key_raw {
            let key = parse_raw::<FileSortKey>(raw).unwrap_or_default();
            return (key, self.sort_ascending.unwrap_or(true));
        }
        if let Some(legacy) = &self.sort_order_raw {
            return match legacy.as_str() {
                "Name (A–Z)" => (FileSortKey::Name, true),
                "Name (Z–A)" => (FileSortKey::Name, false),
                "Extension" => (FileSortKey::FileExtension, true),
                "Folder" => (FileSortKey::Folder, true),
                "Date Created" => (FileSortKey::DateCreated, true),
                "Date Modified" => (FileSortKey::DateModified, true),
                _ => (FileSortKey::OrderAdded, true),
            };
        }
        (FileSortKey::OrderAdded, true)
    }

    /// The list mode this session should restore.
    pub fn resolved_list_mode(&self) -> ListMode {
        self.list_mode_raw
            .as_deref()
            .and_then(parse_raw::<ListMode>)
            .unwrap_or_default()
    }

    /// Set the raw sort fields from typed values (the write side).
    pub fn set_sort(&mut self, key: FileSortKey, ascending: bool) {
        self.sort_key_raw = Some(raw_value(&key));
        self.sort_ascending = Some(ascending);
        self.sort_order_raw = None;
    }

    /// Set the raw list-mode field from the typed value (the write side).
    pub fn set_list_mode(&mut self, mode: ListMode) {
        self.list_mode_raw = Some(raw_value(&mode));
    }
}

fn parse_raw<T: serde::de::DeserializeOwned>(raw: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(raw.to_string())).ok()
}

fn raw_value<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        _ => String::new(),
    }
}
