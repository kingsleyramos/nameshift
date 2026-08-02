//! Name-mapping CSV build and parse (§14.7). The format is the legacy
//! template: two columns, `Current Name,New Name`, comma- or tab-separated.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::rule::split_name;
use crate::state::CoreState;

/// The exported header labels; import requires this exact first row
/// (case-insensitive) so the file that comes back is unambiguously one of
/// our templates.
pub const CSV_HEADER: (&str, &str) = ("Current Name", "New Name");

/// One parsed data row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsvRow {
    /// The current-name column, trimmed.
    pub from: String,
    /// The new-name column, trimmed.
    pub to: String,
}

/// Parse output: data rows plus the raw lines that couldn't be read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CsvParsed {
    /// Header-stripped data rows, in file order.
    pub rows: Vec<CsvRow>,
    /// Non-empty lines that didn't yield two non-empty fields.
    pub unreadable: Vec<String>,
}

/// The file's first row wasn't the exported header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BadHeader;

/// Parse the exported template (comma- or tab-separated). The first
/// readable row MUST be the header; otherwise the whole file is rejected so
/// import never silently treats an unrelated file as data rows.
pub fn parse_name_mapping(text: &str) -> Result<CsvParsed, BadHeader> {
    let mut rows: Vec<CsvRow> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();
    let mut saw_header = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        // Tab fallback: a pasted spreadsheet row is tab-separated.
        let fields: Vec<String> = if line.contains('\t') {
            line.split('\t').map(str::to_string).collect()
        } else {
            parse_csv_line(line)
        };
        let from = fields
            .first()
            .map(|f| f.trim().to_string())
            .unwrap_or_default();
        let to = fields
            .get(1)
            .map(|f| f.trim().to_string())
            .unwrap_or_default();
        if fields.len() < 2 || from.is_empty() || to.is_empty() {
            if saw_header {
                unreadable.push(line.to_string());
            }
            continue;
        }
        if !saw_header {
            // The first readable row must be the header.
            if from.eq_ignore_ascii_case(CSV_HEADER.0) && to.eq_ignore_ascii_case(CSV_HEADER.1) {
                saw_header = true;
                continue;
            }
            return Err(BadHeader);
        }
        rows.push(CsvRow { from, to });
    }
    if !saw_header {
        return Err(BadHeader);
    }
    Ok(CsvParsed { rows, unreadable })
}

/// The legacy quoted-CSV state machine: quotes open only at a field start,
/// `""` escapes a quote, and a closing quote followed by a comma ends the
/// field.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                match chars.next() {
                    Some('"') => current.push('"'),
                    Some(',') => {
                        in_quotes = false;
                        fields.push(std::mem::take(&mut current));
                    }
                    Some(other) => {
                        in_quotes = false;
                        current.push(other);
                    }
                    None => in_quotes = false,
                }
            } else {
                current.push(c);
            }
        } else if c == '"' && current.is_empty() {
            in_quotes = true;
        } else if c == ',' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    fields.push(current);
    fields
}

/// Quote a CSV field when it needs it (commas, quotes, edge whitespace).
pub fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.starts_with(' ') || value.ends_with(' ')
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Two-column template: current name, new name — prefilled with the current
/// name so only edited rows change anything on re-import.
pub fn build_name_mapping_csv(names: &[String]) -> String {
    let header = format!("{},{}", csv_field(CSV_HEADER.0), csv_field(CSV_HEADER.1));
    std::iter::once(header)
        .chain(
            names
                .iter()
                .map(|name| format!("{},{}", csv_field(name), csv_field(name))),
        )
        .collect::<Vec<_>>()
        .join("\n")
}

/// One matched item: applying sets this item's override (§12.1 `csv_apply`).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CsvMatch {
    /// The matched item.
    #[serde(with = "crate::serde_util::uuid_upper")]
    #[ts(as = "String")]
    pub id: Uuid,
    /// The name the row assigns (extension inherited when omitted).
    pub new_name: String,
    /// Whether that name differs from the item's current name.
    pub changes: bool,
}

/// Why a row didn't match (§14.7 disclosure groups).
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CsvMissReason {
    /// No active item has this current name.
    NotInList,
    /// A duplicate row — the FIRST occurrence still applies; later ones are
    /// flagged (§A), a deliberate departure from the legacy
    /// last-row-wins overwrite (docs/DEVIATIONS.md).
    DuplicateRow,
    /// The line didn't parse into two non-empty fields.
    CouldntRead,
}

/// One row that won't apply, with its reason.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CsvMiss {
    /// The row's current-name column (or the raw line for unreadable rows).
    pub name: String,
    /// Why it won't apply.
    pub reason: CsvMissReason,
}

/// The dry-run report powering the Rename by CSV modal — computing it
/// mutates NOTHING (§24 Q30).
#[derive(Serialize, Deserialize, TS, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CsvMatchReport {
    /// Every matched item with its assigned name.
    pub matches: Vec<CsvMatch>,
    /// Rows that won't apply, grouped by the UI per reason.
    pub misses: Vec<CsvMiss>,
    /// Matched items whose name will change (the `{A}` in the modal).
    pub will_change: u32,
    /// Matched items whose assigned name equals their current name.
    pub unchanged: u32,
}

/// Parse `text` and match rows against the active items — case-insensitive
/// on current name, extension inherited when the new name has none and the
/// file has one. Pure: no state is touched.
pub fn csv_dry_run(state: &CoreState, text: &str) -> Result<CsvMatchReport, BadHeader> {
    let parsed = parse_name_mapping(text)?;
    let mut report = CsvMatchReport::default();
    for line in parsed.unreadable {
        report.misses.push(CsvMiss {
            name: line,
            reason: CsvMissReason::CouldntRead,
        });
    }
    let mut consumed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for row in parsed.rows {
        let key = row.from.to_lowercase();
        if !consumed.insert(key.clone()) {
            report.misses.push(CsvMiss {
                name: row.from,
                reason: CsvMissReason::DuplicateRow,
            });
            continue;
        }
        let mut matched_any = false;
        for item in state.active_items() {
            let current = item.name();
            if current.to_lowercase() != key {
                continue;
            }
            matched_any = true;
            let mut new_name = row.to.clone();
            // Extension inheritance: a new name typed without an extension
            // keeps the file's (files only).
            if !item.is_directory {
                let (_, current_ext) = split_name(&current);
                let (_, new_ext) = split_name(&new_name);
                if new_ext.is_empty() && !current_ext.is_empty() {
                    new_name = format!("{new_name}.{current_ext}");
                }
            }
            let changes = new_name != current;
            if changes {
                report.will_change += 1;
            } else {
                report.unchanged += 1;
            }
            report.matches.push(CsvMatch {
                id: item.id,
                new_name,
                changes,
            });
        }
        if !matched_any {
            report.misses.push(CsvMiss {
                name: row.from,
                reason: CsvMissReason::NotInList,
            });
        }
    }
    Ok(report)
}
