//! Pixels shown through shared memory: a surface's image in client memory
//! ([`Pixels`]), and the `wl_shm` buffers the compositor reads
//! ([`Swapchain`]).
//!
//! Each buffer remembers the region it missed while the compositor held
//! it, so that presenting writes only what changed since the buffer was
//! last used. Buffers are written with `pwrite`, never mapped (see
//! [`super::sys`]).

use std::fs::File;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileExt;

use super::connection::Connection;
use super::protocol::Interface;
use super::protocol::core::{self, FORMAT_ARGB8888, FORMAT_XRGB8888};
use super::sys;
use crate::error::Error;
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize};

/// Bytes per pixel of both formats.
const BYTES_PER_PIXEL: usize = 4;
/// Buffers per surface: one shown, one being drawn, one spare while the
/// compositor is slow to release.
const MAX_BUFFERS: usize = 3;

/// How a surface's pixels are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PixelFormat {
    /// Opaque: alpha is dropped, compositing over black (`xrgb8888`).
    Opaque,
    /// Premultiplied alpha (`argb8888`).
    Premultiplied,
}

impl PixelFormat {
    /// The `wl_shm.format` code.
    const fn code(self) -> u32 {
        match self {
            Self::Opaque => FORMAT_XRGB8888,
            Self::Premultiplied => FORMAT_ARGB8888,
        }
    }
}

/// The bytes of a `size` image of 32-bit pixels, if they fit the
/// protocol's 32-bit sizes.
fn byte_len(size: PhysicalSize) -> Result<usize, Error> {
    let len = u64::from(size.width) * u64::from(size.height) * BYTES_PER_PIXEL as u64;
    i32::try_from(len)
        .ok()
        .and_then(|len| usize::try_from(len).ok())
        .ok_or(Error::InvalidArgument("window too large for shared memory"))
}

/// A surface's image in `wl_shm` byte order (B, G, R, A).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pixels {
    size: PhysicalSize,
    format: PixelFormat,
    data: Vec<u8>,
    /// Whether every byte is zero (a transparent image nothing was drawn
    /// on), which fresh buffers already hold.
    zeroed: bool,
}

impl Pixels {
    /// A black (opaque) or transparent image of `size`.
    pub(crate) fn new(size: PhysicalSize, format: PixelFormat) -> Result<Self, Error> {
        let mut pixels = Self {
            size: PhysicalSize::new(0, 0),
            format,
            data: Vec::new(),
            zeroed: format == PixelFormat::Premultiplied,
        };
        pixels.resize(size)?;
        Ok(pixels)
    }

    /// The image size.
    pub(crate) const fn size(&self) -> PhysicalSize {
        self.size
    }

    /// The pixel a resized image is filled with.
    fn background(&self) -> [u8; 4] {
        match self.format {
            PixelFormat::Opaque => [0, 0, 0, 255],
            PixelFormat::Premultiplied => [0; 4],
        }
    }

    /// Changes the size, keeping the overlapping top-left part.
    pub(crate) fn resize(&mut self, size: PhysicalSize) -> Result<(), Error> {
        if size == self.size {
            return Ok(());
        }
        let len = byte_len(size)?;
        let mut data: Vec<u8> = self.background().into_iter().cycle().take(len).collect();
        let keep = PhysicalRect::from_size(self.size).clip_to(size);
        let row_len = keep.width as usize * BYTES_PER_PIXEL;
        for y in 0..keep.height as usize {
            let from = y * self.size.width as usize * BYTES_PER_PIXEL;
            let to = y * size.width as usize * BYTES_PER_PIXEL;
            data[to..to + row_len].copy_from_slice(&self.data[from..from + row_len]);
        }
        self.size = size;
        self.data = data;
        Ok(())
    }

    /// Copies `region` of a premultiplied RGBA `frame` to the same place,
    /// clipped to both images.
    pub(crate) fn blit(&mut self, frame: &Frame<'_>, region: PhysicalRect) {
        let region = region.clip_to(frame.size()).clip_to(self.size);
        self.zeroed &= region.is_empty();
        let opaque = self.format == PixelFormat::Opaque;
        for y in region.y..region.y + region.height {
            let source = &frame.row(y)[region.x as usize * BYTES_PER_PIXEL..]
                [..region.width as usize * BYTES_PER_PIXEL];
            let start = self.offset(region.x, y);
            let target = &mut self.data[start..start + source.len()];
            let pixels = target.as_chunks_mut::<BYTES_PER_PIXEL>().0;
            for (to, from) in pixels
                .iter_mut()
                .zip(source.as_chunks::<BYTES_PER_PIXEL>().0)
            {
                let alpha = if opaque { 255 } else { from[3] };
                *to = [from[2], from[1], from[0], alpha];
            }
        }
    }

    /// The image as opaque RGBA.
    pub(crate) fn to_frame(&self) -> Result<OwnedFrame, Error> {
        let rgba = self
            .data
            .as_chunks::<BYTES_PER_PIXEL>()
            .0
            .iter()
            .flat_map(|px| [px[2], px[1], px[0], 255])
            .collect();
        OwnedFrame::new(self.size.width, self.size.height, rgba)
    }

    fn offset(&self, x: u32, y: u32) -> usize {
        (y as usize * self.size.width as usize + x as usize) * BYTES_PER_PIXEL
    }

    /// The bytes of `rect`'s rows, each with its offset in the image.
    fn rows(&self, rect: PhysicalRect) -> impl Iterator<Item = (usize, &[u8])> {
        let full_width = rect.x == 0 && rect.width == self.size.width;
        let (rows, row_len) = if full_width {
            (
                1,
                rect.width as usize * rect.height as usize * BYTES_PER_PIXEL,
            )
        } else {
            (rect.height, rect.width as usize * BYTES_PER_PIXEL)
        };
        (0..rows).map(move |row| {
            let start = self.offset(rect.x, rect.y + row);
            (start, &self.data[start..start + row_len])
        })
    }
}

/// One `wl_buffer` over its own shared-memory file.
#[derive(Debug)]
struct ShmBuffer {
    id: u32,
    file: File,
    size: PhysicalSize,
    /// Whether the compositor may still read it.
    busy: bool,
    /// What changed in the image since this buffer was last written.
    stale: PhysicalRect,
    /// Whether it was never written (all zeros).
    fresh: bool,
}

impl ShmBuffer {
    /// A buffer of `size` filled with zeros (transparent or black).
    fn new(
        conn: &mut Connection,
        shm: u32,
        size: PhysicalSize,
        format: PixelFormat,
    ) -> Result<Self, Error> {
        let len = byte_len(size)?;
        let file = sys::memfd(c"plt-buffer")
            .and_then(|file| file.set_len(len as u64).map(|()| file))
            .map_err(|error| Error::io("creating shared memory", &error))?;
        let shared = file
            .try_clone()
            .map_err(|error| Error::io("sharing memory", &error))?;
        let pool = conn.new_id(Interface::ShmPool)?;
        let id = conn.new_id(Interface::Buffer)?;
        // `byte_len` checked that the whole buffer fits an `i32`.
        conn.send(core::create_pool(
            shm,
            pool,
            OwnedFd::from(shared),
            len as i32,
        ))?;
        let dimensions = (size.width as i32, size.height as i32);
        conn.send(core::create_buffer(pool, id, 0, dimensions, format.code()))?;
        conn.send(core::destroy_pool(pool))?;
        Ok(Self {
            id,
            file,
            size,
            busy: false,
            stale: PhysicalRect::from_size(size),
            fresh: true,
        })
    }

    /// Copies `rect` of `pixels` into the buffer.
    fn write(&self, pixels: &Pixels, rect: PhysicalRect) -> Result<(), Error> {
        for (offset, bytes) in pixels.rows(rect) {
            self.file
                .write_all_at(bytes, offset as u64)
                .map_err(|error| Error::io("writing shared memory", &error))?;
        }
        Ok(())
    }
}

/// The buffers of one surface.
#[derive(Debug)]
pub(crate) struct Swapchain {
    format: PixelFormat,
    buffers: Vec<ShmBuffer>,
}

impl Swapchain {
    /// No buffers yet, for pixels of `format`.
    pub(crate) const fn new(format: PixelFormat) -> Self {
        Self {
            format,
            buffers: Vec::new(),
        }
    }

    /// Writes `pixels` to a free buffer, refreshing `damage` and whatever
    /// the buffer missed, and attaches it to `surface` with that damage.
    /// Returns `false` without doing anything when every buffer is busy.
    /// The caller commits.
    pub(crate) fn attach(
        &mut self,
        conn: &mut Connection,
        shm: u32,
        surface: u32,
        pixels: &Pixels,
        damage: PhysicalRect,
    ) -> Result<bool, Error> {
        let size = pixels.size();
        // Buffers of an old size are no use; drop the idle ones.
        let mut index = 0;
        while index < self.buffers.len() {
            let buffer = &self.buffers[index];
            if buffer.size != size && !buffer.busy {
                conn.send(core::destroy_buffer(buffer.id))?;
                self.buffers.swap_remove(index);
            } else {
                index += 1;
            }
        }
        let free = self.buffers.iter().position(|b| !b.busy && b.size == size);
        let index = match free {
            Some(index) => index,
            None if self.buffers.len() < MAX_BUFFERS => {
                let buffer = ShmBuffer::new(conn, shm, size, self.format)?;
                self.buffers.push(buffer);
                self.buffers.len() - 1
            }
            None => return Ok(false),
        };
        let damage = damage.clip_to(size);
        let buffer = &mut self.buffers[index];
        let refresh = union(buffer.stale, damage).clip_to(size);
        // Fresh buffers hold zeros already.
        let unchanged = refresh.is_empty() || (buffer.fresh && pixels.zeroed);
        if !unchanged {
            buffer.write(pixels, refresh)?;
            buffer.fresh = false;
        }
        buffer.stale = PhysicalRect::new(0, 0, 0, 0);
        buffer.busy = true;
        let id = buffer.id;
        for other in &mut self.buffers {
            if other.id != id && other.size == size {
                other.stale = union(other.stale, damage);
            }
        }
        conn.send(core::attach(surface, id))?;
        let full = PhysicalRect::from_size(size);
        let damage = if refresh == full { full } else { damage };
        conn.send(core::damage_buffer(
            surface,
            damage.x as i32,
            damage.y as i32,
            damage.width as i32,
            damage.height as i32,
        ))?;
        Ok(true)
    }

    /// How many buffers there are.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.buffers.len()
    }

    /// Notes that the compositor released buffer `id`; returns whether it
    /// is one of these buffers.
    pub(crate) fn release(&mut self, id: u32) -> bool {
        self.buffers
            .iter_mut()
            .find(|buffer| buffer.id == id)
            .map(|buffer| buffer.busy = false)
            .is_some()
    }

    /// Destroys every buffer.
    pub(crate) fn destroy(&mut self, conn: &mut Connection) -> Result<(), Error> {
        for buffer in self.buffers.drain(..) {
            conn.send(core::destroy_buffer(buffer.id))?;
        }
        Ok(())
    }
}

/// The smallest rectangle covering `a` and `b` (empty ones ignored).
pub(crate) fn union(a: PhysicalRect, b: PhysicalRect) -> PhysicalRect {
    if a.is_empty() {
        return b;
    }
    if b.is_empty() {
        return a;
    }
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let right = (a.x + a.width).max(b.x + b.width);
    let bottom = (a.y + a.height).max(b.y + b.height);
    PhysicalRect::new(x, y, right - x, bottom - y)
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixStream;

    use super::*;

    fn rgba(size: PhysicalSize, color: [u8; 4]) -> OwnedFrame {
        OwnedFrame::filled(size, color).unwrap()
    }

    #[test]
    fn pixels_convert_and_resize() {
        let size = PhysicalSize::new(3, 2);
        let mut pixels = Pixels::new(size, PixelFormat::Opaque).unwrap();
        assert_eq!(pixels.to_frame().unwrap().pixel(2, 1), Some([0, 0, 0, 255]));
        let frame = rgba(PhysicalSize::new(2, 2), [10, 20, 30, 128]);
        pixels.blit(&frame.as_frame(), PhysicalRect::new(1, 0, 5, 5));
        assert_eq!(&pixels.data[4..8], [30, 20, 10, 255], "BGRX, opaque");
        assert_eq!(&pixels.data[..4], [0, 0, 0, 255], "outside the region");
        pixels.resize(PhysicalSize::new(2, 3)).unwrap();
        let read = pixels.to_frame().unwrap();
        assert_eq!(read.pixel(1, 1), Some([10, 20, 30, 255]));
        assert_eq!(read.pixel(1, 2), Some([0, 0, 0, 255]));

        let mut clear = Pixels::new(size, PixelFormat::Premultiplied).unwrap();
        assert!(clear.zeroed && !pixels.zeroed);
        clear.blit(&frame.as_frame(), PhysicalRect::new(9, 9, 1, 1));
        assert!(clear.zeroed, "nothing was drawn");
        clear.blit(&frame.as_frame(), PhysicalRect::from_size(size));
        assert!(!clear.zeroed);
        assert_eq!(&clear.data[..4], [30, 20, 10, 128], "alpha kept");
        assert_eq!(&clear.data[8..12], [0; 4], "transparent background");
    }

    #[test]
    fn rows_are_contiguous_for_full_width_rectangles() {
        let pixels = Pixels::new(PhysicalSize::new(4, 3), PixelFormat::Opaque).unwrap();
        let full: Vec<_> = pixels.rows(PhysicalRect::new(0, 1, 4, 2)).collect();
        assert_eq!(full.len(), 1);
        assert_eq!((full[0].0, full[0].1.len()), (16, 32));
        let part: Vec<_> = pixels.rows(PhysicalRect::new(1, 1, 2, 2)).collect();
        assert_eq!(
            part.iter().map(|r| (r.0, r.1.len())).collect::<Vec<_>>(),
            [(20, 8), (36, 8)]
        );
    }

    #[test]
    fn sizes_must_fit_the_protocol() {
        assert_eq!(byte_len(PhysicalSize::new(2, 3)), Ok(24));
        assert!(byte_len(PhysicalSize::new(40_000, 40_000)).is_err());
        assert!(Pixels::new(PhysicalSize::new(u32::MAX, 2), PixelFormat::Opaque).is_err());
    }

    #[test]
    fn unions_cover_both() {
        let a = PhysicalRect::new(1, 1, 2, 2);
        let b = PhysicalRect::new(4, 0, 1, 1);
        assert_eq!(union(a, b), PhysicalRect::new(1, 0, 4, 3));
        let empty = PhysicalRect::new(9, 9, 0, 0);
        assert_eq!(union(empty, b), b);
        assert_eq!(union(a, empty), a);
    }

    #[test]
    fn buffers_kept_through_a_resize_only_catch_up_on_their_size() {
        let (ours, _compositor) = UnixStream::pair().unwrap();
        let mut conn = Connection::new(ours).unwrap();
        let small = PhysicalSize::new(4, 2);
        let mut pixels = Pixels::new(small, PixelFormat::Opaque).unwrap();
        let mut chain = Swapchain::new(PixelFormat::Opaque);
        assert!(
            chain
                .attach(&mut conn, 9, 10, &pixels, PhysicalRect::from_size(small))
                .unwrap()
        );
        let first = chain.buffers[0].id;
        let large = PhysicalSize::new(9, 7);
        pixels.resize(large).unwrap();
        assert!(
            chain
                .attach(&mut conn, 9, 10, &pixels, PhysicalRect::from_size(large))
                .unwrap()
        );
        pixels.resize(small).unwrap();
        chain.release(first);
        assert!(
            chain
                .attach(&mut conn, 9, 10, &pixels, PhysicalRect::new(1, 0, 1, 1))
                .unwrap()
        );
        assert_eq!(
            chain.len(),
            2,
            "the busy large buffer stays, the small one is reused"
        );
    }

    #[test]
    fn buffers_rotate_and_catch_up_on_missed_damage() {
        let (ours, _compositor) = UnixStream::pair().unwrap();
        let mut conn = Connection::new(ours).unwrap();
        let size = PhysicalSize::new(4, 4);
        let mut pixels = Pixels::new(size, PixelFormat::Opaque).unwrap();
        let mut chain = Swapchain::new(PixelFormat::Opaque);
        let full = PhysicalRect::from_size(size);
        assert!(chain.attach(&mut conn, 9, 10, &pixels, full).unwrap());
        let first = chain.buffers[0].id;
        let dot = PhysicalRect::new(1, 1, 1, 1);
        pixels.blit(&rgba(size, [255, 0, 0, 255]).as_frame(), dot);
        assert!(chain.attach(&mut conn, 9, 10, &pixels, dot).unwrap());
        assert_eq!(chain.buffers.len(), 2, "the first is still busy");
        assert_eq!(chain.buffers[0].stale, dot, "the first missed the dot");
        assert!(chain.release(first));
        assert!(!chain.release(12_345));
        assert!(
            chain
                .attach(&mut conn, 9, 10, &pixels, PhysicalRect::new(0, 0, 0, 0))
                .unwrap()
        );
        let mut written = [0; 4];
        chain.buffers[0]
            .file
            .read_exact_at(&mut written, 20)
            .unwrap();
        assert_eq!(
            written,
            [0, 0, 255, 255],
            "the dot reached the reused buffer"
        );
        assert!(chain.attach(&mut conn, 9, 10, &pixels, full).unwrap());
        assert!(
            !chain.attach(&mut conn, 9, 10, &pixels, full).unwrap(),
            "all busy"
        );
        pixels.resize(PhysicalSize::new(2, 2)).unwrap();
        chain.release(first);
        assert!(chain.attach(&mut conn, 9, 10, &pixels, full).unwrap());
        assert!(
            chain.buffers.iter().all(|b| b.busy),
            "the idle old-size buffer was dropped"
        );
        chain.destroy(&mut conn).unwrap();
        assert!(chain.buffers.is_empty());
    }
}
