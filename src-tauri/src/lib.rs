//! The Tauri shell: window setup, IPC command registration, and events.
//! All domain logic lives in `crates/*`; this crate only marshals.

pub mod access;
pub mod app_state;
pub mod commands;
pub mod error;
pub mod events;
pub mod help_window;
pub mod session_glue;
pub mod watch_glue;
pub mod worker;

use tauri::{Emitter, Manager};

use crate::app_state::AppState;

/// Build and run the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // A bare second launch focuses the window; argv paths forward to
            // the import gate (§24 Q21).
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            let paths: Vec<String> = argv.into_iter().skip(1).collect();
            if !paths.is_empty() {
                let _ = app.emit(events::OPEN_PATHS, events::OpenPaths { paths });
            }
        }))
        .setup(|app| {
            let store = nameshift_store::StorePaths::resolve();
            let state = AppState::new(app.handle().clone(), store, access::channel_access());
            app.manage(state);
            session_glue::restore(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::get_preview,
            commands::get_revert_preview,
            commands::get_history,
            commands::get_file_metadata,
            commands::rule_derived_name,
            commands::get_platform,
            commands::import_paths,
            commands::pick_and_import,
            commands::pick_and_import_folders,
            commands::remove_items,
            commands::remove_watched_folder,
            commands::clear_all,
            commands::rescan_watched_folders,
            commands::set_selected,
            commands::set_selected_many,
            commands::set_all_selected,
            commands::invert_selection,
            commands::select_only,
            commands::deselect_conflicted,
            commands::set_rules,
            commands::replace_rules_undoable,
            commands::undo,
            commands::redo,
            commands::set_trims_whitespace,
            commands::set_auto_resolve,
            commands::set_keep_rules,
            commands::set_include_subfolders,
            commands::set_watched_folder_subfolders,
            commands::set_sort,
            commands::set_list_mode,
            commands::set_override,
            commands::clear_all_overrides,
            commands::apply,
            commands::select_snapshot,
            commands::revert_selected,
            commands::cancel_processing,
            commands::clear_history,
            commands::preset_name_exists,
            commands::save_preset,
            commands::apply_preset,
            commands::delete_preset,
            commands::import_presets,
            commands::export_presets,
            commands::export_csv_template,
            commands::csv_dry_run,
            commands::csv_apply,
            commands::copy_preview_tsv,
            commands::reveal_in_file_manager,
            commands::open_help,
            commands::save_session_now,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if window.label() == "main" {
                    // Session save on exit (§4.3).
                    if let Some(state) = window.app_handle().try_state::<AppState>() {
                        state.save_session_now();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
