//! Full-colour vector icons: shapes filled with solid colours or linear
//! gradients, outlined, and casting blurred drop shadows, drawn at any
//! pixel size from their vectors (application icons).

use pltrs_core::{Affine2D, Path, Point, Rgba};
use pltrs_raster::{
    Antialias, Clip, FillRule, Painter, Pixmap, Rgba8, StrokeStyle, draw_coverage, draw_image,
};

use crate::error::UiError;
use crate::geometry::Rect;

/// A linear gradient across the design space, as SVG's `linearGradient`
/// with `gradientUnits="userSpaceOnUse"`: the colour changes along the
/// line from `start` to `end` and is constant across it, padded with the
/// end colours beyond the ends.
///
/// ```
/// use pltrs_core::{Point, Rgba};
/// use pltrs_ui::LinearGradient;
/// let fade = LinearGradient::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0))
///     .with_stop(0.0, Rgba::BLACK)
///     .with_stop(1.0, Rgba::WHITE);
/// assert_eq!(fade.color_at(Point::new(5.0, 3.0)).r, 0.5);
/// assert_eq!(fade.color_at(Point::new(-5.0, 0.0)), Rgba::BLACK);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LinearGradient {
    start: Point,
    end: Point,
    /// Offsets along the gradient (0 at `start`, 1 at `end`), never
    /// decreasing, with their colours.
    stops: Vec<(f64, Rgba)>,
}

impl LinearGradient {
    /// A gradient from `start` to `end` without stops yet (a gradient
    /// without stops paints nothing).
    pub fn new(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
            stops: Vec::new(),
        }
    }

    /// Adds a stop of `color` at `offset` along the gradient, clamped to
    /// `[0, 1]` and to no less than the previous stop's, as SVG does.
    #[must_use]
    pub fn with_stop(mut self, offset: f64, color: Rgba) -> Self {
        let floor = self.stops.last().map_or(0.0, |&(last, _)| last);
        let offset = if offset.is_nan() { floor } else { offset };
        self.stops.push((offset.clamp(0.0, 1.0).max(floor), color));
        self
    }

    /// The colour at `point`, interpolated between the stops around its
    /// projection on the gradient's line (with premultiplied alpha). A
    /// gradient whose ends coincide takes its last stop's colour.
    pub fn color_at(&self, point: Point) -> Rgba {
        let (dx, dy) = (self.end.x - self.start.x, self.end.y - self.start.y);
        let length = dx * dx + dy * dy;
        let t = if length > 0.0 {
            ((point.x - self.start.x) * dx + (point.y - self.start.y) * dy) / length
        } else {
            1.0
        };
        self.color_at_offset(t)
    }

    fn color_at_offset(&self, t: f64) -> Rgba {
        let Some(&(_, first)) = self.stops.first() else {
            return Rgba::TRANSPARENT;
        };
        let after = self.stops.iter().position(|&(offset, _)| offset > t);
        match after {
            Some(0) => first,
            None => self.stops.last().map_or(first, |&(_, color)| color),
            Some(index) => {
                let (from, a) = self.stops[index - 1];
                let (to, b) = self.stops[index];
                mix(a, b, (t - from) / (to - from))
            }
        }
    }

    /// The same gradient in the coordinates `transform` maps to.
    fn transformed(&self, transform: &Affine2D) -> Self {
        Self {
            start: transform.transform_point(self.start),
            end: transform.transform_point(self.end),
            stops: self.stops.clone(),
        }
    }
}

/// `a` and `b` mixed with weight `t` on `b`, in premultiplied alpha.
fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let alpha = a.a + (b.a - a.a) * t;
    if alpha <= 0.0 {
        return Rgba::TRANSPARENT;
    }
    let channel = |x: f64, y: f64| (x * a.a + (y * b.a - x * a.a) * t) / alpha;
    Rgba::new(
        channel(a.r, b.r),
        channel(a.g, b.g),
        channel(a.b, b.b),
        alpha,
    )
}

/// How a shape is filled.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    /// One colour.
    Solid(Rgba),
    /// A linear gradient.
    Linear(LinearGradient),
}

impl From<Rgba> for Paint {
    fn from(color: Rgba) -> Self {
        Self::Solid(color)
    }
}

impl From<LinearGradient> for Paint {
    fn from(gradient: LinearGradient) -> Self {
        Self::Linear(gradient)
    }
}

/// A drop shadow, as SVG's `feGaussianBlur` of a shape's alpha followed
/// by `feOffset` and a fade composes it: the shape's coverage blurred,
/// moved and painted in one colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    /// The blur's standard deviation, in design units.
    pub blur: f64,
    /// How far the shadow falls from the shape, in design units (y down).
    pub offset: Point,
    /// The shadow's colour where the shape covers fully (its alpha is the
    /// shadow's opacity).
    pub color: Rgba,
}

/// One drawing operation of a [`ColorIcon`].
#[derive(Debug, Clone, PartialEq)]
enum Layer {
    Fill(Path, Paint),
    Stroke(Path, f64, Rgba),
    Shadow(Path, Shadow),
}

/// A full-colour vector icon: layers of filled, outlined and shadowed
/// shapes in a design space (y down), drawn in the order they were added
/// and rendered at any size straight from the vectors, never by scaling a
/// bitmap.
///
/// ```
/// use pltrs_core::{Path, Point, Rgba};
/// use pltrs_ui::{ColorIcon, Rect};
/// let square = Path::from_svg("M2 2H14V14H2Z")?;
/// let icon = ColorIcon::new().fill(square, Rgba::opaque(0.2, 0.1, 0.4));
/// let rgba = icon.render(32, Rect::new(0.0, 0.0, 16.0, 16.0))?;
/// assert_eq!(rgba.len(), 32 * 32 * 4);
/// assert_eq!(rgba[3], 0, "transparent outside the square");
/// assert_eq!(&rgba[(16 * 32 + 16) * 4..][..4], [51, 26, 102, 255]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColorIcon {
    layers: Vec<Layer>,
}

impl ColorIcon {
    /// An icon without layers (rendering transparent squares).
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `path` filled with `paint` (non-zero winding).
    #[must_use]
    pub fn fill(mut self, path: Path, paint: impl Into<Paint>) -> Self {
        self.layers.push(Layer::Fill(path, paint.into()));
        self
    }

    /// Adds `path` outlined in `color` with a line `width` design units
    /// wide, centred on the path, with SVG's default butt caps and miter
    /// joins.
    #[must_use]
    pub fn stroke(mut self, path: Path, width: f64, color: Rgba) -> Self {
        self.layers.push(Layer::Stroke(path, width, color));
        self
    }

    /// Adds the drop shadow `path` casts (the path itself is not drawn).
    #[must_use]
    pub fn shadow(mut self, path: Path, shadow: Shadow) -> Self {
        self.layers.push(Layer::Shadow(path, shadow));
        self
    }

    /// Draws the square `view` of the design space (or the largest square
    /// centred in it) onto a transparent `size`-pixel square, as straight
    /// RGBA bytes, row by row. Lengths (line widths, blurs, offsets)
    /// scale with the view.
    ///
    /// # Errors
    ///
    /// When `size` is too large for an image.
    pub fn render(&self, size: u32, view: Rect) -> Result<Vec<u8>, UiError> {
        let mut pixmap = Pixmap::new(size, size)?;
        let side = view.width.min(view.height);
        if size == 0 || side.is_nan() || side <= 0.0 {
            return Ok(pixmap.into_vec());
        }
        let scale = f64::from(size) / side;
        let center = view.center();
        let place =
            Affine2D::translation(side / 2.0 - center.x, side / 2.0 - center.y).scale(scale, scale);
        let mut painter = Painter::new();
        for layer in &self.layers {
            match layer {
                Layer::Fill(path, paint) => {
                    fill(
                        &mut painter,
                        &mut pixmap,
                        &path.transformed(&place),
                        paint,
                        &place,
                    )?;
                }
                Layer::Stroke(path, width, color) => painter.stroke_path(
                    &mut pixmap,
                    &path.transformed(&place),
                    *color,
                    &StrokeStyle::new(width * scale),
                    Antialias::On,
                    &Clip::NONE,
                )?,
                Layer::Shadow(path, shadow) => {
                    let cast = Affine2D::translation(shadow.offset.x, shadow.offset.y).then(place);
                    let coverage = painter
                        .coverage_mask(
                            &path.transformed(&cast),
                            size,
                            size,
                            FillRule::NonZero,
                            Antialias::On,
                            None,
                        )?
                        .blurred(shadow.blur * scale);
                    draw_coverage(&mut pixmap, &coverage, 0, 0, shadow.color, &Clip::NONE)?;
                }
            }
        }
        Ok(pixmap.into_vec())
    }
}

/// Fills `path` (in pixels) of `target` with `paint` (whose gradients
/// `place` maps to pixels).
fn fill(
    painter: &mut Painter,
    target: &mut Pixmap,
    path: &Path,
    paint: &Paint,
    place: &Affine2D,
) -> Result<(), UiError> {
    match paint {
        Paint::Solid(color) => painter.fill_path(
            target,
            path,
            *color,
            FillRule::NonZero,
            Antialias::On,
            &Clip::NONE,
        )?,
        Paint::Linear(gradient) => {
            let (width, height) = (target.width(), target.height());
            let coverage = painter.coverage_mask(
                path,
                width,
                height,
                FillRule::NonZero,
                Antialias::On,
                None,
            )?;
            let gradient = gradient.transformed(place);
            let mut colors = Pixmap::new(width, height)?;
            for y in 0..height {
                for x in 0..width {
                    let center = Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
                    colors.set_pixel(x, y, Rgba8::from(gradient.color_at(center)));
                }
            }
            draw_image(target, &colors, 0, 0, 1.0, &Clip::NONE.with_mask(&coverage))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Rect = Rect::new(0.0, 0.0, 16.0, 16.0);

    fn pixel(rgba: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * size + x) * 4) as usize;
        rgba[at..at + 4].try_into().unwrap()
    }

    fn square(from: f64, to: f64) -> Path {
        Path::from_svg(&format!("M{from} {from}H{to}V{to}H{from}Z")).unwrap()
    }

    #[test]
    fn gradients_interpolate_and_pad() {
        let gradient = LinearGradient::new(Point::new(0.0, 0.0), Point::new(0.0, 10.0))
            .with_stop(0.2, Rgba::opaque(1.0, 0.0, 0.0))
            .with_stop(0.1, Rgba::opaque(0.0, 0.0, 1.0))
            .with_stop(0.8, Rgba::new(0.0, 1.0, 0.0, 0.0));
        assert_eq!(gradient.color_at(Point::new(3.0, 0.0)).r, 1.0, "padded");
        // The second stop moved up to the first: a sharp change there.
        assert_eq!(gradient.color_at(Point::new(0.0, 2.0)).b, 1.0);
        // Towards a transparent stop the colour stays blue in
        // premultiplied interpolation.
        let middle = gradient.color_at(Point::new(9.0, 5.0));
        assert!((middle.a - 0.5).abs() < 1e-12);
        assert_eq!((middle.r, middle.b), (0.0, 1.0));
        assert_eq!(gradient.color_at(Point::new(0.0, 20.0)).a, 0.0);
        let point = LinearGradient::new(Point::ORIGIN, Point::ORIGIN)
            .with_stop(0.0, Rgba::BLACK)
            .with_stop(f64::NAN, Rgba::WHITE);
        assert_eq!(point.color_at(Point::new(4.0, 4.0)), Rgba::WHITE);
        let empty = LinearGradient::new(Point::ORIGIN, Point::new(1.0, 0.0));
        assert_eq!(empty.color_at(Point::ORIGIN), Rgba::TRANSPARENT);
    }

    #[test]
    fn gradient_fills_take_their_end_colours() {
        let top = Rgba::from_rgba8([0x46, 0x32, 0x7e, 255]);
        let bottom = Rgba::from_rgba8([0x3a, 0x0a, 0x55, 255]);
        let gradient = LinearGradient::new(Point::new(0.0, 16.0), Point::new(0.0, 0.0))
            .with_stop(0.0, bottom)
            .with_stop(1.0, top);
        let icon = ColorIcon::new().fill(square(0.0, 16.0), gradient);
        let rgba = icon.render(64, VIEW).unwrap();
        let near = |a: [u8; 4], b: [u8; 4]| a.iter().zip(b).all(|(&x, y)| x.abs_diff(y) <= 1);
        assert!(near(pixel(&rgba, 64, 10, 0), top.to_rgba8()));
        assert!(near(pixel(&rgba, 64, 10, 63), bottom.to_rgba8()));
        assert_eq!(pixel(&rgba, 64, 0, 32)[3], 255, "opaque");
    }

    #[test]
    fn views_choose_what_fills_the_square() {
        let icon = ColorIcon::new().fill(square(4.0, 12.0), Rgba::BLACK);
        let whole = icon.render(16, VIEW).unwrap();
        assert_eq!(pixel(&whole, 16, 2, 8)[3], 0, "margin");
        assert_eq!(pixel(&whole, 16, 4, 8)[3], 255);
        let tight = icon.render(16, Rect::new(4.0, 4.0, 8.0, 8.0)).unwrap();
        assert!(
            tight.as_chunks::<4>().0.iter().all(|p| p[3] == 255),
            "full bleed"
        );
        // A wide view shows the square centred in it.
        let wide = icon.render(16, Rect::new(-4.0, 0.0, 24.0, 16.0)).unwrap();
        assert_eq!(wide, whole);
        assert_eq!(icon.render(0, VIEW).unwrap(), []);
        let nothing = icon.render(4, Rect::new(0.0, 0.0, 0.0, 8.0)).unwrap();
        assert!(nothing.iter().all(|&b| b == 0));
    }

    #[test]
    fn shadows_fall_below_their_shapes() {
        let body = square(4.0, 12.0);
        let shadow = Shadow {
            blur: 1.0,
            offset: Point::new(0.0, 2.0),
            color: Rgba::new(0.0, 0.0, 0.0, 0.5),
        };
        let icon = ColorIcon::new()
            .shadow(body.clone(), shadow)
            .fill(body, Rgba::WHITE);
        let rgba = icon.render(64, VIEW).unwrap();
        let alpha = |y| pixel(&rgba, 64, 32, y)[3];
        assert_eq!(alpha(32), 255, "the body covers its shadow");
        assert!(alpha(50) > 60, "below the body");
        assert!(alpha(10) < alpha(50) / 4, "hardly above it");
        assert_eq!(alpha(0), 0);
        assert_eq!(pixel(&rgba, 64, 32, 50)[..3], [0, 0, 0], "black");
    }

    #[test]
    fn strokes_straddle_their_paths() {
        let icon = ColorIcon::new().stroke(square(4.0, 12.0), 2.0, Rgba::WHITE);
        let rgba = icon.render(16, VIEW).unwrap();
        assert_eq!(pixel(&rgba, 16, 3, 8)[3], 255);
        assert_eq!(pixel(&rgba, 16, 4, 8)[3], 255);
        assert_eq!(pixel(&rgba, 16, 8, 8)[3], 0, "hollow");
        assert_eq!(pixel(&rgba, 16, 1, 8)[3], 0);
    }
}
