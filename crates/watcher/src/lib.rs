//! Folder watching (§9): debounced filesystem events driving watched-root
//! rescans, plus the scan and reconcile logic they trigger.

#![warn(missing_docs)]

mod scan;

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};

pub use scan::{reconcile_folder, scan_folder, ScanEntry};

/// The debounce window before a rescan fires (§9).
pub const DEBOUNCE: Duration = Duration::from_millis(400);

/// A live watcher on one root. Dropping the handle tears the OS watcher
/// down — nothing may survive removal (§9 deallocation invariant, asserted
/// by test).
pub struct FolderWatcher {
    // Held for its Drop: stops the debounce thread and the OS watcher.
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

impl FolderWatcher {
    /// Watch `root` recursively; `on_change` fires (debounced ≈ 400 ms) for
    /// any event under it — including rename/removal of the root itself,
    /// which is caught by additionally watching the root's parent
    /// non-recursively and filtering to events that touch the root.
    pub fn watch(
        root: &Path,
        on_change: impl Fn() + Send + 'static,
    ) -> notify::Result<FolderWatcher> {
        // Backends report event paths in platform-specific forms: macOS
        // canonicalizes (/var → /private/var), while Windows keeps the plain
        // path but `canonicalize()` prepends a \\?\ verbatim prefix the events
        // never carry. Match against both the canonical and the original root
        // so events are recognized on every OS.
        let canonical_root: PathBuf = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let original_root: PathBuf = root.to_path_buf();
        let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
            let Ok(events) = result else { return };
            let touches_root = events.iter().any(|event| {
                event.paths.iter().any(|p| {
                    p.starts_with(&canonical_root)
                        || *p == canonical_root
                        || p.starts_with(&original_root)
                        || *p == original_root
                })
            });
            if touches_root {
                on_change();
            }
        })?;
        debouncer.watch(root, RecursiveMode::Recursive)?;
        if let Some(parent) = root.parent() {
            // Best-effort: some backends can't report root-level events from
            // the recursive watch alone. Parent events for unrelated
            // siblings are filtered out above.
            let _ = debouncer.watch(parent, RecursiveMode::NonRecursive);
        }
        Ok(FolderWatcher {
            _debouncer: debouncer,
        })
    }
}
