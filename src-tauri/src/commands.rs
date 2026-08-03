//! `#[tauri::command]` shims (§12.1) — argument marshaling ONLY; every body
//! is one call into `crates/*` or the state container.

use std::collections::HashSet;
use std::path::PathBuf;

use nameshift_engine::csv::{csv_dry_run as engine_csv_dry_run, CsvMatch, CsvMatchReport};
use nameshift_engine::{
    copy as engine_copy, host_profile, rule_derived_name as engine_rule_derived_name, FileSortKey,
    ListMode, RenameRule, RulePreset,
};
use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::app_state::{
    apply_disabled_reason, AppState, InspectorPayload, PlatformInfo, PreviewPayload,
    RevertPreviewPayload, SnapshotMeta, StateSnapshot, UndoDelta, UndoEntry,
};
use crate::error::{AppError, AppResult};
use crate::{watch_glue, worker};

// ---- state-reading ---------------------------------------------------------

#[tauri::command]
pub fn get_state(state: State<AppState>) -> StateSnapshot {
    state.read(|shared| StateSnapshot::capture(shared))
}

#[tauri::command]
pub fn get_preview(state: State<AppState>, known_version: Option<u64>) -> PreviewPayload {
    let _ = known_version; // the version-keyed cache makes recompute free (§7.8)
    state.read(|shared| PreviewPayload {
        version: shared.state.version,
        preview: shared.preview().clone(),
        apply_disabled_reason: apply_disabled_reason(shared),
    })
}

#[tauri::command]
pub fn get_revert_preview(state: State<AppState>) -> RevertPreviewPayload {
    state.read(|shared| RevertPreviewPayload {
        version: shared.state.version,
        preview: shared.revert_preview().clone(),
    })
}

#[tauri::command]
pub fn get_history(state: State<AppState>) -> Vec<SnapshotMeta> {
    state.read(|shared| {
        shared
            .state
            .history
            .iter()
            .map(|snapshot| SnapshotMeta {
                id: snapshot.id,
                date: snapshot.date,
                summary: snapshot.summary.clone(),
                entry_count: snapshot.entries.len() as u32,
            })
            .collect()
    })
}

#[tauri::command]
pub fn get_file_metadata(state: State<AppState>, id: Uuid) -> AppResult<InspectorPayload> {
    state.read(|shared| {
        let item = shared
            .state
            .files
            .iter()
            .find(|item| item.id == id)
            .ok_or_else(|| AppError::new("That file is no longer in the list."))?;
        let provider = shared.metadata.provider();
        let stringify_date = |time: std::time::SystemTime| {
            nameshift_engine::format_date(
                &chrono::DateTime::<chrono::Local>::from(time),
                nameshift_engine::tokens::DEFAULT_DATE_PATTERN,
            )
        };
        Ok(InspectorPayload {
            name: item.name(),
            path: item.path.to_string_lossy().into_owned(),
            is_directory: item.is_directory,
            kind: provider.kind(&item.path),
            size: provider.size(&item.path).map(nameshift_engine::format_size),
            created: provider.created(&item.path).map(stringify_date),
            modified: provider.modified(&item.path).map(stringify_date),
            folder: item.directory().to_string_lossy().into_owned(),
            attributes: provider
                .all_attributes(&item.path)
                .into_iter()
                .map(|(name, value)| crate::app_state::AttributeEntry { name, value })
                .collect(),
        })
    })
}

#[tauri::command]
pub fn rule_derived_name(state: State<AppState>, id: Uuid) -> String {
    state.read(|shared| {
        engine_rule_derived_name(&shared.state, host_profile(), &shared.metadata, id)
    })
}

#[tauri::command]
pub fn get_platform(app: AppHandle) -> PlatformInfo {
    let channel = if cfg!(feature = "channel-mas") {
        "mas"
    } else if cfg!(feature = "channel-msstore") {
        "msstore"
    } else if cfg!(feature = "channel-flathub") {
        "flathub"
    } else {
        "direct"
    };
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        channel: channel.to_string(),
        has_spotlight: cfg!(target_os = "macos"),
        version: app.package_info().version.to_string(),
    }
}

// ---- import / list ---------------------------------------------------------

#[tauri::command]
pub fn import_paths(app: AppHandle, paths: Vec<String>) {
    watch_glue::import_paths(&app, paths);
}

#[tauri::command]
pub async fn pick_and_import(app: AppHandle) {
    // §14.9: files and directories in one panel isn't offered by the
    // cross-platform dialog; files here, folders via pick_and_import_folders
    // and drag-drop (docs/DEVIATIONS.md).
    let picked = app
        .dialog()
        .file()
        .set_title("Choose files to rename, or folders to watch")
        .blocking_pick_files();
    if let Some(paths) = picked {
        let raw: Vec<String> = paths
            .into_iter()
            .filter_map(|p| p.into_path().ok())
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        watch_glue::import_paths(&app, raw);
    }
}

#[tauri::command]
pub async fn pick_and_import_folders(app: AppHandle) {
    let picked = app
        .dialog()
        .file()
        .set_title("Choose files to rename, or folders to watch")
        .blocking_pick_folders();
    if let Some(paths) = picked {
        let raw: Vec<String> = paths
            .into_iter()
            .filter_map(|p| p.into_path().ok())
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        watch_glue::import_paths(&app, raw);
    }
}

fn record_list_undo(shared: &mut crate::app_state::Shared, action_name: &str) {
    shared.undo.record(UndoEntry {
        action_name: action_name.to_string(),
        delta: UndoDelta::List {
            files: shared.state.files.clone(),
            watched_folders: shared.state.watched_folders.clone(),
            excluded_paths: shared.state.excluded_paths.clone(),
        },
    });
}

#[tauri::command]
pub fn remove_items(app: AppHandle, state: State<AppState>, ids: Vec<Uuid>) {
    let id_set: HashSet<Uuid> = ids.into_iter().collect();
    state.mutate(|shared| {
        record_list_undo(shared, "Remove from List");
        // Removed watched-folder items become exclusions so rescans skip
        // them (§4.1 excluded_paths).
        for item in shared
            .state
            .files
            .iter()
            .filter(|item| id_set.contains(&item.id))
        {
            if item.folder_id.is_some() {
                shared.state.excluded_paths.insert(item.path.clone());
            }
        }
        shared.state.files.retain(|item| !id_set.contains(&item.id));
        shared.invalidate_disk_caches();
        watch_glue::sync_watchers(&app, shared);
    });
}

#[tauri::command]
pub fn remove_watched_folder(app: AppHandle, state: State<AppState>, id: Uuid) {
    state.mutate(|shared| {
        record_list_undo(shared, "Stop Watching");
        shared
            .state
            .watched_folders
            .retain(|folder| folder.id != id);
        shared.state.files.retain(|item| item.folder_id != Some(id));
        shared.invalidate_disk_caches();
        watch_glue::sync_watchers(&app, shared);
    });
}

#[tauri::command]
pub fn clear_all(app: AppHandle, state: State<AppState>) {
    state.mutate(|shared| {
        record_list_undo(shared, "Clear File List");
        shared.state.files.clear();
        shared.state.watched_folders.clear();
        shared.state.excluded_paths.clear();
        shared.invalidate_disk_caches();
        watch_glue::sync_watchers(&app, shared);
    });
    state.save_session_now();
}

#[tauri::command]
pub fn rescan_watched_folders(app: AppHandle) {
    watch_glue::rescan_all(&app);
}

// ---- inclusion (checkboxes; never row selection — §13.1) -------------------

#[tauri::command]
pub fn set_selected(state: State<AppState>, id: Uuid, selected: bool) {
    state.mutate(|shared| {
        if let Some(item) = shared.state.files.iter_mut().find(|item| item.id == id) {
            item.is_selected = selected;
        }
    });
}

#[tauri::command]
pub fn set_selected_many(state: State<AppState>, ids: Vec<Uuid>, selected: bool) {
    let id_set: HashSet<Uuid> = ids.into_iter().collect();
    state.mutate(|shared| {
        for item in &mut shared.state.files {
            if id_set.contains(&item.id) {
                item.is_selected = selected;
            }
        }
    });
}

#[tauri::command]
pub fn set_all_selected(state: State<AppState>, selected: bool) {
    state.mutate(|shared| {
        let folders = shared.state.list_mode == ListMode::Folders;
        for item in &mut shared.state.files {
            if item.is_directory == folders {
                item.is_selected = selected;
            }
        }
    });
}

#[tauri::command]
pub fn invert_selection(state: State<AppState>) {
    state.mutate(|shared| {
        let folders = shared.state.list_mode == ListMode::Folders;
        for item in &mut shared.state.files {
            if item.is_directory == folders {
                item.is_selected = !item.is_selected;
            }
        }
    });
}

#[tauri::command]
pub fn select_only(state: State<AppState>, ids: Vec<Uuid>) {
    let id_set: HashSet<Uuid> = ids.into_iter().collect();
    state.mutate(|shared| {
        let folders = shared.state.list_mode == ListMode::Folders;
        for item in &mut shared.state.files {
            if item.is_directory == folders {
                item.is_selected = id_set.contains(&item.id);
            }
        }
    });
}

#[tauri::command]
pub fn deselect_conflicted(state: State<AppState>) {
    state.mutate(|shared| {
        let conflicted: HashSet<Uuid> = shared
            .preview()
            .entries
            .iter()
            .filter(|entry| entry.problem.is_some())
            .map(|entry| entry.id)
            .collect();
        for item in &mut shared.state.files {
            if conflicted.contains(&item.id) {
                item.is_selected = false;
            }
        }
    });
}

// ---- rules -----------------------------------------------------------------

#[tauri::command]
pub fn set_rules(state: State<AppState>, rules: Vec<RenameRule>) {
    // Full-array idempotent set from the editing UI — NOT undo-recorded
    // (§13.3: text-input undo belongs to the OS text machinery).
    state.mutate(|shared| {
        if !rules.is_empty() {
            shared.state.rules_cleared_by_apply = false;
        }
        shared.state.rules = rules;
    });
}

fn replace_rules(
    state: &AppState,
    rules: Vec<RenameRule>,
    trims_whitespace: bool,
    action_name: &str,
    cleared_by_apply: bool,
) {
    state.mutate(|shared| {
        shared.undo.record(UndoEntry {
            action_name: action_name.to_string(),
            delta: UndoDelta::Rules {
                rules: shared.state.rules.clone(),
                trims_whitespace: shared.state.trims_whitespace,
                rules_cleared_by_apply: shared.state.rules_cleared_by_apply,
            },
        });
        shared.state.rules = rules;
        shared.state.trims_whitespace = trims_whitespace;
        shared.state.rules_cleared_by_apply = cleared_by_apply;
        shared.rules_revision += 1;
    });
}

#[tauri::command]
pub fn replace_rules_undoable(
    state: State<AppState>,
    rules: Vec<RenameRule>,
    trims_whitespace: bool,
    action_name: String,
) {
    replace_rules(&state, rules, trims_whitespace, &action_name, false);
}

/// The post-apply clear (§8.5 step 5): through the undoable funnel, flagged
/// for the rules panel's post-apply empty state.
pub fn clear_rules_after_apply(state: &AppState) {
    let trims = state.read(|shared| shared.state.trims_whitespace);
    replace_rules(state, Vec::new(), trims, "Clear Rules", true);
}

#[tauri::command]
pub fn undo(app: AppHandle, state: State<AppState>) {
    state.mutate(|shared| {
        let Some(entry) = shared.undo.undo.pop() else {
            return;
        };
        let redo_entry = capture_counterpart(shared, &entry);
        apply_delta(shared, entry.delta);
        shared.undo.redo.push(redo_entry);
        watch_glue::sync_watchers(&app, shared);
    });
}

#[tauri::command]
pub fn redo(app: AppHandle, state: State<AppState>) {
    state.mutate(|shared| {
        let Some(entry) = shared.undo.redo.pop() else {
            return;
        };
        let undo_entry = capture_counterpart(shared, &entry);
        apply_delta(shared, entry.delta);
        shared.undo.undo.push(undo_entry);
        watch_glue::sync_watchers(&app, shared);
    });
}

fn capture_counterpart(shared: &crate::app_state::Shared, entry: &UndoEntry) -> UndoEntry {
    let delta = match &entry.delta {
        UndoDelta::Rules { .. } => UndoDelta::Rules {
            rules: shared.state.rules.clone(),
            trims_whitespace: shared.state.trims_whitespace,
            rules_cleared_by_apply: shared.state.rules_cleared_by_apply,
        },
        UndoDelta::List { .. } => UndoDelta::List {
            files: shared.state.files.clone(),
            watched_folders: shared.state.watched_folders.clone(),
            excluded_paths: shared.state.excluded_paths.clone(),
        },
    };
    UndoEntry {
        action_name: entry.action_name.clone(),
        delta,
    }
}

fn apply_delta(shared: &mut crate::app_state::Shared, delta: UndoDelta) {
    match delta {
        UndoDelta::Rules {
            rules,
            trims_whitespace,
            rules_cleared_by_apply,
        } => {
            shared.state.rules = rules;
            shared.state.trims_whitespace = trims_whitespace;
            shared.state.rules_cleared_by_apply = rules_cleared_by_apply;
            shared.rules_revision += 1;
        }
        UndoDelta::List {
            files,
            watched_folders,
            excluded_paths,
        } => {
            // Restores selection and overrides byte-for-byte; the caller
            // re-arms watchers for restored roots (§13.3).
            shared.state.files = files;
            shared.state.watched_folders = watched_folders;
            shared.state.excluded_paths = excluded_paths;
            shared.invalidate_disk_caches();
        }
    }
}

#[tauri::command]
pub fn set_trims_whitespace(state: State<AppState>, trims: bool) {
    state.mutate(|shared| shared.state.trims_whitespace = trims);
}

// ---- options ---------------------------------------------------------------

#[tauri::command]
pub fn set_auto_resolve(state: State<AppState>, enabled: bool) {
    state.mutate(|shared| shared.state.auto_resolves_conflicts = enabled);
}

#[tauri::command]
pub fn set_keep_rules(state: State<AppState>, enabled: bool) {
    state.mutate(|shared| shared.state.keep_rules_after_apply = enabled);
}

#[tauri::command]
pub fn set_include_subfolders(app: AppHandle, state: State<AppState>, enabled: bool) {
    let followers: Vec<Uuid> = state.mutate(|shared| {
        shared.state.include_subfolders = enabled;
        shared
            .state
            .watched_folders
            .iter()
            .filter(|folder| folder.include_subfolders.is_none())
            .map(|folder| folder.id)
            .collect()
    });
    // Toggling the global rescans the roots that follow it (§9).
    for id in followers {
        watch_glue::rescan_one(&app, id);
    }
}

#[tauri::command]
pub fn set_watched_folder_subfolders(
    app: AppHandle,
    state: State<AppState>,
    id: Uuid,
    include: Option<bool>,
) {
    state.mutate(|shared| {
        if let Some(folder) = shared.state.watched_folders.iter_mut().find(|f| f.id == id) {
            folder.include_subfolders = include;
        }
    });
    watch_glue::rescan_one(&app, id);
}

#[tauri::command]
pub fn set_sort(state: State<AppState>, key: FileSortKey, ascending: bool) {
    state.mutate(|shared| {
        shared.state.sort_key = key;
        shared.state.sort_ascending = ascending;
    });
}

#[tauri::command]
pub fn set_list_mode(state: State<AppState>, mode: ListMode) {
    state.mutate(|shared| {
        shared.state.list_mode = mode;
        // Extension sort is meaningless for folders; switching resets it
        // (§7.2).
        if mode == ListMode::Folders && shared.state.sort_key == FileSortKey::FileExtension {
            shared.state.sort_key = FileSortKey::OrderAdded;
        }
    });
}

// ---- overrides -------------------------------------------------------------

#[tauri::command]
pub fn set_override(state: State<AppState>, id: Uuid, name: Option<String>) {
    state.mutate(|shared| {
        if let Some(item) = shared.state.files.iter_mut().find(|item| item.id == id) {
            item.override_name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        }
    });
}

#[tauri::command]
pub fn clear_all_overrides(state: State<AppState>) {
    state.mutate(|shared| {
        record_list_undo(shared, "Clear All Manual Edits");
        let folders = shared.state.list_mode == ListMode::Folders;
        for item in &mut shared.state.files {
            if item.is_directory == folders {
                item.override_name = None;
            }
        }
    });
}

// ---- apply / revert --------------------------------------------------------

#[tauri::command]
pub fn apply(app: AppHandle) {
    worker::start_apply(app);
}

#[tauri::command]
pub fn select_snapshot(state: State<AppState>, id: Option<Uuid>) {
    state.mutate(|shared| shared.state.selected_snapshot_id = id);
}

#[tauri::command]
pub fn revert_selected(app: AppHandle) {
    worker::start_revert(app);
}

#[tauri::command]
pub fn cancel_processing(state: State<AppState>) {
    worker::cancel_processing(&state);
}

#[tauri::command]
pub fn clear_history(state: State<AppState>) {
    state.mutate(|shared| {
        shared.state.history.clear();
        shared.state.selected_snapshot_id = None;
    });
    state.save_session_now();
}

// ---- presets (§14.8) -------------------------------------------------------

#[tauri::command]
pub fn preset_name_exists(state: State<AppState>, name: String) -> bool {
    state.read(|shared| {
        shared
            .state
            .presets
            .iter()
            .any(|preset| preset.name == name)
    })
}

#[tauri::command]
pub fn save_preset(state: State<AppState>, name: String, resolution: String) {
    state.mutate(|shared| {
        let mut final_name = name.clone();
        if resolution == "keepBoth" {
            let mut suffix = 2;
            while shared
                .state
                .presets
                .iter()
                .any(|preset| preset.name == final_name)
            {
                final_name = format!("{name} {suffix}");
                suffix += 1;
            }
        } else {
            shared.state.presets.retain(|preset| preset.name != name);
        }
        shared.state.presets.push(RulePreset {
            id: Uuid::new_v4(),
            name: final_name,
            rules: shared.state.rules.clone(),
            trims_whitespace: shared.state.trims_whitespace,
        });
    });
    state.save_session_now();
}

#[tauri::command]
pub fn apply_preset(state: State<AppState>, id: Uuid) {
    let Some(preset) =
        state.read(|shared| shared.state.presets.iter().find(|p| p.id == id).cloned())
    else {
        return;
    };
    // Loading regenerates rule ids (§14.8).
    let rules: Vec<RenameRule> = preset
        .rules
        .into_iter()
        .map(|mut rule| {
            rule.id = Uuid::new_v4();
            rule
        })
        .collect();
    replace_rules(
        &state,
        rules,
        preset.trims_whitespace,
        "Apply Preset",
        false,
    );
}

#[tauri::command]
pub fn delete_preset(state: State<AppState>, id: Uuid) {
    state.mutate(|shared| shared.state.presets.retain(|preset| preset.id != id));
    state.save_session_now();
}

#[tauri::command]
pub async fn import_presets(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("Presets", &["json"])
        .set_title("Choose a presets file exported from Name Shift")
        .blocking_pick_file()
    else {
        return Ok(());
    };
    let path = path.into_path().map_err(|e| AppError::new(e.to_string()))?;
    let bytes = std::fs::read(&path)?;
    let imported: Vec<RulePreset> = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::new("That file isn’t a Name Shift presets export."))?;
    let count = imported.len();
    state.mutate(|shared| {
        for mut preset in imported {
            preset.id = Uuid::new_v4();
            // Import appends, suffixing “ (imported)” on collisions (§14.8).
            if shared
                .state
                .presets
                .iter()
                .any(|existing| existing.name == preset.name)
            {
                preset.name = format!("{} (imported)", preset.name);
            }
            shared.state.presets.push(preset);
        }
    });
    state.save_session_now();
    let plural = if count == 1 { "" } else { "s" };
    state.alert(
        "info",
        "Presets imported",
        &format!("Imported {count} preset{plural}."),
    );
    Ok(())
}

#[tauri::command]
pub async fn export_presets(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("Presets", &["json"])
        .set_file_name("Name Shift Presets.json")
        .blocking_save_file()
    else {
        return Ok(());
    };
    let path = path.into_path().map_err(|e| AppError::new(e.to_string()))?;
    let presets = state.read(|shared| shared.state.presets.clone());
    let json = serde_json::to_string_pretty(&presets).map_err(|e| AppError::new(e.to_string()))?;
    std::fs::write(path, json)?;
    Ok(())
}

// ---- spreadsheet (§14.7) ---------------------------------------------------

#[tauri::command]
pub async fn export_csv_template(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let names: Vec<String> = state.read(|shared| {
        shared
            .preview()
            .entries
            .iter()
            .map(|e| e.current_name.clone())
            .collect()
    });
    if names.is_empty() {
        return Ok(());
    }
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("CSV", &["csv"])
        .set_file_name("Rename Template.csv")
        .set_title("Edit the New Name column, then bring it back with Import Edited CSV")
        .blocking_save_file()
    else {
        return Ok(());
    };
    let path = path.into_path().map_err(|e| AppError::new(e.to_string()))?;
    std::fs::write(path, nameshift_engine::csv::build_name_mapping_csv(&names))?;
    Ok(())
}

#[tauri::command]
pub fn csv_dry_run(state: State<AppState>, path: String) -> AppResult<CsvMatchReport> {
    let bytes = std::fs::read(PathBuf::from(&path))
        .map_err(|_| AppError::new("Couldn’t read that file."))?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(&text);
    state.read(|shared| {
        engine_csv_dry_run(&shared.state, text).map_err(|_| {
            AppError::new("Its first row must be the exported header “Current Name, New Name”.")
        })
    })
}

#[tauri::command]
pub fn csv_apply(state: State<AppState>, matches: Vec<CsvMatch>) {
    state.mutate(|shared| {
        // The whole import = one undo entry (§13.3).
        record_list_undo(shared, "Import Edited CSV");
        for entry in matches {
            if let Some(item) = shared
                .state
                .files
                .iter_mut()
                .find(|item| item.id == entry.id)
            {
                item.override_name = Some(entry.new_name);
                item.is_selected = true;
            }
        }
    });
}

#[tauri::command]
pub fn copy_preview_tsv(state: State<AppState>) -> AppResult<()> {
    let tsv = state.read(|shared| {
        let os = host_profile().os;
        let mut lines = vec!["Current Name\tNew Name\tStatus".to_string()];
        let entries = shared.preview().entries.clone();
        for entry in entries {
            let status = if let Some(problem) = entry.problem {
                format!(
                    "Naming conflict: {}",
                    engine_copy::problem_message(problem, os)
                )
            } else if !entry.is_selected {
                "Skipped (deselected)".to_string()
            } else if entry.is_changed && entry.has_override {
                "Will change (manual edit)".to_string()
            } else if entry.is_changed {
                "Will change".to_string()
            } else {
                "No change".to_string()
            };
            lines.push(format!(
                "{}\t{}\t{status}",
                entry.current_name, entry.new_name
            ));
        }
        lines.join("\n")
    });
    let app = state.app().clone();
    app.clipboard()
        .write_text(tsv)
        .map_err(|e| AppError::new(e.to_string()))
}

// ---- misc ------------------------------------------------------------------

#[tauri::command]
pub fn reveal_in_file_manager(state: State<AppState>, id: Uuid) -> AppResult<()> {
    let Some(path) = state.read(|shared| {
        shared
            .state
            .files
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.path.clone())
    }) else {
        return Ok(());
    };
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|e| AppError::new(e.to_string()))
}

#[tauri::command]
pub fn open_help(app: AppHandle, topic: Option<String>) {
    crate::help_window::open(&app, topic);
}

#[tauri::command]
pub fn save_session_now(state: State<AppState>) {
    state.save_session_now();
}
