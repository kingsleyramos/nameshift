//! Collision keys (§16.2): how two names are decided to be “the same” on a
//! given platform. Used by duplicate-target detection, auto-resolve, and the
//! on-disk name sets.

use std::path::Path;

use unicode_normalization::UnicodeNormalization;

use crate::platform::PlatformProfile;

/// Fold one name per the profile: NFC-normalize and/or lowercase.
pub fn fold_name(profile: &PlatformProfile, name: &str) -> String {
    let normalized: String = if profile.nfc_normalize_keys {
        name.nfc().collect()
    } else {
        name.to_string()
    };
    if profile.case_fold_keys {
        normalized.to_lowercase()
    } else {
        normalized
    }
}

/// The collision key for a (directory, name) pair. Two targets collide when
/// their keys are equal.
pub fn diff_key(profile: &PlatformProfile, directory: &Path, name: &str) -> String {
    let folded_dir = fold_name(profile, &directory.to_string_lossy());
    let folded_name = fold_name(profile, name);
    // NUL can't appear in path components, so it's a safe separator.
    format!("{folded_dir}\u{0}{folded_name}")
}
