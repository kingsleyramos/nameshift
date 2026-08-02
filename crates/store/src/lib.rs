//! Session, history, and preset persistence: atomic JSON writes with
//! legacy-tolerant decoding (§4.3).

#![warn(missing_docs)]

mod paths;
mod session;

use std::io;
use std::path::Path;

use nameshift_engine::{RulePreset, Snapshot};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub use paths::StorePaths;
pub use session::{SessionFileEntry, SessionState};

/// Write JSON atomically: serialize to `<name>.json.tmp` in the same
/// directory, then rename over the target (§4.3). A crash mid-write leaves
/// the previous file intact.
pub fn atomic_write_json<T: Serialize>(path: &Path, value: &T, pretty: bool) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path has no parent",
        ));
    };
    std::fs::create_dir_all(parent)?;
    let bytes = if pretty {
        serde_json::to_vec_pretty(value).map_err(io::Error::other)?
    } else {
        serde_json::to_vec(value).map_err(io::Error::other)?
    };
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Read and decode a JSON file. Missing or corrupt files fall back to `None`
/// — never crash on bad JSON (§4.3); corruption is logged.
fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;
    match serde_json::from_slice(&bytes) {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::warn!("ignoring corrupt {}: {error}", path.display());
            None
        }
    }
}

/// Load `history.json` (empty when missing or corrupt).
pub fn load_history(paths: &StorePaths) -> Vec<Snapshot> {
    read_json(&paths.history_file()).unwrap_or_default()
}

/// Persist `history.json` (pretty, like the legacy encoder).
pub fn save_history(paths: &StorePaths, history: &[Snapshot]) -> io::Result<()> {
    atomic_write_json(&paths.history_file(), &history, true)
}

/// Load `presets.json` (empty when missing or corrupt).
pub fn load_presets(paths: &StorePaths) -> Vec<RulePreset> {
    read_json(&paths.presets_file()).unwrap_or_default()
}

/// Persist `presets.json` (pretty, like the legacy encoder).
pub fn save_presets(paths: &StorePaths, presets: &[RulePreset]) -> io::Result<()> {
    atomic_write_json(&paths.presets_file(), &presets, true)
}

/// Load `session.json`. `None` means no restorable session (missing or
/// corrupt) — the caller applies fresh-install defaults, including
/// `keepRulesAfterApply = true` (§4.3).
pub fn load_session(paths: &StorePaths) -> Option<SessionState> {
    read_json(&paths.session_file())
}

/// Persist `session.json` (compact).
pub fn save_session(paths: &StorePaths, session: &SessionState) -> io::Result<()> {
    atomic_write_json(&paths.session_file(), session, false)
}
