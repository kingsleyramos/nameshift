//! `RenameRule`: the ten rule kinds and their transforms (§4.2, §5).

use std::cell::RefCell;
use std::collections::HashMap;

use fancy_regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::tokens::{expand_tokens, TokenContext};

/// The ten rule kinds. Serialized values are the legacy camelCase strings —
/// presets are portable files users may already have (§4.2).
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RuleKind {
    /// Delete every occurrence of `text`.
    RemoveText,
    /// Replace every occurrence of `text` with `replacement`.
    ReplaceText,
    /// Regular-expression replace of `text` with `replacement`.
    RegexReplace,
    /// Prepend `text`.
    AddPrefix,
    /// Append `text` (before the extension under default scope).
    AddSuffix,
    /// Lowercase / UPPERCASE / Title Case.
    ChangeCase,
    /// Replace the extension with `text` (files only).
    ChangeExtension,
    /// Make names Windows/NAS/cloud-safe (displayed as “Fix Unsafe Characters”).
    Sanitize,
    /// Insert a sequence number that follows the current view order.
    NumberSequentially,
    /// Rebuild the whole name from a token template.
    Template,
}

/// Case styles for [`RuleKind::ChangeCase`].
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CaseStyle {
    /// Unicode lowercase.
    #[default]
    Lowercase,
    /// Unicode uppercase.
    Uppercase,
    /// Title Case with an all-caps acronym guard (§5.3-e).
    TitleCase,
}

/// Number placement for [`RuleKind::NumberSequentially`].
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NumberPosition {
    /// `001<sep>name`
    Before,
    /// `name<sep>001`
    #[default]
    After,
    /// The number replaces the name entirely.
    ReplaceName,
}

fn default_true() -> bool {
    true
}
fn default_number_start() -> i64 {
    1
}
fn default_number_padding() -> i64 {
    3
}

/// One rule in the stack. A single struct covers all kinds — exactly the
/// legacy persisted shape, so old presets keep importing (§4.2 serde policy:
/// every field except `kind` decodes with a default; unknown fields ignored).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RenameRule {
    /// Stable identity — rule cards are keyed by this, never by position.
    #[serde(default = "Uuid::new_v4", with = "crate::serde_util::uuid_upper")]
    #[ts(as = "String")]
    pub id: Uuid,
    /// Which transform this rule performs (the only required field).
    pub kind: RuleKind,
    /// Disabled rules stay in the stack but are skipped by the pipeline.
    #[serde(default = "default_true")]
    pub is_enabled: bool,
    /// Transform the whole name including the extension (§5.2).
    #[serde(default)]
    pub includes_extension: bool,
    /// Case-sensitive matching for removeText / replaceText / regexReplace.
    #[serde(default = "default_true")]
    pub case_sensitive: bool,
    /// Primary input: pattern / prefix / suffix / template / new extension /
    /// number separator / sanitize replacement.
    #[serde(default)]
    pub text: String,
    /// Replacement for replaceText / regexReplace.
    #[serde(default)]
    pub replacement: String,
    /// Case style for changeCase.
    #[serde(default)]
    pub case_style: CaseStyle,
    /// Number placement for numberSequentially.
    #[serde(default)]
    pub number_position: NumberPosition,
    /// First number in the sequence (clamped to 0…99999 when applied).
    #[serde(default = "default_number_start")]
    pub number_start: i64,
    /// Zero-pad width (clamped to 1…10 when applied).
    #[serde(default = "default_number_padding")]
    pub number_padding: i64,
    /// Sanitize option: strip diacritics (é → e).
    #[serde(default)]
    pub strips_diacritics: bool,
    /// Number Sequentially option: restart the counter in each folder (§7.3).
    #[serde(default)]
    pub restart_per_folder: bool,
    /// Sanitize option: remove emoji.
    #[serde(default)]
    pub removes_emoji: bool,
}

impl RenameRule {
    /// A new rule of `kind` with every option at its default.
    pub fn new(kind: RuleKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            is_enabled: true,
            includes_extension: false,
            case_sensitive: true,
            text: String::new(),
            replacement: String::new(),
            case_style: CaseStyle::default(),
            number_position: NumberPosition::default(),
            number_start: 1,
            number_padding: 3,
            strips_diacritics: false,
            restart_per_folder: false,
            removes_emoji: false,
        }
    }

    /// Whether the rule has enough input to do something; ineffective rules
    /// are skipped entirely by the pipeline (§4.2).
    pub fn is_effective(&self) -> bool {
        match self.kind {
            RuleKind::RemoveText
            | RuleKind::ReplaceText
            | RuleKind::AddPrefix
            | RuleKind::AddSuffix
            | RuleKind::Template
            | RuleKind::ChangeExtension => !self.text.is_empty(),
            RuleKind::RegexReplace => !self.text.is_empty() && self.pattern_compiles(),
            RuleKind::ChangeCase | RuleKind::NumberSequentially | RuleKind::Sanitize => true,
        }
    }

    /// Whether `text` compiles as a regular expression (regex rules only;
    /// other kinds — and the empty pattern — report `true`).
    pub fn pattern_compiles(&self) -> bool {
        if self.kind != RuleKind::RegexReplace || self.text.is_empty() {
            return true;
        }
        compiled_regex(&self.text, self.case_sensitive).is_some()
    }

    /// Human-readable one-line summary (§4.2), used in snapshot summaries and
    /// rule-card subtitles.
    pub fn summary(&self) -> String {
        crate::copy::rule_summary(self)
    }
}

/// Apply one rule to one name (§5). Pure: same inputs, same output.
/// Ineffective rules return the name unchanged.
pub fn apply_rule(
    rule: &RenameRule,
    name: &str,
    ctx: Option<&TokenContext>,
    is_directory: bool,
) -> String {
    if !rule.is_effective() {
        return name.to_string();
    }
    // Folder names are one whole string — no extension concept (§5.2).
    if is_directory {
        if rule.kind == RuleKind::ChangeExtension {
            return name.to_string();
        }
        return transform(rule, name, "", ctx);
    }
    let (stem, ext) = split_name(name);
    if rule.kind == RuleKind::ChangeExtension {
        let base = if ext.is_empty() { name } else { stem };
        let expanded = expand_tokens(&rule.text, base, ext, ctx);
        let new_ext = expanded.trim().trim_start_matches('.');
        return if new_ext.is_empty() {
            base.to_string()
        } else {
            format!("{base}.{new_ext}")
        };
    }
    if rule.includes_extension || ext.is_empty() {
        transform(rule, name, ext, ctx)
    } else {
        format!("{}.{}", transform(rule, stem, ext, ctx), ext)
    }
}

/// Split a name into `(stem, extension)` per §5.1: the extension starts after
/// the last `.` that is neither the first character nor the last.
pub fn split_name(name: &str) -> (&str, &str) {
    let mut search_end = name.len();
    while let Some(i) = name[..search_end].rfind('.') {
        if i > 0 && i + 1 < name.len() {
            return (&name[..i], &name[i + 1..]);
        }
        search_end = i;
    }
    (name, "")
}

/// The global whitespace-trim post-pass (§5.4). Directories trim the whole
/// name; files additionally re-split and trim the stem so `"  draft .txt  "`
/// becomes `draft.txt`.
pub fn trimmed_name(name: &str, is_directory: bool) -> String {
    let whole = name.trim();
    if is_directory {
        return whole.to_string();
    }
    let (stem, ext) = split_name(whole);
    if ext.is_empty() {
        whole.to_string()
    } else {
        format!("{}.{}", stem.trim(), ext)
    }
}

/// Transform the in-scope string `s` per the rule kind (§5.3). `file_ext` is
/// the extension from the split (empty for directories), fed to `{ext}`.
fn transform(rule: &RenameRule, s: &str, file_ext: &str, ctx: Option<&TokenContext>) -> String {
    let expanded = |field: &str| expand_tokens(field, s, file_ext, ctx);
    match rule.kind {
        RuleKind::RemoveText => replace_occurrences(s, &rule.text, "", rule.case_sensitive),
        RuleKind::ReplaceText => replace_occurrences(
            s,
            &rule.text,
            &expanded(&rule.replacement),
            rule.case_sensitive,
        ),
        RuleKind::RegexReplace => {
            let Some(regex) = compiled_regex(&rule.text, rule.case_sensitive) else {
                return s.to_string();
            };
            // Tokens expand FIRST; the result is the regex replacement
            // template, so `$1` group references work — and a token expanding
            // to a literal `$1` is treated as a group reference
            // (legacy-faithful; noted in help).
            let template = expanded(&rule.replacement);
            regex.replace_all(s, template.as_str()).into_owned()
        }
        RuleKind::AddPrefix => format!("{}{}", expanded(&rule.text), s),
        RuleKind::AddSuffix => format!("{}{}", s, expanded(&rule.text)),
        RuleKind::ChangeCase => match rule.case_style {
            CaseStyle::Lowercase => s.to_lowercase(),
            CaseStyle::Uppercase => s.to_uppercase(),
            CaseStyle::TitleCase => title_case(s),
        },
        RuleKind::NumberSequentially => {
            // Clamps are load-bearing: a hand-edited preset can carry any
            // value; the arithmetic must not overflow and the pad width must
            // not allocate huge strings (§5.3-h).
            let start = rule.number_start.clamp(0, 99_999);
            let index = ctx
                .map(|c| {
                    if rule.restart_per_folder {
                        c.folder_index
                    } else {
                        c.index
                    }
                })
                .unwrap_or(1);
            let n = i64::from(index) + start - 1;
            let width = rule.number_padding.clamp(1, 10) as usize;
            let number = format!("{n:0width$}");
            let sep = expanded(&rule.text);
            match rule.number_position {
                NumberPosition::Before => format!("{number}{sep}{s}"),
                NumberPosition::After => format!("{s}{sep}{number}"),
                NumberPosition::ReplaceName => number,
            }
        }
        RuleKind::Template => expanded(&rule.text),
        RuleKind::Sanitize => sanitized(s, &rule.text, rule.strips_diacritics, rule.removes_emoji),
        // Handled in apply_rule; unreachable here but must stay total.
        RuleKind::ChangeExtension => s.to_string(),
    }
}

thread_local! {
    // Compiling a regex per item would dominate a 10k-file preview; memoize
    // per (pattern, case-sensitivity). Bounded: cleared wholesale if it grows
    // past the size any real rule stack reaches.
    static REGEX_CACHE: RefCell<HashMap<(String, bool), Option<Regex>>> =
        RefCell::new(HashMap::new());
}

fn compiled_regex(pattern: &str, case_sensitive: bool) -> Option<Regex> {
    REGEX_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() > 64 {
            cache.clear();
        }
        cache
            .entry((pattern.to_string(), case_sensitive))
            .or_insert_with(|| {
                let source = if case_sensitive {
                    pattern.to_string()
                } else {
                    format!("(?i){pattern}")
                };
                Regex::new(&source).ok()
            })
            .clone()
    })
}

/// Plain-text replace, left-to-right, non-overlapping. Case-insensitive
/// matching folds both sides char-by-char with *simple* (char-aligned) case
/// folding so indices stay aligned (§16.4).
fn replace_occurrences(
    haystack: &str,
    needle: &str,
    replacement: &str,
    case_sensitive: bool,
) -> String {
    if needle.is_empty() {
        return haystack.to_string();
    }
    if case_sensitive {
        return haystack.replace(needle, replacement);
    }
    let original: Vec<char> = haystack.chars().collect();
    let folded: Vec<char> = original.iter().map(|&c| fold_char(c)).collect();
    let needle_folded: Vec<char> = needle.chars().map(fold_char).collect();
    let mut output = String::with_capacity(haystack.len());
    let mut i = 0;
    while i < original.len() {
        if i + needle_folded.len() <= folded.len()
            && folded[i..i + needle_folded.len()] == needle_folded[..]
        {
            output.push_str(replacement);
            i += needle_folded.len();
        } else {
            output.push(original[i]);
            i += 1;
        }
    }
    output
}

/// Simple case folding, one char to one char. Uppercase-then-lowercase
/// catches the mappings plain lowercasing misses (final sigma ς → σ, long s
/// ſ → s, Kelvin K → k); multi-char full mappings (ß → ss, İ → i̇) keep the
/// original char so the fold stays char-aligned — divergence from ICU
/// canonical caseless matching is accepted and documented (§24 Q14).
fn fold_char(c: char) -> char {
    let mut upper = c.to_uppercase();
    if let (Some(u), None) = (upper.next(), upper.next()) {
        let mut lower = u.to_lowercase();
        if let (Some(l), None) = (lower.next(), lower.next()) {
            return l;
        }
    }
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(single), None) => single,
        _ => c,
    }
}

/// Title Case per §5.3-e: a word is a maximal run of alphabetic chars. A word
/// that is entirely uppercase with ≥ 2 letters keeps its capitalization
/// (acronym guard: NASA, HDR, II); every other word gets first-letter
/// uppercase, rest lowercase.
fn title_case(s: &str) -> String {
    let mut output = String::with_capacity(s.len());
    let mut word = String::new();
    for c in s.chars() {
        if c.is_alphabetic() {
            word.push(c);
        } else {
            flush_title_word(&mut output, &word);
            word.clear();
            output.push(c);
        }
    }
    flush_title_word(&mut output, &word);
    output
}

fn flush_title_word(output: &mut String, word: &str) {
    if word.is_empty() {
        return;
    }
    let letter_count = word.chars().count();
    let is_acronym = letter_count >= 2 && word.chars().all(char::is_uppercase);
    if is_acronym {
        output.push_str(word);
        return;
    }
    let mut chars = word.chars();
    if let Some(first) = chars.next() {
        output.extend(first.to_uppercase());
    }
    for c in chars {
        output.extend(c.to_lowercase());
    }
}

fn is_emoji_scalar(c: char) -> bool {
    use icu_properties::props::{Emoji, EmojiPresentation};
    use icu_properties::CodePointSetData;
    CodePointSetData::new::<EmojiPresentation>().contains(c)
        || (CodePointSetData::new::<Emoji>().contains(c) && c as u32 >= 0x1F300)
}

/// Make a name safe for Windows/NAS/cloud sync (§5.3-f): replace the
/// characters Windows forbids (plus control characters) with `replacement`,
/// optionally strip diacritics and emoji, trim trailing dots/spaces, and
/// guard Windows-reserved device names. Runs on every OS — the rule's
/// promise is portability.
pub fn sanitized(
    name: &str,
    replacement: &str,
    strips_diacritics: bool,
    removes_emoji: bool,
) -> String {
    const ILLEGAL: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let input: String = if strips_diacritics {
        name.nfd()
            .filter(|c| !is_combining_mark(*c))
            .collect::<String>()
            .nfc()
            .collect()
    } else {
        name.to_string()
    };
    let mut output = String::with_capacity(input.len());
    for cluster in input.graphemes(true) {
        let mut scalars = cluster.chars();
        let first = scalars.next().unwrap_or('\u{FFFD}');
        let is_single = scalars.next().is_none();
        if (is_single && ILLEGAL.contains(&first)) || cluster.chars().all(char::is_control) {
            output.push_str(replacement);
        } else if removes_emoji && cluster.chars().any(is_emoji_scalar) {
            continue;
        } else {
            output.push_str(cluster);
        }
    }
    while output.ends_with('.') || output.ends_with(' ') {
        output.pop();
    }
    reserved_name_guard(output, replacement)
}

/// If the segment before the first `.` is a Windows reserved device name,
/// append `replacement` (or `_` when it is empty) to that segment:
/// `CON.txt → CON_.txt`, `nul → nul_` (§5.3-f step 4).
fn reserved_name_guard(name: String, replacement: &str) -> String {
    let (segment, rest) = match name.find('.') {
        Some(i) => name.split_at(i),
        None => (name.as_str(), ""),
    };
    if !is_windows_reserved(segment) {
        return name;
    }
    let suffix = if replacement.is_empty() {
        "_"
    } else {
        replacement
    };
    format!("{segment}{suffix}{rest}")
}

/// Case-insensitive check against `CON PRN AUX NUL COM1–COM9 LPT1–LPT9`.
pub fn is_windows_reserved(segment: &str) -> bool {
    let upper = segment.to_ascii_uppercase();
    match upper.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        _ => {
            (upper.len() == 4)
                && (upper.starts_with("COM") || upper.starts_with("LPT"))
                && upper.as_bytes()[3].is_ascii_digit()
                && upper.as_bytes()[3] != b'0'
        }
    }
}
