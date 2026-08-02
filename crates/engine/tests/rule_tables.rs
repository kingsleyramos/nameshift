//! Per-rule unit tables (§18.1): every §5.3 kind × scope variants, with the
//! §5 vectors verbatim.

use std::path::Path;

use nameshift_engine::{
    apply_rule, split_name, trimmed_name, CaseStyle, NoMetadata, NumberPosition, RenameRule,
    RuleKind, TokenContext,
};

static NO_METADATA: NoMetadata = NoMetadata;

fn rule(kind: RuleKind) -> RenameRule {
    RenameRule::new(kind)
}

fn ctx(index: u32) -> TokenContext<'static> {
    TokenContext {
        index,
        folder_index: index,
        path: Path::new("/tmp/Photos/x.jpg"),
        metadata: &NO_METADATA,
    }
}

fn apply(rule: &RenameRule, name: &str) -> String {
    apply_rule(rule, name, None, false)
}

// ---- §5.1 name splitting -------------------------------------------------

#[test]
fn split_name_vectors() {
    assert_eq!(split_name("photo.jpg"), ("photo", "jpg"));
    assert_eq!(split_name("archive.tar.gz"), ("archive.tar", "gz"));
    assert_eq!(split_name(".gitignore"), (".gitignore", ""));
    assert_eq!(split_name("README"), ("README", ""));
    assert_eq!(split_name("file."), ("file.", ""));
    assert_eq!(split_name(".config.json"), (".config", "json"));
}

// ---- removeText ----------------------------------------------------------

#[test]
fn remove_text_keeps_extension() {
    let mut r = rule(RuleKind::RemoveText);
    r.text = "draft_".into();
    assert_eq!(apply(&r, "draft_report.pdf"), "report.pdf");
}

#[test]
fn remove_text_case_insensitive_folds_char_aligned() {
    let mut r = rule(RuleKind::RemoveText);
    r.text = "COPY".into();
    r.case_sensitive = false;
    assert_eq!(apply(&r, "photo copy.jpg"), "photo .jpg");
    // Simple folding is char-aligned: ß does not match SS (§24 Q14).
    let mut r = rule(RuleKind::RemoveText);
    r.text = "STRASSE".into();
    r.case_sensitive = false;
    assert_eq!(apply(&r, "straße.txt"), "straße.txt");
    // But Σ/σ fold together.
    let mut r = rule(RuleKind::RemoveText);
    r.text = "ΣΊΣΥΦΟΣ".into();
    r.case_sensitive = false;
    assert_eq!(apply(&r, "σίσυφος notes.txt"), " notes.txt");
}

#[test]
fn remove_text_non_overlapping_left_to_right() {
    let mut r = rule(RuleKind::RemoveText);
    r.text = "aa".into();
    assert_eq!(apply(&r, "aaa.txt"), "a.txt");
}

#[test]
fn remove_text_includes_extension() {
    let mut r = rule(RuleKind::RemoveText);
    r.text = ".jpg".into();
    r.includes_extension = true;
    assert_eq!(apply(&r, "photo.jpg"), "photo");
}

#[test]
fn remove_text_on_directory_uses_whole_name() {
    let mut r = rule(RuleKind::RemoveText);
    r.text = ".jpg".into();
    assert_eq!(apply_rule(&r, "shots.jpg", None, true), "shots");
}

#[test]
fn remove_text_empty_is_ineffective() {
    let r = rule(RuleKind::RemoveText);
    assert!(!r.is_effective());
    assert_eq!(apply(&r, "file.txt"), "file.txt");
}

// ---- replaceText ---------------------------------------------------------

#[test]
fn replace_text_plain() {
    let mut r = rule(RuleKind::ReplaceText);
    r.text = " ".into();
    r.replacement = "-".into();
    assert_eq!(apply(&r, "my vacation photo.jpg"), "my-vacation-photo.jpg");
}

#[test]
fn replace_text_expands_tokens_in_replacement_only() {
    let mut r = rule(RuleKind::ReplaceText);
    r.text = "cat".into();
    r.replacement = "{ext}".into();
    // The needle is never token-expanded; the replacement is.
    assert_eq!(apply(&r, "cat.txt"), "txt.txt");
}

#[test]
fn replace_text_case_insensitive() {
    let mut r = rule(RuleKind::ReplaceText);
    r.text = "IMG".into();
    r.replacement = "pic".into();
    r.case_sensitive = false;
    assert_eq!(apply(&r, "img_0421.jpeg"), "pic_0421.jpeg");
}

// ---- regexReplace --------------------------------------------------------

#[test]
fn regex_replace_with_group_references() {
    let mut r = rule(RuleKind::RegexReplace);
    r.text = r"(\d+)".into();
    r.replacement = "#$1".into();
    assert_eq!(apply(&r, "IMG_0421.jpeg"), "IMG_#0421.jpeg");
}

#[test]
fn regex_replace_all_matches() {
    let mut r = rule(RuleKind::RegexReplace);
    r.text = r"\d".into();
    r.replacement = "x".into();
    assert_eq!(apply(&r, "a1b2c3.txt"), "axbxcx.txt");
}

#[test]
fn regex_replace_supports_lookaround() {
    let mut r = rule(RuleKind::RegexReplace);
    r.text = r"a(?=b)".into();
    r.replacement = "X".into();
    assert_eq!(apply(&r, "ab ac.txt"), "Xb ac.txt");
}

#[test]
fn regex_replace_case_insensitive_flag() {
    let mut r = rule(RuleKind::RegexReplace);
    r.text = "img".into();
    r.replacement = "pic".into();
    r.case_sensitive = false;
    assert_eq!(apply(&r, "IMG_1.jpg"), "pic_1.jpg");
}

#[test]
fn regex_replace_invalid_pattern_is_ineffective() {
    let mut r = rule(RuleKind::RegexReplace);
    r.text = "(".into();
    assert!(!r.is_effective());
    assert_eq!(apply(&r, "file(1).txt"), "file(1).txt");
}

#[test]
fn regex_replace_tokens_expand_before_template() {
    // A token expanding to a literal $1 is treated as a group reference —
    // legacy-faithful (§5.3-c).
    let mut r = rule(RuleKind::RegexReplace);
    r.text = r"(\d+)".into();
    r.replacement = "{name}".into();
    // {name} expands to the in-scope stem "a1"; "$" is absent so it's literal.
    assert_eq!(apply(&r, "a1.txt"), "aa1.txt");
}

// ---- addPrefix / addSuffix ----------------------------------------------

#[test]
fn prefix_plain() {
    let mut r = rule(RuleKind::AddPrefix);
    r.text = "2026-".into();
    assert_eq!(apply(&r, "invoice.pdf"), "2026-invoice.pdf");
}

#[test]
fn prefix_with_token() {
    let mut r = rule(RuleKind::AddPrefix);
    r.text = "{n:2}_".into();
    assert_eq!(
        apply_rule(&r, "song.mp3", Some(&ctx(7)), false),
        "07_song.mp3"
    );
}

#[test]
fn suffix_goes_before_extension() {
    let mut r = rule(RuleKind::AddSuffix);
    r.text = "_final".into();
    assert_eq!(apply(&r, "essay.docx"), "essay_final.docx");
}

#[test]
fn suffix_including_extension() {
    let mut r = rule(RuleKind::AddSuffix);
    r.text = ".bak".into();
    r.includes_extension = true;
    assert_eq!(apply(&r, "notes.txt"), "notes.txt.bak");
}

#[test]
fn suffix_on_file_without_extension_uses_whole_name() {
    let mut r = rule(RuleKind::AddSuffix);
    r.text = "-v2".into();
    assert_eq!(apply(&r, "Makefile"), "Makefile-v2");
}

#[test]
fn suffix_on_multi_dot_name() {
    let mut r = rule(RuleKind::AddSuffix);
    r.text = "_x".into();
    assert_eq!(apply(&r, "archive.tar.gz"), "archive.tar_x.gz");
}

#[test]
fn prefix_on_dotfile_transforms_whole_name() {
    let mut r = rule(RuleKind::AddPrefix);
    r.text = "old".into();
    assert_eq!(apply(&r, ".gitignore"), "old.gitignore");
}

// ---- changeCase ----------------------------------------------------------

#[test]
fn change_case_does_not_touch_extension() {
    let mut r = rule(RuleKind::ChangeCase);
    r.case_style = CaseStyle::Uppercase;
    assert_eq!(apply(&r, "readme.md"), "README.md");
}

#[test]
fn change_case_lowercase() {
    let r = rule(RuleKind::ChangeCase);
    assert_eq!(apply(&r, "README.MD"), "readme.MD");
}

#[test]
fn title_case_vectors() {
    let mut r = rule(RuleKind::ChangeCase);
    r.case_style = CaseStyle::TitleCase;
    let vectors = [
        ("hello world", "Hello World"),
        ("hello-world", "Hello-World"),
        ("NASA report", "NASA Report"),
        ("4K HDR clip", "4K HDR Clip"),
        ("v2 draft", "V2 Draft"),
        ("iPhone photo", "Iphone Photo"),
        ("it’s", "It’S"),
        ("abc3def", "Abc3Def"),
        ("ΣΊΣΥΦΟΣ", "ΣΊΣΥΦΟΣ"),
    ];
    for (input, expected) in vectors {
        assert_eq!(
            apply_rule(&r, input, None, true),
            expected,
            "input {input:?}"
        );
    }
}

// ---- changeExtension -----------------------------------------------------

#[test]
fn change_extension_replaces() {
    let mut r = rule(RuleKind::ChangeExtension);
    r.text = "jpg".into();
    assert_eq!(apply(&r, "photo.jpeg"), "photo.jpg");
}

#[test]
fn change_extension_gains_one() {
    let mut r = rule(RuleKind::ChangeExtension);
    r.text = "md".into();
    assert_eq!(apply(&r, "README"), "README.md");
}

#[test]
fn change_extension_strips_leading_dots_and_whitespace() {
    let mut r = rule(RuleKind::ChangeExtension);
    r.text = " ..jpg ".into();
    assert_eq!(apply(&r, "photo.png"), "photo.jpg");
}

#[test]
fn change_extension_empty_result_drops_extension() {
    let mut r = rule(RuleKind::ChangeExtension);
    r.text = ".".into();
    assert_eq!(apply(&r, "photo.png"), "photo");
}

#[test]
fn change_extension_is_noop_for_directories() {
    let mut r = rule(RuleKind::ChangeExtension);
    r.text = "jpg".into();
    assert_eq!(apply_rule(&r, "My Folder", None, true), "My Folder");
}

#[test]
fn change_extension_expands_tokens() {
    let mut r = rule(RuleKind::ChangeExtension);
    r.text = "{ext}2".into();
    assert_eq!(apply(&r, "a.png"), "a.png2");
}

#[test]
fn change_extension_empty_is_ineffective() {
    let r = rule(RuleKind::ChangeExtension);
    assert!(!r.is_effective());
    assert_eq!(apply(&r, "a.png"), "a.png");
}

// ---- sanitize ------------------------------------------------------------

#[test]
fn sanitize_replaces_illegal_characters() {
    let r = rule(RuleKind::Sanitize);
    assert_eq!(apply(&r, "a<b>c: d?.txt"), "abc d.txt");
    let mut r = rule(RuleKind::Sanitize);
    r.text = "_".into();
    assert_eq!(apply(&r, "a/b\\c.txt"), "a_b_c.txt");
}

#[test]
fn sanitize_replaces_control_characters() {
    let r = rule(RuleKind::Sanitize);
    assert_eq!(apply(&r, "a\u{0007}b.txt"), "ab.txt");
}

#[test]
fn sanitize_strips_trailing_dots_and_spaces() {
    let r = rule(RuleKind::Sanitize);
    assert_eq!(apply_rule(&r, "ends. ", None, true), "ends");
    assert_eq!(apply_rule(&r, "name....", None, true), "name");
}

#[test]
fn sanitize_diacritics() {
    let mut r = rule(RuleKind::Sanitize);
    r.strips_diacritics = true;
    assert_eq!(apply(&r, "café résumé.txt"), "cafe resume.txt");
    // Off by default.
    let r = rule(RuleKind::Sanitize);
    assert_eq!(apply(&r, "café.txt"), "café.txt");
}

#[test]
fn sanitize_emoji() {
    let mut r = rule(RuleKind::Sanitize);
    r.removes_emoji = true;
    assert_eq!(apply(&r, "hi 👋 there.txt"), "hi  there.txt");
    // ZWJ sequences drop as one cluster.
    assert_eq!(apply(&r, "team 👩‍👩‍👧 pic.txt"), "team  pic.txt");
    // Kept when the option is off.
    let r = rule(RuleKind::Sanitize);
    assert_eq!(apply(&r, "hi 👋.txt"), "hi 👋.txt");
}

#[test]
fn sanitize_reserved_name_vectors() {
    let r = rule(RuleKind::Sanitize);
    assert_eq!(apply(&r, "CON.txt"), "CON_.txt");
    assert_eq!(apply(&r, "nul"), "nul_");
    assert_eq!(apply(&r, "CoN.txt"), "CoN_.txt");
    assert_eq!(apply(&r, "com7.log"), "com7_.log");
    assert_eq!(apply(&r, "LPT9"), "LPT9_");
    // Near-misses stay untouched.
    assert_eq!(apply(&r, "CONSOLE.txt"), "CONSOLE.txt");
    assert_eq!(apply(&r, "COM10.txt"), "COM10.txt");
    assert_eq!(apply(&r, "connect.log"), "connect.log");
    assert_eq!(apply(&r, "COM0.txt"), "COM0.txt");
    // A non-empty replacement string is used instead of the underscore.
    let mut r = rule(RuleKind::Sanitize);
    r.text = "-".into();
    assert_eq!(apply(&r, "aux.tar.gz"), "aux-.tar.gz");
}

// ---- numberSequentially --------------------------------------------------

#[test]
fn number_suffix_with_separator() {
    let mut r = rule(RuleKind::NumberSequentially);
    r.text = " - ".into();
    assert_eq!(
        apply_rule(&r, "photo.jpg", Some(&ctx(1)), false),
        "photo - 001.jpg"
    );
    assert_eq!(
        apply_rule(&r, "photo.jpg", Some(&ctx(12)), false),
        "photo - 012.jpg"
    );
}

#[test]
fn number_replace_name_with_start() {
    let mut r = rule(RuleKind::NumberSequentially);
    r.number_position = NumberPosition::ReplaceName;
    r.number_start = 100;
    r.number_padding = 4;
    assert_eq!(
        apply_rule(&r, "whatever.png", Some(&ctx(3)), false),
        "0102.png"
    );
}

#[test]
fn number_before_name() {
    let mut r = rule(RuleKind::NumberSequentially);
    r.number_position = NumberPosition::Before;
    r.text = "-".into();
    r.number_padding = 2;
    assert_eq!(
        apply_rule(&r, "apple.txt", Some(&ctx(1)), false),
        "01-apple.txt"
    );
}

#[test]
fn number_clamps_start_and_padding() {
    // A hand-edited preset can carry any value; clamps prevent overflow and
    // huge allocations (§5.3-h).
    let mut r = rule(RuleKind::NumberSequentially);
    r.number_position = NumberPosition::ReplaceName;
    r.number_start = i64::MAX;
    r.number_padding = 1;
    assert_eq!(apply_rule(&r, "x.jpg", Some(&ctx(2)), false), "100000.jpg");
    let mut r = rule(RuleKind::NumberSequentially);
    r.number_position = NumberPosition::ReplaceName;
    r.number_start = 1_000_000_000;
    r.number_padding = 99;
    assert_eq!(
        apply_rule(&r, "x.jpg", Some(&ctx(1)), false),
        "0000099999.jpg"
    );
    // Numbers wider than the pad width are never truncated.
    let mut r = rule(RuleKind::NumberSequentially);
    r.number_position = NumberPosition::ReplaceName;
    r.number_start = 123_456;
    r.number_padding = 2;
    assert_eq!(apply_rule(&r, "x.jpg", Some(&ctx(1)), false), "99999.jpg");
}

#[test]
fn number_restart_per_folder_reads_folder_index() {
    let mut r = rule(RuleKind::NumberSequentially);
    r.restart_per_folder = true;
    r.number_position = NumberPosition::ReplaceName;
    let context = TokenContext {
        index: 7,
        folder_index: 2,
        path: Path::new("/tmp/a/x.jpg"),
        metadata: &NO_METADATA,
    };
    assert_eq!(apply_rule(&r, "x.jpg", Some(&context), false), "002.jpg");
    // {n} always uses the global index even when restart is on (§6.3).
    r.number_position = NumberPosition::After;
    r.text = " {n}-".into();
    assert_eq!(
        apply_rule(&r, "x.jpg", Some(&context), false),
        "x 7-002.jpg"
    );
}

#[test]
fn number_without_context_uses_one() {
    let mut r = rule(RuleKind::NumberSequentially);
    r.number_position = NumberPosition::ReplaceName;
    assert_eq!(apply(&r, "x.jpg"), "001.jpg");
}

// ---- template ------------------------------------------------------------

#[test]
fn template_rebuilds_from_tokens() {
    let mut r = rule(RuleKind::Template);
    r.text = "{folder}-{name}-{n:2}".into();
    assert_eq!(
        apply_rule(&r, "x.jpg", Some(&ctx(5)), false),
        "Photos-x-05.jpg"
    );
}

#[test]
fn template_preserves_extension_by_default() {
    let mut r = rule(RuleKind::Template);
    r.text = "renamed".into();
    assert_eq!(apply(&r, "old.tar.gz"), "renamed.gz");
}

#[test]
fn template_empty_is_ineffective() {
    let r = rule(RuleKind::Template);
    assert!(!r.is_effective());
    assert_eq!(apply(&r, "x.jpg"), "x.jpg");
}

// ---- disabled / ineffective in a chain -----------------------------------

#[test]
fn rules_chain_in_order() {
    let mut remove = rule(RuleKind::RemoveText);
    remove.text = "IMG_".into();
    let mut prefix = rule(RuleKind::AddPrefix);
    prefix.text = "vacation-".into();
    let result = [remove, prefix]
        .iter()
        .fold("IMG_0421.jpeg".to_string(), |name, r| {
            apply_rule(r, &name, None, false)
        });
    assert_eq!(result, "vacation-0421.jpeg");
}

// ---- §5.4 whitespace trim ------------------------------------------------

#[test]
fn trim_pass_files_and_directories() {
    assert_eq!(trimmed_name("  draft .txt  ", false), "draft.txt");
    assert_eq!(trimmed_name("  My Folder  ", true), "My Folder");
    assert_eq!(trimmed_name("  notes  ", false), "notes");
    assert_eq!(
        trimmed_name("\tspaced name .pdf ", false),
        "spaced name.pdf"
    );
}

// ---- unicode robustness --------------------------------------------------

#[test]
fn unicode_names_survive_transforms() {
    let mut r = rule(RuleKind::RemoveText);
    r.text = "写真".into();
    assert_eq!(apply(&r, "写真集 2026.jpg"), "集 2026.jpg");

    let mut r = rule(RuleKind::AddSuffix);
    r.text = " ✅".into();
    assert_eq!(apply(&r, "done.txt"), "done ✅.txt");

    // Combining marks stay attached through case changes.
    let mut r = rule(RuleKind::ChangeCase);
    r.case_style = CaseStyle::Uppercase;
    assert_eq!(apply(&r, "étude no. 1.pdf"), "ÉTUDE NO. 1.pdf");
}
