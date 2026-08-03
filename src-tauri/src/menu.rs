//! Native menus (§15). Shortcut ownership rule (hard-learned): each
//! shortcut is bound in exactly ONE place — the menu item. Toolbar buttons
//! show shortcuts in tooltips but never register their own accelerator
//! (double-binding double-fired actions historically).
//!
//! Text-focus-sensitive items (Undo/Redo/Select All) route through the
//! frontend, which keeps native text-editing behavior when a field has
//! focus and applies the workspace action otherwise.

use nameshift_engine::ListMode;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Listener, Manager, Wry};

use crate::app_state::AppState;

/// Frontend-routed menu actions arrive on this event.
pub const MENU_EVENT: &str = "menu";

/// Handles to the items whose label/enabled state tracks the workspace.
pub struct MenuHandles {
    pub rename: MenuItem<Wry>,
    pub revert: MenuItem<Wry>,
    pub skip_conflicted: MenuItem<Wry>,
    pub rename_by_csv: MenuItem<Wry>,
    pub export_presets: MenuItem<Wry>,
    pub undo: MenuItem<Wry>,
    pub redo: MenuItem<Wry>,
    pub inclusion_items: Vec<MenuItem<Wry>>,
    pub clear_edits: MenuItem<Wry>,
    pub add: MenuItem<Wry>,
}

/// Build the menu bar and remember the dynamic items.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let handle = app.clone();

    let add = MenuItem::with_id(
        app,
        "add",
        "Add Files or Folders…",
        true,
        Some("CmdOrCtrl+O"),
    )?;
    let rename = MenuItem::with_id(
        app,
        "rename",
        "Rename Files",
        false,
        Some("CmdOrCtrl+Enter"),
    )?;
    let revert = MenuItem::with_id(
        app,
        "revert",
        "Revert Previewed Renames…",
        false,
        None::<&str>,
    )?;
    let skip_conflicted = MenuItem::with_id(
        app,
        "skip-conflicted",
        "Skip Conflicted",
        false,
        None::<&str>,
    )?;
    let rename_by_csv =
        MenuItem::with_id(app, "rename-by-csv", "Rename by CSV…", false, None::<&str>)?;
    let import_presets =
        MenuItem::with_id(app, "import-presets", "Import Presets…", true, None::<&str>)?;
    let export_presets = MenuItem::with_id(
        app,
        "export-presets",
        "Export Presets…",
        false,
        None::<&str>,
    )?;

    let undo = MenuItem::with_id(app, "undo", "Undo", false, Some("CmdOrCtrl+Z"))?;
    let redo = MenuItem::with_id(app, "redo", "Redo", false, Some("CmdOrCtrl+Shift+Z"))?;
    let select_all = MenuItem::with_id(app, "select-all", "Select All", true, Some("CmdOrCtrl+A"))?;
    let include_all = MenuItem::with_id(app, "include-all", "Include All", false, None::<&str>)?;
    let skip_all = MenuItem::with_id(app, "skip-all", "Skip All", false, None::<&str>)?;
    let invert = MenuItem::with_id(
        app,
        "invert-inclusion",
        "Invert Inclusion",
        false,
        None::<&str>,
    )?;
    let clear_edits = MenuItem::with_id(
        app,
        "clear-manual-edits",
        "Clear All Manual Edits (0)",
        false,
        None::<&str>,
    )?;
    let find = MenuItem::with_id(app, "find", "Find", true, Some("CmdOrCtrl+F"))?;

    let file_info = MenuItem::with_id(
        app,
        "toggle-inspector",
        "Show/Hide File Info",
        true,
        Some("CmdOrCtrl+I"),
    )?;
    let inline_diff =
        MenuItem::with_id(app, "inline-diff", "Inline Diff View", true, None::<&str>)?;

    let main_window =
        MenuItem::with_id(app, "main-window", "Name Shift", true, Some("CmdOrCtrl+0"))?;
    let help = MenuItem::with_id(
        app,
        "help",
        "Name Shift Help",
        true,
        Some(if cfg!(target_os = "macos") {
            "CmdOrCtrl+?"
        } else {
            "F1"
        }),
    )?;

    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &add,
            &PredefinedMenuItem::separator(app)?,
            &rename,
            &revert,
            &skip_conflicted,
            &PredefinedMenuItem::separator(app)?,
            &rename_by_csv,
            &PredefinedMenuItem::separator(app)?,
            &import_presets,
            &export_presets,
            #[cfg(not(target_os = "macos"))]
            &PredefinedMenuItem::separator(app)?,
            #[cfg(not(target_os = "macos"))]
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    #[cfg(not(target_os = "macos"))]
    let settings_item =
        MenuItem::with_id(app, "settings", "Preferences…", true, Some("CmdOrCtrl+,"))?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &undo,
            &redo,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &select_all,
            &PredefinedMenuItem::separator(app)?,
            &include_all,
            &skip_all,
            &invert,
            &clear_edits,
            &PredefinedMenuItem::separator(app)?,
            &find,
            #[cfg(not(target_os = "macos"))]
            &PredefinedMenuItem::separator(app)?,
            #[cfg(not(target_os = "macos"))]
            &settings_item,
        ],
    )?;

    let view_menu = Submenu::with_items(app, "View", true, &[&file_info, &inline_diff])?;

    let help_menu = Submenu::with_items(app, "Help", true, &[&help])?;

    #[cfg(target_os = "macos")]
    let menu = {
        let settings = MenuItem::with_id(app, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
        let app_menu = Submenu::with_items(
            app,
            "Name Shift",
            true,
            &[
                &PredefinedMenuItem::about(app, None, None)?,
                &PredefinedMenuItem::separator(app)?,
                &settings,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::services(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::hide(app, None)?,
                &PredefinedMenuItem::hide_others(app, None)?,
                &PredefinedMenuItem::show_all(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::quit(app, None)?,
            ],
        )?;
        let window_menu = Submenu::with_items(
            app,
            "Window",
            true,
            &[
                &PredefinedMenuItem::minimize(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &main_window,
            ],
        )?;
        Menu::with_items(
            app,
            &[
                &app_menu,
                &file_menu,
                &edit_menu,
                &view_menu,
                &window_menu,
                &help_menu,
            ],
        )?
    };
    #[cfg(not(target_os = "macos"))]
    let menu = {
        let window_menu = Submenu::with_items(app, "Window", true, &[&main_window])?;
        Menu::with_items(
            app,
            &[&file_menu, &edit_menu, &view_menu, &window_menu, &help_menu],
        )?
    };

    app.set_menu(menu)?;
    app.manage(std::sync::Mutex::new(MenuHandles {
        rename,
        revert,
        skip_conflicted,
        rename_by_csv,
        export_presets,
        undo,
        redo,
        inclusion_items: vec![include_all, skip_all, invert],
        clear_edits,
        add,
    }));

    // Keep labels and enablement live (§15 table).
    handle.listen(crate::events::STATE_CHANGED, {
        let handle = handle.clone();
        move |_| update(&handle)
    });
    update(&handle);
    Ok(())
}

/// Refresh dynamic labels/enablement from the workspace (§15 table).
pub fn update(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Some(handles) = app.try_state::<std::sync::Mutex<MenuHandles>>() else {
        return;
    };
    let (
        rename_label,
        can_apply,
        revert_enabled,
        conflict_count,
        active_count,
        idle,
        revert_preview,
        has_presets,
        undo_action,
        redo_action,
        override_count,
    ) = state.read(|shared| {
        let counts = shared.preview().counts;
        let is_folders = shared.state.list_mode == ListMode::Folders;
        let noun = if is_folders { "Folders" } else { "Files" };
        let label = if counts.change_count > 0 {
            format!("Rename {} {}", counts.change_count, noun)
        } else {
            format!("Rename {noun}")
        };
        let revert_restorable = shared.revert_preview().restorable_rename_count;
        (
            label,
            counts.can_apply,
            revert_restorable > 0,
            counts.conflict_count,
            shared.state.active_items().count(),
            shared.processing.is_none(),
            shared.state.selected_snapshot_id.is_some(),
            !shared.state.presets.is_empty(),
            shared
                .undo
                .undo
                .last()
                .map(|entry| entry.action_name.clone()),
            shared
                .undo
                .redo
                .last()
                .map(|entry| entry.action_name.clone()),
            shared.override_count(),
        )
    });
    let handles = handles
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ = handles.rename.set_text(rename_label);
    let _ = handles
        .rename
        .set_enabled(can_apply && idle && !revert_preview);
    let _ = handles
        .revert
        .set_enabled(revert_preview && revert_enabled && idle);
    let _ = handles.skip_conflicted.set_enabled(conflict_count > 0);
    let _ = handles.rename_by_csv.set_enabled(active_count > 0 && idle);
    let _ = handles.export_presets.set_enabled(has_presets);
    let _ = handles.add.set_enabled(idle && !revert_preview);
    let _ = handles.undo.set_text(
        undo_action
            .as_deref()
            .map_or("Undo".to_string(), |a| format!("Undo {a}")),
    );
    let _ = handles.undo.set_enabled(undo_action.is_some());
    let _ = handles.redo.set_text(
        redo_action
            .as_deref()
            .map_or("Redo".to_string(), |a| format!("Redo {a}")),
    );
    let _ = handles.redo.set_enabled(redo_action.is_some());
    for item in &handles.inclusion_items {
        let _ = item.set_enabled(active_count > 0);
    }
    let _ = handles
        .clear_edits
        .set_text(format!("Clear All Manual Edits ({override_count})"));
    let _ = handles.clear_edits.set_enabled(override_count > 0);
}

/// Dispatch a triggered menu item.
pub fn on_menu_event(app: &AppHandle, id: &str) {
    let forward = |action: &str| {
        let _ = app.emit(MENU_EVENT, action.to_string());
    };
    match id {
        "add" => {
            // §14.9's combined panel isn't available cross-platform; ⌘O opens
            // the file picker, folders come via the toolbar Add menu and
            // drag-drop (docs/DEVIATIONS.md).
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = state; // gate lives in import_paths
                }
                let _ = app.emit(MENU_EVENT, "pick-files".to_string());
            });
        }
        "rename" => crate::worker::start_apply(app.clone()),
        "revert" => forward("revert"),
        "skip-conflicted" => {
            if let Some(state) = app.try_state::<AppState>() {
                state.mutate(|shared| {
                    let conflicted: Vec<uuid::Uuid> = shared
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
        }
        "rename-by-csv" => forward("rename-by-csv"),
        "import-presets" | "export-presets" => forward(id),
        // Text-focus-sensitive: the frontend decides between native text
        // editing and the workspace stack (§15).
        "undo" | "redo" | "select-all" => forward(id),
        "include-all" | "skip-all" | "invert-inclusion" | "clear-manual-edits" => {
            if let Some(state) = app.try_state::<AppState>() {
                match id {
                    "include-all" | "skip-all" => {
                        let selected = id == "include-all";
                        state.mutate(|shared| {
                            let folders = shared.state.list_mode == ListMode::Folders;
                            for item in &mut shared.state.files {
                                if item.is_directory == folders {
                                    item.is_selected = selected;
                                }
                            }
                        });
                    }
                    "invert-inclusion" => {
                        state.mutate(|shared| {
                            let folders = shared.state.list_mode == ListMode::Folders;
                            for item in &mut shared.state.files {
                                if item.is_directory == folders {
                                    item.is_selected = !item.is_selected;
                                }
                            }
                        });
                    }
                    _ => forward("clear-manual-edits"),
                }
            }
        }
        "find" => forward("find"),
        "toggle-inspector" => forward("toggle-inspector"),
        "inline-diff" => forward("inline-diff"),
        "settings" => crate::help_window::open_settings(app),
        "main-window" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "help" => crate::help_window::open(app, None),
        _ => {}
    }
}
