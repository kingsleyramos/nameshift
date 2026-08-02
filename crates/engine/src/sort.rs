//! File-list ordering (§7.2): natural compare via ICU collation, one
//! direction flag for every key, dates read once up front.

use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use icu_collator::options::{CollatorOptions, Strength};
use icu_collator::preferences::CollationNumericOrdering;
use icu_collator::{Collator, CollatorBorrowed, CollatorPreferences};

use crate::item::{FileItem, FileSortKey};
use crate::rule::split_name;
use crate::tokens::MetadataSource;

thread_local! {
    // CollatorBorrowed isn't Sync, so it can't live in a global; building it
    // is cheap (compiled data), one per thread.
    static COLLATOR: CollatorBorrowed<'static> = {
        let mut preferences = CollatorPreferences::default();
        // Finder-style: numeric-aware, so `file2 < file10`.
        preferences.numeric_ordering = Some(CollationNumericOrdering::True);
        let mut options = CollatorOptions::default();
        // Secondary strength ignores case but keeps accent distinctions.
        options.strength = Some(Strength::Secondary);
        Collator::try_new(preferences, options).expect("compiled collation data is always available")
    };
}

/// Natural, case-insensitive, numeric-aware string compare (Finder-style).
pub fn natural_compare(a: &str, b: &str) -> Ordering {
    COLLATOR.with(|collator| collator.compare(a, b))
}

/// Sort active items per §7.2: key ascending first, then reverse the whole
/// result when descending — every key gets a direction. All sorts are stable.
pub fn sort_items<'a>(
    mut items: Vec<&'a FileItem>,
    key: FileSortKey,
    ascending: bool,
    metadata: &dyn MetadataSource,
) -> Vec<&'a FileItem> {
    match key {
        FileSortKey::OrderAdded => {}
        FileSortKey::Name => {
            items.sort_by(|a, b| natural_compare(&a.name(), &b.name()));
        }
        FileSortKey::FileExtension => {
            items.sort_by(|a, b| {
                let a_name = a.name();
                let b_name = b.name();
                let a_ext = split_name(&a_name).1.to_lowercase();
                let b_ext = split_name(&b_name).1.to_lowercase();
                natural_compare(&a_ext, &b_ext).then_with(|| natural_compare(&a_name, &b_name))
            });
        }
        FileSortKey::Folder => {
            items.sort_by(|a, b| {
                let a_dir = a.directory();
                let b_dir = b.directory();
                natural_compare(&a_dir.to_string_lossy(), &b_dir.to_string_lossy())
                    .then_with(|| natural_compare(&a.name(), &b.name()))
            });
        }
        FileSortKey::DateCreated | FileSortKey::DateModified => {
            // Read each item's date ONCE up front — never inside the
            // comparator; per-compare stat calls were a real performance bug.
            let mut keyed: Vec<(&FileItem, SystemTime)> = items
                .into_iter()
                .map(|item| {
                    let date = if key == FileSortKey::DateCreated {
                        metadata.created(&item.path)
                    } else {
                        metadata.modified(&item.path)
                    };
                    // Missing dates sort as distant past.
                    (item, date.unwrap_or(UNIX_EPOCH))
                })
                .collect();
            keyed.sort_by(|(a, a_date), (b, b_date)| {
                a_date
                    .cmp(b_date)
                    .then_with(|| natural_compare(&a.name(), &b.name()))
            });
            items = keyed.into_iter().map(|(item, _)| item).collect();
        }
    }
    if !ascending {
        items.reverse();
    }
    items
}
