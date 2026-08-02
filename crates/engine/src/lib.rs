//! The Name Shift rename engine.
//!
//! Pure domain logic with no Tauri or UI dependencies: rule semantics, token
//! expansion, the preview pipeline, and the two-phase apply/revert executor.

#![warn(missing_docs)]

pub mod copy;
pub mod diffkey;
pub mod execute;
pub mod item;
pub mod plan;
pub mod platform;
pub mod preview;
pub mod revert;
pub mod rule;
pub mod serde_util;
pub mod sort;
pub mod state;
pub mod tokens;
pub mod validate;

pub use diffkey::{diff_key, fold_name};
pub use execute::{
    perform_moves_hierarchical, recover_orphaned_temp_files, rewrite_live_paths,
    rewrite_path_prefix, MoveOutcome, TEMP_PREFIX,
};
pub use item::{FileItem, FileSortKey, FilterMode, ListMode, WatchedFolder};
pub use plan::{apply_plan, build_plan, ApplyOutcome, ApplyPlan};
pub use platform::{
    host_profile, HostOs, PlatformProfile, LINUX_PROFILE, MACOS_PROFILE, WINDOWS_PROFILE,
};
pub use preview::{
    compute_preview, rule_derived_name, DirectoryLister, DirectoryNameCache, FsDirectoryLister,
    Preview, PreviewCounts, PreviewEntry, RuleImpact,
};
pub use revert::{
    compute_revert_preview, record_snapshot, revert_through, RevertEntry, RevertOutcome,
    RevertPreview, RevertStatus,
};
pub use rule::{
    apply_rule, split_name, trimmed_name, CaseStyle, NumberPosition, RenameRule, RuleKind,
};
pub use sort::natural_compare;
pub use state::{CoreState, RulePreset, Snapshot, SnapshotEntry, MAX_HISTORY_COUNT};
pub use tokens::{
    expand_tokens, format_date, format_size, MetadataSource, NoMetadata, TokenContext,
};
pub use validate::{intrinsic_problem, Problem};
