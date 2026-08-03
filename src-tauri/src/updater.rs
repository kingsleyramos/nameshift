//! The direct-channel updater (§20.1): a `Check for Updates…` menu item and
//! a quiet daily background check with the standard consent flow. Store
//! channels never compile this (§24 Q19).

#![cfg(feature = "channel-direct")]

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

/// Menu-triggered check: reports "up to date" too.
pub fn check_interactive(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        match fetch_update(&app).await {
            Ok(Some(update)) => offer(app, update),
            Ok(None) => {
                app.dialog()
                    .message("Name Shift is up to date.")
                    .kind(MessageDialogKind::Info)
                    .title("Name Shift")
                    .show(|_| {});
            }
            Err(error) => {
                app.dialog()
                    .message(format!("Couldn’t check for updates: {error}"))
                    .kind(MessageDialogKind::Error)
                    .title("Couldn’t complete that")
                    .show(|_| {});
            }
        }
    });
}

/// Background daily check — silent unless an update exists.
pub fn spawn_background_checks(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Ok(Some(update)) = fetch_update(&app).await {
                offer(app.clone(), update);
            }
            tokio_sleep_daily().await;
        }
    });
}

async fn fetch_update(
    app: &AppHandle,
) -> Result<Option<tauri_plugin_updater::Update>, tauri_plugin_updater::Error> {
    app.updater()?.check().await
}

/// The consent flow: ask, download + install, then offer a restart.
fn offer(app: AppHandle, update: tauri_plugin_updater::Update) {
    let version = update.version.clone();
    app.clone()
        .dialog()
        .message(format!(
            "Name Shift {version} is available. Download and install it now?"
        ))
        .title("Update available")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Install".to_string(),
            "Later".to_string(),
        ))
        .show(move |accepted| {
            if !accepted {
                return;
            }
            tauri::async_runtime::spawn(async move {
                match update.download_and_install(|_, _| {}, || {}).await {
                    Ok(()) => {
                        app.clone()
                            .dialog()
                            .message("The update is installed. Restart Name Shift to use it.")
                            .title("Update installed")
                            .buttons(MessageDialogButtons::OkCancelCustom(
                                "Restart".to_string(),
                                "Later".to_string(),
                            ))
                            .show(move |restart| {
                                if restart {
                                    app.restart();
                                }
                            });
                    }
                    Err(error) => {
                        app.dialog()
                            .message(format!("Couldn’t install the update: {error}"))
                            .kind(MessageDialogKind::Error)
                            .title("Couldn’t complete that")
                            .show(|_| {});
                    }
                }
            });
        });
}

async fn tokio_sleep_daily() {
    tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_secs(60 * 60 * 24));
    })
    .await
    .ok();
}
