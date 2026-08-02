//! Engine-originated user-facing copy (§21.6) — one greppable module.
//! Typographic quotes and the exact wording are normative (§0.1, §A).

use crate::rule::{CaseStyle, NumberPosition, RenameRule, RuleKind};

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
