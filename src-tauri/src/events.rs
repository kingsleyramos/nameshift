//! Typed event names + payload structs (§12.2).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// Emitted after every mutation — the frontend refetches (debounced ~30 ms).
pub const STATE_CHANGED: &str = "state-changed";
/// Worker progress; `null` payload = done.
pub const PROCESSING: &str = "processing";
/// Drives the apply feedback banner.
pub const APPLY_FINISHED: &str = "apply-finished";
/// Drives the revert feedback.
pub const REVERT_FINISHED: &str = "revert-finished";
/// Engine-originated alerts (watcher errors, restore notices).
pub const ALERT: &str = "alert";
/// Single-instance forward / macOS dock & Open-With paths.
pub const OPEN_PATHS: &str = "open-paths";

/// `state-changed` payload.
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[ts(export)]
pub struct StateChanged {
    /// The version after the mutation (§13.1 out-of-order guard).
    pub version: u64,
}

/// `processing` payload while a worker runs.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Processing {
    /// Overlay title (§8.6): `Renaming files…` / `Renaming folders…` /
    /// `Reverting…`.
    pub title: String,
    /// Items completed so far.
    pub completed: u32,
    /// Total items in the batch.
    pub total: u32,
}

/// `apply-finished` payload (§8.5 step 4): the cleared/kept flags are true
/// only if rules were actually non-empty, so applying pure manual edits
/// claims neither.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApplyFinished {
    /// How many items were renamed.
    pub count: u32,
    /// The recorded snapshot, when anything succeeded.
    #[serde(with = "nameshift_engine::serde_util::uuid_upper_opt")]
    #[ts(as = "Option<String>")]
    pub snapshot_id: Option<Uuid>,
    /// Whether this was a Folders-mode apply.
    pub is_folders: bool,
    /// The rule stack was cleared (⌘Z restores it).
    pub cleared_rules: bool,
    /// The rule stack was kept; renamed files deselected.
    pub kept_rules: bool,
}

/// `revert-finished` payload.
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RevertFinished {
    /// How many renames were restored.
    pub restored: u32,
}

/// `alert` payload.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Alert {
    /// `"error"` or `"info"`.
    pub kind: String,
    /// Alert title.
    pub title: String,
    /// Alert body.
    pub message: String,
}

/// `open-paths` payload.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OpenPaths {
    /// Absolute paths forwarded from a second instance or the OS.
    pub paths: Vec<String>,
}
