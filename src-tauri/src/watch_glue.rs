//! Watcher lifecycle + import plumbing: arming, rescans, and the one
//! import gate every entry point shares (§9, §14.1).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use nameshift_engine::item::standardized;
use nameshift_engine::{recover_orphaned_temp_files, FileItem, WatchedFolder};
use nameshift_watcher::{reconcile_folder, scan_folder, FolderWatcher};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::app_state::{AppState, Shared};

/// Bring the live watcher set in line with `state.watched_folders`: arm
/// missing ones, drop removed ones, re-arm those whose root moved. Dropping
/// a handle tears the OS watcher down (§9 deallocation invariant).
pub fn sync_watchers(app: &AppHandle, shared: &mut Shared) {
    let wanted: Vec<(Uuid, PathBuf)> = shared
        .state
        .watched_folders
        .iter()
        .map(|f| (f.id, f.path.clone()))
        .collect();
    let wanted_ids: HashSet<Uuid> = wanted.iter().map(|(id, _)| *id).collect();
    shared.watchers.retain(|id, _| wanted_ids.contains(id));
    for (id, path) in wanted {
        let stale = !shared.watchers.contains_key(&id);
        if !stale {
            continue;
        }
        if !path.is_dir() {
            // Vanished root: keep the chip with its stale path (§9); the
            // watcher re-arms if the folder comes back via a rescan.
            continue;
        }
        let handle = app.clone();
        match FolderWatcher::watch(&path, move || rescan_one(&handle, id)) {
            Ok(watcher) => {
                shared.watchers.insert(id, watcher);
            }
            Err(error) => {
                tracing::warn!("couldn't watch {}: {error}", path.display());
            }
        }
    }
}

/// Drop and re-arm the watchers for folders whose root path changed after a
/// batch (§8.5 step 2): same folder id, new path.
pub fn rearm_watchers(app: &AppHandle, shared: &mut Shared, folder_ids: &[Uuid]) {
    for id in folder_ids {
        shared.watchers.remove(id);
    }
    sync_watchers(app, shared);
}

/// One watched root changed on disk: rescan it (debounced upstream).
/// Scans happen OUTSIDE the state lock; the reconcile commits inside it.
pub fn rescan_one(app: &AppHandle, folder_id: Uuid) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Some((path, recursive)) = state.read(|shared| {
        // Skip while a batch runs: a mid-batch reconcile would drop items
        // whose files sit at phase-1 temp names; the batch's own renames
        // fire fresh events that land after it finishes.
        if shared.processing.is_some() {
            return None;
        }
        shared
            .state
            .watched_folders
            .iter()
            .find(|f| f.id == folder_id)
            .map(|folder| {
                (
                    folder.path.clone(),
                    folder
                        .include_subfolders
                        .unwrap_or(shared.state.include_subfolders),
                )
            })
    }) else {
        return;
    };
    let scanned = scan_folder(&path, recursive);
    state.mutate(|shared| {
        if reconcile_folder(&mut shared.state, folder_id, &scanned) {
            shared.invalidate_disk_caches();
        }
    });
}

/// Rescan every watched folder (menu command + session restore).
pub fn rescan_all(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let folders: Vec<Uuid> =
        state.read(|shared| shared.state.watched_folders.iter().map(|f| f.id).collect());
    for id in folders {
        rescan_one(app, id);
    }
}

/// The one import gate for every entry point — drop, ⌘O, second instance,
/// OS open-with (§14.1): processing → refuse with the modal notice; an idle
/// revert preview auto-dismisses first.
pub fn import_paths(app: &AppHandle, raw_paths: Vec<String>) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let busy = state.read(|shared| shared.processing.is_some());
    if busy {
        state.alert(
            "info",
            "Name Shift",
            "Renaming is in progress. Try again when it finishes.",
        );
        return;
    }
    state.mutate(|shared| shared.state.selected_snapshot_id = None);

    let paths: Vec<PathBuf> = raw_paths
        .into_iter()
        .map(|raw| standardized(Path::new(&raw)))
        .filter(|path| path.symlink_metadata().is_ok())
        .collect();
    if paths.is_empty() {
        return;
    }

    // Orphan recovery once per distinct parent directory of loose files
    // (§8.4) — never while a batch is processing (guarded above).
    let parents: HashSet<PathBuf> = paths
        .iter()
        .filter(|path| !path.is_dir())
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect();
    for parent in &parents {
        recover_orphaned_temp_files(parent);
    }

    let mut new_folder_ids: Vec<Uuid> = Vec::new();
    state.mutate(|shared| {
        for path in &paths {
            shared.access.note_user_granted(path);
            if path.is_dir() {
                // Importing a folder = watching it; re-importing an
                // already-watched root just rescans it (§9).
                if let Some(existing) = shared
                    .state
                    .watched_folders
                    .iter()
                    .find(|f| f.path == *path)
                {
                    new_folder_ids.push(existing.id);
                    continue;
                }
                let folder = WatchedFolder::new(path);
                new_folder_ids.push(folder.id);
                recover_orphaned_temp_files(path);
                shared.state.watched_folders.push(folder);
            } else {
                // Loose file: un-exclude + direct import.
                shared.state.excluded_paths.remove(path);
                if shared.state.files.iter().any(|item| item.path == *path) {
                    continue;
                }
                shared.state.files.push(FileItem::new(path, false));
            }
        }
        shared.invalidate_disk_caches();
        sync_watchers(app, shared);
    });
    for id in new_folder_ids {
        rescan_one(app, id);
    }
    state.save_session_now();
}
