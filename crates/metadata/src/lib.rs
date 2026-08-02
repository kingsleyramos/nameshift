//! File metadata providers (§11): one trait, per-OS implementations
//! (Spotlight, Windows Property System, Linux stat + EXIF + xattr).
//! Powers `{created}` `{modified}` `{size}` `{kind}` `{md:…}` and the
//! File Info inspector.

#![warn(missing_docs)]

mod common;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows_impl;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

pub use common::stringify_value;

/// A typed metadata attribute value; stringification is shared and tested
/// so the inspector and `{md:…}` always agree (§11.1).
#[derive(Clone, Debug, PartialEq)]
pub enum AttrValue {
    /// A plain string.
    Str(String),
    /// A number (rendered shortest-round-trip).
    Num(f64),
    /// A boolean (rendered `Yes`/`No`, never 0/1).
    Bool(bool),
    /// A timestamp (rendered with the §6.4 default pattern).
    Date(SystemTime),
    /// A list (rendered `, `-joined).
    List(Vec<AttrValue>),
}

/// Per-OS metadata access (§11.1). Every accessor is on-demand; providers
/// perform no IO until asked.
pub trait MetadataProvider: Send + Sync {
    /// File creation time, if the platform records one.
    fn created(&self, path: &Path) -> Option<SystemTime>;
    /// File modification time.
    fn modified(&self, path: &Path) -> Option<SystemTime>;
    /// File size in bytes.
    fn size(&self, path: &Path) -> Option<u64>;
    /// Human-readable kind: “JPEG image”, “Folder”, …
    fn kind(&self, path: &Path) -> Option<String>;
    /// One provider-namespaced attribute (`{md:key}`).
    fn attribute(&self, path: &Path, key: &str) -> Option<AttrValue>;
    /// Every attribute the file has, stringified and sorted by name — the
    /// inspector's discovery surface (§11.3).
    fn all_attributes(&self, path: &Path) -> Vec<(String, String)>;
}

/// The provider for the OS this binary runs on.
pub fn host_provider() -> Box<dyn MetadataProvider> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::SpotlightProvider)
    }
    #[cfg(windows)]
    {
        Box::new(windows_impl::PropertySystemProvider)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Box::new(linux::LinuxProvider)
    }
}

#[derive(Clone, Copy, Default)]
struct StatBatch {
    created: Option<SystemTime>,
    modified: Option<SystemTime>,
    size: Option<u64>,
}

/// The lazy per-file handle behind the engine's token seam (§11.1): one
/// stat batch for created/modified/size on the first request, cached until
/// [`CachingMetadataSource::invalidate`]; `{md:…}` and `kind` hit the
/// platform store only on demand (§6.6).
pub struct CachingMetadataSource {
    provider: Box<dyn MetadataProvider>,
    stats: Mutex<HashMap<PathBuf, StatBatch>>,
}

impl CachingMetadataSource {
    /// Wrap a provider with per-path stat caching.
    pub fn new(provider: Box<dyn MetadataProvider>) -> Self {
        Self {
            provider,
            stats: Mutex::new(HashMap::new()),
        }
    }

    /// Drop every cached stat batch. Call on list mutations and after Apply.
    pub fn invalidate(&self) {
        self.stats.lock().expect("stat cache lock").clear();
    }

    /// The wrapped provider (inspector access).
    pub fn provider(&self) -> &dyn MetadataProvider {
        self.provider.as_ref()
    }

    fn batch(&self, path: &Path) -> StatBatch {
        let mut stats = self.stats.lock().expect("stat cache lock");
        if let Some(batch) = stats.get(path) {
            return *batch;
        }
        let batch = StatBatch {
            created: self.provider.created(path),
            modified: self.provider.modified(path),
            size: self.provider.size(path),
        };
        stats.insert(path.to_path_buf(), batch);
        batch
    }
}

impl nameshift_engine::MetadataSource for CachingMetadataSource {
    fn created(&self, path: &Path) -> Option<SystemTime> {
        self.batch(path).created
    }
    fn modified(&self, path: &Path) -> Option<SystemTime> {
        self.batch(path).modified
    }
    fn size(&self, path: &Path) -> Option<u64> {
        self.batch(path).size
    }
    fn kind(&self, path: &Path) -> Option<String> {
        self.provider.kind(path)
    }
    fn attribute(&self, path: &Path, key: &str) -> Option<String> {
        self.provider
            .attribute(path, key)
            .map(|value| stringify_value(&value))
    }
}
