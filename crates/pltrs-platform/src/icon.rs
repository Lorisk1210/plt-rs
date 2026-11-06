//! Window and application icons: one bitmap, or a renderer drawing the
//! icon at each size, and in each layout, a platform asks for, so that
//! no platform scales a bitmap.
//!
//! What each platform asks for:
//!
//! * macOS: the Dock's icon, [`IconLayout::AppleGrid`], from 16 to 1024
//!   pixels, in an `NSImage` holding a representation for each point size
//!   at 1× and 2×.
//! * Windows: the title bar's and the task bar's icons
//!   (`SM_CXSMICON` and `SM_CXICON` at the window's DPI, again when it
//!   moves to a monitor of another DPI), [`IconLayout::FullBleed`].
//! * X11: `_NET_WM_ICON` with 16 to 256 pixels, full bleed.
//! * Wayland: `xdg-toplevel-icon-v1`'s sizes, full bleed, where the
//!   compositor offers the protocol.

use std::fmt;
use std::sync::Arc;

use crate::error::Error;
use crate::geometry::PhysicalSize;

/// How an application icon sits in its square, by the desktop's
/// convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IconLayout {
    /// The icon's body fills the square: Windows, X11 and Wayland.
    FullBleed,
    /// Apple's icon grid (macOS since Big Sur): the body takes the
    /// middle 824/1024 of the square and casts a shadow into the margin,
    /// so that the icon looks as large as the others in the Dock.
    AppleGrid,
}

/// An icon's pixels: straight (non-premultiplied) RGBA8, row-major, top
/// row first.
#[derive(Clone, PartialEq, Eq)]
pub struct IconImage {
    size: PhysicalSize,
    rgba: Vec<u8>,
}

impl IconImage {
    /// An image, checking that `rgba` holds exactly `width * height * 4`
    /// bytes and the image is not empty.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] for an empty size, and
    /// [`Error::BufferSize`] for pixels of the wrong length.
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, Error> {
        let size = PhysicalSize::new(width, height);
        if size.is_empty() {
            return Err(Error::InvalidArgument("icon size must be non-zero"));
        }
        let expected = crate::frame::rgba_len(size)?;
        if rgba.len() != expected {
            return Err(Error::BufferSize {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self { size, rgba })
    }

    /// The image's dimensions.
    pub const fn size(&self) -> PhysicalSize {
        self.size
    }

    /// The straight-alpha RGBA8 pixels.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

impl fmt::Debug for IconImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IconImage")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// Draws an icon `side` pixels wide in a layout; `None` when it cannot.
type Renderer = dyn Fn(u32, IconLayout) -> Option<IconImage> + Send + Sync;

/// A window icon, which macOS shows as the application's in the Dock.
///
/// Clones share their bitmap or renderer; two icons are equal when they
/// are clones of one renderer, or bitmaps of the same pixels.
///
/// ```
/// use pltrs_platform::{Icon, IconImage, IconLayout};
/// let icon = Icon::from_renderer(|side, _layout| {
///     IconImage::from_rgba(side, side, vec![255; (side * side * 4) as usize]).ok()
/// });
/// let images = icon.images(&[16, 32], IconLayout::FullBleed);
/// assert_eq!(images.len(), 2);
/// assert_eq!(images[1].size().width, 32);
/// ```
#[derive(Clone)]
pub struct Icon(Source);

#[derive(Clone)]
enum Source {
    /// One bitmap, shown at its own size.
    Bitmap(Arc<IconImage>),
    /// Draws the icon at the sizes asked for.
    Renderer(Arc<Renderer>),
}

impl Icon {
    /// An icon of one bitmap (see [`IconImage::from_rgba`]), which each
    /// platform scales to the sizes it shows.
    ///
    /// # Errors
    ///
    /// As [`IconImage::from_rgba`].
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(Source::Bitmap(Arc::new(IconImage::from_rgba(
            width, height, rgba,
        )?))))
    }

    /// An icon drawn by `render` at each size and layout a platform asks
    /// for: `render(side, layout)` gives a `side × side` image, or `None`
    /// to leave that size out.
    pub fn from_renderer(
        render: impl Fn(u32, IconLayout) -> Option<IconImage> + Send + Sync + 'static,
    ) -> Self {
        Self(Source::Renderer(Arc::new(render)))
    }

    /// The images to give a platform showing the icon `sides` pixels wide
    /// in `layout`: the renderer's image of each side (those it does not
    /// draw, or draws at another size, left out), or the bitmap alone, at
    /// its own size.
    pub fn images(&self, sides: &[u32], layout: IconLayout) -> Vec<IconImage> {
        match &self.0 {
            Source::Bitmap(image) => vec![IconImage::clone(image)],
            Source::Renderer(render) => sides
                .iter()
                .filter_map(|&side| {
                    render(side, layout).filter(|image| image.size == PhysicalSize::new(side, side))
                })
                .collect(),
        }
    }
}

impl PartialEq for Icon {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Source::Bitmap(a), Source::Bitmap(b)) => a == b,
            (Source::Renderer(a), Source::Renderer(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for Icon {}

impl fmt::Debug for Icon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Source::Bitmap(image) => f.debug_tuple("Icon").field(image).finish(),
            Source::Renderer(_) => f.write_str("Icon(<renderer>)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(side: u32, layout: IconLayout) -> Option<IconImage> {
        let value = match layout {
            IconLayout::FullBleed => 1,
            IconLayout::AppleGrid => 2,
        };
        IconImage::from_rgba(side, side, vec![value; (side * side * 4) as usize]).ok()
    }

    #[test]
    fn images_validate_their_buffers() {
        assert!(IconImage::from_rgba(2, 2, vec![0; 16]).is_ok());
        assert_eq!(
            IconImage::from_rgba(2, 2, vec![0; 15]),
            Err(Error::BufferSize {
                expected: 16,
                actual: 15
            })
        );
        assert!(IconImage::from_rgba(0, 2, Vec::new()).is_err());
        assert!(Icon::from_rgba(2, 2, vec![0; 3]).is_err());
    }

    #[test]
    fn renderers_draw_each_side_in_the_layout_asked_for() {
        let icon = Icon::from_renderer(solid);
        let images = icon.images(&[16, 24, 256], IconLayout::AppleGrid);
        let sides: Vec<u32> = images.iter().map(|image| image.size().width).collect();
        assert_eq!(sides, [16, 24, 256]);
        assert!(images.iter().all(|image| image.rgba()[0] == 2));
        let full = icon.images(&[8], IconLayout::FullBleed);
        assert_eq!(full[0].rgba()[0], 1);
        assert_eq!(icon.images(&[], IconLayout::FullBleed), []);
    }

    #[test]
    fn sizes_a_renderer_skips_or_misdraws_are_left_out() {
        let icon = Icon::from_renderer(|side, layout| match side {
            0..=31 => solid(side, layout),
            32 => solid(side + 1, layout),
            _ => None,
        });
        let images = icon.images(&[16, 32, 64], IconLayout::FullBleed);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].size(), PhysicalSize::new(16, 16));
    }

    #[test]
    fn bitmaps_come_at_their_own_size() {
        let icon = Icon::from_rgba(3, 2, vec![7; 24]).unwrap();
        let images = icon.images(&[16, 32], IconLayout::AppleGrid);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].size(), PhysicalSize::new(3, 2));
        assert_eq!(
            format!("{icon:?}"),
            "Icon(IconImage { size: PhysicalSize { width: 3, height: 2 }, .. })"
        );
    }

    #[test]
    fn clones_of_one_renderer_are_equal() {
        let icon = Icon::from_renderer(solid);
        assert_eq!(icon, icon.clone());
        assert_ne!(icon, Icon::from_renderer(solid));
        let bitmap = Icon::from_rgba(1, 1, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(bitmap, Icon::from_rgba(1, 1, vec![1, 2, 3, 4]).unwrap());
        assert_ne!(bitmap, icon);
        assert_eq!(format!("{icon:?}"), "Icon(<renderer>)");
    }
}
