//! Intrinsic name validation per platform (§7.6 checks 1–5, §16.1).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::platform::PlatformProfile;

/// Why an item can't be renamed as proposed (§7.6). The first seven block
/// Apply; the last two are environmental badges — the item is excluded from
/// the plan instead (§24 Q17).
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[ts(export)]
pub enum Problem {
    /// The new name would be empty (or `.` / `..`).
    EmptyName,
    /// The new name contains characters this platform forbids.
    InvalidCharacters,
    /// The new name exceeds the platform's length limit.
    NameTooLong,
    /// The platform reserves this name (Windows `CON`, `NUL`, …).
    ReservedName,
    /// The platform forbids trailing dots/spaces (Windows).
    EndsWithDotOrSpace,
    /// Two or more selected items would end up with the same name.
    DuplicateTarget,
    /// A different file with this name already exists in the folder.
    ExistingFileCollision,
    /// The current name isn't valid UTF-8 — Name Shift can't edit it safely
    /// (§16.5).
    UnrenamableName,
    /// MAS builds only: no sandbox grant covers this item's folder (§10.2).
    NoFolderPermission,
}

impl Problem {
    /// Whether this problem blocks Apply (§7.6) or merely excludes the item
    /// from the plan (§24 Q17).
    pub fn blocks_apply(self) -> bool {
        !matches!(self, Problem::UnrenamableName | Problem::NoFolderPermission)
    }
}

/// Check a proposed name against the platform's intrinsic rules
/// (§7.6 checks 1–5, in order; first match wins). Duplicate/collision checks
/// live in the preview pipeline — they need the whole batch.
pub fn intrinsic_problem(profile: &PlatformProfile, name: &str) -> Option<Problem> {
    if name.is_empty() || name == "." || name == ".." {
        return Some(Problem::EmptyName);
    }
    if name.chars().any(|c| {
        profile.invalid_chars.contains(&c) || (profile.forbids_control_chars && (c as u32) < 0x20)
    }) {
        return Some(Problem::InvalidCharacters);
    }
    if profile.max_name_bytes.is_some_and(|max| name.len() > max)
        || profile
            .max_name_utf16
            .is_some_and(|max| name.encode_utf16().count() > max)
    {
        return Some(Problem::NameTooLong);
    }
    if is_reserved(profile, name) {
        return Some(Problem::ReservedName);
    }
    if profile.forbid_trailing_dot_space && (name.ends_with('.') || name.ends_with(' ')) {
        return Some(Problem::EndsWithDotOrSpace);
    }
    None
}

/// Whether the platform reserves this name: the segment before the first `.`
/// is compared ASCII-uppercased (`CON.txt` is reserved on Windows).
pub fn is_reserved(profile: &PlatformProfile, name: &str) -> bool {
    if profile.reserved_names.is_empty() {
        return false;
    }
    let segment = name.split('.').next().unwrap_or(name);
    let upper = segment.to_ascii_uppercase();
    profile.reserved_names.contains(&upper.as_str())
}
