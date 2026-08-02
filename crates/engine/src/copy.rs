//! Engine-originated user-facing copy (§21.6) — one greppable module.
//! Typographic quotes and the exact wording are normative (§0.1, §A).

use crate::rule::{CaseStyle, NumberPosition, RenameRule, RuleKind};

/// Per-move error: phase-1 staging failure (§A).
pub fn couldnt_rename(name: &str, reason: &str) -> String {
    format!("Couldn’t rename “{name}”: {reason}")
}

/// Per-move error: phase-2 commit failure (§A).
pub fn couldnt_rename_to(from: &str, to: &str, reason: &str) -> String {
    format!("Couldn’t rename “{from}” to “{to}”: {reason}")
}

/// Per-move error: a cancelled move that could be neither rolled back nor
/// completed — the file is intact under its temp name (§A).
pub fn couldnt_finish_stranded(name: &str, temp: &str) -> String {
    format!("Couldn’t finish renaming “{name}”; it is safe at “{temp}”.")
}

/// Title for batch-error alerts (§A).
pub const SOME_RENAMES_COULDNT_COMPLETE: &str = "Some renames couldn’t complete";

/// Revert-preview status label (§A).
pub fn revert_status_label(status: crate::revert::RevertStatus) -> &'static str {
    match status {
        crate::revert::RevertStatus::Ok => "Will be restored",
        crate::revert::RevertStatus::Missing => "Not found (nothing to restore)",
        crate::revert::RevertStatus::NameTaken => {
            "The original name is taken by a different file — kept as is."
        }
    }
}

/// Tooltip for the `Missing` revert status (§A).
pub const REVERT_MISSING_TOOLTIP: &str =
    "It may have been moved or deleted since this version was applied. This rename will be skipped.";

/// One-line rule summary (§4.2), used in snapshot summaries and rule-card
/// subtitles.
pub fn rule_summary(rule: &RenameRule) -> String {
    match rule.kind {
        RuleKind::RemoveText => format!("Remove “{}”", rule.text),
        RuleKind::ReplaceText => format!("Replace “{}” with “{}”", rule.text, rule.replacement),
        RuleKind::RegexReplace => format!("Regex “{}” → “{}”", rule.text, rule.replacement),
        RuleKind::AddPrefix => format!("Prefix “{}”", rule.text),
        RuleKind::AddSuffix => format!("Suffix “{}”", rule.text),
        RuleKind::ChangeCase => format!("Case → {}", case_style_title(rule.case_style)),
        RuleKind::ChangeExtension => format!("Extension → “{}”", rule.text),
        RuleKind::Sanitize => "Fix unsafe characters".to_string(),
        RuleKind::NumberSequentially => {
            let position = match rule.number_position {
                NumberPosition::Before => "before name",
                NumberPosition::After => "after name",
                NumberPosition::ReplaceName => "replace name",
            };
            let mut summary = format!("Number {position} from {}", rule.number_start);
            if rule.restart_per_folder {
                summary.push_str(" per folder");
            }
            summary
        }
        RuleKind::Template => format!("Template “{}”", rule.text),
    }
}

/// Display title for a case style (also the segmented-control labels).
pub fn case_style_title(style: CaseStyle) -> &'static str {
    match style {
        CaseStyle::Lowercase => "lowercase",
        CaseStyle::Uppercase => "UPPERCASE",
        CaseStyle::TitleCase => "Title Case",
    }
}

/// The trim-spaces pass as it appears in snapshot summaries (§8.1).
pub const SUMMARY_TRIM_SPACES: &str = "Trim spaces";
/// The manual-edits marker in snapshot summaries (§8.1).
pub const SUMMARY_MANUAL_EDITS: &str = "Manual edits";
/// Prepended to snapshot summaries recorded in Folders mode (§8.1).
pub const SUMMARY_FOLDERS_PREFIX: &str = "Folders";

/// Human-readable problem message (§A), varying per OS where the rules do.
pub fn problem_message(problem: crate::validate::Problem, os: crate::platform::HostOs) -> String {
    use crate::platform::HostOs;
    use crate::validate::Problem;
    match problem {
        Problem::EmptyName => "The new name would be empty.".to_string(),
        Problem::InvalidCharacters => match os {
            HostOs::MacOs => "The new name contains “/” or “:”, which aren’t allowed.".to_string(),
            HostOs::Windows => {
                "The new name contains characters Windows doesn’t allow: < > : \" / \\ | ? *"
                    .to_string()
            }
            HostOs::Linux => "The new name contains “/”, which isn’t allowed.".to_string(),
        },
        Problem::NameTooLong => match os {
            HostOs::MacOs => "The new name is longer than macOS allows (255 bytes).".to_string(),
            HostOs::Windows => {
                "The new name is longer than Windows allows (255 characters).".to_string()
            }
            HostOs::Linux => {
                "The new name is longer than this system allows (255 bytes).".to_string()
            }
        },
        Problem::ReservedName => "This name is reserved by Windows and can’t be used.".to_string(),
        Problem::EndsWithDotOrSpace => "Windows names can’t end with a dot or a space.".to_string(),
        Problem::DuplicateTarget => {
            "Two or more files would end up with the same name.".to_string()
        }
        Problem::ExistingFileCollision => {
            "A different file with this name already exists in the folder.".to_string()
        }
        Problem::UnrenamableName => {
            "This name uses an encoding Name Shift can’t edit safely.".to_string()
        }
        Problem::NoFolderPermission => {
            "Name Shift doesn’t have permission to rename items in this folder.".to_string()
        }
    }
}

/// Why Apply is disabled (§A), shown beside the primary button when items
/// exist but `can_apply` is false. `is_folders` selects the mode noun.
pub fn apply_disabled_reason(
    conflict_count: u32,
    change_count: u32,
    is_folders: bool,
) -> Option<String> {
    if conflict_count > 0 {
        let plural = if conflict_count == 1 { "" } else { "s" };
        return Some(format!("Fix or skip the naming conflict{plural} to rename"));
    }
    if change_count == 0 {
        let noun = if is_folders { "folder" } else { "file" };
        return Some(format!(
            "Add a rule that changes at least one included {noun}"
        ));
    }
    None
}
