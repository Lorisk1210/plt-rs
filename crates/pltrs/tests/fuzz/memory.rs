//! Memory: saves needing canvases from small to far beyond what pltrs
//! allocates (an axes many figures tall saved with a tight box, huge
//! resolutions), to every format. A save may succeed or fail, but a
//! raster canvas larger than `MAX_PIXELS` must fail as running out of
//! memory does (matplotlib's `MemoryError`; a side of `MAX_DIMENSION` or
//! more is matplotlib's `ValueError` instead), never end the process.
//! Canvases between the two are left out, so that the run stays light.

use pltrs::artist::ArtistHandle;
use pltrs::figure::{BboxInches, Figure, SaveFormat, SaveOptions};
use pltrs_core::alloc::caused_by_out_of_memory;
use pltrs_raster::{MAX_DIMENSION, MAX_PIXELS};

use crate::harness::{Gen, fuzz};
use crate::support::FORMATS;

/// The most pixels a canvas of a case may have short of `MAX_PIXELS`.
const LIGHT: f64 = (1 << 22) as f64;

/// A figure with one axes and how it is saved.
#[derive(Debug)]
struct Case {
    size: (f64, f64),
    dpi: f64,
    /// The axes, in figure fractions: possibly many figures wide or tall.
    rect: [f64; 4],
    tight: bool,
    rasterized: bool,
    format: SaveFormat,
}

impl Case {
    /// The sides of the canvas the save draws on, roughly, in pixels:
    /// the figure's, or the axes' for a tight box (its decorations are
    /// small beside it).
    fn canvas_sides(&self) -> (f64, f64) {
        let (width, height) = if self.tight {
            (self.rect[2] * self.size.0, self.rect[3] * self.size.1)
        } else {
            self.size
        };
        (width * self.dpi, height * self.dpi)
    }

    /// The pixels of the canvas, roughly.
    fn canvas_pixels(&self) -> f64 {
        let (width, height) = self.canvas_sides();
        width * height
    }

    /// Whether the canvas has more pixels than pltrs allocates, each side
    /// well within the limit of a side.
    fn beyond_memory(&self) -> bool {
        let (width, height) = self.canvas_sides();
        let side_limit = f64::from(MAX_DIMENSION) / 2.0;
        self.canvas_pixels() >= 4.0 * MAX_PIXELS as f64 && width < side_limit && height < side_limit
    }

    /// Whether the save draws on a raster canvas: a raster format, or a
    /// vector one with rasterized artists.
    fn rasters(&self) -> bool {
        self.rasterized
            || !matches!(
                self.format,
                SaveFormat::Svg
                    | SaveFormat::Svgz
                    | SaveFormat::Pdf
                    | SaveFormat::Ps
                    | SaveFormat::Eps
                    | SaveFormat::Pgf
            )
    }
}

/// A case whose canvas is light or plainly beyond `MAX_PIXELS`.
fn case(g: &mut Gen) -> Case {
    loop {
        let extent = |g: &mut Gen| {
            if g.chance(0.5) {
                g.between(0.1, 1.0)
            } else {
                g.between(1.0, 1000.0)
            }
        };
        let candidate = Case {
            size: (g.between(0.5, 8.0), g.between(0.5, 8.0)),
            dpi: g.pick(&[10.0, 72.0, 100.0, 300.0, 1000.0, 4000.0]),
            rect: [0.1, 0.1, extent(g), extent(g)],
            tight: g.chance(0.7),
            rasterized: g.chance(0.3),
            format: g.pick(&FORMATS),
        };
        let pixels = candidate.canvas_pixels();
        if pixels <= LIGHT || pixels >= 4.0 * MAX_PIXELS as f64 {
            return candidate;
        }
    }
}

/// Saves the case: a canvas beyond `MAX_PIXELS` must fail for lack of
/// memory.
fn save(case: &Case) {
    let fig = Figure::with_size(case.size.0, case.size.1, case.dpi);
    let Ok(ax) = fig.add_axes(case.rect) else {
        return;
    };
    if let Ok(line) = ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.5]) {
        line.set_rasterized(case.rasterized);
    }
    let options = SaveOptions {
        bbox_inches: case.tight.then_some(BboxInches::Tight),
        ..SaveOptions::default()
    };
    let saved = fig.to_format(case.format, options);
    if case.rasters() && case.beyond_memory() {
        let err = saved.expect_err("no canvas so large is made");
        assert!(
            caused_by_out_of_memory(&err),
            "{err} is not for lack of memory"
        );
    }
}

#[test]
fn saves_beyond_memory_fail_as_out_of_memory() {
    fuzz("memory", 60, case, save);
}
