//! Axes of an axes beyond its x and y axis, which the program draws: see
//! [`ProgramAxis`].

use std::rc::Rc;

use super::{Axes, AxesState};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ArtistLookup, DrawContext, RawHandle, TypedHandle,
};

/// An axis the program draws for its axes (see [`ProgramAxis`]).
#[derive(Clone, Debug)]
pub(crate) struct ProgramAxisArtist {
    base: ArtistBase,
    /// The name of the axis among the axes of its axes (matplotlib's
    /// `axis_name`).
    name: String,
}

pltrs_core::archive_struct!(ProgramAxisArtist { base, name });

impl Artist for ProgramAxisArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    /// Nothing: what the axis shows is its painter's.
    fn draw(&self, _ctx: &mut DrawContext<'_>) {}
}

/// An axis of an axes beyond its x and y axis, which the program draws
/// (matplotlib's `Axis` objects a projection class adds in `_init_axis`
/// and lists in `_axis_names`: the t, l and r axis of a ternary plot).
///
/// pltrs keeps the ticks, labels and limits of the x and y axis itself. An
/// axis under another name has a geometry only its program knows, so the
/// axes holds its place and nothing more: it is an artist the program
/// paints ([`ArtistPainter`](crate::artist::ArtistPainter)), and the axes
///
/// * draws it where it draws its axes: after the spines and its x and y
///   axis among the artists of equal z-order (the order of matplotlib's
///   `get_children`), at the artist's own z-order, and not at all while
///   [`Axes::set_axis_off`] hides the axes;
/// * measures it as an axis for tight boxes and layouts: whenever the
///   axes shows its axes, whether or not the artists added to the axes
///   count, and without clipping it to the axes;
/// * keeps it when it is cleared, as it keeps its spines and titles;
/// * lists it among none of the artists added to it.
///
/// ```
/// use pltrs::artist::{ArtistHandle, ArtistId, ArtistPainter, DrawContext, Painter};
/// use pltrs::prelude::*;
/// use pltrs::path::Path;
/// use pltrs::transforms::{Affine2D, Point};
///
/// /// Draws a diagonal across the canvas for its axis.
/// #[derive(Debug)]
/// struct Diagonal;
///
/// impl ArtistPainter for Diagonal {
///     fn draw(&self, ctx: &mut DrawContext<'_>, _axis: ArtistId) {
///         let gc = pltrs::artist::GraphicsContext::default();
///         let stroke = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(100.0, 100.0)]);
///         ctx.renderer().draw_path(&gc, &stroke, &Affine2D::IDENTITY, None);
///     }
/// }
///
/// let fig = Figure::new();
/// let ax = fig.subplot();
/// let third = ax.add_program_axis("t");
/// third.raw().set_painter(Some(Painter::new(Diagonal)));
/// assert_eq!(third.name(), "t");
/// assert_eq!(ax.program_axes(), [third.clone()]);
/// assert!(ax.added_children().is_empty());
/// ax.clear();
/// assert_eq!(ax.program_axes(), [third]);
/// fig.to_png()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// The handle of such an axis ([`Axes::add_program_axis`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramAxis {
    raw: RawHandle,
}

impl ProgramAxis {
    /// The name the axis was added under.
    pub fn name(&self) -> String {
        self.raw
            .with::<ProgramAxisArtist, _>(|axis| axis.name.clone())
    }
}

impl ArtistHandle for ProgramAxis {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for ProgramAxis {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<ProgramAxisArtist>()
            .then(|| Self { raw: raw.clone() })
    }
}

impl Axes {
    /// Gives the axes an axis named `name` that the program draws (see
    /// [`ProgramAxis`]), at the z-order the axes gives its axes
    /// ([`Axes::set_axisbelow`]), and returns its handle, for the
    /// program to set its painter on. An axis the axes had under that name
    /// is removed (matplotlib's `self.taxis = TAxis(self)` run again
    /// leaves the first to no one).
    pub fn add_program_axis(&self, name: &str) -> ProgramAxis {
        let figure = Rc::clone(self.raw().figure_state());
        let id = {
            let mut state = figure.borrow_mut();
            let replaced: Vec<ArtistId> = state
                .artist_as::<AxesState>(self.id())
                .map(|axes| axes.program_axes.clone())
                .unwrap_or_default()
                .into_iter()
                .filter(|&id| {
                    state
                        .artist_as::<ProgramAxisArtist>(id)
                        .is_some_and(|axis| axis.name == name)
                })
                .collect();
            for id in replaced {
                state.remove_artist(id);
            }
            let zorder = state
                .artist_as::<AxesState>(self.id())
                .map_or(0.0, |axes| axes.axisbelow.zorder());
            let axis = ProgramAxisArtist {
                base: ArtistBase::with_zorder(zorder),
                name: name.to_owned(),
            };
            let id = state.adopt(self.id(), Box::new(axis));
            if let Some(axes) = state.artist_as_mut::<AxesState>(self.id()) {
                axes.program_axes.push(id);
            }
            id
        };
        ProgramAxis {
            raw: RawHandle::new(figure, id),
        }
    }

    /// The axes the program draws for this axes, in the order they were
    /// added.
    pub fn program_axes(&self) -> Vec<ProgramAxis> {
        let figure = self.raw().figure_state();
        self.raw()
            .with::<AxesState, _>(|axes| axes.program_axes.clone())
            .into_iter()
            .map(|id| ProgramAxis {
                raw: RawHandle::new(Rc::clone(figure), id),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use pltrs_core::Bbox;
    use pltrs_core::archive::NoProgram;

    use super::*;
    use crate::artist::{ArtistPainter, DrawTarget, MeasuredExtent, Painter};
    use crate::figure::Figure;
    use crate::lines::LineProps;
    use crate::testing::RecordingRenderer;

    /// Counts its draws, noting how many of the line's came before, and
    /// measures as a box beside the axes.
    #[derive(Debug, Default)]
    struct Counting {
        prepared: Rc<Cell<usize>>,
        draws: Rc<Cell<usize>>,
        order: Rc<RefCell<Vec<&'static str>>>,
        label: &'static str,
    }

    impl ArtistPainter for Counting {
        fn prepare(&self, _axis: &RawHandle, _target: DrawTarget<'_>) {
            self.prepared.set(self.prepared.get() + 1);
        }

        fn draw(&self, _ctx: &mut DrawContext<'_>, _axis: ArtistId) {
            self.draws.set(self.draws.get() + 1);
            self.order.borrow_mut().push(self.label);
        }

        fn measured_extent(&self) -> MeasuredExtent {
            MeasuredExtent::Extent(Bbox::from_extents(-50.0, -40.0, -10.0, -5.0))
        }
    }

    fn drawn(fig: &Figure) {
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
    }

    #[test]
    fn a_program_axis_is_drawn_once_per_draw_while_the_axes_shows_its_axes() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let (prepared, draws) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let axis = ax.add_program_axis("t");
        axis.raw().set_painter(Some(Painter::new(Counting {
            prepared: Rc::clone(&prepared),
            draws: Rc::clone(&draws),
            ..Counting::default()
        })));
        drawn(&fig);
        assert_eq!((prepared.get(), draws.get()), (1, 1));
        ax.set_axis_off();
        drawn(&fig);
        assert_eq!(
            (prepared.get(), draws.get()),
            (1, 1),
            "neither prepared nor drawn with the axes of the axes hidden"
        );
        ax.set_axis_on();
        axis.set_visible(false);
        drawn(&fig);
        assert_eq!(
            draws.get(),
            2,
            "the painter decides what a hidden axis draws"
        );
    }

    #[test]
    fn program_axes_draw_after_the_artists_of_their_zorder_in_the_order_added() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let order = Rc::new(RefCell::new(Vec::new()));
        for label in ["t", "l"] {
            let axis = ax.add_program_axis(label);
            axis.raw().set_painter(Some(Painter::new(Counting {
                order: Rc::clone(&order),
                label,
                ..Counting::default()
            })));
            // The z-order of lines: the line added later still comes first.
            axis.set_zorder(2.0);
        }
        let line = ax
            .plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default())
            .expect("plottable");
        line.raw().set_painter(Some(Painter::new(Counting {
            order: Rc::clone(&order),
            label: "line",
            ..Counting::default()
        })));
        drawn(&fig);
        assert_eq!(*order.borrow(), ["line", "t", "l"]);
    }

    #[test]
    fn a_program_axis_is_a_part_the_axes_keeps_and_lists_nowhere() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let axis = ax.add_program_axis("t");
        assert_eq!(axis.name(), "t");
        assert_eq!(axis.zorder(), ax.axisbelow().zorder());
        assert!(axis.raw().is_part());
        assert_eq!(ax.added_children(), Vec::<RawHandle>::new());
        assert_eq!(ax.artists(), Vec::<RawHandle>::new());
        ax.clear();
        assert_eq!(ax.program_axes(), std::slice::from_ref(&axis));
        assert!(axis.raw().is_attached());
    }

    #[test]
    fn an_axis_added_under_a_name_again_replaces_the_first() {
        let ax = Figure::new().subplot();
        let first = ax.add_program_axis("t");
        let other = ax.add_program_axis("l");
        let second = ax.add_program_axis("t");
        assert_eq!(ax.program_axes(), [other, second]);
        assert!(!first.raw().is_attached());
    }

    #[test]
    fn the_tight_box_takes_what_the_painter_measured_while_the_axes_are_shown() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let plain = ax.tight_bbox(false).expect("a box");
        let axis = ax.add_program_axis("t");
        axis.raw()
            .set_painter(Some(Painter::new(Counting::default())));
        // Clipped to the axes, as matplotlib's `reset_ticks` clips an
        // axis: an axis is measured whatever clips it.
        axis.set_clip_box(Some(ax.id()));
        let with_axis = ax.tight_bbox(false).expect("a box");
        assert!(with_axis.x0() <= -50.0 && with_axis.y0() <= -40.0);
        ax.set_axis_off();
        let hidden = ax.tight_bbox(false).expect("a box");
        assert!(hidden.x0() >= plain.x0() - 1e-9);
        assert!(hidden.x0() > -50.0);
    }

    #[test]
    fn a_figure_with_a_program_axis_reads_back_from_its_archive() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.add_program_axis("t").set_zorder(4.0);
        let (bytes, externals) = fig.to_archive(&NoProgram).expect("archivable");
        let read = Figure::from_archive(&bytes, &externals, &NoProgram).expect("readable");
        let axes = read.axes();
        let axes = axes.first().expect("the axes");
        let read: Vec<(String, f64)> = axes
            .program_axes()
            .iter()
            .map(|axis| (axis.name(), axis.zorder()))
            .collect();
        assert_eq!(read, [("t".to_owned(), 4.0)]);
    }
}
