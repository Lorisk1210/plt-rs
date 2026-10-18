//! Waiting for clicks and keys in a shown figure (matplotlib's `ginput`
//! and `waitforbuttonpress`), for whichever event loop shows it: figure
//! windows or the web server.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::{Event, EventKind};
use crate::artist::{Artist, ArtistHandle};
use crate::figure::Figure;
use crate::lines::{Line2D, LineArtist, LineProps};
use crate::markers::MarkerSymbol;
use crate::navigation::PointerButton;

/// The mouse buttons of `ginput` (matplotlib's `mouse_add`, `mouse_pop`
/// and `mouse_stop`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GinputOptions {
    /// Whether each point is marked with a red cross until the input ends
    /// (`show_clicks`).
    pub show_clicks: bool,
    /// The button adding a point.
    pub add: PointerButton,
    /// The button removing the last point.
    pub pop: PointerButton,
    /// The button ending the input.
    pub stop: PointerButton,
}

impl Default for GinputOptions {
    /// matplotlib's defaults: the left button adds, the right removes, the
    /// middle stops, and points are marked.
    fn default() -> Self {
        Self {
            show_clicks: true,
            add: PointerButton::Left,
            pop: PointerButton::Right,
            stop: PointerButton::Middle,
        }
    }
}

/// Runs an event loop until the deadline passes or the condition holds.
pub(crate) type RunUntil<'a, E> =
    &'a mut dyn FnMut(Option<Instant>, &mut dyn FnMut() -> bool) -> Result<(), E>;

/// The deadline `timeout` from now.
fn deadline(timeout: Option<Duration>) -> Option<Instant> {
    timeout.and_then(|timeout| Instant::now().checked_add(timeout))
}

/// Collects up to `n` points clicked in `figure` (all until stopped when
/// `n` is 0) while `run` runs the event loop showing it, within `timeout`:
/// the add button or any key over an axes adds the data coordinates of the
/// pointer, the pop button, backspace or delete removes the last point,
/// and the stop button, escape or enter ends the input. The marks are
/// removed afterwards.
pub(crate) fn ginput<E>(
    figure: &Figure,
    n: usize,
    timeout: Option<Duration>,
    options: GinputOptions,
    run: RunUntil<'_, E>,
) -> Result<Vec<(f64, f64)>, E> {
    let input = Rc::new(RefCell::new(Ginput::default()));
    let ids: Vec<_> = [EventKind::ButtonPress, EventKind::KeyPress]
        .into_iter()
        .map(|kind| {
            let input = Rc::clone(&input);
            figure.connect(kind, move |event| {
                input.borrow_mut().handle(event, n, options);
            })
        })
        .collect();
    let result = run(deadline(timeout), &mut || input.borrow().done);
    for id in ids {
        figure.disconnect(id);
    }
    let input = input.take();
    for mark in &input.marks {
        mark.remove();
    }
    result.map(|()| input.points)
}

/// Waits for a key press (`Some(true)`) or a mouse click (`Some(false)`)
/// in `figure` while `run` runs the event loop showing it, or `timeout`
/// (`None`).
pub(crate) fn wait_for_button_press<E>(
    figure: &Figure,
    timeout: Option<Duration>,
    run: RunUntil<'_, E>,
) -> Result<Option<bool>, E> {
    let pressed: Rc<Cell<Option<bool>>> = Rc::default();
    let ids: Vec<_> = [(EventKind::ButtonPress, false), (EventKind::KeyPress, true)]
        .into_iter()
        .map(|(kind, is_key)| {
            let pressed = Rc::clone(&pressed);
            figure.connect(kind, move |_| pressed.set(Some(is_key)))
        })
        .collect();
    let result = run(deadline(timeout), &mut || pressed.get().is_some());
    for id in ids {
        figure.disconnect(id);
    }
    result.map(|()| pressed.get())
}

/// The state of a `ginput` call.
#[derive(Debug, Default)]
struct Ginput {
    points: Vec<(f64, f64)>,
    /// The crosses marking the points.
    marks: Vec<Line2D>,
    done: bool,
}

impl Ginput {
    /// matplotlib's `ginput` handler.
    fn handle(&mut self, event: &Event, n: usize, options: GinputOptions) {
        let (button, key, location) = match event {
            Event::ButtonPress(mouse) => (mouse.button, None, &mouse.location),
            Event::KeyPress(key) => (None, Some(key.key.as_str()), &key.location),
            _ => return,
        };
        if button == Some(options.stop) || matches!(key, Some("escape" | "enter")) {
            self.done = true;
        } else if button == Some(options.pop) || matches!(key, Some("backspace" | "delete")) {
            if self.points.pop().is_some()
                && let Some(mark) = self.marks.pop()
            {
                mark.remove();
            }
        } else if (button == Some(options.add) || key.is_some())
            && let (Some(ax), Some(x), Some(y)) = (&location.inaxes, location.xdata, location.ydata)
        {
            self.points.push((x, y));
            if options.show_clicks {
                let mut line = LineArtist::new(vec![x], vec![y], crate::rc::params());
                LineProps::default()
                    .marker(MarkerSymbol::Plus)
                    .color("r")
                    .apply(&mut line);
                line.base_mut().transform = Some(ax.trans_data());
                self.marks.push(ax.add_line_artist(line));
            }
        }
        if n > 0 && self.points.len() == n {
            self.done = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventSource;

    #[test]
    fn ginput_follows_the_buttons_and_keys() {
        let figure = Figure::with_size(4.0, 3.0, 100.0);
        let ax = figure.add_axes([0.0, 0.0, 1.0, 1.0]);
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        let mut events = EventSource::new();
        let mut run = |_deadline: Option<Instant>, done: &mut dyn FnMut() -> bool| {
            events.button_press(&figure, 40.0, 30.0, PointerButton::Left, false);
            events.button_press(&figure, 40.0, 30.0, PointerButton::Right, false);
            events.button_press(&figure, 80.0, 30.0, PointerButton::Left, false);
            assert!(!done());
            events.key_press(&figure, "a", 200.0, 150.0);
            assert!(done(), "two points end the input");
            Ok::<(), ()>(())
        };
        let points = ginput(&figure, 2, None, GinputOptions::default(), &mut run);
        assert_eq!(points, Ok(vec![(2.0, 1.0), (5.0, 5.0)]));
        assert!(ax.lines().is_empty(), "the marks are gone");
    }

    #[test]
    fn waiting_reports_keys_and_clicks() {
        let figure = Figure::new();
        let mut events = EventSource::new();
        let mut run = |_deadline: Option<Instant>, done: &mut dyn FnMut() -> bool| {
            events.key_press(&figure, "x", 1.0, 1.0);
            assert!(done());
            Ok::<(), ()>(())
        };
        assert_eq!(
            wait_for_button_press(&figure, None, &mut run),
            Ok(Some(true))
        );
        let mut idle =
            |_deadline: Option<Instant>, _done: &mut dyn FnMut() -> bool| Ok::<(), ()>(());
        assert_eq!(wait_for_button_press(&figure, None, &mut idle), Ok(None));
    }
}
