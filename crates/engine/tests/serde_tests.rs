//! Serde-policy tests (§4.2): decode with defaults, ignore unknown fields,
//! exact legacy raw values, uppercase UUID output.

use nameshift_engine::{
    CaseStyle, FileSortKey, ListMode, NumberPosition, RenameRule, RuleKind, Snapshot,
};

#[test]
fn rule_with_only_kind_and_text_decodes_with_defaults() {
    let rule: RenameRule =
        serde_json::from_str(r#"{ "kind": "template", "text": "{created} {name}" }"#)
            .expect("decodes");
    assert_eq!(rule.kind, RuleKind::Template);
    assert_eq!(rule.text, "{created} {name}");
    assert!(rule.is_enabled);
    assert!(rule.case_sensitive);
    assert!(!rule.includes_extension);
    assert_eq!(rule.replacement, "");
    assert_eq!(rule.case_style, CaseStyle::Lowercase);
    assert_eq!(rule.number_position, NumberPosition::After);
    assert_eq!(rule.number_start, 1);
    assert_eq!(rule.number_padding, 3);
    assert!(!rule.strips_diacritics);
    assert!(!rule.restart_per_folder);
    assert!(!rule.removes_emoji);
}

#[test]
fn rule_ignores_unknown_fields() {
    let rule: RenameRule = serde_json::from_str(
        r#"{ "kind": "addPrefix", "text": "x-", "someFutureOption": true, "another": [1,2] }"#,
    )
    .expect("unknown fields are ignored");
    assert_eq!(rule.kind, RuleKind::AddPrefix);
    assert_eq!(rule.text, "x-");
}

#[test]
fn rule_without_kind_fails() {
    assert!(serde_json::from_str::<RenameRule>(r#"{ "text": "x" }"#).is_err());
}

#[test]
fn rule_kind_raw_values() {
    for (kind, raw) in [
        (RuleKind::RemoveText, "removeText"),
        (RuleKind::ReplaceText, "replaceText"),
        (RuleKind::RegexReplace, "regexReplace"),
        (RuleKind::AddPrefix, "addPrefix"),
        (RuleKind::AddSuffix, "addSuffix"),
        (RuleKind::ChangeCase, "changeCase"),
        (RuleKind::ChangeExtension, "changeExtension"),
        (RuleKind::Sanitize, "sanitize"),
        (RuleKind::NumberSequentially, "numberSequentially"),
        (RuleKind::Template, "template"),
    ] {
        assert_eq!(serde_json::to_string(&kind).unwrap(), format!("\"{raw}\""));
        assert_eq!(
            serde_json::from_str::<RuleKind>(&format!("\"{raw}\"")).unwrap(),
            kind
        );
    }
}

#[test]
fn enum_raw_values() {
    assert_eq!(
        serde_json::to_string(&CaseStyle::TitleCase).unwrap(),
        "\"titleCase\""
    );
    assert_eq!(
        serde_json::to_string(&NumberPosition::ReplaceName).unwrap(),
        "\"replaceName\""
    );
    assert_eq!(
        serde_json::to_string(&ListMode::Folders).unwrap(),
        "\"Folders\""
    );
    assert_eq!(
        serde_json::to_string(&FileSortKey::OrderAdded).unwrap(),
        "\"Order Added\""
    );
    assert_eq!(
        serde_json::to_string(&FileSortKey::FileExtension).unwrap(),
        "\"Extension\""
    );
    assert_eq!(
        serde_json::to_string(&FileSortKey::DateCreated).unwrap(),
        "\"Date Created\""
    );
    assert_eq!(
        serde_json::to_string(&FileSortKey::DateModified).unwrap(),
        "\"Date Modified\""
    );
}

#[test]
fn uuids_emit_uppercase_and_accept_any_case() {
    let json = r#"{ "kind": "sanitize", "id": "7b4c2a10-53e5-4d2a-9c6f-2e8b1f0a9d11" }"#;
    let rule: RenameRule = serde_json::from_str(json).expect("lowercase uuid accepted");
    let encoded = serde_json::to_string(&rule).unwrap();
    assert!(
        encoded.contains("7B4C2A10-53E5-4D2A-9C6F-2E8B1F0A9D11"),
        "uppercase-hyphenated on write: {encoded}"
    );
}

#[test]
fn rule_serializes_camel_case_fields() {
    let mut rule = RenameRule::new(RuleKind::NumberSequentially);
    rule.restart_per_folder = true;
    let value = serde_json::to_value(&rule).unwrap();
    let object = value.as_object().unwrap();
    for key in [
        "id",
        "kind",
        "isEnabled",
        "includesExtension",
        "caseSensitive",
        "text",
        "replacement",
        "caseStyle",
        "numberPosition",
        "numberStart",
        "numberPadding",
        "stripsDiacritics",
        "restartPerFolder",
        "removesEmoji",
    ] {
        assert!(object.contains_key(key), "missing {key}");
    }
}

#[test]
fn snapshot_dates_parse_fractional_and_emit_seconds() {
    let json = r#"{
        "id": "7B4C2A10-53E5-4D2A-9C6F-2E8B1F0A9D11",
        "date": "2026-05-11T09:30:00.123Z",
        "summary": "Prefix “photo-”",
        "entries": [{ "from": "/a/old.txt", "to": "/a/new.txt" }]
    }"#;
    let snapshot: Snapshot = serde_json::from_str(json).expect("fractional seconds accepted");
    let encoded = serde_json::to_string(&snapshot).unwrap();
    assert!(
        encoded.contains("\"2026-05-11T09:30:00Z\""),
        "whole seconds on write: {encoded}"
    );
    assert!(
        encoded.contains("“photo-”"),
        "typographic quotes survive: {encoded}"
    );
}
