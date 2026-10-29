//! The crosshair of one axes (matplotlib's `Cursor`).

use std::cell::Cell;
use std::rc::Rc;

use pltrs_core::Point;

use super::{Backgrounds, CursorOptions, Switches, cursor_switches, save_on_draw};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::{Event, EventKind, Location};
use crate::lines::Line2D;
use crate::widgets::{Connections, data_in, ordered};

/// A crosshair spanning an axes that follows the pointer (matplotlib's
/// `Cursor`).
///
/// ```
/// use pltrs::artist::ArtistHandle;
/// use pltrs::figure::Figure;
/// use pltrs::widgets::{Cursor, CursorOptions};
/// let ax = Figure::new().subplot();
/// let cursor = Cursor::new(&ax, CursorOptions::default());
/// assert!(!cursor.vertical_line().visible(), "hidden until the pointer comes");
/// ```
#[derive(Debug)]
pub struct Cursor {
    switches: Rc<Switches>,
    blit: Option<Rc<Backgrounds>>,
    horizontal_line: Line2D,
    vertical_line: Line2D,
    _connections: Connections,
}

cursor_switches!(Cursor);

impl Cursor {
    /// A crosshair over `ax`, hidden until the pointer moves over it;
    /// blitted when the options ask for it and `ax` overlaps no other axes
    /// of its figure (see [`Cursor::useblit`]).
    pub fn new(ax: &Axes, options: CursorOptions) -> Self {
        let props = options.line.clone().visible(false);
        let (x0, _) = ordered(ax.xlim());
        let (y0, _) = ordered(ax.ylim());
        let horizontal_line = ax.axhline(y0, props.clone());
        let vertical_line = ax.axvline(x0, props);
        let blit = (options.useblit && !overlaps_other_axes(ax)).then(|| {
            horizontal_line.set_animated(true);
            vertical_line.set_animated(true);
            Rc::new(Backgrounds::default())
        });
        let switches = Rc::new(Switches::new(&options));
        let mut connections = Connections::new(ax.figure());
        if let Some(backgrounds) = &blit {
            let (ax, backgrounds) = (ax.clone(), Rc::clone(backgrounds));
            save_on_draw(&mut connections, &switches, move || {
                backgrounds.save(&ax.figure().canvas(), &ax.bbox());
            });
        }
        let crosshair = Crosshair {
            ax: ax.clone(),
            horizontal: horizontal_line.clone(),
            vertical: vertical_line.clone(),
            blit: blit.clone(),
            needclear: Cell::new(false),
        };
        let following = Rc::clone(&switches);
        connections.connect(EventKind::Motion, move |event| {
            if let Event::Motion(mouse) = event {
                crosshair.follow(&following, &mouse.location);
            }
        });
        Self {
            switches,
            blit,
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

/// Whether `ax` overlaps another axes of its figure, touching edges
/// included (matplotlib turns blitting off for such a `Cursor`, warning
/// that it does not support overlapping axes).
fn overlaps_other_axes(ax: &Axes) -> bool {
    let position = ax.position();
    ax.figure()
        .axes()
        .iter()
        .any(|other| other != ax && other.position().overlaps(&position))
}

/// The lines a [`Cursor`] moves, and how it redraws them.
#[derive(Debug)]
struct Crosshair {
    ax: Axes,
    horizontal: Line2D,
    vertical: Line2D,
    /// The saved background, when blitting.
    blit: Option<Rc<Backgrounds>>,
    /// Whether the lines were shown since the pointer last left the axes
    /// (matplotlib's `needclear`).
    needclear: Cell<bool>,
}

impl Crosshair {
    /// Moves the lines to the pointer at `location`, or hides them when it
    /// is outside the axes (matplotlib's `onmove`).
    fn follow(&self, switches: &Switches, location: &Location) {
        let ax = &self.ax;
        if !switches.takes(&ax.figure()) {
            return;
        }
        let data =
            data_in(ax, location).filter(|_| ax.contains_point(Point::new(location.x, location.y)));
        let Some(data) = data else {
            self.vertical.set_visible(false);
            self.horizontal.set_visible(false);
            if self.needclear.replace(false) {
                self.erase();
            }
            return;
        };
        self.needclear.set(true);
        let (vertical, horizontal) = switches.shown();
        self.vertical
            .set_xdata(vec![data.x, data.x])
            .set_visible(vertical);
        self.horizontal
            .set_ydata(vec![data.y, data.y])
            .set_visible(horizontal);
        if vertical || horizontal {
            self.redraw();
        }
    }

    /// Draws the moved lines over the saved background and blits the
    /// axes. Without blitting the lines changed the figure, which its
    /// windows draw again.
    fn redraw(&self) {
        let Some(backgrounds) = &self.blit else {
            return;
        };
        backgrounds.restore();
        let canvas = self.ax.figure().canvas();
        for line in [&self.vertical, &self.horizontal] {
            canvas.report(self.ax.draw_artist(line));
        }
        canvas.report(canvas.blit(Some(&self.ax.bbox())));
    }

    /// Erases the hidden lines: puts the saved background back and blits
    /// the axes, or draws the figure when no background was saved yet.
    fn erase(&self) {
        let Some(backgrounds) = &self.blit else {
            return;
        };
        let canvas = self.ax.figure().canvas();
        if backgrounds.restore() {
            canvas.report(canvas.blit(Some(&self.ax.bbox())));
        } else {
            canvas.report(canvas.draw_idle());
        }
    }
}
