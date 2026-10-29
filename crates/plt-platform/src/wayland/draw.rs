//! Just enough drawing for the built-in title bar and the fallback
//! cursors: anti-aliased polygons over premultiplied RGBA8 pixels.
//!
//! Coverage is exact horizontally and sampled on [`SUBSAMPLES`]
//! sub-scanlines vertically, with the non-zero winding rule.

/// Sub-scanlines per pixel row.
const SUBSAMPLES: usize = 4;

/// A point in pixels.
pub(crate) type Point = (f64, f64);

/// Premultiplied RGBA8 pixels to draw on.
#[derive(Debug)]
pub(crate) struct Canvas<'a> {
    pixels: &'a mut [u8],
    width: usize,
    height: usize,
}

impl<'a> Canvas<'a> {
    /// A canvas over `pixels`, `width` pixels wide; rows that `pixels`
    /// does not hold completely are ignored.
    pub(crate) fn new(pixels: &'a mut [u8], width: u32) -> Self {
        let width = width as usize;
        let height = pixels.len().checked_div(width * 4).unwrap_or(0);
        Self {
            pixels,
            width,
            height,
        }
    }

    /// Fills the rectangle from `(x0, y0)` to `(x1, y1)`.
    pub(crate) fn fill_rect(&mut self, (x0, y0): Point, (x1, y1): Point, color: [u8; 4]) {
        self.fill(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], color);
    }

    /// Strokes the segment from `a` to `b` with square ends.
    pub(crate) fn line(&mut self, a: Point, b: Point, width: f64, color: [u8; 4]) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len <= 0.0 || !len.is_finite() {
            return;
        }
        let (ux, uy) = (dx / len * width / 2.0, dy / len * width / 2.0);
        let quad = [
            (a.0 - ux + uy, a.1 - uy - ux),
            (b.0 + ux + uy, b.1 + uy - ux),
            (b.0 + ux - uy, b.1 + uy + ux),
            (a.0 - ux - uy, a.1 - uy + ux),
        ];
        self.fill(&quad, color);
    }

    /// Fills a closed polygon.
    pub(crate) fn fill(&mut self, polygon: &[Point], color: [u8; 4]) {
        if polygon.len() < 3 || polygon.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
            return;
        }
        let mut coverage = vec![0.0_f64; self.width];
        let mut crossings = Vec::new();
        for y in 0..self.height {
            coverage.fill(0.0);
            for sub in 0..SUBSAMPLES {
                let sample_y = y as f64 + (sub as f64 + 0.5) / SUBSAMPLES as f64;
                crossings.clear();
                for (index, &(x0, y0)) in polygon.iter().enumerate() {
                    let (x1, y1) = polygon[(index + 1) % polygon.len()];
                    if (y0 <= sample_y) != (y1 <= sample_y) {
                        let x = x0 + (sample_y - y0) / (y1 - y0) * (x1 - x0);
                        crossings.push((x, if y1 > y0 { 1 } else { -1 }));
                    }
                }
                crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut winding = 0;
                for pair in crossings.windows(2) {
                    winding += pair[0].1;
                    if winding != 0 {
                        add_span(&mut coverage, pair[0].0, pair[1].0);
                    }
                }
            }
            let row = &mut self.pixels[y * self.width * 4..(y + 1) * self.width * 4];
            for (pixel, &covered) in row.as_chunks_mut::<4>().0.iter_mut().zip(&coverage) {
                let alpha = (covered / SUBSAMPLES as f64).clamp(0.0, 1.0);
                if alpha > 0.0 {
                    blend(pixel, color, alpha);
                }
            }
        }
    }
}

/// Adds the horizontal coverage of the span from `x0` to `x1` (one
/// sub-scanline) to `coverage`.
fn add_span(coverage: &mut [f64], x0: f64, x1: f64) {
    let x0 = x0.max(0.0);
    let x1 = x1.min(coverage.len() as f64);
    if x1 <= x0 {
        return;
    }
    let (first, last) = (
        x0.floor() as usize,
        (x1.ceil() as usize).min(coverage.len()),
    );
    for (index, cell) in coverage.iter_mut().enumerate().take(last).skip(first) {
        let left = (index as f64).max(x0);
        let right = (index as f64 + 1.0).min(x1);
        *cell += (right - left).max(0.0);
    }
}

/// Composites premultiplied `color` at `alpha` coverage over `pixel`.
fn blend(pixel: &mut [u8], color: [u8; 4], alpha: f64) {
    let source_alpha = f64::from(color[3]) / 255.0 * alpha;
    for (channel, &value) in pixel.iter_mut().zip(&color) {
        let source = f64::from(value) * alpha;
        *channel = (source + f64::from(*channel) * (1.0 - source_alpha)).round() as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_cover_whole_and_partial_pixels() {
        let mut pixels = vec![0; 4 * 4 * 4];
        let mut canvas = Canvas::new(&mut pixels, 4);
        canvas.fill_rect((1.0, 1.0), (2.5, 3.0), [255, 0, 0, 255]);
        let px = |x: usize, y: usize| &pixels[(y * 4 + x) * 4..(y * 4 + x) * 4 + 4];
        assert_eq!(px(1, 1), [255, 0, 0, 255]);
        assert_eq!(px(2, 2), [128, 0, 0, 128], "half covered");
        assert_eq!(px(0, 0), [0; 4]);
        assert_eq!(px(1, 3), [0; 4]);
    }

    #[test]
    fn blending_is_source_over() {
        let mut pixels = vec![0, 0, 255, 255];
        Canvas::new(&mut pixels, 1).fill_rect((0.0, 0.0), (1.0, 1.0), [128, 0, 0, 128]);
        assert_eq!(pixels, [128, 0, 127, 255]);
    }

    #[test]
    fn lines_and_degenerate_shapes() {
        let mut pixels = vec![0; 5 * 5 * 4];
        let mut canvas = Canvas::new(&mut pixels, 5);
        canvas.line((0.0, 2.5), (5.0, 2.5), 1.0, [0, 0, 0, 255]);
        canvas.line((1.0, 1.0), (1.0, 1.0), 1.0, [9, 9, 9, 255]);
        canvas.fill(&[(0.0, 0.0), (1.0, 1.0)], [9, 9, 9, 255]);
        canvas.fill(&[(0.0, 0.0), (f64::NAN, 1.0), (2.0, 2.0)], [9, 9, 9, 255]);
        canvas.fill_rect((-10.0, -10.0), (-5.0, -5.0), [9, 9, 9, 255]);
        let row = |y: usize| &pixels[y * 20..y * 20 + 20];
        assert!(row(2).chunks(4).all(|px| px == [0, 0, 0, 255]));
        assert!(row(0).iter().all(|&b| b == 0));
        let mut short = vec![0; 7];
        Canvas::new(&mut short, 1).fill_rect((0.0, 0.0), (1.0, 2.0), [1, 1, 1, 1]);
        assert_eq!(short, [1, 1, 1, 1, 0, 0, 0], "incomplete rows are ignored");
        Canvas::new(&mut [], 0).fill_rect((0.0, 0.0), (1.0, 1.0), [1, 1, 1, 1]);
    }
}
