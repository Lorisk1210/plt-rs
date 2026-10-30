//! What a figure keeps of its raster canvas between draws, and what the
//! windows showing it have yet to present.

use std::sync::atomic::{AtomicU64, Ordering};

use pltrs_core::path::convert::SimplifySettings;
use pltrs_render::{RasterBackendError, RasterRenderer};

use super::PixelBox;
use crate::figure::Figure;

/// Identifies one drawing of one figure's canvas: a new canvas and every
/// full draw start a new generation, which the regions saved before
/// cannot be restored into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Generation(u64);

impl Generation {
    /// A generation no canvas had before, in any figure.
    pub(super) fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// What a canvas was made for: matplotlib's renderer key (its size in
/// whole pixels and the resolution) and the rcParams a raster canvas
/// takes when made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CanvasKey {
    pixels: (u32, u32),
    dpi: f64,
    simplify: SimplifySettings,
    chunksize: i64,
}

impl CanvasKey {
    /// The key of a canvas for `figure` as it is now.
    pub(super) fn of(figure: &Figure) -> Self {
        let rc = crate::rc::params();
        Self {
            pixels: figure.width_height(),
            dpi: figure.dpi(),
            simplify: crate::figure::save::simplify_settings(),
            chunksize: rc.agg_path_chunksize,
        }
    }

    /// A blank canvas for this key.
    pub(super) fn make(self) -> Result<KeptCanvas, RasterBackendError> {
        let (width, height) = self.pixels;
        let renderer = crate::figure::save::raster_renderer(width, height, self.dpi)?;
        Ok(KeptCanvas::new(renderer, self))
    }
}

/// The raster canvas a figure keeps (matplotlib's cached `RendererAgg`).
#[derive(Debug)]
pub(crate) struct KeptCanvas {
    /// The canvas.
    pub(super) renderer: RasterRenderer,
    /// What it was made for.
    pub(super) key: CanvasKey,
    /// Its current drawing.
    pub(super) generation: Generation,
}

impl KeptCanvas {
    /// The blank canvas `renderer`, made for `key`.
    fn new(renderer: RasterRenderer, key: CanvasKey) -> Self {
        Self {
            renderer,
            key,
            generation: Generation::next(),
        }
    }
}

/// What the windows showing a figure have not presented yet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Presentation {
    /// Nothing changed.
    #[default]
    Nothing,
    /// These parts of the canvas were blitted.
    Regions(Vec<PixelBox>),
    /// The whole canvas was drawn.
    Whole,
}

pltrs_core::archive_enum!(Presentation {
    Nothing,
    Regions(v0),
    Whole,
});

impl Presentation {
    /// More blitted regions than this are presented as one box around
    /// them, so that a program blitting without returning to the window's
    /// event loop does not pile them up.
    const MAX_REGIONS: usize = 16;

    /// Adds a blitted region.
    fn add(&mut self, region: PixelBox) {
        match self {
            Self::Whole => {}
            Self::Nothing => *self = Self::Regions(vec![region]),
            Self::Regions(regions) => {
                if regions.contains(&region) {
                    return;
                }
                regions.push(region);
                if regions.len() > Self::MAX_REGIONS {
                    let around = regions.iter().copied().reduce(|a, b| {
                        PixelBox::new(
                            a.x0.min(b.x0),
                            a.y0.min(b.y0),
                            a.x1.max(b.x1),
                            a.y1.max(b.y1),
                        )
                    });
                    *regions = around.into_iter().collect();
                }
            }
        }
    }
}

/// The canvas state of a figure.
#[derive(Debug)]
pub(crate) struct CanvasState {
    /// The canvas, once made; taken out while it is drawn into.
    pub(super) kept: Option<KeptCanvas>,
    /// What the windows showing the figure have to present.
    pub(super) presentation: Presentation,
    /// How many windows and browser pages show the figure.
    pub(super) presenters: usize,
    /// Whether a draw asked for by `draw_idle` is running (matplotlib's
    /// `_is_idle_drawing`).
    pub(super) idle_drawing: bool,
    /// Physical pixels per logical pixel of the display showing the
    /// canvas (matplotlib's `device_pixel_ratio`).
    pub(super) device_pixel_ratio: f64,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            kept: None,
            presentation: Presentation::Nothing,
            presenters: 0,
            idle_drawing: false,
            device_pixel_ratio: 1.0,
        }
    }
}

impl CanvasState {
    /// Records that the whole canvas was drawn, for the windows to show.
    pub(super) fn drawn(&mut self) {
        if self.presenters > 0 {
            self.presentation = Presentation::Whole;
        }
    }

    /// Records a blit of `region`, for the windows to show.
    pub(super) fn blitted(&mut self, region: PixelBox) {
        if self.presenters > 0 && !region.is_empty() {
            self.presentation.add(region);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn many_regions_merge_into_one() {
        let mut presentation = Presentation::default();
        let first = PixelBox::new(0, 0, 2, 2);
        presentation.add(first);
        presentation.add(first);
        assert_eq!(presentation, Presentation::Regions(vec![first]));
        for i in 1..=16 {
            presentation.add(PixelBox::new(i, i, i + 2, i + 2));
        }
        assert_eq!(
            presentation,
            Presentation::Regions(vec![PixelBox::new(0, 0, 18, 18)])
        );
        let mut whole = Presentation::Whole;
        whole.add(first);
        assert_eq!(whole, Presentation::Whole);
    }

    #[test]
    fn generations_never_repeat() {
        assert_ne!(Generation::next(), Generation::next());
    }
}
