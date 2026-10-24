//! Crosshair cursors following the pointer (matplotlib's `Cursor` and
//! `MultiCursor`).

use std::cell::Cell;
use std::rc::Rc;

use plt_core::Point;

use super::{Connections, data_in, ordered};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::{Event, EventKind, Location, LockOwner};
use crate::figure::Figure;
use crate::lines::{Line2D, LineProps};

/// Options of a [`Cursor`] or [`MultiCursor`].
#[derive(Clone, Debug)]
pub struct CursorOptions {
    /// Whether the horizontal line is drawn (`horizOn`).
    pub horizontal: bool,
    /// Whether the vertical line is drawn (`vertOn`).
    pub vertical: bool,
    /// Look of the lines (`**lineprops`).
    pub line: LineProps,
}

impl Default for CursorOptions {
    /// [`Cursor`]'s defaults: both lines.
    fn default() -> Self {
        Self {
            horizontal: true,
            vertical: true,
            line: LineProps::default(),
        }
    }
}

impl CursorOptions {
    /// [`MultiCursor`]'s defaults: only the vertical line.
    pub fn multicursor() -> Self {
        Self {
            horizontal: false,
            ..Self::default()
        }
    }
}

/// What both cursors switch at run time.
#[derive(Debug)]
struct Switches {
    active: Cell<bool>,
    visible: Cell<bool>,
    horizontal: Cell<bool>,
    vertical: Cell<bool>,
    /// Identifies the cursor to the figure's widget lock.
    owner: LockOwner,
}

impl Switches {
    fn new(options: &CursorOptions) -> Self {
        Self {
            active: Cell::new(true),
            visible: Cell::new(true),
            horizontal: Cell::new(options.horizontal),
            vertical: Cell::new(options.vertical),
            owner: LockOwner::new(),
        }
    }

    /// Whether a motion over `figure` moves the lines.
    fn takes(&self, figure: &Figure) -> bool {
        self.active.get() && figure.widgets_available(self.owner)
    }

    /// Whether the vertical and the horizontal line show.
    fn shown(&self) -> (bool, bool) {
        let visible = self.visible.get();
        (
            visible && self.vertical.get(),
            visible && self.horizontal.get(),
        )
    }
}

/// The run-time switches of a cursor, generated for both kinds.
macro_rules! cursor_switches {
    ($cursor:ty) => {
        impl $cursor {
            /// Whether the cursor follows the pointer (matplotlib's
            /// `active`).
            pub fn active(&self) -> bool {
                self.switches.active.get()
            }

            /// Turns following the pointer on or off.
            pub fn set_active(&self, active: bool) {
                self.switches.active.set(active);
            }

            /// Whether the lines show while the pointer is over the axes
            /// (matplotlib's `visible`).
            pub fn visible(&self) -> bool {
                self.switches.visible.get()
            }

            /// Shows or hides the lines from the next motion on.
            pub fn set_visible(&self, visible: bool) {
                self.switches.visible.set(visible);
            }

            /// Whether the horizontal line is drawn (matplotlib's
            /// `horizOn`).
            pub fn horizontal(&self) -> bool {
                self.switches.horizontal.get()
            }

            /// Draws the horizontal line or not.
            pub fn set_horizontal(&self, on: bool) {
                self.switches.horizontal.set(on);
            }

            /// Whether the vertical line is drawn (matplotlib's `vertOn`).
            pub fn vertical(&self) -> bool {
                self.switches.vertical.get()
            }

            /// Draws the vertical line or not.
            pub fn set_vertical(&self, on: bool) {
                self.switches.vertical.set(on);
            }
        }
    };
}

/// A crosshair spanning an axes that follows the pointer (matplotlib's
/// `Cursor`).
///
/// ```
/// use plt::artist::ArtistHandle;
/// use plt::figure::Figure;
/// use plt::widgets::{Cursor, CursorOptions};
/// let ax = Figure::new().subplot();
/// let cursor = Cursor::new(&ax, CursorOptions::default());
/// assert!(!cursor.vertical_line().visible(), "hidden until the pointer comes");
/// ```
#[derive(Debug)]
pub struct Cursor {
    switches: Rc<Switches>,
    horizontal_line: Line2D,
    vertical_line: Line2D,
    _connections: Connections,
}

cursor_switches!(Cursor);

impl Cursor {
    /// A crosshair over `ax`, hidden until the pointer moves over it.
    pub fn new(ax: &Axes, options: CursorOptions) -> Self {
        let props = options.line.clone().visible(false);
        let (x0, _) = ordered(ax.xlim());
        let (y0, _) = ordered(ax.ylim());
        let horizontal_line = ax.axhline(y0, props.clone());
        let vertical_line = ax.axvline(x0, props);
        let switches = Rc::new(Switches::new(&options));
        let mut connections = Connections::new(ax.figure());
        {
            let (ax, switches) = (ax.clone(), Rc::clone(&switches));
            let (lineh, linev) = (horizontal_line.clone(), vertical_line.clone());
            connections.connect(EventKind::Motion, move |event| {
                let Event::Motion(mouse) = event else {
                    return;
                };
                if !switches.takes(&ax.figure()) {
                    return;
                }
                let location = &mouse.location;
                let data = data_in(&ax, location)
                    .filter(|_| ax.contains_point(Point::new(location.x, location.y)));
                let Some(data) = data else {
                    linev.set_visible(false);
                    lineh.set_visible(false);
                    return;
                };
                let (vertical, horizontal) = switches.shown();
                linev.set_xdata(vec![data.x, data.x]).set_visible(vertical);
                lineh
                    .set_ydata(vec![data.y, data.y])
                    .set_visible(horizontal);
            });
        }
        Self {
            switches,
            horizontal_line,
            vertical_line,
            _connections: connections,
        }
    }

    /// The horizontal line (matplotlib's `lineh`).
    pub fn horizontal_line(&self) -> &Line2D {
        &self.horizontal_line
    }

    /// The vertical line (matplotlib's `linev`).
    pub fn vertical_line(&self) -> &Line2D {
        &self.vertical_line
    }
}

/// Lines shared by several axes, possibly of several figures, that follow
/// the pointer over any of them (matplotlib's `MultiCursor`): vertical
/// lines by default, to compare plots sharing an x axis.
#[derive(Debug)]
pub struct MultiCursor {
    switches: Rc<Switches>,
    vertical_lines: Vec<Line2D>,
    horizontal_lines: Vec<Line2D>,
    _connections: Vec<Connections>,
}

cursor_switches!(MultiCursor);

/// The lines of a multicursor, moved together.
#[derive(Debug)]
struct SharedLines {
    vertical: Vec<Line2D>,
    horizontal: Vec<Line2D>,
}

impl SharedLines {
    fn moved_to(&self, data: Point, (vertical, horizontal): (bool, bool)) {
        for line in &self.vertical {
            line.set_xdata(vec![data.x, data.x]).set_visible(vertical);
        }
        for line in &self.horizontal {
            line.set_ydata(vec![data.y, data.y]).set_visible(horizontal);
        }
    }
}

impl MultiCursor {
    /// Lines across each of `axes`, starting hidden at the middle of the
    /// last one's view (see [`CursorOptions::multicursor`] for matplotlib's
    /// defaults).
    pub fn new(axes: &[Axes], options: CursorOptions) -> Self {
        let props = options.line.clone().visible(false);
        let (xmid, ymid) = axes.last().map_or((0.5, 0.5), |ax| {
            let ((x0, x1), (y0, y1)) = (ax.xlim(), ax.ylim());
            (f64::midpoint(x0, x1), f64::midpoint(y0, y1))
        });
        let vertical: Vec<Line2D> = axes
            .iter()
            .map(|ax| ax.axvline(xmid, props.clone()))
            .collect();
        let horizontal: Vec<Line2D> = axes
            .iter()
            .map(|ax| ax.axhline(ymid, props.clone()))
            .collect();
        let switches = Rc::new(Switches::new(&options));
        let lines = Rc::new(SharedLines {
            vertical: vertical.clone(),
            horizontal: horizontal.clone(),
        });
        let mut figures: Vec<Figure> = Vec::new();
        for ax in axes {
            let figure = ax.figure();
            if !figures.contains(&figure) {
                figures.push(figure);
            }
        }
        let connections = figures
            .into_iter()
            .map(|figure| {
                let mut connections = Connections::new(figure.clone());
                let members: Vec<Axes> = axes
                    .iter()
                    .filter(|ax| ax.figure() == figure)
                    .cloned()
                    .collect();
                let (switches, lines) = (Rc::clone(&switches), Rc::clone(&lines));
                connections.connect(EventKind::Motion, move |event| {
                    if let Event::Motion(mouse) = event
                        && switches.takes(&figure)
                        && let Some(data) = topmost_data(&members, &mouse.location)
                    {
                        lines.moved_to(data, switches.shown());
                    }
                });
                connections
            })
            .collect();
        Self {
            switches,
            vertical_lines: vertical,
            horizontal_lines: horizontal,
            _connections: connections,
        }
    }

    /// The vertical lines, one per axes (matplotlib's `vlines`).
    pub fn vertical_lines(&self) -> &[Line2D] {
        &self.vertical_lines
    }

    /// The horizontal lines, one per axes (matplotlib's `hlines`).
    pub fn horizontal_lines(&self) -> &[Line2D] {
        &self.horizontal_lines
    }
}

/// The data coordinates of `location` in the topmost of `axes` under it,
/// the last of the highest z-order (matplotlib's `_topmost_artist`).
fn topmost_data(axes: &[Axes], location: &Location) -> Option<Point> {
    let at = Point::new(location.x, location.y);
    let ax = axes
        .iter()
        .filter(|ax| ax.contains_point(at))
        .reduce(|top, ax| if ax.zorder() >= top.zorder() { ax } else { top })?;
    data_in(ax, location)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventSource;

    #[test]
    fn the_crosshair_follows_the_pointer_inside_the_axes() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.add_axes([0.0, 0.0, 0.5, 1.0]);
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        let cursor = Cursor::new(&ax, CursorOptions::default());
        let mut source = EventSource::new();
        source.motion(&fig, 100.0, 60.0);
        assert_eq!(cursor.vertical_line().xdata(), [5.0, 5.0]);
        assert!(cursor.horizontal_line().visible());
        cursor.set_horizontal(false);
        source.motion(&fig, 120.0, 60.0);
        assert!(!cursor.horizontal_line().visible());
        source.motion(&fig, 300.0, 60.0);
        assert!(!cursor.vertical_line().visible(), "outside the axes");
    }

    #[test]
    fn a_multicursor_spans_its_axes() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let top = fig.add_axes([0.0, 0.5, 1.0, 0.5]);
        let bottom = fig.add_axes([0.0, 0.0, 1.0, 0.5]);
        for ax in [&top, &bottom] {
            ax.set_xlim(0.0, 4.0).set_ylim(0.0, 1.0);
        }
        let multi = MultiCursor::new(&[top, bottom], CursorOptions::multicursor());
        EventSource::new().motion(&fig, 100.0, 100.0);
        for line in multi.vertical_lines() {
            assert_eq!(line.xdata(), [1.0, 1.0]);
            assert!(line.visible());
        }
        assert!(multi.horizontal_lines().iter().all(|line| !line.visible()));
    }
}
