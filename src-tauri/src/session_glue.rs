//! Session capture and restore (§4.3).

use std::collections::HashMap;
use std::path::PathBuf;

use nameshift_engine::item::standardized;
use nameshift_engine::{FileItem, WatchedFolder};
use nameshift_store::{save_session, SessionFileEntry, SessionState};
use tauri::{AppHandle, Manager};

use crate::app_state::{AppState, Shared};
use crate::watch_glue;

/// Base64 helpers for MAS bookmark blobs — trivially reversible, no dep.
#[cfg(feature = "channel-mas")]
fn encode_token(token: &[u8]) -> String {
    use std::fmt::Write as _;
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in token.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let _ = write!(
            out,
            "{}{}",
            ALPHABET[(n >> 18) as usize & 63] as char,
            ALPHABET[(n >> 12) as usize & 63] as char
        );
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Capture the persistable workspace (§4.3 schema).
pub fn capture(shared: &Shared) -> SessionState {
    let mut session = SessionState {
        rules: shared.state.rules.clone(),
        trims_whitespace: shared.state.trims_whitespace,
        auto_resolves_conflicts: shared.state.auto_resolves_conflicts,
        keep_rules_after_apply: shared.state.keep_rules_after_apply,
        include_subfolders: shared.state.include_subfolders,
        files: shared
            .state
            .files
            .iter()
            .map(|item| SessionFileEntry {
                path: item.path.clone(),
                is_selected: item.is_selected,
                override_name: item.override_name.clone(),
                is_from_folder: item.folder_id.is_some(),
                // Bookmarks are additive, MAS-only (§4.3/§10): absent in
                // direct-channel output.
                #[cfg(feature = "channel-mas")]
                bookmark: shared
                    .access
                    .persist_token(&item.path)
                    .map(|t| encode_token(&t)),
                #[cfg(not(feature = "channel-mas"))]
                bookmark: None,
            })
            .collect(),
        watched_folder_paths: shared
            .state
            .watched_folders
            .iter()
            .map(|f| f.path.clone())
            .collect(),
        #[cfg(feature = "channel-mas")]
        watched_folder_bookmarks: Some(
            shared
                .state
                .watched_folders
                .iter()
                .map(|f| {
                    shared
                        .access
                        .persist_token(&f.path)
                        .map(|t| encode_token(&t))
                })
                .collect(),
        ),
        #[cfg(not(feature = "channel-mas"))]
        watched_folder_bookmarks: None,
        watched_folder_subfolders: Some(
            shared
                .state
                .watched_folders
                .iter()
                .map(|f| f.include_subfolders)
                .collect(),
        ),
        excluded_paths: {
            let mut paths: Vec<PathBuf> = shared.state.excluded_paths.iter().cloned().collect();
            paths.sort();
            paths
        },
        ..SessionState::default()
    };
    session.set_sort(shared.state.sort_key, shared.state.sort_ascending);
    session.set_list_mode(shared.state.list_mode);
    session
}

/// Persist immediately (the §4.3 immediate triggers + exit).
pub fn save_now(shared: &mut Shared) {
    let session = capture(shared);
    if let Err(error) = save_session(&shared.store, &session) {
        tracing::warn!("session save failed: {error}");
    }
    if let Err(error) = nameshift_store::save_history(&shared.store, &shared.state.history) {
        tracing::warn!("history save failed: {error}");
    }
    if let Err(error) = nameshift_store::save_presets(&shared.store, &shared.state.presets) {
        tracing::warn!("presets save failed: {error}");
    }
}

/// Launch-time restore (§4.3): scalars, direct files that still exist
/// (count the missing), watchers re-armed, folder-discovered selection and
/// overrides re-applied by path (first occurrence wins).
pub fn restore(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let session = state.read(|shared| {
        shared.state.history = nameshift_store::load_history(&shared.store);
        shared.state.presets = nameshift_store::load_presets(&shared.store);
        nameshift_store::load_session(&shared.store)
    });
    let Some(session) = session else {
        // Fresh install: CoreState::default already carries the
        // keep-rules-on default (§24 Q26).
        return;
    };

    let mut missing = 0u32;
    state.mutate(|shared| {
        let (sort_key, sort_ascending) = session.resolved_sort();
        shared.state.rules = session.rules.clone();
        shared.rules_revision += 1;
        shared.state.trims_whitespace = session.trims_whitespace;
        shared.state.auto_resolves_conflicts = session.auto_resolves_conflicts;
        shared.state.keep_rules_after_apply = session.keep_rules_after_apply;
        shared.state.include_subfolders = session.include_subfolders;
        shared.state.sort_key = sort_key;
        shared.state.sort_ascending = sort_ascending;
        shared.state.list_mode = session.resolved_list_mode();
        shared.state.excluded_paths = session.excluded_paths.iter().cloned().collect();

        // Direct files re-import straight from the entry list.
        for entry in session.files.iter().filter(|entry| !entry.is_from_folder) {
            let path = standardized(&entry.path);
            if path.symlink_metadata().is_err() {
                missing += 1;
                continue;
            }
            let mut item = FileItem::new(&path, path.is_dir());
            item.is_selected = entry.is_selected;
            item.override_name = entry.override_name.clone();
            shared.state.files.push(item);
        }

        // Watched roots: keep every entry (stale chips stay visible, §9);
        // watchers arm only for the ones that exist.
        let subfolders = session
            .watched_folder_subfolders
            .clone()
            .unwrap_or_default();
        for (index, path) in session.watched_folder_paths.iter().enumerate() {
            let mut folder = WatchedFolder::new(path);
            folder.include_subfolders = subfolders.get(index).copied().flatten();
            shared.state.watched_folders.push(folder);
        }
        watch_glue::sync_watchers(app, shared);
    });

    // Populate watched folders, then re-apply the session's selection and
    // overrides to folder-discovered items by path (first occurrence wins).
    watch_glue::rescan_all(app);
    let mut folder_prefs: HashMap<PathBuf, (bool, Option<String>)> = HashMap::new();
    for entry in session.files.iter().filter(|entry| entry.is_from_folder) {
        folder_prefs
            .entry(standardized(&entry.path))
            .or_insert((entry.is_selected, entry.override_name.clone()));
    }
    if !folder_prefs.is_empty() {
        state.mutate(|shared| {
            for item in &mut shared.state.files {
                if item.folder_id.is_none() {
                    continue;
                }
                if let Some((selected, override_name)) = folder_prefs.get(&item.path) {
                    item.is_selected = *selected;
                    item.override_name = override_name.clone();
                }
            }
        });
    }

    if missing > 0 {
        let verb = if missing == 1 { "was" } else { "were" };
        let plural = if missing == 1 { "" } else { "s" };
        state.alert(
            "info",
            "Some files were missing",
            &format!(
                "{missing} file{plural} from your last session {verb} missing and removed from the list. Files on disk aren’t changed."
            ),
        );
    }
}
