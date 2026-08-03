//! The Help window (§14.12): separate window, deep-linkable by topic.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Open (or focus) Help, optionally jumping to a topic.
pub fn open(app: &AppHandle, topic: Option<String>) {
    if let Some(window) = app.get_webview_window("help") {
        let _ = window.show();
        let _ = window.set_focus();
        if let Some(topic) = topic {
            let _ = app.emit_to("help", "help-topic", topic);
        }
        return;
    }
    let query = topic.map(|t| format!("?topic={t}")).unwrap_or_default();
    let url = format!("index.html{query}#/help");
    let result = WebviewWindowBuilder::new(app, "help", WebviewUrl::App(url.into()))
        .title("Name Shift Help")
        .inner_size(760.0, 540.0)
        .min_inner_size(640.0, 440.0)
        .build();
    if let Err(error) = result {
        tracing::warn!("couldn't open help: {error}");
    }
}

/// Open (or focus) the Settings window (§14.11).
pub fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let result = WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("index.html#/settings".into()),
    )
    .title("Name Shift Settings")
    .inner_size(480.0, 360.0)
    .resizable(false)
    .build();
    if let Err(error) = result {
        tracing::warn!("couldn't open settings: {error}");
    }
}
