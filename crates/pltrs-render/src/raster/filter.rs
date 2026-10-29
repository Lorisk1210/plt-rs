//! Filtered artists on a raster canvas (matplotlib's
//! `RendererAgg.start_filter` and `stop_filter`).

use pltrs_core::Point;
use pltrs_raster::{Pixmap, Rgba8};

use super::RasterRenderer;
use crate::filter::{FilterFn, FilterImage, Filtered};
use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::renderer::Renderer;

impl RasterRenderer {
    /// Sets the canvas aside and draws on a fresh transparent one.
    pub(super) fn start_filtering(&mut self) {
        let fresh = Pixmap::filled(
            self.pixmap.width(),
            self.pixmap.height(),
            Rgba8::TRANSPARENT_WHITE,
        )
        .expect("a canvas of the current size can be made again");
        self.set_aside
            .push(std::mem::replace(&mut self.pixmap, fresh));
    }

    /// Takes back the canvas set aside last and draws on it what `filter`
    /// makes of the visible part of the drawing since, placed with a plain
    /// graphics context.
    pub(super) fn stop_filtering(&mut self, filter: &FilterFn<'_>) {
        let Some(below) = self.set_aside.pop() else {
            return;
        };
        let drawn = image_of(std::mem::replace(&mut self.pixmap, below));
        let Some(bounds) = drawn.opaque_bounds() else {
            return;
        };
        let (left, _, _, bottom) = bounds;
        let Filtered { image, offset } =
            filter(&FilterImage::from_rgba(&drawn.crop(bounds)), self.dpi);
        let origin = Point::new(
            left as f64 + offset.0,
            f64::from(self.pixmap.height()) - bottom as f64 + offset.1,
        );
        self.draw_image(&GraphicsContext::default(), origin, &image.to_rgba(), None);
    }
}

/// The pixels of `pixmap` as an image.
pub(super) fn image_of(pixmap: Pixmap) -> RgbaImage {
    let (width, height) = (pixmap.width() as usize, pixmap.height() as usize);
    RgbaImage::new(width, height, pixmap.into_vec())
        .expect("a pixmap holds width × height × 4 bytes")
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use pltrs_core::{Affine2D, Path, Rgba};

    use super::*;

    fn square(canvas: &mut RasterRenderer) {
        let gc = GraphicsContext {
            linewidth: 0.0,
            ..GraphicsContext::default()
        };
        let placed = Affine2D::scaling(3.0, 2.0).translate(2.0, 4.0);
        let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
        canvas.draw_path(&gc, &Path::unit_rectangle(), &placed, Some(red));
    }

    #[test]
    fn filters_replace_the_cropped_drawing() {
        let mut plain = RasterRenderer::new(10.0, 10.0, 72.0).unwrap();
        square(&mut plain);
        let (left, top, right, bottom) = plain.to_image().opaque_bounds().unwrap();

        let mut canvas = RasterRenderer::new(10.0, 10.0, 72.0).unwrap();
        canvas.start_filter();
        square(&mut canvas);
        let blue = [0.0, 0.0, 1.0, 1.0];
        canvas.stop_filter(&|image: &FilterImage, dpi| {
            assert_eq!(
                (image.width(), image.height()),
                (right - left, bottom - top)
            );
            assert_eq!(dpi, 72.0);
            Filtered {
                image: FilterImage::filled(1, 1, blue),
                offset: (0.0, 0.0),
            }
        });
        let image = canvas.to_image();
        assert_eq!(
            image.opaque_bounds(),
            Some((left, bottom - 1, left + 1, bottom)),
            "one pixel at the bottom-left corner of the drawing"
        );
        assert_eq!(image.pixel(left, bottom - 1), Some([0, 0, 255, 255]));
    }

    #[test]
    fn nothing_drawn_is_not_filtered() {
        let mut canvas = RasterRenderer::new(4.0, 4.0, 72.0).unwrap();
        let called = Cell::new(false);
        canvas.start_filter();
        canvas.stop_filter(&|image: &FilterImage, _| {
            called.set(true);
            Filtered {
                image: image.clone(),
                offset: (0.0, 0.0),
            }
        });
        assert!(!called.get());
        canvas.stop_filter(&|_, _| unreachable!("no filter was started"));
    }
}
