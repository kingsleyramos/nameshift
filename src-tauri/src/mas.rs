//! Mac App Store sandbox support (§10): security-scoped bookmarks behind
//! the [`FileAccess`] seam. Compiled only with `--features channel-mas`;
//! behavior deltas in MAS builds are exactly §10.2 — nothing else differs.

#![cfg(feature = "channel-mas")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use objc2::rc::Retained;
use objc2_foundation::{NSData, NSString, NSURL};

use crate::access::FileAccess;

/// Security-scoped access: tokens are bookmark blobs; live scopes are held
/// for granted roots and released on shutdown. There is an OS cap on
/// simultaneously-open scoped resources — scopes are keyed by path so a
/// re-grant never doubles up.
pub struct ScopedAccess {
    scopes: Mutex<HashMap<PathBuf, Retained<NSURL>>>,
}

impl ScopedAccess {
    pub fn new() -> Self {
        Self {
            scopes: Mutex::new(HashMap::new()),
        }
    }

    fn url_for(path: &Path) -> Retained<NSURL> {
        let raw = NSString::from_str(&path.to_string_lossy());
        // SAFETY-free: plain Foundation constructor.
        unsafe { NSURL::fileURLWithPath(&raw) }
    }

    fn remember_scope(&self, path: PathBuf, url: Retained<NSURL>) {
        let started = unsafe { url.startAccessingSecurityScopedResource() };
        if started {
            let mut scopes = self
                .scopes
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(previous) = scopes.insert(path, url) {
                unsafe { previous.stopAccessingSecurityScopedResource() };
            }
        }
    }
}

impl Default for ScopedAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl FileAccess for ScopedAccess {
    fn persist_token(&self, path: &Path) -> Option<Vec<u8>> {
        let url = Self::url_for(path);
        // NSURLBookmarkCreationWithSecurityScope = 1 << 11 (§10.1).
        let options = objc2_foundation::NSURLBookmarkCreationOptions(1 << 11);
        let data: Retained<NSData> = unsafe {
            url.bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
                options, None, None,
            )
        }
        .ok()?;
        Some(data.to_vec())
    }

    fn resolve_token(&self, token: &[u8]) -> Option<PathBuf> {
        let data = NSData::with_bytes(token);
        // NSURLBookmarkResolutionWithSecurityScope = 1 << 10.
        let options = objc2_foundation::NSURLBookmarkResolutionOptions(1 << 10);
        let mut stale = objc2::runtime::Bool::NO;
        let url = unsafe {
            NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                &data, options, None, &mut stale,
            )
        }
        .ok()?;
        let path = unsafe { url.path() }.map(|p| PathBuf::from(p.to_string()))?;
        // Stale bookmark → the caller drops the entry and counts it toward
        // the “Some files were missing” alert (§10.1).
        if stale.as_bool() {
            return None;
        }
        self.remember_scope(path.clone(), url);
        Some(path)
    }

    fn note_user_granted(&self, path: &Path) {
        self.remember_scope(path.to_path_buf(), Self::url_for(path));
    }

    fn stop_all(&self) {
        let mut scopes = self
            .scopes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (_, url) in scopes.drain() {
            unsafe { url.stopAccessingSecurityScopedResource() };
        }
    }

    fn covers(&self, path: &Path) -> bool {
        let scopes = self
            .scopes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        scopes.keys().any(|root| path.starts_with(root))
    }
}
