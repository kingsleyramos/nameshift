//! The file-access seam (§10.1). Direct/Microsoft-Store/Flathub builds use
//! plain paths; the MAS build swaps in security-scoped bookmarks — designed
//! in from day one so MAS is a packaging flip, not a rewrite.
//!
//! Invariant for future code: any newly persisted path must go through
//! [`FileAccess::persist_token`] (CLAUDE.md, §22.3).

use std::path::{Path, PathBuf};

/// How the app persists and re-acquires access to user-granted paths.
pub trait FileAccess: Send + Sync {
    /// A persistable token for a path the user granted. Direct builds: the
    /// path itself. MAS builds: a security-scoped bookmark blob.
    fn persist_token(&self, path: &Path) -> Option<Vec<u8>>;

    /// Re-acquire access from a stored token at launch. Returns the
    /// (possibly moved) resolved path, having started security-scoped
    /// access where applicable.
    fn resolve_token(&self, token: &[u8]) -> Option<PathBuf>;

    /// Record a grant obtained via open dialog / drag-drop (MAS: create +
    /// start scope).
    fn note_user_granted(&self, path: &Path);

    /// Release scoped resources on shutdown.
    fn stop_all(&self);

    /// Whether an existing grant covers this path. Renaming is a move within
    /// the parent directory, so MAS needs a DIRECTORY grant (§10.2); plain
    /// paths cover everything.
    fn covers(&self, _path: &Path) -> bool {
        true
    }
}

/// Plain-path implementation (default feature `channel-direct`, also
/// msstore/flathub): tokens are UTF-8 path bytes; every method trivial.
pub struct DirectAccess;

impl FileAccess for DirectAccess {
    fn persist_token(&self, path: &Path) -> Option<Vec<u8>> {
        Some(path.to_string_lossy().into_owned().into_bytes())
    }

    fn resolve_token(&self, token: &[u8]) -> Option<PathBuf> {
        let text = String::from_utf8(token.to_vec()).ok()?;
        (!text.is_empty()).then(|| PathBuf::from(text))
    }

    fn note_user_granted(&self, _path: &Path) {}

    fn stop_all(&self) {}
}

/// The access implementation for the active channel: `ScopedAccess`
/// (security-scoped bookmarks) under `channel-mas` (§20.3), plain paths
/// everywhere else.
pub fn channel_access() -> Box<dyn FileAccess> {
    #[cfg(feature = "channel-mas")]
    {
        Box::new(crate::mas::ScopedAccess::new())
    }
    #[cfg(not(feature = "channel-mas"))]
    {
        Box::new(DirectAccess)
    }
}
