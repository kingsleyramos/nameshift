//! Linux: composed sources with namespaced keys (§11.2) — `exif:<Tag>` via
//! kamadak-exif for images, `xattr:<name>` for user extended attributes,
//! plus the stat basics. `created` comes from btime where the filesystem
//! provides it (statx), else `None` (§24 Q9).

use std::io::BufReader;
use std::path::Path;
use std::time::SystemTime;

use crate::common::{kind_from_extension, stat_created, stat_modified, stat_size};
use crate::{AttrValue, MetadataProvider};

pub struct LinuxProvider;

fn exif_fields(path: &Path) -> Vec<(String, String)> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut reader = BufReader::new(file);
    let Ok(parsed) = exif::Reader::new().read_from_container(&mut reader) else {
        return Vec::new();
    };
    parsed
        .fields()
        .filter(|field| field.ifd_num == exif::In::PRIMARY)
        .map(|field| {
            (
                format!("exif:{}", field.tag),
                field.display_value().with_unit(field).to_string(),
            )
        })
        .collect()
}

fn xattr_entries(path: &Path) -> Vec<(String, String)> {
    let Ok(names) = xattr::list(path) else {
        return Vec::new();
    };
    names
        .filter_map(|name| {
            let key = name.to_string_lossy().into_owned();
            let value = xattr::get(path, &name).ok().flatten()?;
            Some((
                format!("xattr:{key}"),
                String::from_utf8_lossy(&value).into_owned(),
            ))
        })
        .collect()
}

impl MetadataProvider for LinuxProvider {
    fn created(&self, path: &Path) -> Option<SystemTime> {
        stat_created(path)
    }

    fn modified(&self, path: &Path) -> Option<SystemTime> {
        stat_modified(path)
    }

    fn size(&self, path: &Path) -> Option<u64> {
        stat_size(path)
    }

    fn kind(&self, path: &Path) -> Option<String> {
        kind_from_extension(path)
    }

    fn attribute(&self, path: &Path, key: &str) -> Option<AttrValue> {
        if let Some(tag) = key.strip_prefix("exif:") {
            return exif_fields(path)
                .into_iter()
                .find(|(name, _)| name == &format!("exif:{tag}"))
                .map(|(_, value)| AttrValue::Str(value));
        }
        if let Some(name) = key.strip_prefix("xattr:") {
            let value = xattr::get(path, name).ok().flatten()?;
            return Some(AttrValue::Str(String::from_utf8_lossy(&value).into_owned()));
        }
        None
    }

    fn all_attributes(&self, path: &Path) -> Vec<(String, String)> {
        let mut attributes = exif_fields(path);
        attributes.extend(xattr_entries(path));
        attributes.sort_by(|a, b| a.0.cmp(&b.0));
        attributes
    }
}
