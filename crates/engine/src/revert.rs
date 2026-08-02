//! Revert: simulation (§8.8) and execution (§8.7).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::execute::{perform_moves_hierarchical, rewrite_live_paths, rewrite_path_prefix};
use crate::item::name_of;
use crate::state::{CoreState, Snapshot};

/// What a revert would do to one recorded rename (§8.8).
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RevertStatus {
    /// The rename will be restored.
    Ok,
    /// The renamed file is gone — nothing to restore.
    Missing,
    /// The original name is taken by a different file — kept as is.
    NameTaken,
}

/// One row of the revert preview (§8.8).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RevertEntry {
    /// `{snapshotId}-{offset}` — stable row identity for the UI.
    pub id: String,
    /// The snapshot this entry belongs to.
    #[serde(with = "crate::serde_util::uuid_upper")]
    #[ts(as = "String")]
    pub snapshot_id: Uuid,
    /// When that snapshot was applied.
    #[serde(with = "crate::serde_util::utc_seconds")]
    #[ts(as = "String")]
    pub snapshot_date: DateTime<Utc>,
    /// The directory the rename lives in (for grouping).
    pub directory_path: PathBuf,
    /// The name the file has now.
    pub current_name: String,
    /// The name the revert would restore.
    pub restored_name: String,
    /// What the revert will do with this entry.
    pub status: RevertStatus,
}

/// The full revert-preview payload with its derived counts (§8.8).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RevertPreview {
    /// Every affected rename, newest snapshot first.
    pub entries: Vec<RevertEntry>,
    /// Renames that will be restored (`Ok` entries).
    pub restorable_rename_count: u32,
    /// Distinct files restored — `Ok` chain heads: N versions touching one
    /// file count as 1 file, not N.
    pub restorable_file_count: u32,
    /// Entries whose original name is taken.
    pub name_taken_count: u32,
    /// How many newer snapshots the revert also undoes.
    pub newer_snapshot_count: u32,
}

/// Simulate reverting through `selected_id` without touching disk (§8.8).
/// `really_exists` is the filesystem oracle (injectable for tests).
pub fn compute_revert_preview(
    history: &[Snapshot],
    selected_id: Uuid,
    really_exists: &dyn Fn(&Path) -> bool,
) -> RevertPreview {
    let Some(selected_index) = history.iter().position(|s| s.id == selected_id) else {
        return RevertPreview {
            entries: Vec::new(),
            restorable_rename_count: 0,
            restorable_file_count: 0,
            name_taken_count: 0,
            newer_snapshot_count: 0,
        };
    };

    // Simulated successful moves, oldest-simulated first. The virtual
    // existence check walks them newest-first: a path at a move's `to` is
    // remapped to the `from` side (directory moves carry their contents); a
    // path at a `from` has been moved away; anything else asks the real
    // filesystem.
    let mut completed_moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    let exists = |completed: &[(PathBuf, PathBuf)], path: &Path| -> bool {
        let mut current = path.to_path_buf();
        for (from, to) in completed.iter().rev() {
            if let Some(rewritten) = rewrite_path_prefix(&current, to, from) {
                current = rewritten;
            } else if current.strip_prefix(from).is_ok() {
                return false;
            }
        }
        really_exists(&current)
    };

    let mut entries = Vec::new();
    for snapshot in &history[..=selected_index] {
        // The entries a same-snapshot revert will vacate: reverting a move
        // frees its `to` path (two-phase makes same-snapshot swaps safe).
        let vacated: Vec<&PathBuf> = snapshot.entries.iter().map(|entry| &entry.to).collect();
        for (offset, entry) in snapshot.entries.iter().rev().enumerate() {
            let target = &entry.to; // where the file is now
            let restored = &entry.from; // where the revert puts it back
            let status = if !exists(&completed_moves, target) {
                RevertStatus::Missing
            } else if !paths_equal_case_insensitive(restored, target)
                && !vacated.contains(&restored)
                && exists(&completed_moves, restored)
            {
                RevertStatus::NameTaken
            } else {
                completed_moves.push((target.clone(), restored.clone()));
                RevertStatus::Ok
            };
            entries.push(RevertEntry {
                id: format!("{}-{offset}", snapshot.id),
                snapshot_id: snapshot.id,
                snapshot_date: snapshot.date,
                directory_path: target.parent().map(Path::to_path_buf).unwrap_or_default(),
                current_name: name_of(target),
                restored_name: name_of(restored),
                status,
            });
        }
    }

    let ok_entries: Vec<&RevertEntry> = entries
        .iter()
        .filter(|entry| entry.status == RevertStatus::Ok)
        .collect();
    // Chain heads: ok entries whose current path is no other ok entry's
    // restored path — N versions touching one file is 1 file, not N.
    let restorable_file_count = ok_entries
        .iter()
        .filter(|entry| {
            let current = entry.directory_path.join(&entry.current_name);
            !ok_entries.iter().any(|other| {
                other.id != entry.id && other.directory_path.join(&other.restored_name) == current
            })
        })
        .count() as u32;

    RevertPreview {
        restorable_rename_count: ok_entries.len() as u32,
        restorable_file_count,
        name_taken_count: entries
            .iter()
            .filter(|entry| entry.status == RevertStatus::NameTaken)
            .count() as u32,
        newer_snapshot_count: selected_index as u32,
        entries,
    }
}

fn paths_equal_case_insensitive(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

/// What a revert run produced.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RevertOutcome {
    /// How many renames were restored.
    pub restored: u32,
    /// User-presentable error messages.
    pub errors: Vec<String>,
    /// Watched folders whose root path changed — re-arm their watchers.
    pub rearmed_folder_ids: Vec<Uuid>,
}

/// Revert every snapshot from the newest through `snapshot_id` (§8.7):
/// each snapshot's moves reversed (entry order reversed, from↔to swapped),
/// children first (`parents_first = false`) so nested batches undo inner
/// renames before their parents, whose recorded paths become valid again.
/// Fully-processed snapshots are removed from history; a snapshot cut short
/// by cancellation stays (its already-reverted entries will simply skip as
/// missing sources on a later re-revert).
pub fn revert_through(
    state: &mut CoreState,
    snapshot_id: Uuid,
    is_cancelled: &dyn Fn() -> bool,
    on_progress: &mut dyn FnMut(usize, usize),
) -> RevertOutcome {
    let Some(selected_index) = state.history.iter().position(|s| s.id == snapshot_id) else {
        return RevertOutcome::default();
    };
    let snapshot_ids: Vec<Uuid> = state.history[..=selected_index]
        .iter()
        .map(|snapshot| snapshot.id)
        .collect();
    let total: usize = state.history[..=selected_index]
        .iter()
        .map(|snapshot| snapshot.entries.len())
        .sum();

    let mut outcome = RevertOutcome::default();
    let mut completed_base = 0usize;
    for id in snapshot_ids {
        if is_cancelled() {
            break;
        }
        let Some(index) = state.history.iter().position(|s| s.id == id) else {
            continue;
        };
        let moves: Vec<(PathBuf, PathBuf)> = state.history[index]
            .entries
            .iter()
            .rev()
            .map(|entry| (entry.to.clone(), entry.from.clone()))
            .collect();
        let base = completed_base;
        let result = perform_moves_hierarchical(&moves, false, is_cancelled, &mut |done| {
            on_progress(base + done, total)
        });
        completed_base += result.succeeded.len();
        outcome.restored += result.succeeded.len() as u32;
        for folder_id in rewrite_live_paths(state, &result.succeeded) {
            if !outcome.rearmed_folder_ids.contains(&folder_id) {
                outcome.rearmed_folder_ids.push(folder_id);
            }
        }
        outcome.errors.extend(result.errors);
        if result.attempted < moves.len() {
            // Cut short by cancellation — the partially-reverted snapshot
            // stays in history (§8.7).
            break;
        }
        state.history.remove(index);
    }
    outcome
}

/// Insert a freshly applied snapshot at history position 0 and enforce the
/// cap (§8.5 step 3).
pub fn record_snapshot(
    state: &mut CoreState,
    summary: String,
    successes: &[(PathBuf, PathBuf)],
) -> Uuid {
    let snapshot = Snapshot {
        id: Uuid::new_v4(),
        date: Utc::now(),
        summary,
        entries: successes
            .iter()
            .map(|(from, to)| crate::state::SnapshotEntry {
                from: from.clone(),
                to: to.clone(),
            })
            .collect(),
    };
    let id = snapshot.id;
    state.history.insert(0, snapshot);
    state.history.truncate(crate::state::MAX_HISTORY_COUNT);
    id
}
