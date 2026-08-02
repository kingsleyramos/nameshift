//! Where the persisted files live (§4.3): the per-OS app-data directory
//! plus `/NameShift`.

use std::path::{Path, PathBuf};

/// Resolved locations for the three persisted files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorePaths {
    /// The `NameShift` data directory.
    pub base: PathBuf,
}

impl StorePaths {
    /// The platform data directory: `~/Library/Application Support/NameShift`
    /// (macOS — identical to the legacy install so history/presets/session
    /// carry over automatically), `%APPDATA%\NameShift` (Windows), or
    /// `$XDG_DATA_HOME/NameShift` (Linux).
    pub fn resolve() -> Self {
        Self::resolve_in(&platform_data_dir())
    }

    /// Resolve inside an explicit parent directory (tests use a tempdir).
    /// On macOS, a legacy `File Name Editor` sibling is migrated to
    /// `NameShift` on first access when `NameShift` doesn't exist (§4.3).
    pub fn resolve_in(parent: &Path) -> Self {
        let base = parent.join("NameShift");
        if cfg!(target_os = "macos") && !base.exists() {
            let legacy = parent.join("File Name Editor");
            if legacy.is_dir() {
                if let Err(error) = std::fs::rename(&legacy, &base) {
                    tracing::warn!("legacy data-directory migration failed: {error}");
                }
            }
        }
        Self { base }
    }

    /// A store rooted at an explicit directory (tests).
    pub fn with_base(base: PathBuf) -> Self {
        Self { base }
    }

    /// `history.json` — applied snapshots, newest first.
    pub fn history_file(&self) -> PathBuf {
        self.base.join("history.json")
    }

    /// `presets.json` — saved rule presets.
    pub fn presets_file(&self) -> PathBuf {
        self.base.join("presets.json")
    }

    /// `session.json` — the restorable workspace.
    pub fn session_file(&self) -> PathBuf {
        self.base.join("session.json")
    }
}

fn platform_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home_dir().join("Library/Application Support")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join("AppData/Roaming"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home_dir().join(".local/share"))
    }
}

fn home_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
    }
}
