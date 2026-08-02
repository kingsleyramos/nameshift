//! The Name Shift rename engine.
//!
//! Pure domain logic with no Tauri or UI dependencies: rule semantics, token
//! expansion, the preview pipeline, and the two-phase apply/revert executor.

#![warn(missing_docs)]

pub mod copy;
pub mod item;
pub mod rule;
pub mod serde_util;
pub mod state;
pub mod tokens;

pub use item::{FileItem, FileSortKey, FilterMode, ListMode, WatchedFolder};
pub use rule::{
    apply_rule, split_name, trimmed_name, CaseStyle, NumberPosition, RenameRule, RuleKind,
};
pub use state::{CoreState, RulePreset, Snapshot, SnapshotEntry, MAX_HISTORY_COUNT};
pub use tokens::{
    expand_tokens, format_date, format_size, MetadataSource, NoMetadata, TokenContext,
};
