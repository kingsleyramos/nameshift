//! The Tauri shell: window setup, IPC command registration, and events.
//! All domain logic lives in `crates/*`; this crate only marshals.

/// Build and run the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
