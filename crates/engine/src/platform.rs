//! Platform rules as data, not scattered conditionals (§16). The engine
//! takes a profile as a parameter, so Windows rules are unit-tested on every
//! CI OS by injecting [`WINDOWS_PROFILE`].

/// Which operating system a profile models — drives per-OS copy (§A) and
/// OS-specific behaviors like bundle pruning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostOs {
    /// macOS (APFS/HFS+ semantics).
    MacOs,
    /// Windows (NTFS semantics).
    Windows,
    /// Linux (POSIX semantics).
    Linux,
}

/// Name validation, collision folding, and length limits for one platform
/// (§16.1–16.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformProfile {
    /// Which OS this profile models.
    pub os: HostOs,
    /// Characters a name may not contain.
    pub invalid_chars: &'static [char],
    /// Whether control characters (U+0000–U+001F) are forbidden.
    pub forbids_control_chars: bool,
    /// Reserved device names, compared stem-before-first-dot, ASCII-uppercased.
    pub reserved_names: &'static [&'static str],
    /// Whether names may not end with `.` or a space.
    pub forbid_trailing_dot_space: bool,
    /// Maximum name length in UTF-8 bytes.
    pub max_name_bytes: Option<usize>,
    /// Maximum name length in UTF-16 units (Windows).
    pub max_name_utf16: Option<usize>,
    /// Whether collision keys fold case (§16.2).
    pub case_fold_keys: bool,
    /// Whether collision keys NFC-normalize (§16.2).
    pub nfc_normalize_keys: bool,
}

const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// macOS: APFS is case- and normalization-insensitive by default.
pub static MACOS_PROFILE: PlatformProfile = PlatformProfile {
    os: HostOs::MacOs,
    invalid_chars: &['/', ':'],
    forbids_control_chars: false,
    reserved_names: &[],
    forbid_trailing_dot_space: false,
    max_name_bytes: Some(255),
    max_name_utf16: None,
    case_fold_keys: true,
    nfc_normalize_keys: true,
};

/// Windows: NTFS is case-insensitive but normalization-sensitive.
pub static WINDOWS_PROFILE: PlatformProfile = PlatformProfile {
    os: HostOs::Windows,
    invalid_chars: &['<', '>', ':', '"', '/', '\\', '|', '?', '*'],
    forbids_control_chars: true,
    reserved_names: WINDOWS_RESERVED,
    forbid_trailing_dot_space: true,
    max_name_bytes: None,
    max_name_utf16: Some(255),
    case_fold_keys: true,
    nfc_normalize_keys: false,
};

/// Linux: exact bytes, `/` and NUL forbidden.
pub static LINUX_PROFILE: PlatformProfile = PlatformProfile {
    os: HostOs::Linux,
    invalid_chars: &['/', '\0'],
    forbids_control_chars: false,
    reserved_names: &[],
    forbid_trailing_dot_space: false,
    max_name_bytes: Some(255),
    max_name_utf16: None,
    case_fold_keys: false,
    nfc_normalize_keys: false,
};

/// Windows: convert a path to `\\?\` verbatim form so deep trees and
/// 255-char names work regardless of the system MAX_PATH setting (§16.1).
/// Every filesystem call routes through this helper. Already-verbatim and
/// relative paths pass through unchanged.
#[cfg(windows)]
pub fn win_long_path(path: &std::path::Path) -> std::path::PathBuf {
    use std::path::{Component, Prefix};
    match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Verbatim(_) | Prefix::VerbatimDisk(_) | Prefix::VerbatimUNC(..) => {
                path.to_path_buf()
            }
            Prefix::UNC(..) => {
                let raw = path.to_string_lossy();
                std::path::PathBuf::from(format!(r"\\?\UNC{}", &raw[1..]))
            }
            _ => std::path::PathBuf::from(format!(r"\\?\{}", path.display())),
        },
        // Relative paths can't take the verbatim prefix; leave them alone.
        _ => path.to_path_buf(),
    }
}

/// Non-Windows: paths are used as-is.
#[cfg(not(windows))]
pub fn win_long_path(path: &std::path::Path) -> std::path::PathBuf {
    path.to_path_buf()
}

/// The profile for the OS this binary runs on.
pub fn host_profile() -> &'static PlatformProfile {
    #[cfg(target_os = "macos")]
    {
        &MACOS_PROFILE
    }
    #[cfg(target_os = "windows")]
    {
        &WINDOWS_PROFILE
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        &LINUX_PROFILE
    }
}
