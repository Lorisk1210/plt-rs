//! Mouse cursors from the server's built-in cursor font.

use std::collections::HashMap;

use super::connection::Connection;
use super::request;
use crate::error::Error;
use crate::window::CursorIcon;

/// The cursor id meaning "inherit the parent window's cursor".
pub(crate) const INHERIT: u32 = 0;

/// The cursor-font glyph of an icon (`<X11/cursorfont.h>`), or `None` for
/// the default cursor, which is inherited so the desktop's themed arrow
/// shows.
const fn glyph(icon: CursorIcon) -> Option<u16> {
    match icon {
        CursorIcon::Default => None,
        CursorIcon::Pointer => Some(60),           // XC_hand2
        CursorIcon::Crosshair => Some(34),         // XC_crosshair
        CursorIcon::Move => Some(52),              // XC_fleur
        CursorIcon::Wait => Some(150),             // XC_watch
        CursorIcon::Text => Some(152),             // XC_xterm
        CursorIcon::ResizeHorizontal => Some(108), // XC_sb_h_double_arrow
        CursorIcon::ResizeVertical => Some(116),   // XC_sb_v_double_arrow
    }
}

/// Creates cursors on first use and keeps them for the connection's
/// lifetime.
#[derive(Debug, Default)]
pub(crate) struct Cursors {
    font: Option<u32>,
    cursors: HashMap<CursorIcon, u32>,
}

impl Cursors {
    /// The cursor id for `icon`.
    pub(crate) fn get(&mut self, conn: &mut Connection, icon: CursorIcon) -> Result<u32, Error> {
        let Some(glyph) = glyph(icon) else {
            return Ok(INHERIT);
        };
        if let Some(&cursor) = self.cursors.get(&icon) {
            return Ok(cursor);
        }
        let font = if let Some(font) = self.font {
            font
        } else {
            let font = conn.generate_id()?;
            conn.send_checked(&request::open_font(font, "cursor"))?;
            *self.font.insert(font)
        };
        let cursor = conn.generate_id()?;
        conn.send_checked(&request::create_glyph_cursor(cursor, font, glyph))?;
        self.cursors.insert(icon, cursor);
        Ok(cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_are_even_source_glyphs() {
        // Each cursor-font shape is a pair: the source glyph (even) and
        // its mask (odd).
        let icons = [
            CursorIcon::Pointer,
            CursorIcon::Crosshair,
            CursorIcon::Move,
            CursorIcon::Wait,
            CursorIcon::Text,
            CursorIcon::ResizeHorizontal,
            CursorIcon::ResizeVertical,
        ];
        for icon in icons {
            assert_eq!(glyph(icon).map(|g| g % 2), Some(0), "{icon:?}");
        }
        assert_eq!(glyph(CursorIcon::Default), None);
    }
}
