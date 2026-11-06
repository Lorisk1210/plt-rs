//! Mouse cursors at the desktop's cursor size: drawn by the platform
//! ([`cursor_art`](crate::cursor_art)) and handed to the server as ARGB
//! cursors through the RENDER extension, as toolkits hand over their
//! themed cursors. Without RENDER (or when the server refuses one), the
//! server's built-in cursor font stands in, whose cursors are 16 pixels
//! whatever the screen's scale.

use std::collections::HashMap;

use super::connection::Connection;
use super::reply;
use super::request::{self, ImageTarget, Request, render};
use super::setup::ImageByteOrder;
use crate::cursor_art::Outline;
use crate::error::Error;
use crate::window::CursorIcon;

/// The cursor id meaning "inherit the parent window's cursor".
pub(crate) const INHERIT: u32 = 0;

/// The depth of ARGB pixmaps.
const ARGB_DEPTH: u8 = 32;
/// Bytes of a `PutImage` request before its pixels.
const PUT_IMAGE_HEADER: usize = 24;
/// The largest cursor drawn, in pixels.
const MAX_SIZE: u32 = 256;

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

/// What the server offers for ARGB cursors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Argb {
    /// RENDER's major opcode.
    major_opcode: u8,
    /// Its ARGB32 picture format.
    format: u32,
}

impl Argb {
    /// RENDER 0.5 or later, with an ARGB32 picture format and 32-bit
    /// pixmaps, if the server has them; an X error asking counts as no.
    fn query(conn: &mut Connection) -> Result<Option<Self>, Error> {
        let pixmaps = conn.setup().format(ARGB_DEPTH);
        if pixmaps.is_none_or(|format| format.bits_per_pixel != 32) {
            return Ok(None);
        }
        let asked = Self::ask_render(conn);
        match asked {
            Err(Error::X11(_)) => Ok(None),
            asked => asked,
        }
    }

    fn ask_render(conn: &mut Connection) -> Result<Option<Self>, Error> {
        let Some(major_opcode) = conn.extension("RENDER")? else {
            return Ok(None);
        };
        let version = conn.round_trip(&render::query_version(major_opcode))?;
        if reply::render_version(&version)? < (0, 5) {
            return Ok(None);
        }
        let formats = conn.round_trip(&render::query_pict_formats(major_opcode))?;
        Ok(reply::argb32_format(&formats)?.map(|format| Self {
            major_opcode,
            format,
        }))
    }
}

/// Whether the server makes ARGB cursors, once asked.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ArgbCursors {
    /// Not asked yet.
    #[default]
    Unknown,
    /// No: the cursor font serves.
    Missing,
    /// Yes, through this RENDER.
    Present(Argb),
}

/// Creates cursors on first use and keeps them for the connection's
/// lifetime.
#[derive(Debug, Default)]
pub(crate) struct Cursors {
    font: Option<u32>,
    argb: ArgbCursors,
    /// The cursors made, by icon and size.
    made: HashMap<(CursorIcon, u32), u32>,
}

impl Cursors {
    /// The cursor id for `icon`, drawn `size` pixels wide where the server
    /// takes ARGB cursors, for windows on the screen of `root`.
    pub(crate) fn get(
        &mut self,
        conn: &mut Connection,
        root: u32,
        icon: CursorIcon,
        size: u32,
    ) -> Result<u32, Error> {
        if icon == CursorIcon::Default {
            return Ok(INHERIT);
        }
        let size = size.clamp(1, MAX_SIZE);
        if let Some(&cursor) = self.made.get(&(icon, size)) {
            return Ok(cursor);
        }
        if self.argb == ArgbCursors::Unknown {
            self.argb = Argb::query(conn)?.map_or(ArgbCursors::Missing, ArgbCursors::Present);
        }
        let drawn = if let ArgbCursors::Present(argb) = self.argb {
            match drawn_cursor(conn, argb, root, icon, size) {
                Err(Error::X11(_)) => None,
                drawn => Some(drawn?),
            }
        } else {
            None
        };
        let cursor = if let Some(cursor) = drawn {
            cursor
        } else {
            // A server refusing one ARGB cursor is not asked again.
            self.argb = ArgbCursors::Missing;
            self.glyph_cursor(conn, icon)?
        };
        self.made.insert((icon, size), cursor);
        Ok(cursor)
    }

    /// Whether the cursors are drawn ARGB cursors.
    #[cfg(test)]
    pub(crate) fn draws(&self) -> bool {
        matches!(self.argb, ArgbCursors::Present(_))
    }

    /// The cursor of `icon` from the cursor font.
    fn glyph_cursor(&mut self, conn: &mut Connection, icon: CursorIcon) -> Result<u32, Error> {
        let Some(glyph) = glyph(icon) else {
            return Ok(INHERIT);
        };
        let font = if let Some(font) = self.font {
            font
        } else {
            let font = conn.generate_id()?;
            conn.send_checked(&request::open_font(font, "cursor"))?;
            *self.font.insert(font)
        };
        let cursor = conn.generate_id()?;
        conn.send_checked(&request::create_glyph_cursor(cursor, font, glyph))?;
        Ok(cursor)
    }
}

/// Draws `icon` `size` pixels wide and makes it an ARGB cursor; the
/// pixmap, graphics context and picture made on the way are freed.
fn drawn_cursor(
    conn: &mut Connection,
    argb: Argb,
    root: u32,
    icon: CursorIcon,
    size: u32,
) -> Result<u32, Error> {
    let mut made = Vec::new();
    let cursor = make_cursor(conn, argb, root, icon, size, &mut made);
    for free in made.iter().rev() {
        conn.send(free)?;
    }
    cursor
}

/// The steps of [`drawn_cursor`], each checked, so that a refusal shows
/// here rather than later; `made` collects the requests freeing what was
/// made.
fn make_cursor(
    conn: &mut Connection,
    argb: Argb,
    root: u32,
    icon: CursorIcon,
    size: u32,
    made: &mut Vec<Request>,
) -> Result<u32, Error> {
    let outline = Outline::of_icon(icon);
    // At most `MAX_SIZE`: the casts are exact.
    let side = size as u16;
    let pixels = argb_pixels(&outline.draw(size), conn.setup().image_byte_order);
    let pixmap = conn.generate_id()?;
    conn.send_checked(&request::create_pixmap(
        pixmap,
        root,
        ARGB_DEPTH,
        (side, side),
    ))?;
    made.push(request::free_pixmap(pixmap));
    let gc = conn.generate_id()?;
    conn.send_checked(&request::create_gc(gc, pixmap))?;
    made.push(request::free_gc(gc));
    let target = ImageTarget {
        drawable: pixmap,
        gc,
        depth: ARGB_DEPTH,
    };
    let stride = size as usize * 4;
    let rows = (conn.max_request_bytes().saturating_sub(PUT_IMAGE_HEADER) / stride).max(1);
    for (index, band) in pixels.chunks(rows * stride).enumerate() {
        let (y, height) = ((index * rows) as i16, (band.len() / stride) as u16);
        let put = request::put_image(target, (0, y, side, height), band.len(), |data| {
            data.extend_from_slice(band);
        });
        conn.send_checked(&put)?;
    }
    let picture = conn.generate_id()?;
    conn.send_checked(&render::create_picture(
        argb.major_opcode,
        picture,
        pixmap,
        argb.format,
    ))?;
    made.push(render::free_picture(argb.major_opcode, picture));
    let cursor = conn.generate_id()?;
    let (x, y) = outline.hotspot_at(size);
    conn.send_checked(&render::create_cursor(
        argb.major_opcode,
        cursor,
        picture,
        (x as u16, y as u16),
    ))?;
    Ok(cursor)
}

/// Premultiplied RGBA pixels as ARGB32 pixel values in the server's byte
/// order.
fn argb_pixels(rgba: &[u8], order: ImageByteOrder) -> Vec<u8> {
    let (pixels, _) = rgba.as_chunks::<4>();
    pixels
        .iter()
        .flat_map(|&[r, g, b, a]| match order {
            ImageByteOrder::LsbFirst => [b, g, r, a],
            ImageByteOrder::MsbFirst => [a, r, g, b],
        })
        .collect()
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

    #[test]
    fn pixels_follow_the_servers_byte_order() {
        let rgba = [1, 2, 3, 4, 255, 0, 0, 255];
        assert_eq!(
            argb_pixels(&rgba, ImageByteOrder::LsbFirst),
            [3, 2, 1, 4, 0, 0, 255, 255]
        );
        assert_eq!(
            argb_pixels(&rgba, ImageByteOrder::MsbFirst),
            [4, 1, 2, 3, 255, 255, 0, 0]
        );
    }
}
