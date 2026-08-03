//! macOS: Spotlight via the CoreServices MDItem C API (§11.2). `{md:…}`
//! keys are raw Spotlight attribute names (`kMDItemPixelHeight`, …).

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::date::CFDate;
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};

use crate::common::{kind_from_extension, stat_created, stat_modified, stat_size, stringify_value};
use crate::{AttrValue, MetadataProvider};

// The three MDItem entry points; everything else goes through the safe
// core-foundation wrappers. SAFETY: documented CoreServices API, called
// with valid CF objects whose lifetimes the wrappers manage.
#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn MDItemCreate(allocator: *const std::ffi::c_void, path: CFStringRef) -> CFTypeRef;
    fn MDItemCopyAttribute(item: CFTypeRef, name: CFStringRef) -> CFTypeRef;
    fn MDItemCopyAttributeNames(item: CFTypeRef) -> CFTypeRef;
}

/// Seconds between the CF absolute-time epoch (2001-01-01) and Unix's.
const CF_EPOCH_OFFSET: f64 = 978_307_200.0;

pub struct SpotlightProvider;

fn md_item(path: &Path) -> Option<CFType> {
    let cf_path = CFString::new(&path.to_string_lossy());
    // SAFETY: MDItemCreate copies the path string; the returned object (if
    // any) is owned by us and released by the CFType wrapper.
    let raw = unsafe { MDItemCreate(std::ptr::null(), cf_path.as_concrete_TypeRef()) };
    if raw.is_null() {
        return None;
    }
    Some(unsafe { CFType::wrap_under_create_rule(raw) })
}

fn copy_attribute(item: &CFType, name: &str) -> Option<AttrValue> {
    let cf_name = CFString::new(name);
    // SAFETY: item is a live MDItem; the copy rule transfers ownership.
    let raw = unsafe { MDItemCopyAttribute(item.as_CFTypeRef(), cf_name.as_concrete_TypeRef()) };
    if raw.is_null() {
        return None;
    }
    let value = unsafe { CFType::wrap_under_create_rule(raw) };
    Some(convert(&value))
}

fn convert(value: &CFType) -> AttrValue {
    if let Some(string) = value.downcast::<CFString>() {
        return AttrValue::Str(string.to_string());
    }
    if let Some(boolean) = value.downcast::<CFBoolean>() {
        return AttrValue::Bool(boolean.into());
    }
    if let Some(number) = value.downcast::<CFNumber>() {
        return AttrValue::Num(number.to_f64().unwrap_or_default());
    }
    if let Some(date) = value.downcast::<CFDate>() {
        let unix_seconds = date.abs_time() + CF_EPOCH_OFFSET;
        let time = if unix_seconds >= 0.0 {
            UNIX_EPOCH + Duration::from_secs_f64(unix_seconds)
        } else {
            UNIX_EPOCH
        };
        return AttrValue::Date(time);
    }
    if let Some(array) = value.downcast::<CFArray>() {
        let items = (0..array.len())
            .filter_map(|index| array.get(index))
            .map(|pointer| {
                // SAFETY: the array member stays alive for the get-rule wrap.
                let item = unsafe { CFType::wrap_under_get_rule(*pointer) };
                convert(&item)
            })
            .collect();
        return AttrValue::List(items);
    }
    AttrValue::Str(format!("{value:?}"))
}

impl MetadataProvider for SpotlightProvider {
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
        // kMDItemKind, falling back to a type description from the
        // extension (§11.2).
        md_item(path)
            .and_then(|item| copy_attribute(&item, "kMDItemKind"))
            .map(|value| stringify_value(&value))
            .or_else(|| kind_from_extension(path))
    }

    fn attribute(&self, path: &Path, key: &str) -> Option<AttrValue> {
        copy_attribute(&md_item(path)?, key)
    }

    fn all_attributes(&self, path: &Path) -> Vec<(String, String)> {
        let Some(item) = md_item(path) else {
            return Vec::new();
        };
        // SAFETY: item is live; the returned array follows the copy rule.
        let raw = unsafe { MDItemCopyAttributeNames(item.as_CFTypeRef()) };
        if raw.is_null() {
            return Vec::new();
        }
        let names: CFArray<CFString> = unsafe { CFArray::wrap_under_create_rule(raw as *const _) };
        let mut attributes: Vec<(String, String)> = names
            .iter()
            .filter_map(|name| {
                let key = name.to_string();
                let value = copy_attribute(&item, &key)?;
                Some((key, stringify_value(&value)))
            })
            .collect();
        attributes.sort_by(|a, b| a.0.cmp(&b.0));
        attributes
    }
}
