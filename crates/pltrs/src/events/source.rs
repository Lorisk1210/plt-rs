//! Turning raw pointer and key input into figure events, as matplotlib's
//! canvas does (`LocationEvent`, `_mouse_handler`, `_key_handler`).

use pltrs_core::Point;

use super::{Event, KeyEvent, Location, MouseEvent};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::figure::{Figure, FigureState};
use crate::navigation::{PointerButton, axes_at};
use crate::transforms::TransformContext;

/// The input state of one figure canvas: the button and key held and the
/// axes last under the pointer.
#[derive(Debug, Default)]
pub struct EventSource {
    button: Option<PointerButton>,
    key: Option<String>,
    last_axes: Option<Axes>,
}

/// The data coordinates of display point `(x, y)` in `ax`.
fn data_at(ax: &Axes, x: f64, y: f64) -> (Option<f64>, Option<f64>) {
    let figure = ax.raw().figure_state();
    let state = figure.prepared();
    let state: &FigureState = &state;
    let inverse = state
        .data_transform(ax.id())
        .and_then(|trans| trans.inverted());
    inverse.map_or((None, None), |inverse| {
        let p = inverse.transform_point(Point::new(x, y));
        (Some(p.x), Some(p.y))
    })
}

/// `(x, y)` with the axes under it and the data coordinates there.
/// The axes is the one grabbing the mouse, if any.
/// Whole pixels from the bottom left: like matplotlib's `LocationEvent`,
/// pointer positions are truncated to integers.
fn locate(figure: &Figure, x: f64, y: f64) -> Location {
    let (x, y) = (x.trunc(), y.trunc());
    let inaxes = figure
        .mouse_grabber()
        .or_else(|| axes_at(figure, Point::new(x, y)));
    location_in(inaxes, x, y)
}

fn location_in(inaxes: Option<Axes>, x: f64, y: f64) -> Location {
    let (xdata, ydata) = inaxes.as_ref().map_or((None, None), |ax| data_at(ax, x, y));
    Location {
        x,
        y,
        inaxes,
        xdata,
        ydata,
    }
}

impl EventSource {
    /// A source with nothing held.
    pub fn new() -> Self {
        Self::default()
    }

    fn mouse(&self, location: Location, button: Option<PointerButton>) -> MouseEvent {
        MouseEvent {
            location,
            button,
            key: self.key.clone(),
            step: 0.0,
            dblclick: false,
        }
    }

    /// A button went down at display point `(x, y)`.
    pub fn button_press(
        &mut self,
        figure: &Figure,
        x: f64,
        y: f64,
        button: PointerButton,
        dblclick: bool,
    ) {
        self.button = Some(button);
        let event = MouseEvent {
            dblclick,
            ..self.mouse(locate(figure, x, y), Some(button))
        };
        figure.process_event(&Event::ButtonPress(event));
    }

    /// A double click at display point `(x, y)`, reported after the
    /// second click's release, as browsers do: a press event with
    /// `dblclick` set, leaving no button held (matplotlib's `WebAgg`
    /// `dblclick` message).
    pub fn double_click(&mut self, figure: &Figure, x: f64, y: f64, button: PointerButton) {
        let event = MouseEvent {
            dblclick: true,
            ..self.mouse(locate(figure, x, y), Some(button))
        };
        figure.process_event(&Event::ButtonPress(event));
    }

    /// A button went up at display point `(x, y)`.
    pub fn button_release(&mut self, figure: &Figure, x: f64, y: f64, button: PointerButton) {
        self.button = None;
        let event = self.mouse(locate(figure, x, y), Some(button));
        figure.process_event(&Event::ButtonRelease(event));
    }

    /// The pointer moved to display point `(x, y)`; emits axes enter and
    /// leave events when it crosses an axes boundary.
    pub fn motion(&mut self, figure: &Figure, x: f64, y: f64) {
        let location = locate(figure, x, y);
        if self.last_axes != location.inaxes {
            if let Some(last) = self.last_axes.take() {
                let left = location_in(Some(last), location.x, location.y);
                figure.process_event(&Event::AxesLeave(left));
            }
            if location.inaxes.is_some() {
                figure.process_event(&Event::AxesEnter(location.clone()));
            }
            self.last_axes.clone_from(&location.inaxes);
        }
        let event = self.mouse(location, self.button);
        figure.process_event(&Event::Motion(event));
    }

    /// The wheel turned by `step` (positive up) at display point `(x, y)`.
    pub fn scroll(&mut self, figure: &Figure, x: f64, y: f64, step: f64) {
        let event = MouseEvent {
            step,
            ..self.mouse(locate(figure, x, y), None)
        };
        figure.process_event(&Event::Scroll(event));
    }

    /// Key `key` (matplotlib's naming) went down with the pointer at `(x, y)`.
    pub fn key_press(&mut self, figure: &Figure, key: &str, x: f64, y: f64) {
        self.key = Some(key.to_owned());
        let event = KeyEvent {
            location: locate(figure, x, y),
            key: key.to_owned(),
        };
        figure.process_event(&Event::KeyPress(event));
    }

    /// Key `key` went up with the pointer at `(x, y)`.
    pub fn key_release(&mut self, figure: &Figure, key: &str, x: f64, y: f64) {
        self.key = None;
        let event = KeyEvent {
            location: locate(figure, x, y),
            key: key.to_owned(),
        };
        figure.process_event(&Event::KeyRelease(event));
    }

    /// The pointer entered the figure at `(x, y)`.
    pub fn figure_enter(&mut self, figure: &Figure, x: f64, y: f64) {
        figure.process_event(&Event::FigureEnter(locate(figure, x, y)));
    }

    /// The pointer left the figure; the axes it was over are left too.
    pub fn figure_leave(&mut self, figure: &Figure) {
        if let Some(last) = self.last_axes.take() {
            let location = Location {
                x: f64::NAN,
                y: f64::NAN,
                inaxes: Some(last),
                xdata: None,
                ydata: None,
            };
            figure.process_event(&Event::AxesLeave(location));
        }
        figure.process_event(&Event::FigureLeave);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::events::EventKind;

    fn record(fig: &Figure, kinds: &[EventKind]) -> Rc<RefCell<Vec<Event>>> {
        let log = Rc::new(RefCell::new(Vec::new()));
        for &kind in kinds {
            let log = Rc::clone(&log);
            fig.connect(kind, move |event| log.borrow_mut().push(event.clone()));
        }
        log
    }

    #[test]
    fn events_carry_axes_and_data_coordinates() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let log = record(&fig, &[EventKind::ButtonPress, EventKind::Motion]);
        let mut source = EventSource::new();
        source.button_press(&fig, 200.0, 150.0, PointerButton::Left, false);
        source.motion(&fig, 1.0, 1.0);
        let log = log.borrow();
        let Event::ButtonPress(press) = &log[0] else {
            panic!("a press first");
        };
        assert_eq!(press.location.inaxes.as_ref(), Some(&ax));
        assert!(
            press
                .location
                .xdata
                .is_some_and(|x| (0.0..10.0).contains(&x))
        );
        let Event::Motion(motion) = &log[1] else {
            panic!("then a motion");
        };
        assert_eq!(motion.button, Some(PointerButton::Left), "the held button");
        assert_eq!(motion.location.inaxes, None);
    }

    #[test]
    fn crossing_axes_emits_enter_and_leave() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot();
        let log = record(&fig, &[EventKind::AxesEnter, EventKind::AxesLeave]);
        let mut source = EventSource::new();
        source.motion(&fig, 200.0, 150.0);
        source.motion(&fig, 210.0, 150.0);
        source.motion(&fig, 1.0, 1.0);
        let kinds: Vec<EventKind> = log.borrow().iter().map(Event::kind).collect();
        assert_eq!(kinds, [EventKind::AxesEnter, EventKind::AxesLeave]);
        let Event::AxesLeave(left) = &log.borrow()[1] else {
            panic!("a leave event");
        };
        assert_eq!(
            left.inaxes.as_ref(),
            Some(&ax),
            "the leave names the axes left"
        );
    }
}
