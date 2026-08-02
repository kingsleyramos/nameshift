//! Watched-root scanning and list reconciliation (§9).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use nameshift_engine::item::standardized;
use nameshift_engine::{natural_compare, CoreState, FileItem, TEMP_PREFIX};
use uuid::Uuid;

/// One scanned entry: a regular file, or a directory (Folders-mode target).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanEntry {
    /// Standardized absolute path.
    pub path: PathBuf,
    /// Whether the entry is a directory.
    pub is_directory: bool,
}

/// macOS bundle directories are opaque documents — never descend into them
/// (§9). No-op elsewhere.
const BUNDLE_EXTENSIONS: [&str; 8] = [
    ".app",
    ".bundle",
    ".framework",
    ".photoslibrary",
    ".fcpbundle",
    ".imovielibrary",
    ".band",
    ".logicx",
];

fn is_bundle_name(name: &str) -> bool {
    cfg!(target_os = "macos")
        && BUNDLE_EXTENSIONS
            .iter()
            .any(|ext| name.to_ascii_lowercase().ends_with(ext))
}

#[cfg(windows)]
fn has_hidden_attribute(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, INVALID_FILE_ATTRIBUTES,
    };
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
    attributes != INVALID_FILE_ATTRIBUTES && (attributes & FILE_ATTRIBUTE_HIDDEN) != 0
}

#[cfg(not(windows))]
fn has_hidden_attribute(_path: &Path) -> bool {
    false
}

/// List a watched root's contents: regular files, and directories (the
/// Folders-mode targets). The root itself is never a target. Skips hidden
/// entries (dotfiles everywhere; hidden-attribute files on Windows) and
/// anything matching `.fne-tmp-*` — mid-batch FSEvents must not import the
/// two-phase mover's temps (§8.4). Results are sorted by path (natural
/// compare).
pub fn scan_folder(root: &Path, recursive: bool) -> Vec<ScanEntry> {
    let mut found: Vec<ScanEntry> = Vec::new();
    walk(root, recursive, &mut found);
    found.sort_by(|a, b| natural_compare(&a.path.to_string_lossy(), &b.path.to_string_lossy()));
    found
}

fn walk(directory: &Path, recursive: bool, found: &mut Vec<ScanEntry>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.starts_with(TEMP_PREFIX) {
            continue;
        }
        let path = entry.path();
        if has_hidden_attribute(&path) {
            continue;
        }
        // Classify by the (followed) target type, per §24 Q8; broken links
        // error here and are skipped.
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if metadata.is_file() {
            found.push(ScanEntry {
                path: standardized(&path),
                is_directory: false,
            });
        } else if metadata.is_dir() {
            found.push(ScanEntry {
                path: standardized(&path),
                is_directory: true,
            });
            if recursive && !is_bundle_name(&name) {
                walk(&path, true, found);
            }
        }
    }
}

/// Fold a scan into the tracked list (§9): drop this root's items that
/// vanished, add new ones (selected by default, `folder_id` set), skip
/// excluded paths. Existing items keep their id, selection, and override.
/// Returns whether anything changed.
pub fn reconcile_folder(state: &mut CoreState, folder_id: Uuid, scanned: &[ScanEntry]) -> bool {
    let scanned: Vec<&ScanEntry> = scanned
        .iter()
        .filter(|entry| !state.excluded_paths.contains(&entry.path))
        .collect();
    let scanned_paths: HashSet<&PathBuf> = scanned.iter().map(|entry| &entry.path).collect();

    let before = state.files.len();
    state
        .files
        .retain(|item| item.folder_id != Some(folder_id) || scanned_paths.contains(&item.path));
    let mut changed = state.files.len() != before;

    let tracked: HashSet<PathBuf> = state.files.iter().map(|item| item.path.clone()).collect();
    for entry in scanned {
        if tracked.contains(&entry.path) {
            continue;
        }
        let mut item = FileItem::new(&entry.path, entry.is_directory);
        item.folder_id = Some(folder_id);
        state.files.push(item);
        changed = true;
    }
    changed
}
