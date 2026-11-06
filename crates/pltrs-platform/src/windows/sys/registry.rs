//! The string values of a registry key (`advapi32`), read in this
//! process: what pltrs' font catalogue asks the registry for on Windows,
//! where running `reg query` for each key costs a new process (some 170
//! ms with the system's virus scanner).

use std::ptr::null_mut;

use super::ffi::{
    self, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, Handle,
    KEY_READ, REG_EXPAND_SZ, REG_SZ,
};
use crate::windows::convert::{from_wide, string_of_bytes, to_wide};

/// An open registry key, closed when dropped.
struct Key(Handle);

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the handle is a key `RegOpenKeyExW` opened, closed once.
        unsafe { ffi::RegCloseKey(self.0) };
    }
}

impl Key {
    /// Opens `path` below the root key `root` for reading.
    fn open(root: Handle, path: &str) -> Option<Self> {
        let path = to_wide(path);
        let mut key: Handle = 0;
        // SAFETY: `path` is NUL-terminated and outlives the call; `key` is
        // a live handle the call writes.
        let status = unsafe { ffi::RegOpenKeyExW(root, path.as_ptr(), 0, KEY_READ, &raw mut key) };
        (status == ERROR_SUCCESS).then_some(Self(key))
    }

    /// The number of values, and the longest value name (in UTF-16
    /// units, without its NUL) and data (in bytes).
    fn sizes(&self) -> Option<(u32, usize, usize)> {
        let (mut count, mut name, mut data) = (0_u32, 0_u32, 0_u32);
        // SAFETY: the three out pointers are live `u32`s; the others are
        // optional and NULL.
        let status = unsafe {
            ffi::RegQueryInfoKeyW(
                self.0,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                &raw mut count,
                &raw mut name,
                &raw mut data,
                null_mut(),
                null_mut(),
            )
        };
        (status == ERROR_SUCCESS).then_some((
            count,
            usize::try_from(name).ok()?,
            usize::try_from(data).ok()?,
        ))
    }

    /// The key's string values, as [`string_values`] gives them.
    fn string_values(&self) -> Option<Vec<(String, String)>> {
        let (count, longest_name, longest_data) = self.sizes()?;
        // Room for the name's NUL, and for a string stored without one.
        let mut name = vec![0_u16; longest_name + 1];
        let mut data = vec![0_u8; longest_data + 2];
        let mut values = Vec::new();
        for index in 0..count {
            let mut name_len = u32::try_from(name.len()).ok()?;
            let mut data_len = u32::try_from(data.len()).ok()?;
            let mut kind = 0_u32;
            // SAFETY: `name` holds `name_len` units and `data` `data_len`
            // bytes, the lengths the call may write; `kind` is a live
            // `u32` and the reserved pointer NULL.
            let status = unsafe {
                ffi::RegEnumValueW(
                    self.0,
                    index,
                    name.as_mut_ptr(),
                    &raw mut name_len,
                    null_mut(),
                    &raw mut kind,
                    data.as_mut_ptr(),
                    &raw mut data_len,
                )
            };
            if status == ERROR_NO_MORE_ITEMS {
                break;
            }
            // A value written meanwhile, longer than the key's longest
            // was, is skipped as one that cannot be read.
            if status != ERROR_SUCCESS || !matches!(kind, REG_SZ | REG_EXPAND_SZ) {
                continue;
            }
            let name = name.get(..usize::try_from(name_len).ok()?)?;
            let data = data.get(..usize::try_from(data_len).ok()?)?;
            values.push((from_wide(name), string_of_bytes(data)));
        }
        Some(values)
    }
}

/// The string values (`REG_SZ`, and `REG_EXPAND_SZ` unexpanded, as
/// Python's `winreg` gives them) of the registry key `key`, named as `reg
/// query` names it (`HKLM\SOFTWARE\...` or `HKCU\Software\...`): `(name,
/// data)` pairs in the registry's order, `None` when the key cannot be
/// read.
pub fn string_values(key: &str) -> Option<Vec<(String, String)>> {
    let (root, path) = if let Some(path) = key.strip_prefix(r"HKLM\") {
        (HKEY_LOCAL_MACHINE, path)
    } else {
        (HKEY_CURRENT_USER, key.strip_prefix(r"HKCU\")?)
    };
    Key::open(root, path)?.string_values()
}
