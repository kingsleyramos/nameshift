//! File-list domain types: tracked items, watched folders, list mode, sorting.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// One tracked file or folder in the workspace list.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FileItem {
    /// Stable identity — rows are keyed by this, never by list position.
    #[serde(with = "crate::serde_util::uuid_upper")]
    #[ts(as = "String")]
    pub id: Uuid,
    /// Absolute, standardized path (no trailing separator, no `.`/`..`).
    pub path: PathBuf,
    /// `Some` when the item was discovered by a watched folder.
    #[serde(default, with = "crate::serde_util::uuid_upper_opt")]
    #[ts(as = "Option<String>")]
    pub folder_id: Option<Uuid>,
    /// Inclusion checkbox — only selected items are renamed.
    pub is_selected: bool,
    /// Whether this item is a directory (a Folders-mode target).
    pub is_directory: bool,
    /// Manual edit; wins over rules when set.
    pub override_name: Option<String>,
}

impl FileItem {
    /// A freshly imported, selected item with a standardized path.
    pub fn new(path: &Path, is_directory: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            path: standardized(path),
            folder_id: None,
            is_selected: true,
            is_directory,
            override_name: None,
        }
    }

    /// The final path component — the item's current name on disk.
    /// Non-UTF-8 names render lossily (U+FFFD) per §16.5.
    pub fn name(&self) -> String {
        name_of(&self.path)
    }

    /// The parent directory of the item.
    pub fn directory(&self) -> PathBuf {
        self.path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    }
}

/// The final path component of `path`, lossily decoded.
pub fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Lexically standardize a path: drop `.` components, resolve `..` where
/// possible, and strip trailing separators. Does not touch the filesystem.
pub fn standardized(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    result.push(component.as_os_str());
                }
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

/// A folder the user imported: watched live, its contents populate the list.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WatchedFolder {
    /// Stable identity, preserved across path rewrites.
    #[serde(with = "crate::serde_util::uuid_upper")]
    #[ts(as = "String")]
    pub id: Uuid,
    /// Absolute path of the watched root.
    pub path: PathBuf,
    /// Per-folder subfolder override; `None` follows the global toggle (§9).
    pub include_subfolders: Option<bool>,
}

impl WatchedFolder {
    /// A new watched folder following the global subfolder toggle.
    pub fn new(path: &Path) -> Self {
        Self {
            id: Uuid::new_v4(),
            path: standardized(path),
            include_subfolders: None,
        }
    }

    /// The folder's display name (its final path component).
    pub fn name(&self) -> String {
        name_of(&self.path)
    }
}

/// Which kind of item the workspace is operating on (§7.1).
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[ts(export)]
pub enum ListMode {
    /// Files are the active targets.
    #[default]
    Files,
    /// Directories are the active targets.
    Folders,
}

/// File-list sort keys, serialized as the legacy raw values (§4.1).
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[ts(export)]
pub enum FileSortKey {
    /// Insertion order (stable).
    #[default]
    #[serde(rename = "Order Added")]
    OrderAdded,
    /// Natural, case-insensitive name compare.
    #[serde(rename = "Name")]
    Name,
    /// Lowercased extension, ties by name. Files mode only.
    #[serde(rename = "Extension")]
    FileExtension,
    /// Parent directory path, ties by name.
    #[serde(rename = "Folder")]
    Folder,
    /// Creation date, ties by name.
    #[serde(rename = "Date Created")]
    DateCreated,
    /// Modification date, ties by name.
    #[serde(rename = "Date Modified")]
    DateModified,
}

/// UI list filter — affects visibility only, never the preview computation.
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[ts(export)]
pub enum FilterMode {
    /// Show every active item.
    #[default]
    All,
    /// Show items whose name will change.
    WillChange,
    /// Show items with a naming conflict.
    Conflicts,
}
