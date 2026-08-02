//! Token expansion tests (§18.1): every §6.2 token, grammar edge cases,
//! LDML date vectors, size vectors, and the §6.6 laziness invariant.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local, TimeZone};
use nameshift_engine::{
    expand_tokens, format_date, format_size, MetadataSource, NoMetadata, TokenContext,
};

static NO_METADATA: NoMetadata = NoMetadata;

/// A metadata source with fixed values that counts every access (§6.6).
#[derive(Default)]
struct CountingMetadata {
    stat_calls: AtomicUsize,
}

impl CountingMetadata {
    fn calls(&self) -> usize {
        self.stat_calls.load(Ordering::SeqCst)
    }
}

impl MetadataSource for CountingMetadata {
    fn created(&self, _: &Path) -> Option<SystemTime> {
        self.stat_calls.fetch_add(1, Ordering::SeqCst);
        // 2026-07-15T12:00:00Z — mid-day so no timezone shifts the date parts
        // asserted below.
        Some(UNIX_EPOCH + Duration::from_secs(1_784_116_800))
    }
    fn modified(&self, _: &Path) -> Option<SystemTime> {
        self.stat_calls.fetch_add(1, Ordering::SeqCst);
        Some(UNIX_EPOCH + Duration::from_secs(1_784_116_800))
    }
    fn size(&self, _: &Path) -> Option<u64> {
        self.stat_calls.fetch_add(1, Ordering::SeqCst);
        Some(1_200_000)
    }
    fn kind(&self, _: &Path) -> Option<String> {
        self.stat_calls.fetch_add(1, Ordering::SeqCst);
        Some("JPEG image".to_string())
    }
    fn attribute(&self, _: &Path, key: &str) -> Option<String> {
        self.stat_calls.fetch_add(1, Ordering::SeqCst);
        (key == "kMDItemPixelHeight").then(|| "3024".to_string())
    }
}

fn ctx<'a>(index: u32, metadata: &'a dyn MetadataSource) -> TokenContext<'a> {
    TokenContext {
        index,
        folder_index: index,
        path: Path::new("/tmp/Photos/x.jpg"),
        metadata,
    }
}

fn expand(template: &str, ctx: Option<&TokenContext>) -> String {
    expand_tokens(template, "x", "jpg", ctx)
}

// ---- token table (§6.2) --------------------------------------------------

#[test]
fn name_and_ext_tokens() {
    assert_eq!(expand("{name}.bak", None), "x.bak");
    assert_eq!(expand("a{ext}b", None), "ajpgb");
    // Empty extension expands to empty — never left in place.
    assert_eq!(expand_tokens("a{ext}b", "x", "", None), "ab");
}

#[test]
fn folder_token() {
    let c = ctx(1, &NO_METADATA);
    assert_eq!(expand("{folder}", Some(&c)), "Photos");
    // Without context it's unknown.
    assert_eq!(expand("{folder}", None), "{folder}");
}

#[test]
fn sequence_token_with_width() {
    let c = ctx(7, &NO_METADATA);
    assert_eq!(expand("{n}", Some(&c)), "7");
    assert_eq!(expand("{n:3}", Some(&c)), "007");
    // Width clamps to 1…10; non-numeric widths fall back to 1.
    assert_eq!(expand("{n:99999999}", Some(&c)), "0000000007");
    assert_eq!(expand("{n:abc}", Some(&c)), "7");
    assert_eq!(expand("{n:0}", Some(&c)), "7");
    assert_eq!(expand("{n:-3}", Some(&c)), "7");
    // Without context the sequence number is 1.
    assert_eq!(expand("{n:2}", None), "01");
}

#[test]
fn date_tokens_from_metadata() {
    let meta = CountingMetadata::default();
    let c = ctx(1, &meta);
    // Cross-validate our LDML formatter against chrono's own strftime output
    // for the same instant, so the assertion is timezone-independent.
    let instant = UNIX_EPOCH + Duration::from_secs(1_784_116_800);
    let local: DateTime<Local> = instant.into();
    assert_eq!(
        expand("{created}", Some(&c)),
        local.format("%Y-%m-%d").to_string()
    );
    assert_eq!(
        expand("{modified:yyyy}", Some(&c)),
        local.format("%Y").to_string()
    );
    // Unknown when the source has no value.
    let c = ctx(1, &NO_METADATA);
    assert_eq!(expand("{created}", Some(&c)), "{created}");
    assert_eq!(expand("{modified}", Some(&c)), "{modified}");
}

#[test]
fn date_token_always_resolves() {
    let output = expand("{date:yyyy}", None);
    assert_eq!(output.len(), 4);
    assert!(output.chars().all(|c| c.is_ascii_digit()), "{output}");
}

#[test]
fn size_and_kind_tokens() {
    let meta = CountingMetadata::default();
    let c = ctx(1, &meta);
    assert_eq!(expand("{size}", Some(&c)), "1.2 MB");
    assert_eq!(expand("{kind}", Some(&c)), "JPEG image");
    let c = ctx(1, &NO_METADATA);
    assert_eq!(expand("{size}", Some(&c)), "{size}");
    assert_eq!(expand("{kind}", Some(&c)), "{kind}");
}

#[test]
fn metadata_attribute_token() {
    let meta = CountingMetadata::default();
    let c = ctx(1, &meta);
    assert_eq!(expand("{md:kMDItemPixelHeight}p", Some(&c)), "3024p");
    // Absent attribute → unknown, left in place (§11.3).
    assert_eq!(
        expand("{md:System.Photo.CameraModel}", Some(&c)),
        "{md:System.Photo.CameraModel}"
    );
    // `{md:…}` argument may contain colons (split on the FIRST colon only).
    assert_eq!(expand("{md:exif:Model}", Some(&c)), "{md:exif:Model}");
}

// ---- grammar (§6.1) ------------------------------------------------------

#[test]
fn unknown_tokens_left_in_place() {
    let c = ctx(1, &NO_METADATA);
    assert_eq!(expand("{nope}-{name}", Some(&c)), "{nope}-x");
    assert_eq!(expand("{:}-{name}", Some(&c)), "{:}-x");
    assert_eq!(expand("{::}-{name}", Some(&c)), "{::}-x");
    // A colon-prefixed key is empty → unknown (§6.1).
    assert_eq!(expand("{:n}", Some(&c)), "{:n}");
}

#[test]
fn braces_cannot_nest() {
    let c = ctx(1, &NO_METADATA);
    assert_eq!(expand("{}", Some(&c)), "{}");
    assert_eq!(expand("{a{name}", Some(&c)), "{ax");
    assert_eq!(expand("x{", Some(&c)), "x{");
    assert_eq!(expand("}{name}", Some(&c)), "}x");
}

#[test]
fn expansion_is_single_pass() {
    // A file literally named `{n}` cannot inject: expanded values are never
    // rescanned (§6.1).
    let c = ctx(5, &NO_METADATA);
    assert_eq!(expand_tokens("{name}.bak", "{n}", "", Some(&c)), "{n}.bak");
    assert_eq!(
        expand_tokens("{name}", "{created}", "", Some(&c)),
        "{created}"
    );
}

// ---- laziness (§6.6) -----------------------------------------------------

#[test]
fn constructing_context_performs_no_io() {
    let meta = CountingMetadata::default();
    let c = ctx(1, &meta);
    // No metadata token → no metadata access, even for nonexistent paths.
    assert_eq!(expand("{name}-{n:2}-{folder}", Some(&c)), "x-01-Photos");
    assert_eq!(meta.calls(), 0);
    // A date token performs exactly one access.
    expand("{created}", Some(&c));
    assert_eq!(meta.calls(), 1);
}

// ---- LDML date vectors (§6.4) --------------------------------------------

fn vector_date() -> DateTime<Local> {
    // 2026-07-27 18:04:11 local wall time (a Monday).
    Local
        .with_ymd_and_hms(2026, 7, 27, 18, 4, 11)
        .single()
        .expect("unambiguous local time")
}

#[test]
fn ldml_vectors() {
    let d = vector_date();
    assert_eq!(format_date(&d, "yyyy-MM-dd HH.mm"), "2026-07-27 18.04");
    assert_eq!(format_date(&d, "d/M/yy"), "27/7/26");
    assert_eq!(format_date(&d, "EEE MMM d"), "Mon Jul 27");
    assert_eq!(format_date(&d, "EEEE MMMM d"), "Monday July 27");
    assert_eq!(format_date(&d, "yyyy-MM-dd"), "2026-07-27");
    assert_eq!(format_date(&d, "HH:mm:ss"), "18:04:11");
    assert_eq!(format_date(&d, "h:mm a"), "6:04 PM");
    assert_eq!(format_date(&d, "hh"), "06");
}

#[test]
fn ldml_quoting_and_passthrough() {
    let d = vector_date();
    assert_eq!(format_date(&d, "yyyy'y'"), "2026y");
    assert_eq!(format_date(&d, "''"), "'");
    assert_eq!(format_date(&d, "'at' HH"), "at 18");
    assert_eq!(format_date(&d, "'o''clock'"), "o'clock");
    // Unrecognized pattern letters pass through as literals.
    assert_eq!(format_date(&d, "yyyy-QQ"), "2026-QQ");
    // Unlisted run lengths fall back to the longest listed prefix.
    assert_eq!(format_date(&d, "yyy"), "26y");
    assert_eq!(format_date(&d, "yyyyy"), "2026y");
}

#[test]
fn ldml_twelve_hour_boundaries() {
    let midnight = Local
        .with_ymd_and_hms(2026, 1, 1, 0, 5, 0)
        .single()
        .expect("unambiguous");
    assert_eq!(format_date(&midnight, "h a"), "12 AM");
    let noon = Local
        .with_ymd_and_hms(2026, 1, 1, 12, 5, 0)
        .single()
        .expect("unambiguous");
    assert_eq!(format_date(&noon, "h a"), "12 PM");
}

// ---- size vectors (§6.5) -------------------------------------------------

#[test]
fn size_vectors() {
    assert_eq!(format_size(0), "Zero bytes");
    assert_eq!(format_size(1), "1 byte");
    assert_eq!(format_size(2), "2 bytes");
    assert_eq!(format_size(999), "999 bytes");
    assert_eq!(format_size(1000), "1 KB");
    assert_eq!(format_size(1200), "1.2 KB");
    assert_eq!(format_size(1_200_000), "1.2 MB");
    assert_eq!(format_size(12_000_000), "12 MB");
    assert_eq!(format_size(3_000_000_000), "3 GB");
    assert_eq!(format_size(5_600_000_000_000), "5.6 TB");
    // TB is the cap.
    assert_eq!(format_size(2_500_000_000_000_000), "2500 TB");
}
