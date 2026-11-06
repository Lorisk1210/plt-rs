//! Conversions between Win32's units and pltrs's: DPI and scale factors,
//! wait deadlines and timeouts, UTF-16 strings and pixel formats.

use std::time::Instant;

use crate::backend::Wait;
use crate::icon::{Icon, IconImage, IconLayout};
use crate::window::CursorIcon;

/// The DPI of scale factor 1 (`USER_DEFAULT_SCREEN_DPI`).
const BASE_DPI: u32 = 96;

/// `INFINITE`: a timeout that never expires.
pub(crate) const INFINITE: u32 = u32::MAX;

/// The scale factor (physical / logical pixels) of a DPI.
pub(crate) fn scale_factor(dpi: u32) -> f64 {
    f64::from(dpi.max(1)) / f64::from(BASE_DPI)
}

/// How many milliseconds to wait for messages at `now`, rounded up so
/// that the wait never ends before the deadline; `None` when not to wait
/// at all, [`INFINITE`] without a deadline.
pub(crate) fn timeout_ms(wait: Wait, now: Instant) -> Option<u32> {
    match wait {
        Wait::Poll => None,
        Wait::Forever => Some(INFINITE),
        Wait::Until(deadline) => {
            let left = deadline.checked_duration_since(now)?;
            if left.is_zero() {
                return None;
            }
            let millis = left.as_nanos().div_ceil(1_000_000);
            Some(u32::try_from(millis).map_or(INFINITE - 1, |ms| ms.min(INFINITE - 1)))
        }
    }
}

/// `text` as a NUL-terminated UTF-16 string for Win32 (stopping at an
/// interior NUL, as Win32 would).
pub(crate) fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16()
        .take_while(|&unit| unit != 0)
        .chain([0])
        .collect()
}

/// The UTF-16 string `units` up to its first NUL, if any.
pub(crate) fn from_wide(units: &[u16]) -> String {
    let end = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

/// The UTF-16 string of the little-endian `bytes` (a registry string's
/// data) up to its first NUL, if any; an odd last byte is left out.
pub(crate) fn string_of_bytes(bytes: &[u8]) -> String {
    let (pairs, _odd) = bytes.as_chunks::<2>();
    let units: Vec<u16> = pairs.iter().map(|&pair| u16::from_le_bytes(pair)).collect();
    from_wide(&units)
}

/// The text a key typed, from the UTF-16 units of its `WM_CHAR`
/// messages: `None` when empty or a control character (Backspace, Enter,
/// Control+letters, ...).
pub(crate) fn typed_text(units: &[u16]) -> Option<String> {
    let text = String::from_utf16_lossy(units);
    (!text.is_empty() && !text.chars().any(char::is_control)).then_some(text)
}

/// The single printable character `ToUnicodeEx` wrote, given how many
/// UTF-16 units it reports (negative for dead keys).
pub(crate) fn single_character(units: &[u16], count: i32) -> Option<char> {
    let written = units.get(..usize::try_from(count).ok()?)?;
    let mut chars = char::decode_utf16(written.iter().copied());
    match (chars.next(), chars.next()) {
        (Some(Ok(c)), None) if !c.is_control() => Some(c),
        _ => None,
    }
}

/// The system cursor (`IDC_*` resource) showing `icon`.
pub(crate) const fn cursor_resource(icon: CursorIcon) -> u16 {
    match icon {
        CursorIcon::Default => 32512,          // IDC_ARROW
        CursorIcon::Text => 32513,             // IDC_IBEAM
        CursorIcon::Wait => 32514,             // IDC_WAIT
        CursorIcon::Crosshair => 32515,        // IDC_CROSS
        CursorIcon::ResizeHorizontal => 32644, // IDC_SIZEWE
        CursorIcon::ResizeVertical => 32645,   // IDC_SIZENS
        CursorIcon::Move => 32646,             // IDC_SIZEALL
        CursorIcon::Pointer => 32649,          // IDC_HAND
    }
}

/// The images of `icon` for a window's title bar and task bar, `small`
/// and `big` pixels wide, in Windows' full-bleed layout (a bitmap serves
/// both); `None` when the icon draws neither.
pub(crate) fn window_icon_images(
    icon: &Icon,
    (small, big): (u32, u32),
) -> Option<(IconImage, IconImage)> {
    let mut images = icon.images(&[small, big], IconLayout::FullBleed);
    let large = images.pop()?;
    let small = images.pop().unwrap_or_else(|| large.clone());
    Some((small, large))
}

/// Straight-alpha RGBA pixels (an [`IconImage`]) in the BGRA order of
/// icon bitmaps.
pub(crate) fn icon_pixels(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, a]| [b, g, r, a])
        .collect()
}

/// The bytes of a cleared monochrome icon mask of `width` × `height`:
/// rows of whole 16-bit words.
pub(crate) fn mask_len(width: u32, height: u32) -> usize {
    width.div_ceil(16) as usize * 2 * height as usize
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn registry_strings_read_up_to_their_nul() {
        let bytes: Vec<u8> = "arial.ttf\0junk"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(string_of_bytes(&bytes), "arial.ttf");
        assert_eq!(
            string_of_bytes(&[b'a', 0, b'b']),
            "a",
            "an odd byte is left out"
        );
        assert_eq!(string_of_bytes(&[]), "");
        assert_eq!(string_of_bytes(&[0x3d, 0xd8, 0x00, 0xde]), "\u{1f600}");
    }

    #[test]
    fn dpi_maps_to_scale_factors() {
        assert!((scale_factor(96) - 1.0).abs() < f64::EPSILON);
        assert!((scale_factor(144) - 1.5).abs() < f64::EPSILON);
        assert!((scale_factor(192) - 2.0).abs() < f64::EPSILON);
        assert!(scale_factor(0) > 0.0, "a bogus DPI keeps a usable scale");
    }

    #[test]
    fn timeouts_round_up_and_saturate() {
        let now = Instant::now();
        assert_eq!(timeout_ms(Wait::Poll, now), None);
        assert_eq!(timeout_ms(Wait::Forever, now), Some(INFINITE));
        assert_eq!(timeout_ms(Wait::Until(now), now), None);
        let soon = now + Duration::from_micros(1500);
        assert_eq!(timeout_ms(Wait::Until(soon), now), Some(2));
        assert_eq!(timeout_ms(Wait::Until(now), soon), None, "passed");
        let far = now + Duration::from_hours(2400);
        assert_eq!(timeout_ms(Wait::Until(far), now), Some(INFINITE - 1));
    }

    #[test]
    fn wide_strings() {
        assert_eq!(to_wide("Fig é"), [70, 105, 103, 32, 0xE9, 0]);
        assert_eq!(to_wide("a\0b"), [97, 0]);
        assert_eq!(to_wide(""), [0]);
        assert_eq!(from_wide(&[0xD83D, 0xDE00, 0, 65]), "😀");
        assert_eq!(from_wide(&[0xD83D]), "\u{FFFD}", "a lone surrogate");
        assert_eq!(from_wide(&[104, 105]), "hi");
    }

    #[test]
    fn typed_text_drops_control_characters() {
        assert_eq!(typed_text(&[0xD83D, 0xDE00]).as_deref(), Some("😀"));
        assert_eq!(typed_text(&[0x20AC]).as_deref(), Some("€"));
        assert_eq!(typed_text(&[]), None);
        assert_eq!(typed_text(&[0x08]), None, "Backspace");
        assert_eq!(typed_text(&[0x13]), None, "Control+S");
    }

    #[test]
    fn single_characters_of_to_unicode() {
        let units = [u16::from(b'A'), 0, 0, 0];
        assert_eq!(single_character(&units, 1), Some('A'));
        assert_eq!(single_character(&[0xD83D, 0xDE00], 2), Some('😀'));
        assert_eq!(single_character(&units, 0), None, "no character");
        assert_eq!(single_character(&units, -1), None, "a dead key");
        assert_eq!(single_character(&[97, 98], 2), None, "two characters");
        assert_eq!(single_character(&[0x1B], 1), None, "Escape");
        assert_eq!(single_character(&units, 9), None, "more than written");
    }

    #[test]
    fn cursors_are_system_cursors() {
        assert_eq!(cursor_resource(CursorIcon::Default), 32512);
        assert_eq!(cursor_resource(CursorIcon::Pointer), 32649);
        assert_eq!(cursor_resource(CursorIcon::Move), 32646);
        assert_eq!(cursor_resource(CursorIcon::ResizeVertical), 32645);
    }

    #[test]
    fn windows_get_a_small_and_a_big_icon() {
        let rendered = Icon::from_renderer(|side, layout| {
            assert_eq!(layout, IconLayout::FullBleed);
            IconImage::from_rgba(side, side, vec![0; (side * side * 4) as usize]).ok()
        });
        let (small, big) = window_icon_images(&rendered, (20, 40)).unwrap();
        assert_eq!((small.size().width, big.size().width), (20, 40));
        let bitmap = Icon::from_rgba(3, 3, vec![9; 36]).unwrap();
        let (small, big) = window_icon_images(&bitmap, (16, 32)).unwrap();
        assert_eq!(small, big);
        assert_eq!(small.size().width, 3);
        let none = Icon::from_renderer(|_, _| None);
        assert_eq!(window_icon_images(&none, (16, 32)), None);
    }

    #[test]
    fn icons_are_bgra_with_word_aligned_masks() {
        assert_eq!(
            icon_pixels(&[1, 2, 3, 4, 5, 6, 7, 8]),
            [3, 2, 1, 4, 7, 6, 5, 8]
        );
        assert_eq!(mask_len(16, 16), 32);
        assert_eq!(mask_len(17, 2), 8);
        assert_eq!(mask_len(1, 1), 2);
    }
}
