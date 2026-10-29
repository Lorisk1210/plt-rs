//! Figure windows drawn off screen: what a window shows, captured without
//! a display.

use std::time::Duration;

use pltrs_platform::{Event, EventLoop, HeadlessHandle, WindowId};

use super::FigureWindow;
use crate::figure::Figure;
use crate::gui::GuiError;
use crate::image::RgbaImage;

/// The title of a window opened for a screenshot, which shows nowhere.
const TITLE: &str = "Figure 1";

/// How many times a window opened off screen handles the events queued
/// before it is captured: those that opening it queued, then the redraws
/// they ask for.
const SETTLE_ROUNDS: usize = 3;

impl FigureWindow {
    /// What a window showing `figure` looks like when it opens: the
    /// figure above the toolbar, in the chrome of the current
    /// [look](crate::look), drawn off screen at `scale_factor` physical
    /// pixels per logical pixel (2 for a high-density display). No
    /// display is needed, so this works on a server, in a test or in CI.
    /// The image is as wide as the figure at `scale_factor` and taller by
    /// the toolbar.
    ///
    /// The figure is drawn as a window draws it: its callbacks see the
    /// window's size and its timers may tick. It gets back its size and
    /// resolution afterwards.
    ///
    /// ```
    /// use pltrs::gui::FigureWindow;
    /// use pltrs::image::{ImsaveOptions, imsave_png};
    /// use pltrs::prelude::*;
    ///
    /// let fig = Figure::with_size(4.0, 3.0, 100.0);
    /// fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
    /// let window = FigureWindow::screenshot(&fig, 1.0)?;
    /// assert_eq!(window.width(), 400);
    /// assert!(window.height() > 300, "the toolbar is below the figure");
    /// let png = imsave_png(window, ImsaveOptions::default())?; // or `imsave(path, ..)`
    /// assert!(png.starts_with(b"\x89PNG"));
    ///
    /// let sharp = FigureWindow::screenshot(&fig, 2.0)?;
    /// assert_eq!(sharp.width(), 800);
    /// assert_eq!(fig.width_height(), (400, 300), "the figure is as it was");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `scale_factor` is not finite and positive, or the window
    /// cannot be drawn.
    pub fn screenshot(figure: &Figure, scale_factor: f64) -> Result<RgbaImage, GuiError> {
        let (mut event_loop, handle) = EventLoop::headless();
        handle.set_scale_factor(scale_factor)?;
        let size = figure.size_inches();
        let mut window = Self::open(&mut event_loop.context(), figure, TITLE)?;
        let shown = window
            .settle(&mut event_loop)
            .and_then(|()| Self::capture(&handle, window.id()));
        window.close(&mut event_loop.context())?;
        // The window fitted the figure to whole pixels; closing it gave the
        // figure back its resolution.
        if figure.size_inches() != size {
            let mut state = figure.state_mut();
            state.size_inches = size;
            state.stale = true;
        }
        shown
    }

    /// What `window` shows in a headless event loop (one made by
    /// `EventLoop::headless` or [`Session::headless`](crate::gui::Session::headless)):
    /// a figure window or one of the dialogs it opened, as opaque RGBA
    /// rows from the top. Programs that drive windows with synthetic
    /// input capture them with this; [`FigureWindow::screenshot`] opens,
    /// draws and captures a window in one call.
    ///
    /// # Errors
    ///
    /// When no window `window` is open.
    pub fn capture(handle: &HeadlessHandle, window: WindowId) -> Result<RgbaImage, GuiError> {
        let frame = handle
            .surface(window)
            .ok_or(pltrs_platform::Error::UnknownWindow(window))?;
        let size = frame.size();
        let image = RgbaImage::new(
            size.width as usize,
            size.height as usize,
            frame.into_pixels(),
        )
        .expect("a surface holds four bytes per pixel");
        Ok(image)
    }

    /// Handles the events queued for the window and those they cause.
    fn settle(&mut self, event_loop: &mut EventLoop) -> Result<(), GuiError> {
        let mut failure = None;
        for _ in 0..SETTLE_ROUNDS {
            event_loop.pump_events(Some(Duration::ZERO), |cx, event| {
                let handled = match event {
                    Event::Window { window, event } if self.owns_window(window) => {
                        self.handle(cx, window, &event).map(drop)
                    }
                    Event::Timer(timer) if self.owns_timer(timer) => self.tick(cx, timer),
                    _ => Ok(()),
                };
                if let Err(err) = handled {
                    failure.get_or_insert(err);
                }
            })?;
        }
        failure.map_or(Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A figure of `width × height` inches at 100 dpi with a line.
    fn figure(width: f64, height: f64) -> Figure {
        let fig = Figure::with_size(width, height, 100.0);
        fig.subplot().plot([0.0, 1.0], [0.0, 1.0]).expect("a line");
        fig
    }

    #[test]
    fn a_screenshot_shows_the_figure_above_the_toolbar() {
        let fig = figure(3.0, 2.0);
        let window = FigureWindow::screenshot(&fig, 1.0).expect("drawn off screen");
        assert_eq!(window.width(), 300);
        assert!(window.height() > 200);
        let figure = fig.canvas().buffer_rgba().expect("drawn");
        let row = |image: &RgbaImage, y: usize| {
            let width = image.width() * 4;
            image.data()[y * width..(y + 1) * width].to_vec()
        };
        // The figure fills the top of the window, pixel for pixel.
        assert_eq!(row(&window, 100), row(&figure, 100));
        assert!(
            window
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| pixel[3] == 255),
            "the window is opaque"
        );
    }

    #[test]
    fn a_screenshot_leaves_the_figure_as_it_was() {
        let fig = figure(3.33, 2.0);
        let window = FigureWindow::screenshot(&fig, 1.5).expect("drawn off screen");
        assert_eq!(window.width(), 500);
        assert_eq!(fig.size_inches(), (3.33, 2.0));
        assert_eq!(fig.dpi(), 100.0);
        assert_eq!(fig.canvas().device_pixel_ratio(), 1.0);
    }

    #[test]
    fn degenerate_scale_factors_and_closed_windows_are_refused() {
        for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(FigureWindow::screenshot(&figure(1.0, 1.0), scale).is_err());
        }
        let (mut event_loop, handle) = EventLoop::headless();
        let window = FigureWindow::open(&mut event_loop.context(), &figure(1.0, 1.0), TITLE)
            .expect("the window opens");
        let id = window.id();
        assert!(FigureWindow::capture(&handle, id).is_ok());
        window.close(&mut event_loop.context()).expect("it closes");
        assert!(matches!(
            FigureWindow::capture(&handle, id),
            Err(GuiError::Platform(pltrs_platform::Error::UnknownWindow(_)))
        ));
    }
}
