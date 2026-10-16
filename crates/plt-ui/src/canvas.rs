//! Drawing into the window framebuffer, clipped to one widget.

use plt_core::{Affine2D, Path, Point, Rgba};
use plt_raster::{Antialias, Clip, ClipBox, FillRule, Painter, Pixmap, draw_image};
use plt_text::TextEngine;

use crate::error::UiError;
use crate::geometry::Rect;
use crate::text::TextLayout;

/// How far the control points of a cubic Bézier quarter circle sit from
/// its ends, relative to the radius.
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// A drawing surface for one widget: the framebuffer, clipped to the
/// widget's rectangle. Coordinates are window pixels.
#[derive(Debug)]
pub struct Canvas<'a> {
    target: &'a mut Pixmap,
    painter: &'a mut Painter,
    clip: Rect,
    scale_factor: f64,
}

impl<'a> Canvas<'a> {
    /// A canvas drawing into `target` within `clip`.
    pub(crate) fn new(
        target: &'a mut Pixmap,
        painter: &'a mut Painter,
        clip: Rect,
        scale_factor: f64,
    ) -> Self {
        Self {
            target,
            painter,
            clip,
            scale_factor,
        }
    }

    /// The window's scale factor, to convert logical lengths.
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Fills a rectangle.
    pub fn fill_rect(&mut self, rect: Rect, color: Rgba) -> Result<(), UiError> {
        let corners = [
            Point::new(rect.x, rect.y),
            Point::new(rect.max_x(), rect.y),
            Point::new(rect.max_x(), rect.max_y()),
            Point::new(rect.x, rect.max_y()),
        ];
        self.fill_path(&Path::polygon(corners), color)
    }

    /// Fills a rectangle with corners rounded to `radius` pixels.
    pub fn fill_rounded_rect(
        &mut self,
        rect: Rect,
        radius: f64,
        color: Rgba,
    ) -> Result<(), UiError> {
        self.fill_path(&rounded_rect(rect, radius), color)
    }

    /// Fills an arbitrary path given in window pixels.
    pub fn fill_path(&mut self, path: &Path, color: Rgba) -> Result<(), UiError> {
        let clip = Clip::rect(self.clip_box());
        let rule = FillRule::NonZero;
        self.painter
            .fill_path(self.target, path, color, rule, Antialias::On, &clip)?;
        Ok(())
    }

    /// Draws an image with its top-left corner at `origin` (rounded to
    /// whole pixels).
    pub fn draw_pixmap(&mut self, image: &Pixmap, origin: Point) -> Result<(), UiError> {
        let clip = Clip::rect(self.clip_box());
        draw_image(
            self.target,
            image,
            round(origin.x),
            round(origin.y),
            1.0,
            &clip,
        )?;
        Ok(())
    }

    /// Draws laid-out text with the top-left corner of its box at
    /// `origin`.
    pub fn draw_text(
        &mut self,
        text: &TextLayout,
        origin: Point,
        color: Rgba,
    ) -> Result<(), UiError> {
        let engine = TextEngine::bundled();
        let run = text.run();
        let baseline = origin.y + text.baseline();
        for glyph in &run.glyphs {
            // Outlines are y-up around the glyph origin.
            let outline = engine.glyph_outline(glyph.font, glyph.glyph, glyph.size, run.dpi)?;
            let place =
                Affine2D::scaling(1.0, -1.0).translate(origin.x + glyph.x, baseline - glyph.y);
            self.fill_path(&outline.transformed(&place), color)?;
        }
        Ok(())
    }

    fn clip_box(&self) -> ClipBox {
        let rect = self.clip;
        ClipBox::new(
            round(rect.x),
            round(rect.y),
            round(rect.max_x()),
            round(rect.max_y()),
        )
    }
}

/// Rounds to the nearest pixel, saturating at the `i32` range.
fn round(value: f64) -> i32 {
    // Float-to-int `as` casts saturate (and map NaN to 0).
    value.round() as i32
}

/// A rectangle with quarter-circle corners of `radius` (limited to half
/// the shorter side).
fn rounded_rect(rect: Rect, radius: f64) -> Path {
    let r = radius.min(rect.width / 2.0).min(rect.height / 2.0).max(0.0);
    let k = r * KAPPA;
    let (x0, y0, x1, y1) = (rect.x, rect.y, rect.max_x(), rect.max_y());
    Path::builder()
        .move_to(Point::new(x0 + r, y0))
        .line_to(Point::new(x1 - r, y0))
        .cubic_to(
            Point::new(x1 - r + k, y0),
            Point::new(x1, y0 + r - k),
            Point::new(x1, y0 + r),
        )
        .line_to(Point::new(x1, y1 - r))
        .cubic_to(
            Point::new(x1, y1 - r + k),
            Point::new(x1 - r + k, y1),
            Point::new(x1 - r, y1),
        )
        .line_to(Point::new(x0 + r, y1))
        .cubic_to(
            Point::new(x0 + r - k, y1),
            Point::new(x0, y1 - r + k),
            Point::new(x0, y1 - r),
        )
        .line_to(Point::new(x0, y0 + r))
        .cubic_to(
            Point::new(x0, y0 + r - k),
            Point::new(x0 + r - k, y0),
            Point::new(x0 + r, y0),
        )
        .close()
        .build()
}

#[cfg(test)]
mod tests {
    use plt_raster::Rgba8;

    use super::*;
    use crate::text::TextStyle;

    const RED: Rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);

    fn pixel(pixmap: &Pixmap, x: u32, y: u32) -> [u8; 4] {
        pixmap.pixel(x, y).unwrap().to_array()
    }

    #[test]
    fn fills_are_clipped_to_the_widget() {
        let mut target = Pixmap::filled(8, 8, Rgba8::new(255, 255, 255, 255)).unwrap();
        let mut painter = Painter::new();
        let clip = Rect::new(0.0, 0.0, 4.0, 8.0);
        let mut canvas = Canvas::new(&mut target, &mut painter, clip, 1.0);
        canvas
            .fill_rect(Rect::new(2.0, 2.0, 4.0, 4.0), RED)
            .unwrap();
        assert_eq!(pixel(&target, 3, 3), [255, 0, 0, 255]);
        assert_eq!(pixel(&target, 4, 3), [255; 4], "outside the clip");
        assert_eq!(pixel(&target, 1, 3), [255; 4], "outside the rectangle");
    }

    #[test]
    fn rounded_corners_are_cut() {
        let mut target = Pixmap::filled(20, 20, Rgba8::new(255, 255, 255, 255)).unwrap();
        let mut painter = Painter::new();
        let mut canvas = Canvas::new(
            &mut target,
            &mut painter,
            Rect::new(0.0, 0.0, 20.0, 20.0),
            1.0,
        );
        canvas
            .fill_rounded_rect(Rect::new(0.0, 0.0, 20.0, 20.0), 8.0, RED)
            .unwrap();
        assert_eq!(pixel(&target, 0, 0), [255; 4]);
        assert_eq!(pixel(&target, 10, 10), [255, 0, 0, 255]);
        assert_eq!(pixel(&target, 10, 0), [255, 0, 0, 255]);
    }

    #[test]
    fn images_and_text_are_drawn() {
        let mut target = Pixmap::filled(60, 20, Rgba8::new(255, 255, 255, 255)).unwrap();
        let mut painter = Painter::new();
        let mut canvas = Canvas::new(
            &mut target,
            &mut painter,
            Rect::new(0.0, 0.0, 60.0, 20.0),
            1.0,
        );
        let image = Pixmap::filled(2, 2, Rgba8::new(0, 0, 255, 255)).unwrap();
        canvas.draw_pixmap(&image, Point::new(57.6, 17.6)).unwrap();
        let text = TextLayout::new("Hi", &TextStyle::new(10.0), 1.0).unwrap();
        canvas
            .draw_text(&text, Point::new(2.0, 2.0), Rgba::new(0.0, 0.0, 0.0, 1.0))
            .unwrap();
        assert_eq!(pixel(&target, 58, 18), [0, 0, 255, 255]);
        let inked = (0..20)
            .flat_map(|y| (0..30).map(move |x| (x, y)))
            .filter(|&(x, y)| pixel(&target, x, y)[0] < 128)
            .count();
        assert!(inked > 10, "text leaves ink ({inked} dark pixels)");
    }
}
