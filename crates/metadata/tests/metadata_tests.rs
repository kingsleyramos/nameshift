//! Metadata provider tests (§18.1, host-OS gated where needed).

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nameshift_engine::MetadataSource;
use nameshift_metadata::{
    host_provider, stringify_value, AttrValue, CachingMetadataSource, MetadataProvider,
};
use tempfile::TempDir;

// ---- stringification rules (§11.1, shared and tested) ----------------------

#[test]
fn booleans_render_yes_no_never_zero_one() {
    assert_eq!(stringify_value(&AttrValue::Bool(true)), "Yes");
    assert_eq!(stringify_value(&AttrValue::Bool(false)), "No");
}

#[test]
fn numbers_render_shortest_round_trip() {
    assert_eq!(stringify_value(&AttrValue::Num(3024.0)), "3024");
    assert_eq!(stringify_value(&AttrValue::Num(1.5)), "1.5");
    assert_eq!(stringify_value(&AttrValue::Num(0.1)), "0.1");
}

#[test]
fn dates_render_default_pattern() {
    // 2026-07-15T12:00:00Z — mid-day so no timezone shifts the date.
    let time = UNIX_EPOCH + Duration::from_secs(1_784_116_800);
    let rendered = stringify_value(&AttrValue::Date(time));
    let expected = chrono::DateTime::<chrono::Local>::from(time)
        .format("%Y-%m-%d")
        .to_string();
    assert_eq!(rendered, expected);
}

#[test]
fn lists_join_with_comma_space() {
    let list = AttrValue::List(vec![
        AttrValue::Str("a".into()),
        AttrValue::Num(2.0),
        AttrValue::Bool(true),
    ]);
    assert_eq!(stringify_value(&list), "a, 2, Yes");
}

// ---- stat basics ------------------------------------------------------------

#[test]
fn stat_basics_from_a_real_file() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("data.bin");
    fs::write(&file, vec![0u8; 1234]).unwrap();
    let provider = host_provider();
    assert_eq!(provider.size(&file), Some(1234));
    assert!(provider.modified(&file).is_some());
    #[cfg(any(target_os = "macos", windows))]
    assert!(
        provider.created(&file).is_some(),
        "birth time exists on macOS/Windows"
    );
    // Directories have no size.
    assert_eq!(provider.size(tmp.path()), None);
}

#[test]
fn kind_for_common_extensions_and_folders() {
    let tmp = TempDir::new().unwrap();
    let jpg = tmp.path().join("shot.jpg");
    fs::write(&jpg, b"x").unwrap();
    let odd = tmp.path().join("data.xyz");
    fs::write(&odd, b"x").unwrap();
    let provider = host_provider();
    // Platform providers may report richer localized kinds; the fallback
    // table's answers are always acceptable.
    let jpg_kind = provider.kind(&jpg).expect("kind exists");
    assert!(!jpg_kind.is_empty());
    let folder_kind = provider.kind(tmp.path()).expect("kind exists");
    assert!(!folder_kind.is_empty());
    let odd_kind = provider.kind(&odd).expect("kind exists");
    assert!(!odd_kind.is_empty());
}

#[test]
fn missing_files_yield_none_not_errors() {
    let provider = host_provider();
    let ghost = Path::new("/definitely/not/here.txt");
    assert_eq!(provider.size(ghost), None);
    assert_eq!(provider.created(ghost), None);
    assert_eq!(provider.modified(ghost), None);
    assert!(provider.all_attributes(ghost).is_empty());
}

// ---- the caching source (§11.1 lazy handle) --------------------------------

struct CountingProvider {
    stats: std::sync::Arc<AtomicUsize>,
}

impl MetadataProvider for CountingProvider {
    fn created(&self, _: &Path) -> Option<SystemTime> {
        self.stats.fetch_add(1, Ordering::SeqCst);
        Some(UNIX_EPOCH)
    }
    fn modified(&self, _: &Path) -> Option<SystemTime> {
        self.stats.fetch_add(1, Ordering::SeqCst);
        Some(UNIX_EPOCH)
    }
    fn size(&self, _: &Path) -> Option<u64> {
        self.stats.fetch_add(1, Ordering::SeqCst);
        Some(1)
    }
    fn kind(&self, _: &Path) -> Option<String> {
        None
    }
    fn attribute(&self, _: &Path, _: &str) -> Option<AttrValue> {
        None
    }
    fn all_attributes(&self, _: &Path) -> Vec<(String, String)> {
        Vec::new()
    }
}

#[test]
fn caching_source_stats_once_per_path_until_invalidated() {
    let stats = std::sync::Arc::new(AtomicUsize::new(0));
    let source = CachingMetadataSource::new(Box::new(CountingProvider {
        stats: std::sync::Arc::clone(&stats),
    }));
    let path = Path::new("/x/a.txt");

    // The first access performs one whole stat batch (created+modified+size).
    let _ = MetadataSource::created(&source, path);
    assert_eq!(
        stats.load(Ordering::SeqCst),
        3,
        "one batch on first request"
    );

    // Further reads — any field, same path — are served from the cache.
    let _ = MetadataSource::modified(&source, path);
    let _ = MetadataSource::size(&source, path);
    assert_eq!(
        stats.load(Ordering::SeqCst),
        3,
        "cached for the handle's life"
    );

    // A different path costs its own batch.
    let _ = MetadataSource::size(&source, Path::new("/x/b.txt"));
    assert_eq!(stats.load(Ordering::SeqCst), 6);

    // Invalidation forces a fresh batch on the next request.
    source.invalidate();
    let _ = MetadataSource::created(&source, path);
    assert_eq!(stats.load(Ordering::SeqCst), 9);
}

// ---- macOS Spotlight (host-gated, lenient off the indexed volume) ----------

#[cfg(target_os = "macos")]
#[test]
fn spotlight_filesystem_attributes() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("spotlight probe.txt");
    fs::write(&file, b"hello").unwrap();
    let provider = host_provider();
    // Filesystem-level Spotlight attributes exist even for unindexed files;
    // stay lenient in case the volume forbids MDItem entirely (CI).
    if let Some(value) = provider.attribute(&file, "kMDItemFSName") {
        assert_eq!(stringify_value(&value), "spotlight probe.txt");
        let all = provider.all_attributes(&file);
        assert!(
            all.iter().any(|(name, _)| name == "kMDItemFSName"),
            "{all:?}"
        );
        let mut sorted = all.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(all, sorted, "inspector list is sorted by name");
    } else {
        eprintln!("skipping: MDItem unavailable on this volume");
    }
}
