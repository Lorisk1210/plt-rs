//! Glyphs without anti-aliasing, as `FreeType`'s monochrome rasterizer
//! (`ftraster.c`) renders them for matplotlib's `text.antialiased: False`.
//!
//! The glyph outline is transformed as `FT_Load_Glyph` transforms it, its
//! bitmap box is rounded as `ft_glyphslot_preset_bitmap` rounds it for
//! monochrome bitmaps, and the outline is scan-converted in two passes:
//! rows set the pixels whose centres lie inside, columns add the edges and
//! drop-outs the rows missed. Thin features keep a pixel per scanline as
//! the glyph's drop-out rules ask.

mod profiles;
mod sweep;

use plt_core::{Path, Point};
use plt_font::{ScanOutline, ScanPoint};

use self::profiles::{Converter, Precision};
use self::sweep::{Pass, draw_sweep};
use super::glyph::GlyphTransform;

/// Why the rasterizer gave up on a glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RasterError {
    /// A band does not fit the render pool (`Raster_Overflow`).
    Overflow,
    /// The outline is malformed (`Invalid_Outline`).
    InvalidOutline,
}

/// A monochrome glyph bitmap placed in display pixels (y up).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MonoBitmap {
    /// The display column of the leftmost pixels (`bitmap_left`).
    left: i64,
    /// The display row above the topmost pixels (`bitmap_top`).
    top: i64,
    width: usize,
    rows: usize,
    /// Row-major from the top row.
    pixels: Vec<bool>,
}

impl MonoBitmap {
    fn new(left: i64, top: i64, width: usize, rows: usize) -> Self {
        Self {
            left,
            top,
            width,
            rows,
            pixels: vec![false; width * rows],
        }
    }

    /// The rightmost column (`bRight`).
    fn right(&self) -> i64 {
        self.width as i64 - 1
    }

    /// The topmost row counted from the bottom (`bTop`).
    fn top(&self) -> i64 {
        self.rows as i64 - 1
    }

    /// The index of pixel `x` in row `row` (from the bottom).
    fn index(&self, x: i64, row: i64) -> Option<usize> {
        let x = usize::try_from(x).ok().filter(|&x| x < self.width)?;
        let row = usize::try_from(row).ok().filter(|&row| row < self.rows)?;
        Some((self.rows - 1 - row) * self.width + x)
    }

    fn get(&self, x: i64, row: i64) -> bool {
        self.index(x, row).is_some_and(|i| self.pixels[i])
    }

    fn set(&mut self, x: i64, row: i64) {
        if let Some(i) = self.index(x, row) {
            self.pixels[i] = true;
        }
    }

    /// The set pixels as pixel-aligned rectangles in display pixels, one
    /// per run of a row.
    pub(super) fn to_path(&self) -> Path {
        let mut builder = Path::builder();
        for (r, row) in self.pixels.chunks(self.width.max(1)).enumerate() {
            let y1 = (self.top - r as i64) as f64;
            let y0 = y1 - 1.0;
            let mut x = 0;
            while x < row.len() {
                if !row[x] {
                    x += 1;
                    continue;
                }
                let run = row[x..].iter().take_while(|&&set| set).count();
                let (x0, x1) = (
                    (self.left + x as i64) as f64,
                    (self.left + (x + run) as i64) as f64,
                );
                builder = builder
                    .move_to(Point::new(x0, y0))
                    .line_to(Point::new(x1, y0))
                    .line_to(Point::new(x1, y1))
                    .line_to(Point::new(x0, y1))
                    .close();
                x += run;
            }
        }
        builder.build()
    }
}

/// The monochrome bitmap of `outline` placed by `transform`, or `None`
/// when `FreeType` renders nothing (an empty outline or box) or fails.
pub(super) fn render(outline: &ScanOutline, transform: &GlyphTransform) -> Option<MonoBitmap> {
    if outline.contour_ends.last() != Some(&outline.points.len()) {
        return None;
    }
    let mut points: Vec<ScanPoint> = outline
        .points
        .iter()
        .map(|point| {
            let (x, y) = transform.transform(point.x, point.y);
            ScanPoint { x, y, ..*point }
        })
        .collect();
    let (left, right) = pixel_extent(points.iter().map(|p| p.x))?;
    let (bottom, top) = pixel_extent(points.iter().map(|p| p.y))?;
    let width = usize::try_from(right - left).ok()?;
    let rows = usize::try_from(top - bottom).ok()?;
    if width >= 0x10000 || rows >= 0x10000 || width == 0 || rows == 0 {
        return None;
    }
    for point in &mut points {
        point.x -= left * 64;
        point.y -= bottom * 64;
    }
    let mut bitmap = MonoBitmap::new(left, top, width, rows);
    let rules = outline.rules;
    let precision = Precision::new(rules.high_precision && bitmap.top() + bitmap.right() < 256);
    let mut converter = Converter::new(precision, rules.dropouts.control_bits());
    for (pass, last) in [
        (Pass::Vertical, bitmap.top()),
        (Pass::Horizontal, bitmap.right()),
    ] {
        render_pass(
            &mut converter,
            &points,
            &outline.contour_ends,
            pass,
            last,
            &mut bitmap,
        )
        .ok()?;
    }
    Some(bitmap)
}

/// `Render_Single_Pass` over scanlines `0..=last`: bands that overflow
/// the render pool are bisected, the upper half first.
fn render_pass(
    converter: &mut Converter,
    points: &[ScanPoint],
    contour_ends: &[usize],
    pass: Pass,
    last: i64,
    bitmap: &mut MonoBitmap,
) -> Result<(), RasterError> {
    let flipped = pass == Pass::Horizontal;
    let mut pending = Vec::new();
    let (mut low, mut high) = (0, last);
    loop {
        match converter.convert(points, contour_ends, (low, high), flipped) {
            Err(RasterError::Overflow) if low < high => {
                pending.push(low);
                low = ((low + high) >> 1) + 1;
            }
            Err(error) => return Err(error),
            Ok(mut profiles) => {
                if !profiles.list.is_empty() {
                    draw_sweep(&mut profiles, converter.precision(), pass, bitmap);
                }
                let Some(below) = pending.pop() else {
                    return Ok(());
                };
                high = low - 1;
                low = below;
            }
        }
    }
}

/// The pixel range a monochrome bitmap spans along one axis for 26.6
/// coordinates `values` (`ft_glyphslot_preset_bitmap`): rounded so that a
/// pixel centre is always included, and at least one pixel wide.
fn pixel_extent(values: impl Iterator<Item = i64>) -> Option<(i64, i64)> {
    let (min, max) = values.fold(None, |range: Option<(i64, i64)>, v| {
        Some(range.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))))
    })?;
    let (min_frac, max_frac) = (min & 63, max & 63);
    let mut low = (min >> 6) + ((min_frac + 31) >> 6);
    let mut high = (max >> 6) + ((max_frac + 32) >> 6);
    if low == high {
        if ((min_frac + 31) & 63) - 31 + ((max_frac + 32) & 63) - 32 < 0 {
            low -= 1;
        } else {
            high += 1;
        }
    }
    Some((low, high))
}

#[cfg(test)]
mod tests {
    use plt_font::{Dropouts, ScanPointKind, ScanRules};

    use super::*;

    fn outline(points: &[(i64, i64)], rules: ScanRules) -> ScanOutline {
        ScanOutline {
            points: points
                .iter()
                .map(|&(x, y)| ScanPoint {
                    x,
                    y,
                    kind: ScanPointKind::On,
                })
                .collect(),
            contour_ends: vec![points.len()],
            rules,
        }
    }

    const SIMPLE: ScanRules = ScanRules {
        high_precision: false,
        dropouts: Dropouts::Simple { stubs: false },
    };

    fn identity() -> GlyphTransform {
        GlyphTransform::new(0.0, Point::ORIGIN)
    }

    fn rows(bitmap: &MonoBitmap) -> Vec<String> {
        bitmap
            .pixels
            .chunks(bitmap.width)
            .map(|row| row.iter().map(|&set| if set { '#' } else { '.' }).collect())
            .collect()
    }

    #[test]
    fn bitmap_boxes_include_pixel_centres() {
        assert_eq!(pixel_extent([64, 192].into_iter()), Some((1, 3)));
        // 0.4 to 1.4 pixels: the centre at 0.5 rounds in, 1.5 out.
        assert_eq!(pixel_extent([26, 90].into_iter()), Some((0, 1)));
        // A collapsed box grows by one pixel.
        assert_eq!(pixel_extent([10, 20].into_iter()), Some((0, 1)));
        assert_eq!(pixel_extent([-20, -10].into_iter()), Some((-1, 0)));
        assert_eq!(pixel_extent(std::iter::empty()), None);
    }

    #[test]
    fn squares_fill_the_pixels_they_cover() {
        let square = outline(&[(64, 64), (256, 64), (256, 192), (64, 192)], SIMPLE);
        let bitmap = render(&square, &identity()).unwrap();
        assert_eq!((bitmap.left, bitmap.top), (1, 3));
        assert_eq!(rows(&bitmap), vec!["###", "###"]);
    }

    #[test]
    fn thin_stems_keep_a_pixel_per_row_as_their_rules_ask() {
        // A stem 0.2 pixels wide between two pixel centres.
        let stem = [(70, 0), (83, 0), (83, 192), (70, 192)];
        let rendered = |dropouts| {
            let rules = ScanRules { dropouts, ..SIMPLE };
            rows(&render(&outline(&stem, rules), &identity()).unwrap())
        };
        assert_eq!(rendered(Dropouts::Simple { stubs: true }), ["#", "#", "#"]);
        // The ends are stubs: thinner than half a pixel.
        assert_eq!(rendered(Dropouts::Simple { stubs: false }), [".", "#", "."]);
        assert_eq!(rendered(Dropouts::Ignore), [".", ".", "."]);
    }

    #[test]
    fn runs_become_rectangles() {
        let square = outline(&[(0, 0), (128, 0), (128, 64), (0, 64)], SIMPLE);
        let path = render(&square, &identity()).unwrap().to_path();
        let bbox = path.extents();
        assert_eq!(
            (bbox.xmin(), bbox.ymin(), bbox.xmax(), bbox.ymax()),
            (0.0, 0.0, 2.0, 1.0)
        );
    }
}
