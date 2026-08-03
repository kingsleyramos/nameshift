//! The Apply/Revert workers (§8.6): exactly one in flight, cancellation via
//! a shared flag, moves performed OUTSIDE the state lock and committed
//! inside it. The synchronous engine functions are the tested reference
//! path; these wrappers add only threading, progress, and cancel.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use nameshift_engine::{
    build_plan, finish_apply, perform_moves_hierarchical, rewrite_live_paths, ListMode,
};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::app_state::{AppState, ProcessingHandle};
use crate::events;
use crate::watch_glue;

fn emit_progress(app: &AppHandle, title: &str, completed: usize, total: usize) {
    let _ = app.emit(
        events::PROCESSING,
        events::Processing {
            title: title.to_string(),
            completed: completed as u32,
            total: total as u32,
        },
    );
}

fn emit_done(app: &AppHandle) {
    let _ = app.emit(events::PROCESSING, Option::<events::Processing>::None);
}

/// Surface batch errors: at most the first 8 + `…and N more.` (§8.5 step 7).
fn surface_errors(state: &AppState, errors: &[String]) {
    if errors.is_empty() {
        return;
    }
    let mut message = errors
        .iter()
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    if errors.len() > 8 {
        message.push_str(&format!("\n…and {} more.", errors.len() - 8));
    }
    state.alert("error", "Some renames couldn’t complete", &message);
}

/// Start the Apply worker. Rejects quietly when a worker is already in
/// flight (§8.6 — the toolbar double-click race is real).
pub fn start_apply(app: AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let Some((plan, title)) = state.read(|shared| {
        if shared.processing.is_some() {
            return None;
        }
        let plan = {
            let preview = shared.preview().clone();
            build_plan(&shared.state, &preview)?
        };
        let title = if plan.is_folders {
            "Renaming folders…"
        } else {
            "Renaming files…"
        };
        shared.processing = Some(ProcessingHandle {
            cancel: Arc::clone(&cancel),
            title: title.to_string(),
        });
        Some((plan, title.to_string()))
    }) else {
        return;
    };
    // The processing flag flipped: announce it (menu/action-bar gating).
    state.mutate(|_| {});

    let total = plan.moves.len();
    emit_progress(&app, &title, 0, total);
    std::thread::spawn(move || {
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        let progress_app = app.clone();
        let progress_title = title.clone();
        let cancel_flag = Arc::clone(&cancel);

        // Phase A — the moves, lock-free (§12.2 compute outside).
        let result = perform_moves_hierarchical(
            &plan.moves,
            true,
            &move || cancel_flag.load(Ordering::SeqCst),
            &mut |completed| emit_progress(&progress_app, &progress_title, completed, total),
        );

        // Phase B — commit into state under the lock (§8.5 order).
        let outcome = state.mutate(|shared| {
            let outcome = finish_apply(&mut shared.state, &plan, &result);
            shared.invalidate_disk_caches();
            watch_glue::rearm_watchers(&app, shared, &outcome.rearmed_folder_ids);
            shared.processing = None;
            outcome
        });

        // Step 4: the feedback event.
        let _ = app.emit(
            events::APPLY_FINISHED,
            events::ApplyFinished {
                count: outcome.renamed,
                snapshot_id: outcome.snapshot_id,
                is_folders: outcome.is_folders,
                cleared_rules: outcome.cleared_rules,
                kept_rules: outcome.kept_rules,
            },
        );
        // Step 5 (rules half): clear through the undoable funnel (§13.3).
        if outcome.cleared_rules {
            crate::commands::clear_rules_after_apply(&state);
        }
        // Step 7: surfaced errors.
        surface_errors(&state, &outcome.errors);
        emit_done(&app);
        // Step 8: session save.
        state.save_session_now();
    });
}

/// Start the Revert worker for the selected snapshot (§8.7): per snapshot
/// newest→selected, moves reversed, children first; fully-processed
/// snapshots leave history; a cancelled one stays.
pub fn start_revert(app: AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let Some((snapshot_ids, total)) = state.read(|shared| {
        if shared.processing.is_some() {
            return None;
        }
        let selected = shared.state.selected_snapshot_id?;
        let index = shared.state.history.iter().position(|s| s.id == selected)?;
        let ids: Vec<Uuid> = shared.state.history[..=index]
            .iter()
            .map(|s| s.id)
            .collect();
        let total: usize = shared.state.history[..=index]
            .iter()
            .map(|s| s.entries.len())
            .sum();
        shared.processing = Some(ProcessingHandle {
            cancel: Arc::clone(&cancel),
            title: "Reverting…".to_string(),
        });
        Some((ids, total))
    }) else {
        return;
    };
    state.mutate(|_| {});

    emit_progress(&app, "Reverting…", 0, total);
    std::thread::spawn(move || {
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        let mut restored = 0u32;
        let mut errors: Vec<String> = Vec::new();
        let mut completed_base = 0usize;
        let mut rearmed: Vec<Uuid> = Vec::new();

        for id in snapshot_ids {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            // Read the snapshot briefly; move files lock-free; commit.
            let Some(moves) = state.read(|shared| {
                shared
                    .state
                    .history
                    .iter()
                    .find(|s| s.id == id)
                    .map(|snapshot| {
                        snapshot
                            .entries
                            .iter()
                            .rev()
                            .map(|entry| (entry.to.clone(), entry.from.clone()))
                            .collect::<Vec<(PathBuf, PathBuf)>>()
                    })
            }) else {
                continue;
            };
            let cancel_flag = Arc::clone(&cancel);
            let progress_app = app.clone();
            let base = completed_base;
            let result = perform_moves_hierarchical(
                &moves,
                false,
                &move || cancel_flag.load(Ordering::SeqCst),
                &mut |done| emit_progress(&progress_app, "Reverting…", base + done, total),
            );
            completed_base += result.succeeded.len();
            restored += result.succeeded.len() as u32;
            errors.extend(result.errors.clone());
            let fully_processed = result.attempted >= moves.len();
            state.mutate(|shared| {
                for folder_id in rewrite_live_paths(&mut shared.state, &result.succeeded) {
                    if !rearmed.contains(&folder_id) {
                        rearmed.push(folder_id);
                    }
                }
                if fully_processed {
                    // Only cancellation keeps a snapshot (§8.7).
                    shared.state.history.retain(|s| s.id != id);
                }
                shared.invalidate_disk_caches();
            });
            if !fully_processed {
                break;
            }
        }

        state.mutate(|shared| {
            shared.state.selected_snapshot_id = None;
            watch_glue::rearm_watchers(&app, shared, &rearmed);
            shared.processing = None;
        });
        let _ = app.emit(events::REVERT_FINISHED, events::RevertFinished { restored });
        surface_errors(&state, &errors);
        emit_done(&app);
        state.save_session_now();
    });
}

/// Flip the shared cancellation flag (Esc / the overlay button).
pub fn cancel_processing(state: &AppState) {
    state.read(|shared| {
        if let Some(processing) = &shared.processing {
            processing.cancel.store(true, Ordering::SeqCst);
        }
    });
}

/// Which mode noun the worker titles use.
pub fn is_folders(state: &AppState) -> bool {
    state.read(|shared| shared.state.list_mode == ListMode::Folders)
}
