//! Token expansion (§6): `{key}` / `{key:argument}` placeholders expanded
//! per file, plus the LDML date formatter and size formatter they use.

use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Datelike, Local, Timelike};

/// What tokens may ask about a file. Implementations must be lazy: no IO
/// happens until a value is actually requested (§6.6), and `attribute`
/// returns values already stringified per the shared rules (§11.1).
pub trait MetadataSource: Sync {
    /// File creation time, if the filesystem records one.
    fn created(&self, path: &Path) -> Option<SystemTime>;
    /// File modification time.
    fn modified(&self, path: &Path) -> Option<SystemTime>;
    /// File size in bytes.
    fn size(&self, path: &Path) -> Option<u64>;
    /// Human-readable kind (“JPEG image”, “Folder”).
    fn kind(&self, path: &Path) -> Option<String>;
    /// Provider-namespaced metadata attribute for `{md:…}`, stringified.
    fn attribute(&self, path: &Path, key: &str) -> Option<String>;
}

/// A metadata source that knows nothing — for contexts without metadata.
pub struct NoMetadata;

impl MetadataSource for NoMetadata {
    fn created(&self, _: &Path) -> Option<SystemTime> {
        None
    }
    fn modified(&self, _: &Path) -> Option<SystemTime> {
        None
    }
    fn size(&self, _: &Path) -> Option<u64> {
        None
    }
    fn kind(&self, _: &Path) -> Option<String> {
        None
    }
    fn attribute(&self, _: &Path, _: &str) -> Option<String> {
        None
    }
}

/// Per-file context handed to rules: the file's 1-based position among
/// selected items in the current view order (§6.3), its per-directory
/// counterpart (§7.3), its path, and lazy metadata.
pub struct TokenContext<'a> {
    /// 1-based position among selected items in view order.
    pub index: u32,
    /// 1-based position among selected items of the same directory.
    pub folder_index: u32,
    /// The item's absolute path.
    pub path: &'a Path,
    /// Lazy metadata access — constructing this performs no IO (§6.6).
    pub metadata: &'a dyn MetadataSource,
}

/// Expand `{token}` occurrences (§6.1). Single pass — expanded values are
/// never rescanned, so a file literally named `{n}.txt` cannot inject.
/// Unknown tokens are left in place verbatim so typos stay visible in the
/// preview; never “helpfully” strip them.
pub fn expand_tokens(
    template: &str,
    base_name: &str,
    file_extension: &str,
    ctx: Option<&TokenContext>,
) -> String {
    if !template.contains('{') {
        return template.to_string();
    }
    let mut output = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        output.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after.find('}');
        let next_open = after.find('{');
        match close {
            // A token needs 1+ chars inside and no nested brace before `}`.
            Some(end) if end > 0 && next_open.is_none_or(|n| end < n) => {
                let inner = &after[..end];
                match token_value(inner, base_name, file_extension, ctx) {
                    Some(value) => output.push_str(&value),
                    None => {
                        output.push('{');
                        output.push_str(inner);
                        output.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                output.push('{');
                rest = after;
            }
        }
    }
    output.push_str(rest);
    output
}

/// Resolve one token's value; `None` = unknown, left in place (§6.2).
fn token_value(
    inner: &str,
    base_name: &str,
    file_extension: &str,
    ctx: Option<&TokenContext>,
) -> Option<String> {
    // Split on the FIRST colon; the argument may itself contain colons.
    // `{:}` has key "" → unknown (§6.1).
    let (key, argument) = match inner.find(':') {
        Some(i) => (&inner[..i], Some(&inner[i + 1..])),
        None => (inner, None),
    };
    match key {
        "name" => Some(base_name.to_string()),
        // Empty extension expands to "", not left in place (§6.2).
        "ext" => Some(file_extension.to_string()),
        "folder" => {
            let parent = ctx?.path.parent()?;
            let name = parent.file_name()?;
            Some(name.to_string_lossy().into_owned())
        }
        "n" => {
            // Clamp width so `{n:99999999}` can't allocate huge strings.
            let width = argument
                .and_then(|w| w.parse::<i64>().ok())
                .unwrap_or(1)
                .clamp(1, 10) as usize;
            let index = ctx.map_or(1, |c| c.index);
            Some(format!("{index:0width$}"))
        }
        "created" => {
            let time = ctx?.metadata.created(ctx?.path)?;
            Some(format_date(
                &DateTime::<Local>::from(time),
                argument.unwrap_or(DEFAULT_DATE_PATTERN),
            ))
        }
        "modified" => {
            let time = ctx?.metadata.modified(ctx?.path)?;
            Some(format_date(
                &DateTime::<Local>::from(time),
                argument.unwrap_or(DEFAULT_DATE_PATTERN),
            ))
        }
        "date" => Some(format_date(
            &Local::now(),
            argument.unwrap_or(DEFAULT_DATE_PATTERN),
        )),
        "size" => {
            let bytes = ctx?.metadata.size(ctx?.path)?;
            Some(format_size(bytes))
        }
        "kind" => ctx?.metadata.kind(ctx?.path),
        "md" => ctx?.metadata.attribute(ctx?.path, argument?),
        _ => None,
    }
}

/// The default pattern for `{created}` / `{modified}` / `{date}` (§6.4).
pub const DEFAULT_DATE_PATTERN: &str = "yyyy-MM-dd";

const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_LONG: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS_SHORT: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const WEEKDAYS_LONG: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Format a date with the supported LDML-pattern subset (§6.4):
/// `yyyy yy MM M dd d HH H hh h mm m ss s a EEEE EEE MMMM MMM`, `'…'`
/// literals with `''` = one quote, longest-match-first; anything else passes
/// through unchanged. Rendering is locale-independent: English names,
/// Gregorian calendar, host-local timezone — filenames must be stable
/// (a non-Gregorian system calendar once baked year 2569 into filenames).
pub fn format_date(date: &DateTime<Local>, pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut output = String::with_capacity(pattern.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            if chars.get(i + 1) == Some(&'\'') {
                output.push('\'');
                i += 2;
                continue;
            }
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    if chars.get(i + 1) == Some(&'\'') {
                        output.push('\'');
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    output.push(chars[i]);
                    i += 1;
                }
            }
            continue;
        }
        let mut run = 1;
        while i + run < chars.len() && chars[i + run] == c {
            run += 1;
        }
        let consumed = emit_date_token(&mut output, date, c, run);
        if consumed == 0 {
            output.push(c);
            i += 1;
        } else {
            i += consumed;
        }
    }
    output
}

/// Emit the longest listed token for letter `c` that fits in `run`
/// repetitions; returns how many chars were consumed (0 = not a token).
fn emit_date_token(output: &mut String, date: &DateTime<Local>, c: char, run: usize) -> usize {
    let month = date.month() as usize;
    let weekday = date.weekday().num_days_from_monday() as usize;
    let hour12 = {
        let h = date.hour() % 12;
        if h == 0 {
            12
        } else {
            h
        }
    };
    let (text, consumed): (String, usize) = match c {
        'y' if run >= 4 => (format!("{:04}", date.year()), 4),
        'y' if run >= 2 => (format!("{:02}", date.year().rem_euclid(100)), 2),
        'M' if run >= 4 => (MONTHS_LONG[month - 1].to_string(), 4),
        'M' if run >= 3 => (MONTHS_SHORT[month - 1].to_string(), 3),
        'M' if run >= 2 => (format!("{month:02}"), 2),
        'M' => (month.to_string(), 1),
        'd' if run >= 2 => (format!("{:02}", date.day()), 2),
        'd' => (date.day().to_string(), 1),
        'H' if run >= 2 => (format!("{:02}", date.hour()), 2),
        'H' => (date.hour().to_string(), 1),
        'h' if run >= 2 => (format!("{hour12:02}"), 2),
        'h' => (hour12.to_string(), 1),
        'm' if run >= 2 => (format!("{:02}", date.minute()), 2),
        'm' => (date.minute().to_string(), 1),
        's' if run >= 2 => (format!("{:02}", date.second()), 2),
        's' => (date.second().to_string(), 1),
        'a' => (if date.hour() < 12 { "AM" } else { "PM" }.to_string(), 1),
        'E' if run >= 4 => (WEEKDAYS_LONG[weekday].to_string(), 4),
        'E' if run >= 3 => (WEEKDAYS_SHORT[weekday].to_string(), 3),
        _ => return 0,
    };
    output.push_str(&text);
    consumed
}

/// Human-readable size, 1000-based decimal units like macOS (§6.5):
/// `Zero bytes`, `N bytes`, then KB/MB/GB/TB with at most one decimal and
/// trailing `.0` dropped. Locale-independent.
pub fn format_size(bytes: u64) -> String {
    match bytes {
        0 => return "Zero bytes".to_string(),
        // English pluralization (§A) — matches the reference formatter.
        1 => return "1 byte".to_string(),
        b if b < 1000 => return format!("{b} bytes"),
        _ => {}
    }
    let mut value = bytes as f64;
    let mut unit = "KB";
    for candidate in ["KB", "MB", "GB", "TB"] {
        value /= 1000.0;
        unit = candidate;
        if value < 1000.0 {
            break;
        }
    }
    let rounded = (value * 10.0).round() / 10.0;
    if ((rounded * 10.0) as u64).is_multiple_of(10) {
        format!("{rounded:.0} {unit}")
    } else {
        format!("{rounded:.1} {unit}")
    }
}
