//! Figure windows kept open between calls: the event loop behind
//! [`show`](super::show), pyplot's interactive mode, `pause`, `ginput` and
//! `waitforbuttonpress`.

use std::time::{Duration, Instant};

use plt_platform::{Context, Event, EventLoop, HeadlessHandle};

use super::{FigureWindow, GuiError};
use crate::events::{GinputOptions, wait};
use crate::figure::Figure;

/// An event loop and the figure windows it runs.
#[derive(Debug)]
pub struct Session {
    event_loop: EventLoop,
    windows: Vec<FigureWindow>,
    /// Figures whose windows the user closed, not yet reported.
    closed: Vec<Figure>,
}

impl Session {
    /// A session on the system's window system.
    ///
    /// # Errors
    ///
    /// When there is no window system to connect to.
    pub fn new() -> Result<Self, GuiError> {
        Ok(Self::with_loop(EventLoop::new()?))
    }

    /// A session without a display, whose windows are driven through the
    /// returned handle (for tests and off-screen use).
    pub fn headless() -> (Self, HeadlessHandle) {
        let (event_loop, handle) = EventLoop::headless();
        (Self::with_loop(event_loop), handle)
    }

    fn with_loop(event_loop: EventLoop) -> Self {
        Self {
            event_loop,
            windows: Vec::new(),
            closed: Vec::new(),
        }
    }

    /// Opens a window titled `title` for each figure of `figures` without
    /// one, brings the windows of the others to the front when
    /// `figure.raise_window` is on (as matplotlib's backends do on every
    /// show, `pause` included), and redraws the figures that changed.
    ///
    /// # Errors
    ///
    /// When a window cannot be created or a figure cannot be rendered.
    pub fn show(&mut self, figures: &[(Figure, String)]) -> Result<(), GuiError> {
        let mut cx = self.event_loop.context();
        let raise = crate::rc::params().figure_raise_window;
        for (figure, title) in figures {
            match self
                .windows
                .iter_mut()
                .find(|window| window.figure() == figure)
            {
                Some(window) => {
                    window.retitle(&mut cx, title)?;
                    if raise {
                        cx.raise_window(window.id())?;
                    }
                }
                None => self
                    .windows
                    .push(FigureWindow::open(&mut cx, figure, title)?),
            }
        }
        self.windows
            .iter_mut()
            .try_for_each(|window| window.refresh(&mut cx))
    }

    /// Closes the window of `figure`, if it has one (matplotlib's
    /// `pyplot.close(figure)`).
    ///
    /// # Errors
    ///
    /// When the window system fails to close the window.
    pub fn close(&mut self, figure: &Figure) -> Result<(), GuiError> {
        match self
            .windows
            .iter()
            .position(|window| window.figure() == figure)
        {
            Some(index) => self
                .windows
                .remove(index)
                .close(&mut self.event_loop.context()),
            None => Ok(()),
        }
    }

    /// Opens the subplot configuration tool of the window showing
    /// `figure`, or raises it when it is open (matplotlib's
    /// `configure_subplots` of the figure's toolbar).
    ///
    /// # Errors
    ///
    /// [`GuiError::NotShown`] when no window shows the figure, or when the
    /// tool window cannot be opened.
    pub fn configure_subplots(&mut self, figure: &Figure) -> Result<(), GuiError> {
        let mut cx = self.event_loop.context();
        self.windows
            .iter_mut()
            .find(|window| window.figure() == figure)
            .ok_or(GuiError::NotShown)?
            .configure_subplots(&mut cx)
    }

    /// Asks for the canvas of the window showing `figure` to be
    /// `width × height` physical pixels (matplotlib's
    /// `manager.resize(width, height)`); the figure follows once the
    /// window system has resized the window (see [`FigureWindow::resize`]).
    ///
    /// # Errors
    ///
    /// [`GuiError::NotShown`] when no window shows the figure, or when the
    /// window system refuses the size.
    pub fn resize(&mut self, figure: &Figure, width: u32, height: u32) -> Result<(), GuiError> {
        let mut cx = self.event_loop.context();
        self.windows
            .iter_mut()
            .find(|window| window.figure() == figure)
            .ok_or(GuiError::NotShown)?
            .resize(&mut cx, width, height)
    }

    /// Whether `figure` is shown in a window.
    pub fn shows(&self, figure: &Figure) -> bool {
        self.windows.iter().any(|window| window.figure() == figure)
    }

    /// Whether no window is open.
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    /// The figures whose windows the user closed since the last call.
    pub fn take_closed(&mut self) -> Vec<Figure> {
        std::mem::take(&mut self.closed)
    }

    /// Runs the windows until `deadline` passes (forever when `None`),
    /// `done` returns `true` after an event, or the last window closes
    /// (matplotlib's `start_event_loop`).
    ///
    /// # Errors
    ///
    /// When the window system fails or a figure cannot be rendered.
    pub fn run(
        &mut self,
        deadline: Option<Instant>,
        mut done: impl FnMut() -> bool,
    ) -> Result<(), GuiError> {
        self.run_loop(deadline, |session| !done() && !session.windows.is_empty())
    }

    /// Runs the windows for `duration` (matplotlib's `pause`).
    ///
    /// # Errors
    ///
    /// As [`Session::run`].
    pub fn pause(&mut self, duration: Duration) -> Result<(), GuiError> {
        self.run(Instant::now().checked_add(duration), || false)
    }

    /// Collects up to `n` points clicked in the window of `figure` (all
    /// until stopped when `n` is 0), within `timeout` (matplotlib's
    /// `ginput`): the add button or any key over an axes adds the data
    /// coordinates of the pointer, the pop button, backspace or delete
    /// removes the last point, and the stop button, escape or enter ends
    /// the input.
    ///
    /// # Errors
    ///
    /// As [`Session::run`].
    pub fn ginput(
        &mut self,
        figure: &Figure,
        n: usize,
        timeout: Option<Duration>,
        options: GinputOptions,
    ) -> Result<Vec<(f64, f64)>, GuiError> {
        let points = wait::ginput(figure, n, timeout, options, &mut |deadline, done| {
            self.run_while_shown(figure, deadline, done)
        });
        self.refresh()?;
        points
    }

    /// Waits for a key press (`Some(true)`) or a mouse click
    /// (`Some(false)`) in the window of `figure`, or `timeout` (`None`)
    /// (matplotlib's `waitforbuttonpress`).
    ///
    /// # Errors
    ///
    /// As [`Session::run`].
    pub fn wait_for_button_press(
        &mut self,
        figure: &Figure,
        timeout: Option<Duration>,
    ) -> Result<Option<bool>, GuiError> {
        wait::wait_for_button_press(figure, timeout, &mut |deadline, done| {
            self.run_while_shown(figure, deadline, done)
        })
    }

    /// Runs until `done`, `deadline`, or the window of `figure` closes.
    fn run_while_shown(
        &mut self,
        figure: &Figure,
        deadline: Option<Instant>,
        mut done: impl FnMut() -> bool,
    ) -> Result<(), GuiError> {
        self.run_loop(deadline, |session| !done() && session.shows(figure))
    }

    /// Dispatches events while `keep_going` holds, until `deadline`.
    fn run_loop(
        &mut self,
        deadline: Option<Instant>,
        mut keep_going: impl FnMut(&Self) -> bool,
    ) -> Result<(), GuiError> {
        let mut failure = None;
        while failure.is_none() && keep_going(self) {
            let timeout = match deadline {
                Some(deadline) => match deadline.checked_duration_since(Instant::now()) {
                    Some(left) if !left.is_zero() => Some(left),
                    _ => break,
                },
                None => None,
            };
            let (windows, closed) = (&mut self.windows, &mut self.closed);
            self.event_loop.pump_events(timeout, |cx, event| {
                if let Err(err) = dispatch(windows, closed, cx, event) {
                    failure.get_or_insert(err);
                }
            })?;
        }
        failure.map_or(Ok(()), Err)
    }

    /// Redraws the figures that changed.
    fn refresh(&mut self) -> Result<(), GuiError> {
        let mut cx = self.event_loop.context();
        self.windows
            .iter_mut()
            .try_for_each(|window| window.refresh(&mut cx))
    }
}

/// Delivers a platform event to the window it belongs to; windows asking
/// to close are closed and their figures recorded in `closed`.
fn dispatch(
    windows: &mut Vec<FigureWindow>,
    closed: &mut Vec<Figure>,
    cx: &mut Context<'_>,
    event: Event,
) -> Result<(), GuiError> {
    match event {
        Event::Timer(timer) => windows
            .iter_mut()
            .find(|window| window.owns_timer(timer))
            .map_or(Ok(()), |window| window.tick(cx, timer)),
        Event::Window { window, event } => {
            let Some(index) = windows.iter().position(|w| w.owns_window(window)) else {
                return Ok(());
            };
            if windows[index].handle(cx, window, &event)? {
                return Ok(());
            }
            let shut = windows.remove(index);
            let all = shut.quit_all_requested();
            closed.push(shut.figure().clone());
            shut.close(cx)?;
            if all {
                for other in windows.drain(..) {
                    closed.push(other.figure().clone());
                    other.close(cx)?;
                }
            }
            Ok(())
        }
        // Callbacks may change figures shown in other windows.
        Event::AboutToWait => windows.iter_mut().try_for_each(|window| window.refresh(cx)),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use plt_platform::{
        ButtonState, Key, KeyEvent, Modifiers, MouseButton, PhysicalPosition, WindowEvent, WindowId,
    };

    use super::*;

    /// A session showing a 400 × 300 figure with one axes over [0, 10]².
    fn shown() -> (Session, HeadlessHandle, Figure, WindowId) {
        let figure = Figure::with_size(4.0, 3.0, 100.0);
        let ax = figure
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let (mut session, handle) = Session::headless();
        session
            .show(&[(figure.clone(), "Figure 1".to_owned())])
            .expect("the window opens");
        let id = session.windows[0].id();
        (session, handle, figure, id)
    }

    fn click(handle: &HeadlessHandle, id: WindowId, button: MouseButton, (x, y): (f64, f64)) {
        let position = PhysicalPosition::new(x, y);
        let moved = WindowEvent::CursorMoved {
            position,
            modifiers: Modifiers::NONE,
        };
        handle.inject(id, moved).expect("the window exists");
        for state in [ButtonState::Pressed, ButtonState::Released] {
            let event = WindowEvent::MouseInput {
                button,
                state,
                position,
                modifiers: Modifiers::NONE,
                click_count: 1,
            };
            handle.inject(id, event).expect("the window exists");
        }
    }

    #[test]
    fn showing_again_raises_windows_unless_told_not_to() {
        let (mut session, handle) = Session::headless();
        let figures = [Figure::new(), Figure::new()];
        let titled = |figure: &Figure| (figure.clone(), "Figure".to_owned());
        session
            .show(&[titled(&figures[0]), titled(&figures[1])])
            .unwrap();
        let first = session.windows[0].id();
        crate::rc::context(|| {
            crate::rc::set("figure.raise_window", "False").unwrap();
            session.show(&[titled(&figures[0])]).unwrap();
        });
        assert_ne!(handle.stacking_order().last(), Some(&first));
        session.show(&[titled(&figures[0])]).unwrap();
        assert_eq!(handle.stacking_order().last(), Some(&first));
    }

    #[test]
    fn showing_again_under_another_title_renames_the_window() {
        let (mut session, handle, figure, id) = shown();
        session
            .show(&[(figure, "My plot".to_owned())])
            .expect("the window stays");
        assert_eq!(handle.window(id).expect("open").title, "My plot");
    }

    #[test]
    fn quit_all_closes_every_window() {
        crate::rc::context(|| {
            crate::rc::set("keymap.quit_all", "W").unwrap();
            let (mut session, handle) = Session::headless();
            let figures = [Figure::new(), Figure::new()];
            let shown: Vec<_> = figures
                .iter()
                .enumerate()
                .map(|(i, f)| (f.clone(), format!("Figure {i}")))
                .collect();
            session.show(&shown).expect("the windows open");
            let id = session.windows[1].id();
            for state in [ButtonState::Pressed, ButtonState::Released] {
                let key = KeyEvent {
                    key: Key::Character('W'),
                    state,
                    repeat: false,
                    text: None,
                    modifiers: Modifiers::SHIFT,
                    scancode: 0,
                };
                handle
                    .inject(id, WindowEvent::Key(key))
                    .expect("the window exists");
            }
            session
                .pause(Duration::from_millis(50))
                .expect("the loop runs");
            assert!(session.is_empty(), "every window closed");
            assert_eq!(session.take_closed().len(), 2);
        });
    }

    #[test]
    fn ginput_collects_clicks() {
        let (mut session, handle, figure, id) = shown();
        click(&handle, id, MouseButton::Left, (40.0, 270.0));
        click(&handle, id, MouseButton::Left, (80.0, 270.0));
        click(&handle, id, MouseButton::Right, (80.0, 270.0));
        click(&handle, id, MouseButton::Left, (200.0, 150.0));
        let points = session
            .ginput(
                &figure,
                2,
                Some(Duration::from_secs(5)),
                GinputOptions::default(),
            )
            .expect("the input runs");
        assert_eq!(points, [(1.0, 1.0), (5.0, 5.0)]);
        assert_eq!(figure.axes()[0].lines().len(), 0, "the marks are gone");
    }

    #[test]
    fn the_subplot_tool_opens_once_for_a_shown_figure() {
        let (mut session, handle, figure, _) = shown();
        session.configure_subplots(&figure).expect("the tool opens");
        session
            .configure_subplots(&figure)
            .expect("the tool is raised");
        let windows = handle.windows();
        assert_eq!(windows.len(), 2);
        let tool = handle.window(windows[1]).expect("the tool window is open");
        assert_eq!(tool.title, "Subplot configuration tool");
        assert!(matches!(
            session.configure_subplots(&Figure::new()),
            Err(GuiError::NotShown)
        ));
    }

    #[test]
    fn waiting_for_a_key_or_the_timeout() {
        let (mut session, handle, figure, id) = shown();
        let timeout = Some(Duration::from_millis(20));
        assert_eq!(
            session.wait_for_button_press(&figure, timeout).unwrap(),
            None
        );
        // A click answers too.
        click(&handle, id, MouseButton::Left, (200.0, 150.0));
        assert_eq!(
            session.wait_for_button_press(&figure, timeout).unwrap(),
            Some(false)
        );
        for state in [ButtonState::Pressed, ButtonState::Released] {
            let key = KeyEvent {
                key: Key::Character('a'),
                state,
                repeat: false,
                text: None,
                modifiers: Modifiers::NONE,
                scancode: 0,
            };
            handle
                .inject(id, WindowEvent::Key(key))
                .expect("the window exists");
        }
        assert_eq!(
            session.wait_for_button_press(&figure, timeout).unwrap(),
            Some(true)
        );
        handle
            .inject(id, WindowEvent::CloseRequested)
            .expect("the window exists");
        session.pause(Duration::from_millis(10)).unwrap();
        assert!(session.is_empty());
        assert_eq!(session.take_closed(), [figure]);
    }
}
