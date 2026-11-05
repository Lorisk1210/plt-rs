//! The figure window in a window of the display, driven by synthetic
//! input handed to it as the platform hands it input: what the display
//! does with the window (its scale factor and sizes, the dialogs as
//! windows of their own, the timers) shows in what the window presents.

use std::error::Error;
use std::time::{Duration, Instant};

use pltrs::figure::Figure;
use pltrs::gui::FigureWindow;
use pltrs::image::RgbaImage;
use pltrs_platform::{Context, Event, EventLoop, LogicalSize, WindowEvent, WindowId};

use crate::review::{Driver, dispatch};

/// How long a run lasts at least: the display delivers the redraws the
/// input asks for in its own time.
const SETTLE: Duration = Duration::from_millis(150);
/// How long a window takes to open: macOS animates it meanwhile.
const OPENING: Duration = Duration::from_millis(500);

/// A figure window shown on the display, driven by synthetic input.
pub(crate) struct Onscreen {
    event_loop: EventLoop,
    window: FigureWindow,
    /// Synthetic input waiting to be handed over, oldest first.
    queued: Vec<(WindowId, WindowEvent)>,
    /// The windows open, in the order they opened.
    opened: Vec<WindowId>,
    /// Whether the figure window asked to close.
    closing: bool,
}

impl Onscreen {
    /// Shows `figure` in a window of the display.
    pub(crate) fn open(figure: &Figure) -> Result<Self, Box<dyn Error>> {
        let mut event_loop = EventLoop::new()?;
        let window = FigureWindow::open(&mut event_loop.context(), figure, "Figure 1")?;
        let mut shown = Self {
            opened: vec![window.id()],
            event_loop,
            window,
            queued: Vec::new(),
            closing: false,
        };
        shown.run(Instant::now() + OPENING)?;
        Ok(shown)
    }

    /// Resizes the figure window to `size` and waits for it to redraw;
    /// returns it.
    pub(crate) fn resize(&mut self, size: LogicalSize) -> Result<WindowId, Box<dyn Error>> {
        let main = self.window.id();
        self.event_loop.context().set_inner_size(main, size)?;
        self.settle()?;
        Ok(main)
    }

    /// Whether the figure window asked to close.
    pub(crate) fn closing(&self) -> bool {
        self.closing
    }
}

impl Driver for Onscreen {
    fn main(&self) -> WindowId {
        self.window.id()
    }

    fn geometry(&mut self, window: WindowId) -> (f64, f64) {
        let cx = self.event_loop.context();
        let scale = cx.scale_factor(window).unwrap_or(1.0);
        let height = cx
            .inner_size(window)
            .map_or(0.0, |size| f64::from(size.height));
        (scale, height)
    }

    fn inject(&mut self, window: WindowId, event: WindowEvent) -> Result<(), Box<dyn Error>> {
        self.queued.push((window, event));
        Ok(())
    }

    fn run(&mut self, deadline: Instant) -> Result<WindowId, Box<dyn Error>> {
        let deadline = deadline.max(Instant::now() + SETTLE);
        let Self {
            event_loop,
            window,
            queued,
            opened,
            closing,
        } = self;
        let mut failure = None;
        let mut handle = |cx: &mut Context<'_>, event: Event| match dispatch(window, cx, event) {
            Ok(open) => *closing |= !open,
            Err(err) => {
                failure.get_or_insert(err);
            }
        };
        {
            let mut cx = event_loop.context();
            for (id, event) in queued.drain(..) {
                handle(&mut cx, Event::Window { window: id, event });
            }
        }
        event_loop.run_until(deadline, &mut handle)?;
        if let Some(err) = failure {
            return Err(err.into());
        }
        let open = event_loop.context().windows();
        opened.retain(|id| open.contains(id));
        let new: Vec<WindowId> = open.into_iter().filter(|id| !opened.contains(id)).collect();
        opened.extend(new);
        Ok(opened.last().copied().unwrap_or_else(|| window.id()))
    }

    fn capture(&mut self, window: WindowId) -> Result<RgbaImage, Box<dyn Error>> {
        let frame = self.event_loop.context().read_pixels(window)?;
        let size = frame.size();
        Ok(RgbaImage::new(
            size.width as usize,
            size.height as usize,
            frame.into_pixels(),
        )?)
    }
}
