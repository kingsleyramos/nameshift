//! CSV build/parse tests (§18.1): round-trip, quoting, tab fallback, header
//! rejection, extension inheritance, duplicate-row flagging.

use std::path::Path;

use nameshift_engine::csv::{
    build_name_mapping_csv, csv_dry_run, csv_field, parse_name_mapping, CsvMissReason,
};
use nameshift_engine::{CoreState, FileItem};

fn state_with_names(names: &[&str]) -> CoreState {
    CoreState {
        files: names
            .iter()
            .map(|name| FileItem::new(&Path::new("/d").join(name), false))
            .collect(),
        ..CoreState::default()
    }
}

#[test]
fn build_and_parse_round_trip() {
    let names = vec![
        "plain.txt".to_string(),
        "with, comma.txt".to_string(),
        "with \"quotes\".txt".to_string(),
        " leading space.txt".to_string(),
    ];
    let csv = build_name_mapping_csv(&names);
    let parsed = parse_name_mapping(&csv).expect("header valid");
    assert_eq!(parsed.rows.len(), 4);
    for (row, name) in parsed.rows.iter().zip(&names) {
        // Fields are trimmed on parse; quoting preserved the inner text.
        assert_eq!(&row.from, &row.to);
        assert_eq!(row.from, name.trim());
    }
    assert!(parsed.unreadable.is_empty());
}

#[test]
fn csv_field_quoting() {
    assert_eq!(csv_field("plain"), "plain");
    assert_eq!(csv_field("a,b"), "\"a,b\"");
    assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    assert_eq!(csv_field(" padded "), "\" padded \"");
}

#[test]
fn tab_separated_fallback() {
    let text = "Current Name\tNew Name\nold.txt\tnew.txt";
    let parsed = parse_name_mapping(text).expect("tab header accepted");
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].from, "old.txt");
    assert_eq!(parsed.rows[0].to, "new.txt");
}

#[test]
fn header_is_required_case_insensitively() {
    assert!(parse_name_mapping("current name,NEW NAME\na.txt,b.txt").is_ok());
    assert!(
        parse_name_mapping("a.txt,b.txt").is_err(),
        "data first → rejected"
    );
    assert!(parse_name_mapping("Wrong,Header\na,b").is_err());
    assert!(parse_name_mapping("").is_err(), "empty file has no header");
    // Blank lines before the header are fine.
    assert!(parse_name_mapping("\n\nCurrent Name,New Name\n").is_ok());
}

#[test]
fn unreadable_lines_are_reported() {
    let text = "Current Name,New Name\nok.txt,fine.txt\nonly-one-field\n,empty-from";
    let parsed = parse_name_mapping(text).expect("header valid");
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(
        parsed.unreadable,
        vec!["only-one-field".to_string(), ",empty-from".to_string()]
    );
}

#[test]
fn dry_run_matches_case_insensitively_and_inherits_extension() {
    let state = state_with_names(&["IMG_001.jpg", "notes"]);
    let text = "Current Name,New Name\nimg_001.JPG,beach day\nnotes,journal";
    let report = csv_dry_run(&state, text).expect("header valid");
    assert_eq!(report.matches.len(), 2);
    // The new name typed without an extension keeps the file's.
    assert_eq!(report.matches[0].new_name, "beach day.jpg");
    // A file without an extension inherits nothing.
    assert_eq!(report.matches[1].new_name, "journal");
    assert_eq!(report.will_change, 2);
    assert_eq!(report.unchanged, 0);
    assert!(report.misses.is_empty());
}

#[test]
fn dry_run_counts_unchanged_and_misses() {
    let state = state_with_names(&["a.txt", "b.txt"]);
    let text = "Current Name,New Name\na.txt,a.txt\nb.txt,renamed.txt\nghost.txt,x.txt";
    let report = csv_dry_run(&state, text).expect("header valid");
    assert_eq!(report.will_change, 1);
    assert_eq!(report.unchanged, 1);
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].name, "ghost.txt");
    assert_eq!(report.misses[0].reason, CsvMissReason::NotInList);
}

#[test]
fn duplicate_rows_flag_the_second_occurrence() {
    // The FIRST occurrence applies; later ones are flagged (§A — a
    // deliberate departure from the legacy last-row-wins overwrite).
    let state = state_with_names(&["a.txt"]);
    let text = "Current Name,New Name\na.txt,first.txt\nA.TXT,second.txt";
    let report = csv_dry_run(&state, text).expect("header valid");
    assert_eq!(report.matches.len(), 1);
    assert_eq!(report.matches[0].new_name, "first.txt");
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].reason, CsvMissReason::DuplicateRow);
    assert_eq!(report.misses[0].name, "A.TXT");
}

#[test]
fn dry_run_reports_unreadable_lines_as_misses() {
    let state = state_with_names(&["a.txt"]);
    let text = "Current Name,New Name\nbroken-line\na.txt,b.txt";
    let report = csv_dry_run(&state, text).expect("header valid");
    assert_eq!(report.matches.len(), 1);
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].reason, CsvMissReason::CouldntRead);
}

#[test]
fn dry_run_is_scoped_to_the_active_mode() {
    let mut state = state_with_names(&["a.txt"]);
    state
        .files
        .push(FileItem::new(Path::new("/d/Photos"), true));
    let text = "Current Name,New Name\nPhotos,Shoots";
    let report = csv_dry_run(&state, text).expect("header valid");
    assert!(
        report.matches.is_empty(),
        "folders don't match in Files mode"
    );
    assert_eq!(report.misses[0].reason, CsvMissReason::NotInList);

    state.list_mode = nameshift_engine::ListMode::Folders;
    let report = csv_dry_run(&state, text).expect("header valid");
    assert_eq!(report.matches.len(), 1);
    assert_eq!(
        report.matches[0].new_name, "Shoots",
        "folders never inherit extensions"
    );
}

#[test]
fn quoted_fields_with_commas_and_quotes() {
    let text = "Current Name,New Name\n\"a, b.txt\",\"c \"\"d\"\".txt\"";
    let parsed = parse_name_mapping(text).expect("header valid");
    assert_eq!(parsed.rows[0].from, "a, b.txt");
    assert_eq!(parsed.rows[0].to, "c \"d\".txt");
}
