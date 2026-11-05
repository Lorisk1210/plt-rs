//! The figure window drawn off screen, by the headless platform, which
//! takes synthetic input as the display's input.

use std::error::Error;
use std::time::{Duration, Instant};

use pltrs::figure::Figure;
use pltrs::gui::FigureWindow;
use pltrs::image::RgbaImage;
use pltrs_platform::{Event, EventLoop, HeadlessHandle, WindowEvent, WindowId};

use crate::review::{Driver, dispatch};

/// A figure window drawn off screen, driven by synthetic input.
pub(crate) struct Offscreen {
    event_loop: EventLoop,
    handle: HeadlessHandle,
    window: FigureWindow,
}

impl Offscreen {
    /// Shows `figure` at pixel ratio `ratio`.
    pub(crate) fn open(figure: &Figure, ratio: f64) -> Result<Self, Box<dyn Error>> {
        let (mut event_loop, handle) = EventLoop::headless();
        handle.set_scale_factor(ratio)?;
        let window = FigureWindow::open(&mut event_loop.context(), figure, "Figure 1")?;
        let mut shown = Self {
            event_loop,
            handle,
            window,
        };
        shown.settle()?;
        Ok(shown)
    }
}

impl Driver for Offscreen {
    fn main(&self) -> WindowId {
        self.window.id()
    }

    fn geometry(&mut self, window: WindowId) -> (f64, f64) {
        self.handle.window(window).map_or((1.0, 0.0), |snapshot| {
            (snapshot.scale_factor, f64::from(snapshot.size.height))
        })
    }

    fn inject(&mut self, window: WindowId, event: WindowEvent) -> Result<(), Box<dyn Error>> {
        Ok(self.handle.inject(window, event)?)
    }

    fn run(&mut self, deadline: Instant) -> Result<WindowId, Box<dyn Error>> {
        let window = &mut self.window;
        let mut failure = None;
        let mut handle = |cx: &mut pltrs_platform::Context<'_>, event: Event| {
            if let Err(err) = dispatch(window, cx, event) {
                failure.get_or_insert(err);
            }
        };
        // Redraws follow the events that asked for them.
        for _ in 0..3 {
            self.event_loop
                .pump_events(Some(Duration::ZERO), &mut handle)?;
        }
        self.event_loop.run_until(deadline, &mut handle)?;
        match failure {
            Some(err) => Err(err.into()),
            None => Ok(self
                .handle
                .windows()
                .into_iter()
                .last()
                .unwrap_or(self.window.id())),
        }
    }

    fn capture(&mut self, window: WindowId) -> Result<RgbaImage, Box<dyn Error>> {
        Ok(FigureWindow::capture(&self.handle, window)?)
    }
}
