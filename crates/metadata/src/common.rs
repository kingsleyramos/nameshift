//! Shared pieces: stringification rules and the extension→kind table.

use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Local};
use nameshift_engine::tokens::DEFAULT_DATE_PATTERN;

use crate::AttrValue;

/// Stringify an attribute value (§11.1, shared and tested): booleans render
/// `Yes`/`No` — never 0/1, so the inspector and `{md:…}` agree; dates use
/// the §6.4 default pattern; lists join with `, `; numbers use the shortest
/// round-trip decimal.
pub fn stringify_value(value: &AttrValue) -> String {
    match value {
        AttrValue::Str(s) => s.clone(),
        AttrValue::Num(n) => format!("{n}"),
        AttrValue::Bool(b) => if *b { "Yes" } else { "No" }.to_string(),
        AttrValue::Date(time) => {
            nameshift_engine::format_date(&DateTime::<Local>::from(*time), DEFAULT_DATE_PATTERN)
        }
        AttrValue::List(items) => items
            .iter()
            .map(stringify_value)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// Stat-based basics shared by every provider.
pub fn stat_created(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.created().ok()
}

/// Modification time from stat.
pub fn stat_modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// Size in bytes from stat (files only; directories return `None`).
pub fn stat_size(path: &Path) -> Option<u64> {
    let metadata = std::fs::metadata(path).ok()?;
    metadata.is_file().then(|| metadata.len())
}

/// Humanized kind from the extension — the Linux primary source and the
/// fallback elsewhere (§11.2): ~40 common entries, `EXT file` uppercased
/// otherwise, `Folder` for directories.
pub fn kind_from_extension(path: &Path) -> Option<String> {
    if std::fs::metadata(path).ok()?.is_dir() {
        return Some("Folder".to_string());
    }
    let name = path.file_name()?.to_string_lossy().into_owned();
    let (_, extension) = nameshift_engine::split_name(&name);
    if extension.is_empty() {
        return Some("Document".to_string());
    }
    let known = match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "JPEG image",
        "png" => "PNG image",
        "gif" => "GIF image",
        "webp" => "WebP image",
        "heic" => "HEIC image",
        "tif" | "tiff" => "TIFF image",
        "bmp" => "Bitmap image",
        "svg" => "SVG image",
        "raw" | "dng" | "cr2" | "cr3" | "nef" | "arw" => "Raw image",
        "pdf" => "PDF document",
        "txt" => "Plain text document",
        "md" => "Markdown document",
        "rtf" => "Rich text document",
        "doc" | "docx" => "Word document",
        "xls" | "xlsx" => "Excel spreadsheet",
        "ppt" | "pptx" => "PowerPoint presentation",
        "csv" => "CSV document",
        "json" => "JSON document",
        "xml" => "XML document",
        "html" | "htm" => "HTML document",
        "mp3" => "MP3 audio",
        "wav" => "WAV audio",
        "flac" => "FLAC audio",
        "aac" | "m4a" => "AAC audio",
        "ogg" => "Ogg audio",
        "mp4" | "m4v" => "MPEG-4 movie",
        "mov" => "QuickTime movie",
        "mkv" => "Matroska movie",
        "avi" => "AVI movie",
        "webm" => "WebM movie",
        "zip" => "ZIP archive",
        "tar" => "tar archive",
        "gz" => "gzip archive",
        "7z" => "7-Zip archive",
        "rar" => "RAR archive",
        "dmg" => "Disk image",
        "iso" => "Disk image",
        "app" => "Application",
        "exe" => "Application",
        _ => "",
    };
    if known.is_empty() {
        Some(format!("{} file", extension.to_ascii_uppercase()))
    } else {
        Some(known.to_string())
    }
}
