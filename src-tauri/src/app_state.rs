//! The shell's state container: `Mutex<CoreState>` behind the one mutation
//! funnel (§13.2) — version bump = cache invalidation = event emission.
//! Never mutate `CoreState` outside [`AppState::mutate`]; never add a
//! second preview cache.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};

use nameshift_engine::{
    compute_preview, compute_revert_preview, copy as engine_copy, host_profile, DirectoryNameCache,
    FileItem, FileSortKey, FsDirectoryLister, ListMode, Preview, RenameRule, RevertPreview,
    RulePreset, WatchedFolder,
};
use nameshift_metadata::CachingMetadataSource;
use nameshift_store::StorePaths;
use nameshift_watcher::FolderWatcher;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;
use uuid::Uuid;

use crate::access::FileAccess;
use crate::events;

/// A running Apply/Revert: exactly one in flight (§8.6).
pub struct ProcessingHandle {
    /// Shared cancellation flag the worker polls.
    pub cancel: Arc<AtomicBool>,
    /// Overlay title.
    pub title: String,
}

/// One recorded undo step (§13.3).
pub struct UndoEntry {
    /// Edit-menu label: `Undo {action_name}`.
    pub action_name: String,
    /// What to restore.
    pub delta: UndoDelta,
}

/// The two delta kinds of the workspace undo stack (§13.3).
pub enum UndoDelta {
    /// Rules delta: recorded by rule delete, clear-stack, preset load, and
    /// the post-apply clear — never by field typing/reorder/enable-toggle.
    Rules {
        rules: Vec<RenameRule>,
        trims_whitespace: bool,
        rules_cleared_by_apply: bool,
    },
    /// List delta: recorded by removals, clear list, stop-watching,
    /// clear-all-overrides, and csv_apply.
    List {
        files: Vec<FileItem>,
        watched_folders: Vec<WatchedFolder>,
        excluded_paths: HashSet<PathBuf>,
    },
}

/// The workspace undo stack, capped at 100 (§13.3).
#[derive(Default)]
pub struct UndoStack {
    pub undo: Vec<UndoEntry>,
    pub redo: Vec<UndoEntry>,
}

impl UndoStack {
    pub fn record(&mut self, entry: UndoEntry) {
        self.undo.push(entry);
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
}

/// Everything the state lock guards.
pub struct Shared {
    /// The single source of truth (§13.1).
    pub state: nameshift_engine::CoreState,
    /// On-disk name sets, invalidated on list mutations (§7.5).
    pub disk: DirectoryNameCache,
    /// Lazy stat batches for tokens/sorting (§11.1).
    pub metadata: CachingMetadataSource,
    /// The one preview cache, keyed on `state.version` (§7.8).
    preview_cache: Option<(u64, Preview)>,
    /// The revert-preview cache, keyed the same way (§8.8).
    revert_cache: Option<(u64, RevertPreview)>,
    /// Workspace undo/redo (§13.3).
    pub undo: UndoStack,
    /// Live watcher handles by folder id.
    pub watchers: HashMap<Uuid, FolderWatcher>,
    /// The in-flight worker, if any.
    pub processing: Option<ProcessingHandle>,
    /// Where the persisted files live.
    pub store: StorePaths,
    /// The channel's file-access seam (§10.1).
    pub access: Box<dyn FileAccess>,
    /// Bumped whenever rules change from a non-typing source; the frontend
    /// reconciles its optimistic rules draft on it (§13.1).
    pub rules_revision: u64,
}

impl Shared {
    /// The cached preview for the current version (§7.8).
    pub fn preview(&mut self) -> &Preview {
        let version = self.state.version;
        let stale = !matches!(&self.preview_cache, Some((v, _)) if *v == version);
        if stale {
            let preview =
                compute_preview(&self.state, host_profile(), &mut self.disk, &self.metadata);
            self.preview_cache = Some((version, preview));
        }
        &self.preview_cache.as_ref().expect("just computed").1
    }

    /// The cached revert preview for the current version (§8.8); empty when
    /// no snapshot is selected.
    pub fn revert_preview(&mut self) -> &RevertPreview {
        let version = self.state.version;
        let stale = !matches!(&self.revert_cache, Some((v, _)) if *v == version);
        if stale {
            let preview = match self.state.selected_snapshot_id {
                Some(id) => compute_revert_preview(&self.state.history, id, &|path| {
                    nameshift_engine::platform::win_long_path(path)
                        .symlink_metadata()
                        .is_ok()
                }),
                None => RevertPreview {
                    entries: Vec::new(),
                    restorable_rename_count: 0,
                    restorable_file_count: 0,
                    name_taken_count: 0,
                    newer_snapshot_count: 0,
                },
            };
            self.revert_cache = Some((version, preview));
        }
        &self.revert_cache.as_ref().expect("just computed").1
    }

    /// Invalidate the listing + stat caches — call on every file-list
    /// mutation (imports, rescans, apply, revert).
    pub fn invalidate_disk_caches(&mut self) {
        self.disk.invalidate();
        self.metadata.invalidate();
    }

    /// Count of manual overrides in the active mode (§14.4 edited chip).
    pub fn override_count(&self) -> u32 {
        self.state
            .active_items()
            .filter(|item| item.override_name.is_some())
            .count() as u32
    }
}

/// The managed Tauri state: the lock plus the app handle for events.
pub struct AppState {
    shared: Mutex<Shared>,
    app: AppHandle,
    /// Generation counter for the debounced session save (§4.3).
    save_generation: Arc<std::sync::atomic::AtomicU64>,
}

impl AppState {
    /// Build the container (called once in setup).
    pub fn new(app: AppHandle, store: StorePaths, access: Box<dyn FileAccess>) -> Self {
        Self {
            shared: Mutex::new(Shared {
                state: nameshift_engine::CoreState::default(),
                disk: DirectoryNameCache::new(Box::new(FsDirectoryLister)),
                metadata: CachingMetadataSource::new(nameshift_metadata::host_provider()),
                preview_cache: None,
                revert_cache: None,
                undo: UndoStack::default(),
                watchers: HashMap::new(),
                processing: None,
                store,
                access,
                rules_revision: 0,
            }),
            app,
            save_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// The app handle (event emission, dialogs).
    pub fn app(&self) -> &AppHandle {
        &self.app
    }

    /// Read-only access; never bumps the version.
    pub fn read<R>(&self, f: impl FnOnce(&mut Shared) -> R) -> R {
        let mut shared = self.lock();
        f(&mut shared)
    }

    /// THE mutation funnel (§13.2): every `CoreState` change goes through
    /// here — version bump, cache invalidation, `state-changed` emission,
    /// and a debounced session save. Structurally un-forgettable.
    pub fn mutate<R>(&self, f: impl FnOnce(&mut Shared) -> R) -> R {
        let (result, version) = {
            let mut shared = self.lock();
            let result = f(&mut shared);
            shared.state.version += 1;
            shared.preview_cache = None;
            shared.revert_cache = None;
            (result, shared.state.version)
        };
        let _ = self
            .app
            .emit(events::STATE_CHANGED, events::StateChanged { version });
        self.save_session_debounced();
        result
    }

    fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Emit an alert event (§12.2).
    pub fn alert(&self, kind: &str, title: &str, message: &str) {
        let _ = self.app.emit(
            events::ALERT,
            events::Alert {
                kind: kind.to_string(),
                title: title.to_string(),
                message: message.to_string(),
            },
        );
    }

    /// Persist the session immediately (§4.3 triggers: import, clear-all,
    /// apply, revert, live-path rewrite, exit).
    pub fn save_session_now(&self) {
        let mut shared = self.lock();
        crate::session_glue::save_now(&mut shared);
    }

    /// Debounced (1 s) save after any other mutation (§4.3).
    pub fn save_session_debounced(&self) {
        use std::sync::atomic::Ordering;
        let generation = self.save_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let counter = Arc::clone(&self.save_generation);
        let app = self.app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if counter.load(Ordering::SeqCst) == generation {
                if let Some(state) = app.try_state::<AppState>() {
                    state.save_session_now();
                }
            }
        });
    }
}

// ---- IPC payloads ----------------------------------------------------------

/// The full state mirror the frontend refreshes on `state-changed` (§13.1).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StateSnapshot {
    /// Every tracked item, insertion order.
    pub files: Vec<FileItem>,
    /// The rule stack.
    pub rules: Vec<RenameRule>,
    /// Watched folder roots.
    pub watched_folders: Vec<WatchedFolder>,
    /// Saved presets (menu contents).
    pub presets: Vec<RulePreset>,
    /// §4.1 scalars.
    pub trims_whitespace: bool,
    pub auto_resolves_conflicts: bool,
    pub keep_rules_after_apply: bool,
    pub include_subfolders: bool,
    pub sort_key: FileSortKey,
    pub sort_ascending: bool,
    pub list_mode: ListMode,
    #[serde(with = "nameshift_engine::serde_util::uuid_upper_opt")]
    #[ts(as = "Option<String>")]
    pub selected_snapshot_id: Option<Uuid>,
    pub rules_cleared_by_apply: bool,
    /// Bumped by every mutation (§13.1 out-of-order guard).
    pub version: u64,
    /// Reconciles the optimistic rules draft (§13.1).
    pub rules_revision: u64,
    /// Whether an Apply/Revert is in flight (menu/action gating).
    pub is_processing: bool,
    /// Manual overrides in the active mode (the `{N} edited` chip).
    pub override_count: u32,
    /// History count (side-tab badge).
    pub history_count: u32,
    /// Edit-menu label for Undo, when available.
    pub undo_action: Option<String>,
    /// Edit-menu label for Redo, when available.
    pub redo_action: Option<String>,
}

impl StateSnapshot {
    /// Capture the mirror from the shared state.
    pub fn capture(shared: &Shared) -> Self {
        Self {
            files: shared.state.files.clone(),
            rules: shared.state.rules.clone(),
            watched_folders: shared.state.watched_folders.clone(),
            presets: shared.state.presets.clone(),
            trims_whitespace: shared.state.trims_whitespace,
            auto_resolves_conflicts: shared.state.auto_resolves_conflicts,
            keep_rules_after_apply: shared.state.keep_rules_after_apply,
            include_subfolders: shared.state.include_subfolders,
            sort_key: shared.state.sort_key,
            sort_ascending: shared.state.sort_ascending,
            list_mode: shared.state.list_mode,
            selected_snapshot_id: shared.state.selected_snapshot_id,
            rules_cleared_by_apply: shared.state.rules_cleared_by_apply,
            version: shared.state.version,
            rules_revision: shared.rules_revision,
            is_processing: shared.processing.is_some(),
            override_count: shared.override_count(),
            history_count: shared.state.history.len() as u32,
            undo_action: shared
                .undo
                .undo
                .last()
                .map(|entry| entry.action_name.clone()),
            redo_action: shared
                .undo
                .redo
                .last()
                .map(|entry| entry.action_name.clone()),
        }
    }
}

/// `get_preview` payload (§12.1).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PreviewPayload {
    /// The version this preview was computed for.
    pub version: u64,
    /// The §7 preview.
    #[serde(flatten)]
    #[ts(flatten)]
    pub preview: Preview,
    /// §A reason beside the primary button, when gating blocks Apply.
    pub apply_disabled_reason: Option<String>,
}

/// `get_revert_preview` payload.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RevertPreviewPayload {
    /// The version this simulation was computed for.
    pub version: u64,
    /// The §8.8 simulation.
    #[serde(flatten)]
    #[ts(flatten)]
    pub preview: RevertPreview,
}

/// One history row (§12.1 `get_history`).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SnapshotMeta {
    #[serde(with = "nameshift_engine::serde_util::uuid_upper")]
    #[ts(as = "String")]
    pub id: Uuid,
    #[serde(with = "nameshift_engine::serde_util::utc_seconds")]
    #[ts(as = "String")]
    pub date: chrono::DateTime<chrono::Utc>,
    pub summary: String,
    pub entry_count: u32,
}

/// `get_platform` payload — drives per-OS UI copy (§12.1).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlatformInfo {
    /// `"macos"` / `"windows"` / `"linux"`.
    pub os: String,
    /// `"direct"` / `"mas"` / `"msstore"` / `"flathub"` (§20.1).
    pub channel: String,
    /// Whether `{md:…}` comes from Spotlight (inspector section title).
    pub has_spotlight: bool,
    /// App version.
    pub version: String,
}

/// The inspector payload (§12.1 `get_file_metadata`).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InspectorPayload {
    /// Display name.
    pub name: String,
    /// Absolute path.
    pub path: String,
    /// Whether the item is a folder.
    pub is_directory: bool,
    /// General section: kind/size/created/modified/folder, pre-stringified.
    pub kind: Option<String>,
    pub size: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub folder: String,
    /// Every provider attribute, stringified and sorted (§11.3).
    pub attributes: Vec<AttributeEntry>,
}

/// One inspector metadata row.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AttributeEntry {
    /// Provider attribute name (`kMDItemPixelHeight`, `System.…`, `exif:…`).
    pub name: String,
    /// Stringified value.
    pub value: String,
}

/// Build the §A apply-disabled reason from the preview counts.
pub fn apply_disabled_reason(shared: &mut Shared) -> Option<String> {
    let is_folders = shared.state.list_mode == ListMode::Folders;
    let active_count = shared.state.active_items().count();
    if active_count == 0 {
        return None;
    }
    let counts = shared.preview().counts;
    if counts.can_apply {
        return None;
    }
    engine_copy::apply_disabled_reason(counts.conflict_count, counts.change_count, is_folders)
}
