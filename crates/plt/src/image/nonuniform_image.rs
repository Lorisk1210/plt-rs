//! Images whose pixel centres lie on a non-uniform rectilinear grid
//! (matplotlib's `NonUniformImage`).
//!
//! Every pixel of the axes box takes the colour of the nearest grid centre,
//! or a bilinear blend of the four around it; pixels beyond the grid repeat
//! its edge. The centres are stored, and blended, in single precision as
//! matplotlib does.

use std::cell::RefCell;

use plt_core::{Bbox, Point};
use plt_render::RgbaImage;

use super::make::RenderedImage;
use super::pcolor_image::{cell_colors, cell_of};
use super::shared::{axes_image_contains, draw_rendered};
use super::{Extent, ImageData, ImageError};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, LayoutContext, LimitPoints, RawHandle,
};
use crate::colors::ScalarMappable;
use crate::numeric::{Grid, linspace};
use crate::transforms::ResolvedTransform;

/// How a [`NonUniformImage`] colours the pixels between its grid centres.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NonUniformInterpolation {
    /// The colour of the nearest centre along each axis.
    #[default]
    Nearest,
    /// A blend of the four surrounding centres, clamped at the grid edges.
    Bilinear,
}

/// State of an image with pixel centres on a rectilinear grid.
#[derive(Debug)]
pub(crate) struct NonUniformImageArtist {
    base: ArtistBase,
    data: ImageData,
    mappable: RefCell<ScalarMappable>,
    interpolation: NonUniformInterpolation,
    /// Pixel centres along x, rounded to single precision.
    x: Vec<f64>,
    /// Pixel centres along y, rounded to single precision.
    y: Vec<f64>,
}

impl NonUniformImageArtist {
    /// An image with column centres `x` and row centres `y` (matplotlib's
    /// `NonUniformImage.set_data`, which keeps them as `float32`).
    ///
    /// # Errors
    ///
    /// When `data` does not have one row per `y` and one column per `x`.
    pub(crate) fn new(
        data: ImageData,
        mappable: ScalarMappable,
        interpolation: NonUniformInterpolation,
        x: &[f64],
        y: &[f64],
    ) -> Result<Self, ImageError> {
        let grid = (y.len(), x.len());
        if data.shape() != grid {
            return Err(ImageError::ShapeMismatch {
                what: "grid",
                expected: data.shape(),
                actual: grid,
            });
        }
        let single = |values: &[f64]| values.iter().map(|&v| f64::from(v as f32)).collect();
        let mut base = ArtistBase::with_zorder(0.0);
        base.mouseover = true;
        Ok(Self {
            base,
            data,
            mappable: RefCell::new(mappable),
            interpolation,
            x: single(x),
            y: single(y),
        })
    }

    /// matplotlib's `NonUniformImage.make_image`: the axes box at
    /// `magnification` pixels per display unit, each pixel coloured from
    /// the grid at its centre in data coordinates.
    fn render(
        &self,
        ctx: &DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
        let layout = ctx.layout();
        let (Some(axes), Some(transform)) = (self.base.parent, self.base.transform.as_ref()) else {
            return Ok(None);
        };
        let (Some(bbox), Some(to_data)) = (
            layout.axes_extent(axes),
            layout.resolve(transform).and_then(|t| t.inverted()),
        ) else {
            return Ok(None);
        };
        let frame = PixelFrame::new(bbox, magnification);
        if frame.width == 0 || frame.height == 0 || self.x.is_empty() || self.y.is_empty() {
            return Ok(None);
        }
        let (x_centres, y_centres) = frame.data_centres(&to_data);
        let colors = cell_colors(&self.data, &mut self.mappable.borrow_mut())?;
        let data = match self.interpolation {
            NonUniformInterpolation::Nearest => {
                let columns = nearest(&self.x, &x_centres);
                let rows = nearest(&self.y, &y_centres);
                rows.iter()
                    .rev()
                    .flat_map(|&row| columns.iter().map(move |&col| (row, col)))
                    .flat_map(|cell| colors[cell])
                    .collect()
            }
            NonUniformInterpolation::Bilinear => {
                let columns = taps(&self.x, &x_centres);
                let rows = taps(&self.y, &y_centres);
                rows.iter()
                    .rev()
                    .flat_map(|row| columns.iter().map(move |col| (row, col)))
                    .flat_map(|(row, col)| blend(&colors, row, col))
                    .collect()
            }
        };
        Ok(Some(RenderedImage {
            pixels: RgbaImage::new(frame.width, frame.height, data).expect("four bytes per pixel"),
            origin: Point::new(frame.left, frame.bottom),
        }))
    }
}

/// The axes box rounded to whole output pixels, as `make_image` rounds it.
struct PixelFrame {
    left: f64,
    bottom: f64,
    right: f64,
    top: f64,
    width: usize,
    height: usize,
}

impl PixelFrame {
    fn new(bbox: Bbox, magnification: f64) -> Self {
        let round = |v: f64| (v * magnification + 0.5) as i64 as f64 / magnification;
        let (left, bottom, right, top) = (
            round(bbox.x0()),
            round(bbox.y0()),
            round(bbox.x1()),
            round(bbox.y1()),
        );
        Self {
            left,
            bottom,
            right,
            top,
            width: ((right - left) * magnification) as usize,
            height: ((top - bottom) * magnification) as usize,
        }
    }

    /// The data coordinates of the pixel centres along x and y: pixel
    /// edges mapped back through `to_data`, then averaged.
    fn data_centres(&self, to_data: &ResolvedTransform) -> (Vec<f64>, Vec<f64>) {
        let along = |low: f64, high: f64, count: usize, at: &dyn Fn(f64) -> f64| {
            let edges: Vec<f64> = linspace(low, high, count + 1).into_iter().map(at).collect();
            edges
                .windows(2)
                .map(|w| f64::midpoint(w[0], w[1]))
                .collect::<Vec<_>>()
        };
        let x = along(self.left, self.right, self.width, &|x| {
            to_data.transform_point(Point::new(x, self.bottom)).x
        });
        let y = along(self.bottom, self.top, self.height, &|y| {
            to_data.transform_point(Point::new(self.left, y)).y
        });
        (x, y)
    }
}

/// For each pixel centre, the grid centre nearest to it: the number of
/// single-precision midpoints between centres that lie below it.
fn nearest(centres: &[f64], pixels: &[f64]) -> Vec<usize> {
    let midpoints: Vec<f64> = centres
        .windows(2)
        .map(|w| f64::from(f32::midpoint(w[0] as f32, w[1] as f32)))
        .collect();
    pixels
        .iter()
        .map(|&p| midpoints.partition_point(|&m| m < p))
        .collect()
}

/// Where a pixel centre falls between two grid centres.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Tap {
    /// The centre below (or the first one).
    low: usize,
    /// The centre above (or the last one).
    high: usize,
    /// How far from `low` towards `high`, clamped to `[0, 1]`.
    frac: f32,
}

/// The bilinear taps of each pixel centre along one axis, computed in
/// single precision like matplotlib's `make_image`.
fn taps(centres: &[f64], pixels: &[f64]) -> Vec<Tap> {
    let last = centres.len().saturating_sub(1);
    pixels
        .iter()
        .map(|&p| {
            let low = centres
                .partition_point(|&c| c < p)
                .saturating_sub(1)
                .min(last.saturating_sub(1));
            let high = (low + 1).min(last);
            let frac = if low == high {
                0.0
            } else {
                let span = centres[high] as f32 - centres[low] as f32;
                ((p - centres[low]) as f32 / span).clamp(0.0, 1.0)
            };
            Tap { low, high, frac }
        })
        .collect()
}

/// The colour blended from the four cells around `(row, col)`, summed in
/// matplotlib's order and truncated to bytes.
fn blend(colors: &Grid<[u8; 4]>, row: &Tap, col: &Tap) -> [u8; 4] {
    let weights = [
        ((row.low, col.low), (1.0 - row.frac) * (1.0 - col.frac)),
        ((row.high, col.low), row.frac * (1.0 - col.frac)),
        ((row.low, col.high), (1.0 - row.frac) * col.frac),
        ((row.high, col.high), row.frac * col.frac),
    ];
    std::array::from_fn(|channel| {
        weights.iter().fold(0.0, |sum, &(cell, weight)| {
            sum + weight * f32::from(colors[cell][channel])
        }) as u8
    })
}

impl Artist for NonUniformImageArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let magnification = ctx.renderer().image_magnification();
        let rendered = self.render(ctx, magnification);
        draw_rendered(ctx, &self.base, rendered);
    }

    /// The box between the first and last centres (the image's extent).
    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let resolved = ctx.resolve(self.base.transform.as_ref()?)?;
        let [first, last] = self.extent_corners()?;
        Some(Bbox::from_points([
            resolved.transform_point(first),
            resolved.transform_point(last),
        ]))
    }

    /// The first and last centres (matplotlib's `_update_image_limits`
    /// with `get_extent`), which `relim` takes into account.
    fn limit_points(&self, _axes: ArtistId) -> Option<LimitPoints> {
        Some(LimitPoints {
            points: self.extent_corners()?.to_vec(),
            directions: (true, true),
        })
    }

    /// matplotlib's `AxesImage.contains`: inside the axes and the extent.
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        axes_image_contains(&self.base, ctx, point, || self.window_extent(ctx))
    }

    /// matplotlib's `NonUniformImage.get_cursor_data`: the value whose
    /// centre is the last one at or below `at` on each axis.
    fn cursor_text(&self, ctx: &LayoutContext<'_>, at: Point) -> Option<String> {
        let data = ctx
            .resolve(self.base.transform.as_ref()?)?
            .inverted()?
            .transform_point(at);
        let element = (cell_of(&self.y, data.y)?, cell_of(&self.x, data.x)?);
        Some(self.data.cursor_text(element, &self.mappable.borrow()))
    }
}

impl NonUniformImageArtist {
    /// The data points of the first and last centres.
    fn extent_corners(&self) -> Option<[Point; 2]> {
        Some([
            Point::new(*self.x.first()?, *self.y.first()?),
            Point::new(*self.x.last()?, *self.y.last()?),
        ])
    }
}

/// A handle to an image with pixel centres on a non-uniform grid
/// (matplotlib's `NonUniformImage`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonUniformImage {
    raw: RawHandle,
}

impl ArtistHandle for NonUniformImage {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl NonUniformImage {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// The pixel centres `(x, y)`, rounded to single precision.
    pub fn centres(&self) -> (Vec<f64>, Vec<f64>) {
        self.raw
            .with::<NonUniformImageArtist, _>(|image| (image.x.clone(), image.y.clone()))
    }

    /// The rectangle between the first and last centres (matplotlib's
    /// `get_extent`); `None` for an empty image.
    pub fn extent(&self) -> Option<Extent> {
        self.raw.with::<NonUniformImageArtist, _>(|image| {
            let [first, last] = image.extent_corners()?;
            Some(Extent::new(first.x, last.x, first.y, last.y))
        })
    }

    /// How pixels between the centres are coloured.
    pub fn interpolation(&self) -> NonUniformInterpolation {
        self.raw
            .with::<NonUniformImageArtist, _>(|image| image.interpolation)
    }

    /// Changes how pixels between the centres are coloured.
    pub fn set_interpolation(&self, interpolation: NonUniformInterpolation) -> &Self {
        self.raw
            .with_mut::<NonUniformImageArtist, _>(|image| image.interpolation = interpolation);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::colormaps;

    #[test]
    fn pixels_take_the_nearest_centre() {
        let centres = [0.0, 1.0, 4.0];
        assert_eq!(
            nearest(&centres, &[-3.0, 0.4, 0.6, 2.4, 2.6, 9.0]),
            vec![0, 0, 1, 1, 2, 2]
        );
        assert_eq!(nearest(&[2.0], &[-1.0, 5.0]), vec![0, 0], "one centre");
    }

    #[test]
    fn taps_clamp_at_the_grid_edges() {
        let centres = [0.0, 1.0, 3.0];
        let tap = |low, high, frac| Tap { low, high, frac };
        assert_eq!(
            taps(&centres, &[-1.0, 0.5, 2.0, 5.0]),
            vec![
                tap(0, 1, 0.0),
                tap(0, 1, 0.5),
                tap(1, 2, 0.5),
                tap(1, 2, 1.0)
            ]
        );
        assert_eq!(taps(&[2.0], &[7.0]), vec![tap(0, 0, 0.0)], "one centre");
    }

    #[test]
    fn blends_truncate_to_bytes() {
        let colors = Grid::from_vec(1, 2, vec![[0, 0, 0, 255], [255, 255, 255, 255]]).unwrap();
        let row = Tap {
            low: 0,
            high: 0,
            frac: 0.0,
        };
        let col = Tap {
            low: 0,
            high: 1,
            frac: 0.5,
        };
        assert_eq!(blend(&colors, &row, &col), [127, 127, 127, 255]);
    }

    #[test]
    fn the_grid_must_fit_the_data() {
        let data = ImageData::from(Grid::filled(2, 3, 1.0));
        let mappable = ScalarMappable::new(colormaps::get("gray").unwrap());
        let error = NonUniformImageArtist::new(
            data,
            mappable,
            NonUniformInterpolation::Nearest,
            &[0.0, 1.0],
            &[0.0, 1.0],
        )
        .unwrap_err();
        assert_eq!(
            error,
            ImageError::ShapeMismatch {
                what: "grid",
                expected: (2, 3),
                actual: (2, 2)
            }
        );
    }

    #[test]
    fn centres_are_single_precision() {
        let data = ImageData::from(Grid::filled(1, 2, 1.0));
        let mappable = ScalarMappable::new(colormaps::get("gray").unwrap());
        let image = NonUniformImageArtist::new(
            data,
            mappable,
            NonUniformInterpolation::Bilinear,
            &[0.1, 0.2],
            &[0.0],
        )
        .unwrap();
        assert_eq!(image.x, vec![f64::from(0.1_f32), f64::from(0.2_f32)]);
    }
}
