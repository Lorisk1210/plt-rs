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
use crate::axes::AxesState;
use crate::figure::{FigureFrame, FigureState};
use crate::mplot3d::Axes3DState;

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
            let mut to_measure = in_tree_order(&state, state.root, true, Reach::All);
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
        self.measure_in_order(&to_measure, false);
    }

    /// [`measure_painters`](Self::measure_painters) for a layout engine,
    /// which measures what matplotlib's layouts measure: the axes (and
    /// the artists they hold) and the figures' super titles and labels,
    /// not the other artists of the figures (a figure's text, legend or
    /// patch), which matplotlib's layouts never ask for their extents.
    pub(crate) fn measure_painters_for_layout(&self) {
        let to_measure = {
            let state = self.figure_state().borrow();
            in_tree_order(&state, state.root, true, Reach::LaidOut)
        };
        self.measure_in_order(&to_measure, true);
    }

    /// Has the painters of the artists this artist holds measure them,
    /// parents first, before its tight box is computed (matplotlib's
    /// `Axes.get_tightbbox` asking each of its artists for theirs): for
    /// a layout with `for_layout`, as `get_tightbbox(renderer,
    /// for_layout_only=True)` asks the axes of the axes.
    pub(crate) fn measure_painters_under(&self, for_layout: bool) {
        let to_measure = {
            let state = self.figure_state().borrow();
            in_tree_order(&state, self.id(), false, Reach::All)
        };
        self.measure_in_order(&to_measure, for_layout);
    }

    /// Measures the artists of `to_measure` in order, past the artists
    /// held by one its painter measured: for a layout engine with
    /// `for_layout` ([`ArtistPainter::measure_for_layout`]).
    ///
    /// [`ArtistPainter::measure_for_layout`]: super::ArtistPainter::measure_for_layout
    fn measure_in_order(&self, to_measure: &[ToMeasure], for_layout: bool) {
        let mut next = 0;
        while let Some(entry) = to_measure.get(next) {
            let painter = entry.painter.get();
            let artist = self.sibling(entry.id);
            if for_layout {
                painter.measure_for_layout(&artist);
            } else {
                painter.measure(&artist);
            }
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
    /// [`RawHandle::measure_painters_for_layout`] in the middle of a layout working on
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
        self.lend(|cell| RawHandle::new(Rc::clone(cell), root).measure_painters_for_layout());
        (self.layout.pending, self.layout.when_drawn_only) = layout;
    }
}

/// Which artists of a tree are measured.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reach {
    /// All of them (a tight box).
    All,
    /// Those a layout engine measures: in a figure, its axes, its
    /// subfigures and its super texts; in an axes, all of them.
    LaidOut,
}

/// The painted artists of the tree of `root` in the figure's store that
/// `reach` takes, each before the artists it holds, `root` itself only
/// with `itself`.
fn in_tree_order(
    state: &FigureState,
    root: ArtistId,
    itself: bool,
    reach: Reach,
) -> Vec<ToMeasure> {
    let mut found = Vec::new();
    visit(state, root, itself, reach, &mut found);
    found
}

/// Adds the painted artists of the tree of `id` that `reach` takes to
/// `found`, `id` itself only with `itself`.
fn visit(
    state: &FigureState,
    id: ArtistId,
    itself: bool,
    reach: Reach,
    found: &mut Vec<ToMeasure>,
) {
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
    let frame = state.store.get_as::<FigureFrame>(id);
    for child in artist.children() {
        if reach == Reach::LaidOut
            && let Some(frame) = frame
            && !laid_out(state, frame, child)
        {
            continue;
        }
        let reach = if is_axes(state, child) {
            Reach::All
        } else {
            reach
        };
        visit(state, child, true, reach, found);
    }
    if painted {
        found[at].past_held = found.len();
    }
}

/// Whether a layout engine measures `child` of the figure `frame`: an
/// axes, a subfigure or one of the figure's super texts.
fn laid_out(state: &FigureState, frame: &FigureFrame, child: ArtistId) -> bool {
    let texts = frame.super_texts;
    is_axes(state, child)
        || state.store.get_as::<FigureFrame>(child).is_some()
        || [texts.title, texts.xlabel, texts.ylabel]
            .into_iter()
            .flatten()
            .any(|text| text.id == child)
}

/// Whether `id` is an axes, 2-D or 3-D.
fn is_axes(state: &FigureState, id: ArtistId) -> bool {
    state.store.get_as::<AxesState>(id).is_some() || state.store.get_as::<Axes3DState>(id).is_some()
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::cell::{Cell, RefCell};

    use pltrs_core::Bbox;

    use super::*;
    use crate::artist::{ArtistHandle, ArtistPainter, DrawContext};
    use crate::figure::Figure;
    use crate::layout::TightLayout;

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
        first.raw().measure_painters_under(false);
        assert_eq!(*log.borrow(), [line.id()]);
        let painter = line.raw().painter().expect("painted");
        let any: &dyn Any = painter.get();
        assert!(any.is::<Logged>());
    }

    #[test]
    fn layouts_measure_axes_and_super_texts_not_the_figures_other_artists() {
        let log = Rc::default();
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        let note = fig.text(0.5, 0.5, "note");
        let title = fig.suptitle("title");
        for artist in [line.raw(), note.raw(), title.raw()] {
            artist.set_painter(Some(logged(&log, false)));
        }
        fig.root_handle().measure_painters_for_layout();
        assert_eq!(*log.borrow(), [line.id(), title.id()]);
        // A tight box measures them all.
        log.borrow_mut().clear();
        fig.root_handle().measure_painters();
        assert_eq!(log.borrow().len(), 3);
        assert!(log.borrow().contains(&note.id()));
    }

    /// Counts its measures for tight boxes and for layout engines apart.
    #[derive(Debug, Default)]
    struct Asked {
        plainly: Cell<usize>,
        for_layout: Cell<usize>,
    }

    impl ArtistPainter for Asked {
        fn draw(&self, _ctx: &mut DrawContext<'_>, _artist: ArtistId) {}

        fn measure(&self, _artist: &RawHandle) {
            self.plainly.set(self.plainly.get() + 1);
        }

        fn measure_for_layout(&self, _artist: &RawHandle) {
            self.for_layout.set(self.for_layout.get() + 1);
        }
    }

    #[test]
    fn a_layout_engine_asks_painters_to_measure_for_a_layout() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let axis = ax.add_program_axis("t");
        let painter = Painter::new(Asked::default());
        axis.raw().set_painter(Some(painter.clone()));
        let any: &dyn Any = painter.get();
        let asked = any.downcast_ref::<Asked>().expect("the painter");
        // matplotlib's `get_tightbbox(renderer, for_layout_only=True)`.
        fig.tight_layout(TightLayout::default()).expect("laid out");
        assert!(asked.for_layout.get() > 0);
        assert_eq!(asked.plainly.get(), 0, "no plain measure by a layout");
        let measured_for_layouts = asked.for_layout.get();
        // A tight box asks plainly, unless it is one for a layout.
        ax.tight_bbox(false);
        assert_eq!(asked.for_layout.get(), measured_for_layouts);
        assert_eq!(asked.plainly.get(), 1);
        ax.tight_bbox(true);
        assert_eq!(asked.for_layout.get(), measured_for_layouts + 1);
        assert_eq!(asked.plainly.get(), 1);
        // The default measures for a layout as for a tight box.
        let log = Rc::default();
        axis.raw().set_painter(Some(logged(&log, false)));
        fig.root_handle().measure_painters_for_layout();
        assert_eq!(*log.borrow(), [axis.raw().id()]);
    }
}
