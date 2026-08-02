//! Validation and collision-key tests (§18.1): every §16.1 rule exercised on
//! every OS via injected profiles.

use std::path::Path;

use nameshift_engine::{
    diff_key, fold_name, intrinsic_problem, Problem, LINUX_PROFILE, MACOS_PROFILE, WINDOWS_PROFILE,
};

#[test]
fn empty_dot_dotdot_names() {
    for profile in [&MACOS_PROFILE, &WINDOWS_PROFILE, &LINUX_PROFILE] {
        assert_eq!(intrinsic_problem(profile, ""), Some(Problem::EmptyName));
        assert_eq!(intrinsic_problem(profile, "."), Some(Problem::EmptyName));
        assert_eq!(intrinsic_problem(profile, ".."), Some(Problem::EmptyName));
        assert_eq!(intrinsic_problem(profile, "ok.txt"), None);
    }
}

#[test]
fn invalid_characters_per_platform() {
    assert_eq!(
        intrinsic_problem(&MACOS_PROFILE, "a:b"),
        Some(Problem::InvalidCharacters)
    );
    assert_eq!(
        intrinsic_problem(&MACOS_PROFILE, "a/b"),
        Some(Problem::InvalidCharacters)
    );
    assert_eq!(intrinsic_problem(&MACOS_PROFILE, "a?b*c|d"), None);

    for bad in [
        "a<b", "a>b", "a:b", "a\"b", "a/b", "a\\b", "a|b", "a?b", "a*b", "a\u{1F}b",
    ] {
        assert_eq!(
            intrinsic_problem(&WINDOWS_PROFILE, bad),
            Some(Problem::InvalidCharacters),
            "windows should reject {bad:?}"
        );
    }
    assert_eq!(
        intrinsic_problem(&LINUX_PROFILE, "a/b"),
        Some(Problem::InvalidCharacters)
    );
    assert_eq!(intrinsic_problem(&LINUX_PROFILE, "a:b?<>|"), None);
}

#[test]
fn name_length_limits() {
    let ascii_256 = "a".repeat(256);
    let ascii_255 = "a".repeat(255);
    for profile in [&MACOS_PROFILE, &WINDOWS_PROFILE, &LINUX_PROFILE] {
        assert_eq!(
            intrinsic_problem(profile, &ascii_256),
            Some(Problem::NameTooLong)
        );
        assert_eq!(intrinsic_problem(profile, &ascii_255), None);
    }
    // 100 three-byte chars = 300 UTF-8 bytes but only 100 UTF-16 units:
    // too long for byte-limited platforms, fine on Windows.
    let cjk_100 = "文".repeat(100);
    assert_eq!(
        intrinsic_problem(&MACOS_PROFILE, &cjk_100),
        Some(Problem::NameTooLong)
    );
    assert_eq!(
        intrinsic_problem(&LINUX_PROFILE, &cjk_100),
        Some(Problem::NameTooLong)
    );
    assert_eq!(intrinsic_problem(&WINDOWS_PROFILE, &cjk_100), None);
}

#[test]
fn windows_reserved_names_tested_on_every_os() {
    for name in [
        "CON",
        "con",
        "CON.txt",
        "Nul.tar.gz",
        "COM1",
        "lpt9.doc",
        "AUX.c",
    ] {
        assert_eq!(
            intrinsic_problem(&WINDOWS_PROFILE, name),
            Some(Problem::ReservedName),
            "windows should reserve {name:?}"
        );
    }
    for name in [
        "CONSOLE",
        "COM10",
        "COM0",
        "connect.log",
        "LPT",
        "auxiliary.txt",
    ] {
        assert_eq!(
            intrinsic_problem(&WINDOWS_PROFILE, name),
            None,
            "{name:?} is not reserved"
        );
    }
    // Only Windows reserves them.
    assert_eq!(intrinsic_problem(&MACOS_PROFILE, "CON.txt"), None);
    assert_eq!(intrinsic_problem(&LINUX_PROFILE, "CON.txt"), None);
}

#[test]
fn trailing_dot_or_space_is_windows_only() {
    assert_eq!(
        intrinsic_problem(&WINDOWS_PROFILE, "name."),
        Some(Problem::EndsWithDotOrSpace)
    );
    assert_eq!(
        intrinsic_problem(&WINDOWS_PROFILE, "name "),
        Some(Problem::EndsWithDotOrSpace)
    );
    assert_eq!(intrinsic_problem(&MACOS_PROFILE, "name."), None);
    assert_eq!(intrinsic_problem(&LINUX_PROFILE, "name "), None);
}

#[test]
fn check_order_is_normative() {
    // A name that is both invalid and too long reports InvalidCharacters —
    // §7.6 first match wins.
    let long_invalid = format!("{}:", "a".repeat(300));
    assert_eq!(
        intrinsic_problem(&WINDOWS_PROFILE, &long_invalid),
        Some(Problem::InvalidCharacters)
    );
    // Reserved beats trailing-dot: `CON.` hits ReservedName first.
    assert_eq!(
        intrinsic_problem(&WINDOWS_PROFILE, "CON."),
        Some(Problem::ReservedName)
    );
}

// ---- diff_key fold/normalization matrix (§16.2) ---------------------------

const NFC_CAFE: &str = "caf\u{E9}.txt"; // café with precomposed é
const NFD_CAFE: &str = "cafe\u{301}.txt"; // café with combining acute

#[test]
fn diff_key_macos_folds_case_and_normalization() {
    let dir = Path::new("/a");
    assert_eq!(
        diff_key(&MACOS_PROFILE, dir, "Photo.JPG"),
        diff_key(&MACOS_PROFILE, dir, "photo.jpg")
    );
    assert_eq!(
        diff_key(&MACOS_PROFILE, dir, NFC_CAFE),
        diff_key(&MACOS_PROFILE, dir, NFD_CAFE)
    );
    // Different directories never collide.
    assert_ne!(
        diff_key(&MACOS_PROFILE, Path::new("/a"), "x.txt"),
        diff_key(&MACOS_PROFILE, Path::new("/b"), "x.txt")
    );
}

#[test]
fn diff_key_windows_folds_case_but_not_normalization() {
    let dir = Path::new("/a");
    assert_eq!(
        diff_key(&WINDOWS_PROFILE, dir, "Photo.JPG"),
        diff_key(&WINDOWS_PROFILE, dir, "photo.jpg")
    );
    assert_ne!(
        diff_key(&WINDOWS_PROFILE, dir, NFC_CAFE),
        diff_key(&WINDOWS_PROFILE, dir, NFD_CAFE)
    );
}

#[test]
fn diff_key_linux_is_exact() {
    let dir = Path::new("/a");
    assert_ne!(
        diff_key(&LINUX_PROFILE, dir, "Photo.JPG"),
        diff_key(&LINUX_PROFILE, dir, "photo.jpg")
    );
    assert_ne!(
        diff_key(&LINUX_PROFILE, dir, NFC_CAFE),
        diff_key(&LINUX_PROFILE, dir, NFD_CAFE)
    );
    assert_eq!(
        diff_key(&LINUX_PROFILE, dir, "x.txt"),
        diff_key(&LINUX_PROFILE, dir, "x.txt")
    );
}

#[test]
fn fold_name_variants() {
    assert_eq!(fold_name(&MACOS_PROFILE, "ReadMe.MD"), "readme.md");
    assert_eq!(fold_name(&WINDOWS_PROFILE, "ReadMe.MD"), "readme.md");
    assert_eq!(fold_name(&LINUX_PROFILE, "ReadMe.MD"), "ReadMe.MD");
    assert_eq!(
        fold_name(&MACOS_PROFILE, NFD_CAFE),
        fold_name(&MACOS_PROFILE, NFC_CAFE)
    );
}
