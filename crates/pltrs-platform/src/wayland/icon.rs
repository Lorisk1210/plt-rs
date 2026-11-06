//! Window icons, where the compositor offers `xdg-toplevel-icon-v1`
//! (`KWin`, labwc, Jay and Muffin do; Mutter and Weston do not, and show
//! the icon of the desktop entry the window's app id names instead).
//!
//! The icon is drawn full bleed at the sizes the compositor prefers (or
//! at X11's sizes when it names none), at scale 1 and at the window's
//! scale, into `wl_shm` buffers of one pool; it is set before the
//! window's first commit, which applies it, and destroyed after it (the
//! toplevel keeps it).

use std::fs::File;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileExt;

use super::connection::Connection;
use super::protocol::Interface;
use super::protocol::core::{self, FORMAT_ARGB8888};
use super::protocol::icon as protocol;
use super::sys;
use crate::error::Error;
use crate::icon::{Icon, IconImage, IconLayout};

/// The icon sizes, in surface coordinates, drawn when the compositor
/// names none.
const DEFAULT_SIDES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
/// The most sizes taken from the compositor.
const MAX_SIDES: usize = 16;
/// The widest icon image drawn, in pixels: a compositor's absurd sizes
/// or scales are left out.
const MAX_PIXELS: u32 = 1024;

/// The icon sizes the compositor prefers: `icon_size` events, up to
/// their `done`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct IconSizes {
    /// Sizes announced since the last `done`.
    pending: Vec<u32>,
    /// The sizes of the last complete announcement.
    done: Vec<u32>,
}

impl IconSizes {
    /// Notes an `icon_size` event (sizes that are not positive, too
    /// large or too many are ignored).
    pub(crate) fn size(&mut self, side: i32) {
        if let Ok(side) = u32::try_from(side)
            && (1..=MAX_PIXELS).contains(&side)
            && !self.pending.contains(&side)
            && self.pending.len() < MAX_SIDES
        {
            self.pending.push(side);
        }
    }

    /// Notes the `done` event ending the sizes.
    pub(crate) fn done(&mut self) {
        self.done = std::mem::take(&mut self.pending);
    }

    /// The sizes to draw icons at.
    fn sides(&self) -> &[u32] {
        if self.done.is_empty() {
            &DEFAULT_SIDES
        } else {
            &self.done
        }
    }

    /// The `(pixel side, scale)` of each icon buffer for a window at
    /// integer `scale`: every size at scale 1, and at `scale` too.
    fn buffers(&self, scale: u32) -> Vec<(u32, u32)> {
        let scales = if scale > 1 { vec![1, scale] } else { vec![1] };
        scales
            .into_iter()
            .flat_map(|scale| {
                self.sides()
                    .iter()
                    .filter_map(move |side| Some((side.checked_mul(scale)?, scale)))
            })
            .filter(|&(pixels, _)| pixels <= MAX_PIXELS)
            .collect()
    }
}

/// An icon set on a toplevel, kept until a commit applied it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PendingIcon {
    icon: u32,
    buffers: Vec<u32>,
}

impl PendingIcon {
    /// Draws `icon` for a window at integer `scale` and sets it on
    /// `toplevel` through `manager`, in buffers of `shm`; it shows from the
    /// surface's next commit. `None` when the icon draws no square image.
    pub(crate) fn set(
        conn: &mut Connection,
        (manager, shm): (u32, u32),
        toplevel: u32,
        (icon, sizes): (&Icon, &IconSizes),
        scale: u32,
    ) -> Result<Option<Self>, Error> {
        let wanted = sizes.buffers(scale);
        let mut sides: Vec<u32> = wanted.iter().map(|&(pixels, _)| pixels).collect();
        sides.sort_unstable();
        sides.dedup();
        let images: Vec<(IconImage, u32)> = icon
            .images(&sides, IconLayout::FullBleed)
            .into_iter()
            .filter(|image| image.size().width == image.size().height)
            .flat_map(|image| {
                let side = image.size().width;
                let mut scales: Vec<u32> = wanted
                    .iter()
                    .filter(|&&(pixels, _)| pixels == side)
                    .map(|&(_, scale)| scale)
                    .collect();
                if scales.is_empty() {
                    // A bitmap of its own size.
                    scales.push(1);
                }
                scales.into_iter().map(move |scale| (image.clone(), scale))
            })
            .collect();
        if images.is_empty() {
            return Ok(None);
        }
        let buffers = share(conn, shm, &images)?;
        let id = conn.new_id(Interface::ToplevelIcon)?;
        conn.send(protocol::create_icon(manager, id))?;
        for (&buffer, &(_, scale)) in buffers.iter().zip(&images) {
            // Scales are at most `MAX_PIXELS`.
            conn.send(protocol::add_buffer(id, buffer, scale as i32))?;
        }
        conn.send(protocol::set_icon(manager, toplevel, id))?;
        Ok(Some(Self { icon: id, buffers }))
    }

    /// Destroys the icon and its buffers, once a commit applied it.
    pub(crate) fn destroy(self, conn: &mut Connection) -> Result<(), Error> {
        conn.send(protocol::destroy_icon(self.icon))?;
        for buffer in self.buffers {
            conn.send(core::destroy_buffer(buffer))?;
        }
        Ok(())
    }
}

/// Writes `images` one after the other into shared memory, as
/// premultiplied `argb8888`, and makes a buffer of each; returns their
/// ids.
fn share(conn: &mut Connection, shm: u32, images: &[(IconImage, u32)]) -> Result<Vec<u32>, Error> {
    let bytes: Vec<u8> = images
        .iter()
        .flat_map(|(image, _)| premultiplied_bgra(image.rgba()))
        .collect();
    let too_large = || Error::InvalidArgument("icon too large for shared memory");
    let len = i32::try_from(bytes.len()).map_err(|_| too_large())?;
    let file: File = sys::memfd(c"pltrs-icon")
        .and_then(|file| file.write_all_at(&bytes, 0).map(|()| file))
        .map_err(|error| Error::io("writing the icon to shared memory", &error))?;
    let pool = conn.new_id(Interface::ShmPool)?;
    conn.send(core::create_pool(shm, pool, OwnedFd::from(file), len))?;
    let mut buffers = Vec::with_capacity(images.len());
    let mut offset = 0;
    for (image, _) in images {
        let size = image.size();
        let buffer = conn.new_id(Interface::Buffer)?;
        // Every image fits the pool, whose length fits an `i32`.
        let dimensions = (size.width as i32, size.height as i32);
        conn.send(core::create_buffer(
            pool,
            buffer,
            offset,
            dimensions,
            FORMAT_ARGB8888,
        ))?;
        offset += image.rgba().len() as i32;
        buffers.push(buffer);
    }
    conn.send(core::destroy_pool(pool))?;
    Ok(buffers)
}

/// Straight RGBA pixels as premultiplied `argb8888` bytes (B, G, R, A).
fn premultiplied_bgra(rgba: &[u8]) -> impl Iterator<Item = u8> + '_ {
    rgba.as_chunks::<4>().0.iter().flat_map(|&[r, g, b, a]| {
        let scale = |c: u8| ((u32::from(c) * u32::from(a) + 127) / 255) as u8;
        [scale(b), scale(g), scale(r), a]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_follow_the_last_announcement() {
        let mut sizes = IconSizes::default();
        assert_eq!(sizes.sides(), DEFAULT_SIDES);
        sizes.size(48);
        sizes.size(48);
        sizes.size(-1);
        sizes.size(0);
        sizes.size(100_000);
        assert_eq!(sizes.sides(), DEFAULT_SIDES, "not done yet");
        sizes.done();
        assert_eq!(sizes.sides(), [48]);
        for side in 1..100 {
            sizes.size(side);
        }
        sizes.done();
        assert_eq!(sizes.sides().len(), MAX_SIDES);
        // A `done` alone says the compositor has no preference.
        sizes.done();
        assert_eq!(sizes.sides(), DEFAULT_SIDES);
    }

    #[test]
    fn buffers_come_at_scale_one_and_the_windows() {
        let mut sizes = IconSizes::default();
        sizes.size(32);
        sizes.size(600);
        sizes.done();
        assert_eq!(sizes.buffers(1), [(32, 1), (600, 1)]);
        assert_eq!(sizes.buffers(2), [(32, 1), (600, 1), (64, 2)]);
        assert_eq!(sizes.buffers(u32::MAX), [(32, 1), (600, 1)]);
    }

    #[test]
    fn pixels_are_premultiplied_bgra() {
        let bytes: Vec<u8> = premultiplied_bgra(&[255, 128, 0, 128, 10, 20, 30, 255]).collect();
        assert_eq!(bytes, [0, 64, 128, 128, 30, 20, 10, 255]);
    }
}
