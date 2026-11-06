//! The keyboard layout: what keys type, dead keys included, from
//! `UCKeyTranslate` (Core Services) on the layout Text Input Sources
//! (`HIToolbox`) say is current.
//!
//! pltrs links neither framework, as it does not link `AppKit`, which
//! loads both: their functions are looked up in the process once, the
//! first time a dead key may be involved, which is after `AppKit` has
//! delivered a key event.

use std::ffi::{CStr, c_char, c_void};
use std::sync::OnceLock;

use crate::macos::keys::Translate;

/// `dlsym`'s `RTLD_DEFAULT`: every image loaded.
const RTLD_DEFAULT: *mut c_void = -2_isize as *mut c_void;
/// `kUCKeyActionDown`.
const KEY_ACTION_DOWN: u16 = 0;
/// `kUCKeyTranslateNoDeadKeysMask`.
const NO_DEAD_KEYS: u32 = 1;
/// The longest text one key types (`UCKeyTranslate` writes UTF-16).
const MAX_UNITS: usize = 16;

unsafe extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

type CopyCurrentLayout = unsafe extern "C" fn() -> *mut c_void;
type GetProperty = unsafe extern "C" fn(*mut c_void, *const c_void) -> *const c_void;
type KeyTranslate = unsafe extern "C" fn(
    *const c_void,
    u16,
    u16,
    u32,
    u32,
    u32,
    *mut u32,
    usize,
    *mut usize,
    *mut u16,
) -> i32;
type KeyboardType = unsafe extern "C" fn() -> u8;
type DataBytes = unsafe extern "C" fn(*const c_void) -> *const u8;
type Release = unsafe extern "C" fn(*const c_void);

/// The functions and the constant the keyboard layout is read with.
struct Functions {
    /// `TISCopyCurrentKeyboardLayoutInputSource`.
    copy_current_layout: CopyCurrentLayout,
    /// `TISGetInputSourceProperty`.
    get_property: GetProperty,
    /// `UCKeyTranslate`.
    translate: KeyTranslate,
    /// `LMGetKbdType`.
    keyboard_type: KeyboardType,
    /// `CFDataGetBytePtr`.
    data_bytes: DataBytes,
    /// `CFRelease`.
    release: Release,
    /// `kTISPropertyUnicodeKeyLayoutData`, an immortal `CFStringRef`.
    unicode_layout_key: usize,
}

/// The address of `symbol` in the process, if an image defines it.
fn symbol(name: &CStr) -> Option<*mut c_void> {
    // SAFETY: the name is NUL-terminated; looking a symbol up has no side
    // effects.
    let address = unsafe { dlsym(RTLD_DEFAULT, name.as_ptr()) };
    (!address.is_null()).then_some(address)
}

impl Functions {
    /// The functions, looked up once; `None` if any is missing.
    fn get() -> Option<&'static Self> {
        static FUNCTIONS: OnceLock<Option<Functions>> = OnceLock::new();
        FUNCTIONS.get_or_init(Self::look_up).as_ref()
    }

    fn look_up() -> Option<Self> {
        let key = symbol(c"kTISPropertyUnicodeKeyLayoutData")?;
        // SAFETY: each symbol is the function of that name, whose C type
        // the alias it is turned into declares (`HIToolbox`'s
        // `TextInputSources.h` and `Events.h`, Core Services'
        // `UnicodeUtilities.h`, Core Foundation's `CFData.h` and
        // `CFBase.h`); the constant is a `CFStringRef` variable, read once.
        unsafe {
            Some(Self {
                copy_current_layout: std::mem::transmute::<*mut c_void, CopyCurrentLayout>(symbol(
                    c"TISCopyCurrentKeyboardLayoutInputSource",
                )?),
                get_property: std::mem::transmute::<*mut c_void, GetProperty>(symbol(
                    c"TISGetInputSourceProperty",
                )?),
                translate: std::mem::transmute::<*mut c_void, KeyTranslate>(symbol(
                    c"UCKeyTranslate",
                )?),
                keyboard_type: std::mem::transmute::<*mut c_void, KeyboardType>(symbol(
                    c"LMGetKbdType",
                )?),
                data_bytes: std::mem::transmute::<*mut c_void, DataBytes>(symbol(
                    c"CFDataGetBytePtr",
                )?),
                release: std::mem::transmute::<*mut c_void, Release>(symbol(c"CFRelease")?),
                unicode_layout_key: key.cast::<*const c_void>().read() as usize,
            })
        }
    }
}

/// The current keyboard layout, held while keys are translated. Main
/// thread only.
pub(crate) struct KeyboardLayout {
    functions: &'static Functions,
    /// The input source, owned (released when dropped).
    source: *mut c_void,
    /// Its `UCKeyboardLayout`, which lives as long as the source.
    layout: *const c_void,
    /// The keyboard's type (`LMGetKbdType`).
    keyboard_type: u32,
}

impl KeyboardLayout {
    /// The keyboard layout in use, if its Unicode layout can be read.
    pub(crate) fn current() -> Option<Self> {
        let functions = Functions::get()?;
        // SAFETY: a call without arguments, returning an owned input
        // source or null.
        let source = unsafe { (functions.copy_current_layout)() };
        if source.is_null() {
            return None;
        }
        // SAFETY: `source` is a live input source and the key a property
        // name; the data returned (not owned) lives as long as the
        // source, and its bytes are a `UCKeyboardLayout`.
        let layout = unsafe {
            let data =
                (functions.get_property)(source, functions.unicode_layout_key as *const c_void);
            if data.is_null() {
                std::ptr::null()
            } else {
                (functions.data_bytes)(data).cast::<c_void>()
            }
        };
        // SAFETY: a call without arguments.
        let keyboard_type = u32::from(unsafe { (functions.keyboard_type)() });
        let held = Self {
            functions,
            source,
            layout,
            keyboard_type,
        };
        (!held.layout.is_null()).then_some(held)
    }
}

impl Drop for KeyboardLayout {
    fn drop(&mut self) {
        // SAFETY: the source was copied, so it is this value's to release.
        unsafe { (self.functions.release)(self.source) };
    }
}

impl Translate for KeyboardLayout {
    fn translate(
        &self,
        code: u16,
        modifiers: u32,
        dead_state: &mut u32,
        dead_keys: bool,
    ) -> Option<String> {
        let mut units = [0_u16; MAX_UNITS];
        let mut length = 0_usize;
        let options = if dead_keys { 0 } else { NO_DEAD_KEYS };
        // SAFETY: the layout lives while `self` does; `dead_state` and
        // `length` are valid for writes, and `units` holds the length
        // passed.
        let status = unsafe {
            (self.functions.translate)(
                self.layout,
                code,
                KEY_ACTION_DOWN,
                modifiers,
                self.keyboard_type,
                options,
                dead_state,
                units.len(),
                &raw mut length,
                units.as_mut_ptr(),
            )
        };
        let written = units.get(..length)?;
        (status == 0).then(|| String::from_utf16_lossy(written))
    }
}
