//! The program's measures of its painted artists (see
//! [`ArtistPainter::measure`]), taken before a figure is laid out or a
//! tight box computed.
//!
//! matplotlib's tight boxes ask each artist for its own
//! (`get_tightbbox`), a container asking the artists it holds from inside
//! its own: an `Axes` subclass of the program's prepares itself (cartopy's
//! `GeoAxes` autoscales and applies its aspect), then measures its
//! artists through `Axes.get_tightbbox`, among them artists of the
//! program's that measure themselves where the axes now is (cartopy's
//! gridliner places its labels). So the painters are measured parents
//! first, and the artists a measuring painter's artist holds are measured
//! by it, through the tight box of the axes it asks for
//! ([`RawHandle::measure_painters_under`]), not again after it.

use std::collections::HashSet;
use std::rc::Rc;

use super::{ArtistId, MeasuredExtent, Painter, RawHandle};
use crate::figure::FigureState;

/// A painted artist to measure, in tree order.
struct ToMeasure {
    /// The artist.
    id: ArtistId,
    /// Its painter.
    painter: Painter,
    /// The index, in the list of artists to measure, past the artists it
    /// holds.
    past_held: usize,
}

impl RawHandle {
    /// Has the painters of the figure's artists measure them (see
    /// [`ArtistPainter::measure`]), before it is laid out or its tight box
    /// computed: parents first, an artist whose painter measured it
    /// measuring the artists it holds itself.
    ///
    /// [`ArtistPainter::measure`]: super::ArtistPainter::measure
    pub(crate) fn measure_painters(&self) {
        let to_measure = {
            let state = self.figure_state().borrow();
            let mut to_measure = in_tree_order(&state, state.root, true);
            // Those no artist of the figure holds, after them.
            let reached: HashSet<ArtistId> = to_measure.iter().map(|entry| entry.id).collect();
            for id in state.store.ids().filter(|id| !reached.contains(id)) {
                if let Some(painter) = state.store.get(id).and_then(|a| a.base().painter.clone()) {
                    let past_held = to_measure.len() + 1;
                    to_measure.push(ToMeasure {
                        id,
                        painter,
                        past_held,
                    });
                }
            }
            to_measure
        };
        self.measure_in_order(&to_measure);
    }

    /// Has the painters of the artists this artist holds measure them,
    /// parents first, before its tight box is computed (matplotlib's
    /// `Axes.get_tightbbox` asking each of its artists for theirs).
    pub(crate) fn measure_painters_under(&self) {
        let to_measure = {
            let state = self.figure_state().borrow();
            in_tree_order(&state, self.id(), false)
        };
        self.measure_in_order(&to_measure);
    }

    /// Measures the artists of `to_measure` in order, past the artists
    /// held by one its painter measured.
    fn measure_in_order(&self, to_measure: &[ToMeasure]) {
        let mut next = 0;
        while let Some(entry) = to_measure.get(next) {
            let painter = entry.painter.get();
            painter.measure(&self.sibling(entry.id));
            (self.figure_state().borrow_mut()).note_measured_axes(entry.id);
            next = if painter.measured_extent() == MeasuredExtent::Unmeasured {
                next + 1
            } else {
                entry.past_held
            };
        }
    }
}

impl FigureState {
    /// [`RawHandle::measure_painters`] in the middle of a layout working on
    /// the state, which lends it to the painters (see
    /// [`FigureState::lend`]); no layout engine runs meanwhile, and what
    /// they change leaves no layout due. Nothing without painters, or when
    /// the state cannot be lent.
    pub(crate) fn measure_painters_lent(&mut self) {
        let painted =
            |id| (self.store.get(id)).is_some_and(|artist| artist.base().painter.is_some());
        if !self.store.ids().any(painted) {
            return;
        }
        let root = self.root;
        let layout = (self.layout.pending, self.layout.when_drawn_only);
        self.layout.when_drawn_only = true;
        self.lend(|cell| RawHandle::new(Rc::clone(cell), root).measure_painters());
        (self.layout.pending, self.layout.when_drawn_only) = layout;
    }
}

/// The painted artists of the tree of `root` in the figure's store, each
/// before the artists it holds, `root` itself only with `itself`.
fn in_tree_order(state: &FigureState, root: ArtistId, itself: bool) -> Vec<ToMeasure> {
    let mut found = Vec::new();
    visit(state, root, itself, &mut found);
    found
}

/// Adds the painted artists of the tree of `id` to `found`, `id` itself
/// only with `itself`.
fn visit(state: &FigureState, id: ArtistId, itself: bool, found: &mut Vec<ToMeasure>) {
    let Some(artist) = state.store.get(id) else {
        return;
    };
    let at = found.len();
    let painted = itself && artist.base().painter.is_some();
    if let Some(painter) = artist.base().painter.clone().filter(|_| painted) {
        found.push(ToMeasure {
            id,
            painter,
            past_held: at + 1,
        });
    }
    for child in artist.children() {
        visit(state, child, true, found);
    }
    if painted {
        found[at].past_held = found.len();
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::cell::{Cell, RefCell};

    use pltrs_core::Bbox;

    use super::*;
    use crate::artist::{ArtistHandle, ArtistPainter, DrawContext};
    use crate::figure::Figure;

    /// Notes the artists it measures in a shared log, measuring them as a
    /// fixed box when `measures`.
    #[derive(Debug)]
    struct Logged {
        log: Rc<RefCell<Vec<ArtistId>>>,
        measures: bool,
        measured: Cell<MeasuredExtent>,
    }

    impl ArtistPainter for Logged {
        fn draw(&self, _ctx: &mut DrawContext<'_>, _artist: ArtistId) {}

        fn measure(&self, artist: &RawHandle) {
            self.log.borrow_mut().push(artist.id());
            if self.measures {
                self.measured.set(MeasuredExtent::Extent(Bbox::from_extents(
                    0.0, 0.0, 10.0, 10.0,
                )));
            }
        }

        fn measured_extent(&self) -> MeasuredExtent {
            self.measured.get()
        }
    }

    /// A painter noting its measures in `log`, measuring with `measures`.
    fn logged(log: &Rc<RefCell<Vec<ArtistId>>>, measures: bool) -> Painter {
        Painter::new(Logged {
            log: Rc::clone(log),
            measures,
            measured: Cell::new(MeasuredExtent::Unmeasured),
        })
    }

    #[test]
    fn parents_are_measured_first_and_measure_what_they_hold() {
        let log = Rc::default();
        let fig = Figure::new();
        let first = fig.subplot();
        let line = first.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        line.raw().set_painter(Some(logged(&log, true)));
        // An axes painted without measuring: its line is measured.
        first.raw().set_painter(Some(logged(&log, false)));
        fig.root_handle().measure_painters();
        assert_eq!(*log.borrow(), [first.id(), line.id()]);
        // A measuring axes measures its line itself.
        first.raw().set_painter(Some(logged(&log, true)));
        log.borrow_mut().clear();
        fig.root_handle().measure_painters();
        assert_eq!(*log.borrow(), [first.id()]);
        // An axes' tight box measures the artists it holds.
        log.borrow_mut().clear();
        first.raw().measure_painters_under();
        assert_eq!(*log.borrow(), [line.id()]);
        let painter = line.raw().painter().expect("painted");
        let any: &dyn Any = painter.get();
        assert!(any.is::<Logged>());
    }
}
