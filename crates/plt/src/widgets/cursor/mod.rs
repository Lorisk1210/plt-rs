//! Crosshair cursors following the pointer (matplotlib's `Cursor` and
//! `MultiCursor`), redrawn with their figure or, blitting, alone over a
//! saved background: the options and run-time switches both share, and
//! the backgrounds they blit over. [`crosshair`] holds the cursor of one
//! axes, [`multi`] the lines shared by several.

mod crosshair;
mod multi;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use plt_core::Bbox;

pub use crosshair::Cursor;
pub use multi::MultiCursor;

use super::Connections;
use crate::events::{EventKind, LockOwner};
use crate::figure::{BufferRegion, Figure, FigureCanvas};
use crate::lines::LineProps;

/// Options of a [`Cursor`] or [`MultiCursor`].
#[derive(Clone, Debug)]
pub struct CursorOptions {
    /// Whether the horizontal line is drawn (`horizOn`).
    pub horizontal: bool,
    /// Whether the vertical line is drawn (`vertOn`).
    pub vertical: bool,
    /// Whether a motion redraws only the lines, by blitting (`useblit`):
    /// the lines are animated, the background under them (the axes of a
    /// cursor, every figure of a multicursor) is saved whenever the canvas
    /// is drawn, and a motion puts it back, draws the lines over it and
    /// blits that part of the canvas. A [`Cursor`] over axes that overlap
    /// other axes of the figure is redrawn with the figure instead, as in
    /// matplotlib.
    pub useblit: bool,
    /// Look of the lines (`**lineprops`).
    pub line: LineProps,
}

impl Default for CursorOptions {
    /// [`Cursor`]'s defaults: both lines, redrawn with the figure.
    fn default() -> Self {
        Self {
            horizontal: true,
            vertical: true,
            useblit: false,
            line: LineProps::default(),
        }
    }
}

impl CursorOptions {
    /// [`MultiCursor`]'s defaults: only the vertical line, blitted.
    pub fn multicursor() -> Self {
        Self {
            horizontal: false,
            useblit: true,
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
            pub fn set_active(&self, active: bool) -> &Self {
                self.switches.active.set(active);
                self
            }

            /// Whether the lines show while the pointer is over the axes
            /// (matplotlib's `visible`).
            pub fn visible(&self) -> bool {
                self.switches.visible.get()
            }

            /// Shows or hides the lines from the next motion on.
            pub fn set_visible(&self, visible: bool) -> &Self {
                self.switches.visible.set(visible);
                self
            }

            /// Whether the horizontal line is drawn (matplotlib's
            /// `horizOn`).
            pub fn horizontal(&self) -> bool {
                self.switches.horizontal.get()
            }

            /// Draws the horizontal line or not.
            pub fn set_horizontal(&self, on: bool) -> &Self {
                self.switches.horizontal.set(on);
                self
            }

            /// Whether the vertical line is drawn (matplotlib's `vertOn`).
            pub fn vertical(&self) -> bool {
                self.switches.vertical.get()
            }

            /// Draws the vertical line or not.
            pub fn set_vertical(&self, on: bool) -> &Self {
                self.switches.vertical.set(on);
                self
            }

            /// Whether moving the lines blits them rather than redrawing
            /// the figure (matplotlib's `useblit`).
            pub fn useblit(&self) -> bool {
                self.blit.is_some()
            }
        }
    };
}

use cursor_switches;

/// The backgrounds a blitting cursor saved, one per canvas it draws on
/// (the blit background of matplotlib's `Cursor`, the `background` of
/// each canvas of its `MultiCursor`).
#[derive(Debug, Default)]
struct Backgrounds(RefCell<Vec<(FigureCanvas, BufferRegion)>>);

impl Backgrounds {
    /// Saves the part of `canvas` inside `bbox` as its background, in
    /// place of the one saved before.
    fn save(&self, canvas: &FigureCanvas, bbox: &Bbox) {
        match canvas.copy_from_bbox(bbox) {
            Ok(region) => {
                let mut saved = self.0.borrow_mut();
                saved.retain(|(kept, _)| kept != canvas);
                saved.push((canvas.clone(), region));
            }
            Err(err) => canvas.report(Err(err)),
        }
    }

    /// Puts every saved background back; whether one was.
    fn restore(&self) -> bool {
        let mut restored = false;
        for (canvas, region) in self.0.borrow().iter() {
            match canvas.restore_region(region) {
                Ok(()) => restored = true,
                Err(err) => canvas.report(Err(err)),
            }
        }
        restored
    }
}

/// Runs `save` at every draw of the figure of `connections` while the
/// cursor is active: a blitting cursor saves its background whenever the
/// canvas is drawn (matplotlib's `clear` on `draw_event`).
fn save_on_draw(connections: &mut Connections, switches: &Rc<Switches>, save: impl Fn() + 'static) {
    let switches = Rc::clone(switches);
    connections.connect(EventKind::Draw, move |_| {
        if switches.active.get() {
            save();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::Axes;
    use crate::events::EventSource;

    /// A 4 × 3 in figure at 100 dpi with axes over its left half showing
    /// [0, 10]².
    fn half_axes() -> (Figure, Axes) {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig
            .add_axes([0.0, 0.0, 0.5, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        (fig, ax)
    }

    /// The pixel at `(x, y)` of the canvas, from its top-left corner.
    fn pixel(fig: &Figure, x: usize, y: usize) -> [u8; 4] {
        let image = fig.canvas().buffer_rgba().expect("drawn");
        image.pixel(x, y).expect("on the canvas")
    }

    const WHITE: [u8; 4] = [255; 4];

    #[test]
    fn the_crosshair_follows_the_pointer_inside_the_axes() {
        let (fig, ax) = half_axes();
        let cursor = Cursor::new(&ax, CursorOptions::default());
        assert!(!cursor.useblit());
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
    fn a_blitting_crosshair_redraws_only_its_lines() {
        let (fig, ax) = half_axes();
        let options = CursorOptions {
            useblit: true,
            line: LineProps::default().color("k").linewidth(4.0),
            ..CursorOptions::default()
        };
        let cursor = Cursor::new(&ax, options);
        assert!(cursor.useblit() && cursor.vertical_line().animated());
        fig.canvas().draw().expect("drawn");
        let mut source = EventSource::new();
        source.motion(&fig, 100.0, 150.0);
        assert!(
            !fig.is_stale(),
            "the animated lines leave the figure as drawn"
        );
        assert_ne!(pixel(&fig, 100, 50), WHITE, "the vertical line, blitted");
        source.motion(&fig, 50.0, 150.0);
        assert_eq!(pixel(&fig, 100, 50), WHITE, "the background put back");
        source.motion(&fig, 300.0, 150.0);
        assert_eq!(pixel(&fig, 50, 50), WHITE, "erased as the pointer leaves");
        assert!(!cursor.vertical_line().visible());
    }

    #[test]
    fn crosshairs_over_overlapping_axes_do_not_blit() {
        let (fig, ax) = half_axes();
        fig.add_axes([0.2, 0.2, 0.6, 0.6])
            .expect("a valid rectangle");
        let options = CursorOptions {
            useblit: true,
            ..CursorOptions::default()
        };
        let cursor = Cursor::new(&ax, options);
        assert!(!cursor.useblit() && !cursor.vertical_line().animated());
    }

    #[test]
    fn a_multicursor_spans_its_axes() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let top = fig
            .add_axes([0.0, 0.5, 1.0, 0.5])
            .expect("a valid rectangle");
        let bottom = fig
            .add_axes([0.0, 0.0, 1.0, 0.5])
            .expect("a valid rectangle");
        for ax in [&top, &bottom] {
            ax.set_xlim(0.0, 4.0)
                .and_then(|ax| ax.set_ylim(0.0, 1.0))
                .expect("finite limits");
        }
        let multi = MultiCursor::new(&[top, bottom], CursorOptions::multicursor());
        assert!(multi.useblit());
        fig.canvas().draw().expect("drawn");
        EventSource::new().motion(&fig, 100.0, 100.0);
        for line in multi.vertical_lines() {
            assert_eq!(line.xdata(), [1.0, 1.0]);
            assert!(line.visible());
        }
        assert!(multi.horizontal_lines().iter().all(|line| !line.visible()));
        assert!(!fig.is_stale());
        assert_ne!(pixel(&fig, 100, 50), WHITE, "the top line, blitted");
        assert_ne!(pixel(&fig, 100, 350), WHITE, "the bottom line, blitted");
    }
}
