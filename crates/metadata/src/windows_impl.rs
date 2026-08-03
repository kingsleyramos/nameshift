//! Windows: the Property System via `SHGetPropertyStoreFromParsingName`
//! (§11.2). `{md:…}` keys are canonical property names
//! (`System.Image.HorizontalSize`, `System.Photo.CameraModel`, …).

use std::path::Path;
use std::time::SystemTime;

use windows::core::{HSTRING, PWSTR};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::StructuredStorage::{PropVariantToStringAlloc, PROPVARIANT};
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, IBindCtx, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::PropertiesSystem::{
    IPropertyStore, PSGetNameFromPropertyKey, PSGetPropertyKeyFromName,
    SHGetPropertyStoreFromParsingName, GPS_DEFAULT,
};

use crate::common::{kind_from_extension, stat_created, stat_modified, stat_size};
use crate::{AttrValue, MetadataProvider};

pub struct PropertySystemProvider;

thread_local! {
    // COM must be initialized once per thread before the shell APIs work.
    // RPC_E_CHANGED_MODE just means something else initialized differently
    // first — the APIs still work, so the value is only a marker.
    static COM_READY: () = {
        // SAFETY: documented single-call thread setup.
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    };
}

/// Copy a COM-allocated wide string and free it. Manual length scan keeps
/// this independent of helper-method churn across windows-rs versions.
///
/// SAFETY: `raw` must be a valid, NUL-terminated string allocated by the
/// COM task allocator (the contract of the `…Alloc` APIs used below).
unsafe fn take_com_string(raw: PWSTR) -> Option<String> {
    if raw.is_null() {
        return None;
    }
    let mut length = 0usize;
    while *raw.0.add(length) != 0 {
        length += 1;
    }
    let text = String::from_utf16_lossy(std::slice::from_raw_parts(raw.0, length));
    CoTaskMemFree(Some(raw.0 as *const _));
    Some(text)
}

fn store_for(path: &Path) -> Option<IPropertyStore> {
    COM_READY.with(|()| {});
    let wide = HSTRING::from(nameshift_engine::platform::win_long_path(path).as_os_str());
    // SAFETY: documented shell API; the returned store is reference-counted.
    unsafe { SHGetPropertyStoreFromParsingName(&wide, None::<&IBindCtx>, GPS_DEFAULT).ok() }
}

fn stringify_propvariant(value: &PROPVARIANT) -> Option<String> {
    // SAFETY: value is a live PROPVARIANT; the returned allocation is freed
    // by take_com_string.
    unsafe {
        let raw = PropVariantToStringAlloc(value).ok()?;
        take_com_string(raw).filter(|text| !text.is_empty())
    }
}

fn key_from_name(name: &str) -> Option<PROPERTYKEY> {
    let wide = HSTRING::from(name);
    let mut key = PROPERTYKEY::default();
    // SAFETY: documented propsys API filling the out-param on success.
    unsafe { PSGetPropertyKeyFromName(&wide, &mut key).ok()? };
    Some(key)
}

fn name_from_key(key: &PROPERTYKEY) -> Option<String> {
    // SAFETY: allocates a wide string on success; freed by take_com_string.
    unsafe {
        let raw = PSGetNameFromPropertyKey(key).ok()?;
        take_com_string(raw)
    }
}

impl MetadataProvider for PropertySystemProvider {
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
        self.attribute(path, "System.ItemTypeText")
            .map(|value| crate::stringify_value(&value))
            .or_else(|| kind_from_extension(path))
    }

    fn attribute(&self, path: &Path, key: &str) -> Option<AttrValue> {
        let store = store_for(path)?;
        let property_key = key_from_name(key)?;
        // SAFETY: documented property-store read; PROPVARIANT frees on drop.
        let value: PROPVARIANT = unsafe { store.GetValue(&property_key).ok()? };
        stringify_propvariant(&value).map(AttrValue::Str)
    }

    fn all_attributes(&self, path: &Path) -> Vec<(String, String)> {
        let Some(store) = store_for(path) else {
            return Vec::new();
        };
        let mut attributes: Vec<(String, String)> = Vec::new();
        // SAFETY: documented enumeration; indices are bounded by GetCount.
        unsafe {
            let count = store.GetCount().unwrap_or(0);
            for index in 0..count {
                let mut key = PROPERTYKEY::default();
                if store.GetAt(index, &mut key).is_err() {
                    continue;
                }
                // Skip unnamed keys (§11.2).
                let Some(name) = name_from_key(&key) else {
                    continue;
                };
                let Ok(value) = store.GetValue(&key) else {
                    continue;
                };
                let Some(text) = stringify_propvariant(&value) else {
                    continue;
                };
                attributes.push((name, text));
            }
        }
        attributes.sort_by(|a, b| a.0.cmp(&b.0));
        attributes
    }
}
