//! `FreeType`'s smooth rasterizer (`ftgrays.c` as `FreeType` 2.14.3 builds it
//! with 64-bit integers): the coverage bitmap `FT_Render_Glyph` makes of a
//! glyph outline in `FT_RENDER_MODE_NORMAL`, which matplotlib blits for
//! every glyph it draws.
//!
//! Agg's rasterizer computes the same exact areas but splits slanted
//! lines into cells with different rounding (`FreeType` divides by
//! multiplying with a 32-bit reciprocal), so glyphs are rendered here the
//! way `FreeType` renders them, to the coverage byte.
//!
//! The outline arrives flattened (curves are split as `gray_render_conic`
//! and `gray_render_cubic` split them), in `FreeType`'s 24.8 subpixel
//! coordinates with y up. Outlines whose glyph data asks for overlap
//! handling (`FT_OUTLINE_OVERLAP`) are rendered four times oversampled in
//! each direction and averaged, as `ft_smooth_raster_overlap` does.

use crate::error::RasterError;
use crate::mask::Mask;

/// `PIXEL_BITS`: subpixel bits of the coordinates.
const PIXEL_BITS: u32 = 8;
/// `ONE_PIXEL` in subpixel units.
const ONE_PIXEL: i64 = 1 << PIXEL_BITS;
/// The oversampling of `ft_smooth_raster_overlap`, in each direction.
const OVERLAP_SCALE: i64 = 4;

/// `TRUNC`: the pixel a subpixel coordinate falls in.
fn trunc(v: i64) -> i32 {
    i32::try_from(v >> PIXEL_BITS).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX })
}

/// `FRACT`: the subpixel offset within its pixel.
fn fract(v: i64) -> i32 {
    // The mask keeps the value below `ONE_PIXEL`, which fits.
    i32::try_from(v & (ONE_PIXEL - 1)).expect("a fraction of a pixel")
}

/// `FT_UDIVPREP`: the 32-bit reciprocal of `divisor` (zero when unused).
fn reciprocal(used: bool, divisor: i64) -> i64 {
    if used { 0xFFFF_FFFF / divisor } else { 0 }
}

/// `FT_UDIV`: `dividend / divisor` through the divisor's reciprocal, both
/// non-negative.
fn udiv(dividend: i64, reciprocal: i64) -> i32 {
    let quotient = (dividend
        .cast_unsigned()
        .wrapping_mul(reciprocal.cast_unsigned()))
        >> 32;
    // FreeType truncates to `TCoord`, as `as` does.
    quotient as i32
}

/// A pixel cell: the signed height (`cover`) and doubled area of the
/// outline's edges crossing it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Cell {
    y: i32,
    x: i32,
    cover: i32,
    area: i32,
}

/// The cells of an outline, accumulated as `gray_render_line` does.
#[derive(Debug, Default)]
struct Worker {
    /// Finished cell contributions (several may share a position).
    cells: Vec<Cell>,
    /// The cell edges are integrated into.
    current: Cell,
    /// The last point, in subpixels.
    x: i64,
    y: i64,
}

impl Worker {
    /// `gray_set_cell`: moves to the cell `(ex, ey)`.
    fn set_cell(&mut self, ex: i32, ey: i32) {
        if self.current.cover != 0 || self.current.area != 0 {
            self.cells.push(self.current);
        }
        self.current = Cell {
            y: ey,
            x: ex,
            cover: 0,
            area: 0,
        };
    }

    /// `FT_INTEGRATE`: adds an edge of height `delta` whose horizontal
    /// positions in the cell sum to `twice_x`.
    fn integrate(&mut self, delta: i32, twice_x: i32) {
        self.current.cover = self.current.cover.wrapping_add(delta);
        self.current.area = self.current.area.wrapping_add(delta.wrapping_mul(twice_x));
    }

    /// `gray_move_to`.
    fn move_to(&mut self, x: i64, y: i64) {
        self.set_cell(trunc(x), trunc(y));
        self.x = x;
        self.y = y;
    }

    /// `gray_render_line`, the 64-bit variant.
    fn line_to(&mut self, to_x: i64, to_y: i64) {
        let (mut ex1, ex2) = (trunc(self.x), trunc(to_x));
        let (mut ey1, ey2) = (trunc(self.y), trunc(to_y));
        let (mut fx1, mut fy1) = (fract(self.x), fract(self.y));
        let (dx, dy) = (to_x - self.x, to_y - self.y);
        let one = ONE_PIXEL as i32;
        if ex1 == ex2 && ey1 == ey2 {
            // Inside one cell.
        } else if dy == 0 {
            // Any horizontal line.
            self.set_cell(ex2, ey2);
            self.x = to_x;
            self.y = to_y;
            return;
        } else if dx == 0 {
            if dy > 0 {
                while ey1 != ey2 {
                    self.integrate(one - fy1, fx1 * 2);
                    fy1 = 0;
                    ey1 += 1;
                    self.set_cell(ex1, ey1);
                }
            } else {
                while ey1 != ey2 {
                    self.integrate(-fy1, fx1 * 2);
                    fy1 = one;
                    ey1 -= 1;
                    self.set_cell(ex1, ey1);
                }
            }
        } else {
            let mut prod = dx * i64::from(fy1) - dy * i64::from(fx1);
            let dx_r = reciprocal(ex1 != ex2, dx);
            let dy_r = reciprocal(ey1 != ey2, dy);
            loop {
                if prod - dx * ONE_PIXEL > 0 && prod <= 0 {
                    // Leaves through the left edge.
                    let fy2 = udiv(-prod, -dx_r);
                    prod -= dy * ONE_PIXEL;
                    self.integrate(fy2 - fy1, fx1);
                    fx1 = one;
                    fy1 = fy2;
                    ex1 -= 1;
                } else if prod - dx * ONE_PIXEL + dy * ONE_PIXEL > 0 && prod - dx * ONE_PIXEL <= 0 {
                    // Leaves through the top edge.
                    prod -= dx * ONE_PIXEL;
                    let fx2 = udiv(-prod, dy_r);
                    self.integrate(one - fy1, fx1 + fx2);
                    fx1 = fx2;
                    fy1 = 0;
                    ey1 += 1;
                } else if prod + dy * ONE_PIXEL >= 0 && prod - dx * ONE_PIXEL + dy * ONE_PIXEL <= 0
                {
                    // Leaves through the right edge.
                    prod += dy * ONE_PIXEL;
                    let fy2 = udiv(prod, dx_r);
                    self.integrate(fy2 - fy1, fx1 + one);
                    fx1 = 0;
                    fy1 = fy2;
                    ex1 += 1;
                } else {
                    // Leaves through the bottom edge.
                    let fx2 = udiv(prod, -dy_r);
                    prod += dx * ONE_PIXEL;
                    self.integrate(-fy1, fx1 + fx2);
                    fx1 = fx2;
                    fy1 = one;
                    ey1 -= 1;
                }
                self.set_cell(ex1, ey1);
                if ex1 == ex2 && ey1 == ey2 {
                    break;
                }
            }
        }
        let (fx2, fy2) = (fract(to_x), fract(to_y));
        self.integrate(fy2 - fy1, fx1 + fx2);
        self.x = to_x;
        self.y = to_y;
    }

    /// The finished cells sorted by row and column, contributions to the
    /// same cell summed.
    fn into_cells(mut self) -> Vec<Cell> {
        self.set_cell(0, 0);
        let mut cells = self.cells;
        cells.sort_by_key(|cell| (cell.y, cell.x));
        let mut merged: Vec<Cell> = Vec::with_capacity(cells.len());
        for cell in cells {
            match merged.last_mut() {
                Some(last) if last.y == cell.y && last.x == cell.x => {
                    last.cover = last.cover.wrapping_add(cell.cover);
                    last.area = last.area.wrapping_add(cell.area);
                }
                _ => merged.push(cell),
            }
        }
        merged
    }
}

/// `FT_FILL_RULE` for the non-zero rule: the coverage byte of an
/// accumulated area.
fn coverage(area: i32) -> u8 {
    let mut coverage = area >> (PIXEL_BITS * 2 + 1 - 8);
    if coverage < 0 {
        coverage = !coverage;
    }
    u8::try_from(coverage).unwrap_or(u8::MAX)
}

/// A rendered glyph: its coverage and where its top-left pixel is
/// (`bitmap_left` and `bitmap_top`, pixels with y up).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrayBitmap {
    /// The coverage, rows from the top.
    pub coverage: Mask,
    /// The column of the bitmap's left edge.
    pub left: i32,
    /// The row above the bitmap's top edge (y up).
    pub top: i32,
}

/// The pixel box `[x0, x1) × [y0, y1)` of the contours, as
/// `ft_glyphslot_preset_bitmap` grid-fits their control box.
fn pixel_box(contours: &[Vec<(i64, i64)>]) -> Option<(i32, i32, i32, i32)> {
    let mut points = contours.iter().flatten();
    let &(x, y) = points.next()?;
    let (x0, y0, x1, y1) = points.fold((x, y, x, y), |(x0, y0, x1, y1), &(x, y)| {
        (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
    });
    let ceil = |v: i64| trunc(v + ONE_PIXEL - 1);
    Some((trunc(x0), trunc(y0), ceil(x1), ceil(y1)))
}

/// The cells of the closed `contours` (24.8 subpixels, y up).
fn cells(contours: &[Vec<(i64, i64)>], scale: i64) -> Vec<Cell> {
    let mut worker = Worker::default();
    for contour in contours {
        let Some(&(x, y)) = contour.first() else {
            continue;
        };
        worker.move_to(x * scale, y * scale);
        for &(x, y) in &contour[1..] {
            worker.line_to(x * scale, y * scale);
        }
        worker.line_to(x * scale, y * scale);
    }
    worker.into_cells()
}

/// Calls `write(y, x0, x1, coverage)` for each run of equal coverage of
/// the cells' rows (`gray_sweep`), clipped to columns `[min_x, max_x)` and
/// rows `[min_y, max_y)`.
fn sweep(
    cells: &[Cell],
    (min_x, max_x): (i32, i32),
    (min_y, max_y): (i32, i32),
    mut write: impl FnMut(i32, i32, i32, u8),
) {
    for row in cells.chunk_by(|a, b| a.y == b.y) {
        let y = row[0].y;
        if y < min_y || y >= max_y {
            continue;
        }
        let mut cover: i32 = 0;
        let mut x = min_x;
        for cell in row.iter().filter(|cell| cell.x < max_x) {
            let cell_x = cell.x.max(min_x - 1);
            if cover != 0 && cell_x > x {
                write(y, x, cell_x, coverage(cover));
            }
            cover = cover.wrapping_add(cell.cover.wrapping_mul((ONE_PIXEL * 2) as i32));
            let area = cover.wrapping_sub(cell.area);
            if area != 0 && cell_x >= min_x {
                write(y, cell_x, cell_x + 1, coverage(area));
            }
            x = cell_x + 1;
        }
        if cover != 0 && x < max_x {
            write(y, x, max_x, coverage(cover));
        }
    }
}

/// Renders the closed, flattened `contours` (24.8 subpixels, y up) as
/// `FT_Render_Glyph` renders an outline in `FT_RENDER_MODE_NORMAL`;
/// `overlap` is the outline's `FT_OUTLINE_OVERLAP` flag. `None` for an
/// outline without area.
///
/// # Errors
///
/// When the bitmap would exceed the size a [`Mask`] can hold.
pub fn render_glyph(
    contours: &[Vec<(i64, i64)>],
    overlap: bool,
) -> Result<Option<GrayBitmap>, RasterError> {
    let Some((x0, y0, x1, y1)) = pixel_box(contours) else {
        return Ok(None);
    };
    let (width, height) = (x1 - x0, y1 - y0);
    if width <= 0 || height <= 0 {
        return Ok(None);
    }
    let mut coverage = Mask::new(width.unsigned_abs(), height.unsigned_abs())?;
    let row_width = width.unsigned_abs() as usize;
    let data = coverage.data_mut();
    let mut put = |y: i32, from: i32, to: i32, value: u8, add: bool| {
        let row = (y1 - 1 - y).unsigned_abs() as usize;
        let start = row * row_width + (from - x0).unsigned_abs() as usize;
        let end = row * row_width + (to - x0).unsigned_abs() as usize;
        for byte in &mut data[start..end] {
            if add {
                // `ft_smooth_overlap_spans`: averaged, 256 clamped to 255.
                let sum = u32::from(*byte) + u32::from(value);
                *byte = u8::try_from(sum - (sum >> 8)).unwrap_or(u8::MAX);
            } else {
                *byte = value;
            }
        }
    };
    if overlap {
        let scale = i32::try_from(OVERLAP_SCALE).expect("a small scale");
        let cells = cells(contours, OVERLAP_SCALE);
        let area = OVERLAP_SCALE * OVERLAP_SCALE;
        sweep(
            &cells,
            (x0 * scale, x1 * scale),
            (y0 * scale, y1 * scale),
            |y, from, to, value| {
                let cover = u8::try_from((i64::from(value) + area / 2) / area)
                    .expect("a quarter of a byte");
                for x in from..to {
                    put(
                        y.div_euclid(scale),
                        x.div_euclid(scale),
                        x.div_euclid(scale) + 1,
                        cover,
                        true,
                    );
                }
            },
        );
    } else {
        let cells = cells(contours, 1);
        sweep(&cells, (x0, x1), (y0, y1), |y, from, to, value| {
            put(y, from, to, value, false);
        });
    }
    Ok(Some(GrayBitmap {
        coverage,
        left: x0,
        top: y1,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rectangle `[x0, x1] × [y0, y1]` in pixels, wound like TrueType
    /// outlines (clockwise, y up).
    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(i64, i64)> {
        let s = |v: f64| (v * 256.0) as i64;
        vec![
            (s(x0), s(y0)),
            (s(x0), s(y1)),
            (s(x1), s(y1)),
            (s(x1), s(y0)),
        ]
    }

    fn render(contours: &[Vec<(i64, i64)>], overlap: bool) -> GrayBitmap {
        render_glyph(contours, overlap)
            .expect("small bitmap")
            .expect("an outline with area")
    }

    #[test]
    fn whole_pixels_are_fully_covered() {
        let glyph = render(&[rect(1.0, 2.0, 3.0, 5.0)], false);
        assert_eq!((glyph.left, glyph.top), (1, 5));
        assert_eq!((glyph.coverage.width(), glyph.coverage.height()), (2, 3));
        assert!(glyph.coverage.data().iter().all(|&c| c == 255));
    }

    #[test]
    fn half_pixels_are_half_covered_with_freetypes_rounding() {
        // Area 128 × 256 subpixels: 0x8000 doubled, >> 9 = 128;
        // counter-clockwise outlines have negative areas, which lose one
        // level (`~c`).
        let glyph = render(&[rect(0.0, 0.0, 0.5, 1.0)], false);
        assert_eq!(glyph.coverage.data(), [128]);
        let counter_clockwise: Vec<_> = rect(0.0, 0.0, 0.5, 1.0).into_iter().rev().collect();
        assert_eq!(render(&[counter_clockwise], false).coverage.data(), [127]);
    }

    #[test]
    fn slanted_edges_split_cells_with_reciprocal_division() {
        let triangle = vec![(0, 0), (3 * 256 + 17, 2 * 256 + 101), (5 * 256, 0)];
        let glyph = render(&[triangle], false);
        assert_eq!((glyph.coverage.width(), glyph.coverage.height()), (5, 3));
        let data = glyph.coverage.data();
        assert!(data.iter().any(|&c| c > 0 && c < 255));
        assert!(
            data[..5].iter().all(|&c| c < 255),
            "the apex row is partial"
        );
    }

    #[test]
    fn overlapping_contours_are_averaged_when_flagged() {
        let contours = [rect(0.0, 0.0, 1.0, 1.0), rect(0.0, 0.0, 1.0, 1.0)];
        assert_eq!(render(&contours, true).coverage.data(), [255]);
        assert_eq!(
            render(&[rect(0.0, 0.0, 0.5, 1.0)], true).coverage.data(),
            [128]
        );
    }

    #[test]
    fn empty_and_degenerate_outlines_render_nothing() {
        assert_eq!(render_glyph(&[], false).expect("no bitmap"), None);
        let line = vec![(0, 0), (512, 0)];
        assert_eq!(render_glyph(&[line], false).expect("no bitmap"), None);
    }
}
