//! The device pixel ratio of a figure's canvas: how many physical pixels
//! of a high-density display stand for one logical pixel (matplotlib's
//! `device_pixel_ratio`).

use super::{CanvasError, FigureCanvas};

impl FigureCanvas {
    /// Physical pixels per logical pixel of the display showing the
    /// canvas (matplotlib's `device_pixel_ratio`): 1 until a window or a
    /// browser page on a high-density display sets another.
    pub fn device_pixel_ratio(&self) -> f64 {
        self.figure.state().canvas.device_pixel_ratio
    }

    /// Shows the canvas on a display of `ratio` physical pixels per
    /// logical pixel (matplotlib's `_set_device_pixel_ratio`): the figure
    /// is drawn at `ratio` times the resolution it was made at, so that it
    /// keeps its size on that display with as many more pixels, while
    /// saving keeps the resolution it was made at. Returns whether the
    /// ratio changed; an unchanged ratio leaves the resolution alone.
    ///
    /// ```
    /// use pltrs::figure::{Figure, SaveOptions};
    /// let fig = Figure::with_size(2.0, 1.0, 100.0);
    /// let canvas = fig.canvas();
    /// assert!(canvas.set_device_pixel_ratio(2.0)?);
    /// assert_eq!(fig.dpi(), 200.0);
    /// assert_eq!(fig.width_height(), (400, 200));
    /// assert!(!canvas.set_device_pixel_ratio(2.0)?);
    /// let saved = fig.render_raster_with(SaveOptions::default())?.to_image();
    /// assert_eq!((saved.width(), saved.height()), (200, 100));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `ratio` is not finite and positive.
    pub fn set_device_pixel_ratio(&self, ratio: f64) -> Result<bool, CanvasError> {
        if !(ratio.is_finite() && ratio > 0.0) {
            return Err(CanvasError::InvalidPixelRatio(ratio));
        }
        let mut state = self.figure.state_mut();
        if ratio.total_cmp(&state.canvas.device_pixel_ratio).is_eq() {
            return Ok(false);
        }
        state.canvas.device_pixel_ratio = ratio;
        let dpi = ratio * state.original_dpi;
        state.set_dpi(dpi);
        state.stale = true;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::{Figure, Presenting, SaveOptions};

    /// The size of the image `fig` saves by default.
    fn saved_size(fig: &Figure) -> (usize, usize) {
        let image = fig
            .render_raster_with(SaveOptions::default())
            .expect("saved")
            .to_image();
        (image.width(), image.height())
    }

    #[test]
    fn saving_ignores_the_device_pixel_ratio() {
        let fig = Figure::with_size(3.0, 2.0, 100.0);
        fig.subplot()
            .plot(vec![0.0, 1.0, 2.0], vec![1.0, 2.0, 3.0])
            .expect("plotted");
        let before = saved_size(&fig);
        let canvas = fig.canvas();
        assert_eq!(canvas.device_pixel_ratio(), 1.0);
        assert!(canvas.set_device_pixel_ratio(2.0).expect("a valid ratio"));
        assert_eq!(canvas.device_pixel_ratio(), 2.0);
        assert_eq!(fig.width_height(), (600, 400));
        assert_eq!(saved_size(&fig), before);
        assert_eq!(fig.dpi(), 200.0, "saving restores the resolution");
    }

    #[test]
    fn ratios_scale_the_resolution_the_figure_was_made_at() {
        let fig = Figure::with_size(3.0, 2.0, 100.0);
        let canvas = fig.canvas();
        fig.set_dpi(50.0);
        assert!(!canvas.set_device_pixel_ratio(1.0).expect("a valid ratio"));
        assert_eq!(fig.dpi(), 50.0, "an unchanged ratio keeps the resolution");
        assert!(canvas.set_device_pixel_ratio(1.25).expect("a valid ratio"));
        assert_eq!(canvas.device_pixel_ratio(), 1.25);
        assert_eq!(fig.dpi(), 125.0);
        // Showing the figure again does not scale it twice.
        assert!(canvas.set_device_pixel_ratio(2.0).expect("a valid ratio"));
        assert!(!canvas.set_device_pixel_ratio(2.0).expect("a valid ratio"));
        assert_eq!(fig.dpi(), 200.0);
        assert_eq!(saved_size(&fig), (300, 200));
    }

    #[test]
    fn the_last_presenter_gone_the_figure_is_unscaled() {
        let fig = Figure::with_size(3.0, 2.0, 100.0);
        let shown = Presenting::new(&fig);
        let also_shown = Presenting::new(&fig);
        fig.canvas()
            .set_device_pixel_ratio(2.0)
            .expect("a valid ratio");
        drop(shown);
        assert_eq!(fig.dpi(), 200.0, "still shown");
        drop(also_shown);
        assert_eq!(fig.canvas().device_pixel_ratio(), 1.0);
        assert_eq!(fig.dpi(), 100.0);
    }

    #[test]
    fn degenerate_ratios_are_rejected() {
        let fig = Figure::with_size(3.0, 2.0, 100.0);
        let canvas = fig.canvas();
        for ratio in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(canvas.set_device_pixel_ratio(ratio).is_err(), "{ratio}");
        }
        assert_eq!(canvas.device_pixel_ratio(), 1.0);
        assert_eq!(fig.dpi(), 100.0);
    }
}
