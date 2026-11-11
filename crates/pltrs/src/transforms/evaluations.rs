//! Where matplotlib evaluates its transform graph while pltrs works
//! without one.
//!
//! matplotlib composes its transforms lazily: each node of the graph
//! (`transLimits`, `transData`, the blends an axis places its ticks with,
//! a rectangle's patch transform) computes its matrix with numpy when an
//! artist's `draw` first asks for it after a leaf changed, and keeps it
//! until the next change, or computes it at every call
//! (`CompositeGenericTransform.get_affine`). Where a leaf is not a
//! finite, ordinary number (view limits of no width, an infinite
//! rectangle), that arithmetic raises floating-point exceptions, which
//! numpy reports as `RuntimeWarning`s: how many, and of which operation,
//! follows from which nodes are computed where.
//!
//! pltrs resolves its [`Transform`](super::Transform)s from its state
//! instead, and warns of nothing. So it tells whoever keeps matplotlib's
//! graph over its leaves (pltrs (Python), see
//! [`Figure::tree_leaves`](crate::figure::Figure::tree_leaves)) where
//! matplotlib evaluates it: a figure's [`EvaluationHook`] is handed each
//! [`Evaluation`] of a draw as the draw reaches it, and each one of a
//! measurement (an axes' tight box, which layout engines and tight
//! prints take too) as it is taken, to evaluate the same nodes there.
//! That costs a figure whose leaves are all [tame](is_tame) (nearly
//! every figure) one look at them per draw and per axes measured, and
//! each patch one at its own transform: only a figure with a
//! [wild](FigureState::graph_is_wild) leaf, or a wild patch, reaches the
//! hook.

use std::fmt;
use std::rc::Rc;

use pltrs_core::Bbox;

use super::TransformContext;
use crate::artist::{Artist, ArtistId, ArtistLookup, RawHandle};
use crate::axes::AxesState;
use crate::figure::{Figure, FigureState};
use crate::ticker::AxisName;

/// One evaluation of matplotlib's transform graph by a draw or a
/// measurement.
#[derive(Clone, Debug, PartialEq)]
pub enum Evaluation {
    /// The `draw` of an artist of the figure.
    Draw(ArtistId),
    /// The `draw` of an axes itself, before it draws its background and
    /// its children (a polar axes fits its wedge to the view there).
    Axes(ArtistId),
    /// The draw of the background patch of an axes (`Axes.patch`).
    Background(ArtistId),
    /// `Axis.draw` of an axis of an axes: the labels of the ticks it
    /// draws measured, the ticks drawn, the axis label placed and drawn;
    /// or, `measured`, its `Axis.get_tightbbox`: the axis label placed,
    /// the labels of the ticks and the axis label measured.
    Axis {
        /// The axes.
        axes: ArtistId,
        /// Which of its axes.
        axis: AxisName,
        /// The ticks drawn, in matplotlib's order (major, then minor).
        ticks: Vec<DrawnTick>,
        /// Whether the axis label is drawn (shown, with a text), or
        /// measured (shown).
        label: bool,
        /// Whether the axis is measured rather than drawn.
        measured: bool,
    },
    /// `Axes.get_tightbbox` of an axes once its axes are measured (each
    /// an [`Axis`](Self::Axis) evaluation before this one): its titles
    /// placed and measured, then its artists that take part in layout,
    /// unless the program named the artists to measure (not `children`).
    Measured {
        /// The axes.
        axes: ArtistId,
        /// Whether the artists of the axes are measured with it.
        children: bool,
    },
}

/// A tick `Axis.draw` draws, with the parts of it that evaluate
/// transforms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawnTick {
    /// Whether it is a minor tick.
    pub minor: bool,
    /// Its index among the ticks of its level.
    pub index: usize,
    /// Whether its gridline is drawn.
    pub gridline: bool,
    /// Whether its marks on the first and second side are drawn.
    pub marks: [bool; 2],
    /// Whether its labels on the first and second side are shown
    /// (measured, and drawn when the tick has a text).
    pub labels: [bool; 2],
    /// Where it is, in data coordinates.
    pub loc: f64,
    /// Whether its labels have a text.
    pub text: bool,
    /// How far its labels are from the axes' edge, in points
    /// (matplotlib's `Tick._pad`, which their transforms translate by).
    pub pad: f64,
}

/// What a figure hands the evaluations of its draws to (see
/// [`Figure::set_evaluation_hook`]).
#[derive(Clone)]
pub struct EvaluationHook(Rc<dyn Fn(&Evaluation)>);

impl EvaluationHook {
    /// The hook running `hook`.
    pub fn new(hook: impl Fn(&Evaluation) + 'static) -> Self {
        Self(Rc::new(hook))
    }

    /// Hands `evaluation` to the hook.
    pub(crate) fn run(&self, evaluation: &Evaluation) {
        (self.0)(evaluation);
    }
}

impl fmt::Debug for EvaluationHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EvaluationHook(..)")
    }
}

/// The largest magnitude of a tame number: products of three of them
/// (a composed matrix entry) stay finite.
const TAME_LIMIT: f64 = 1e100;

/// Whether `value` is a finite number that matplotlib's matrix
/// arithmetic neither overflows with nor turns into an infinity when it
/// divides by it as a width (zero is tame as a coordinate, not as a
/// width: see [`is_tame_span`]).
pub fn is_tame(value: f64) -> bool {
    value.abs() <= TAME_LIMIT
}

/// Whether the interval from `low` to `high` is one matplotlib's box
/// transforms divide by without an exception: its ends tame and apart by
/// a tame width that is not tiny.
pub fn is_tame_span(low: f64, high: f64) -> bool {
    is_tame(low) && is_tame(high) && (high - low).abs() >= 1.0 / TAME_LIMIT
}

/// Whether every corner of `bbox` is tame.
fn is_tame_bbox(bbox: &Bbox) -> bool {
    [bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1()]
        .into_iter()
        .all(is_tame)
}

impl AxesState {
    /// Whether a leaf of the axes' transforms is wild: its view limits
    /// through its scales are no tame spans (a scale of the program's is
    /// not asked: its last answer for these limits counts, if any), or
    /// the labels of a tick are a wild distance from the axes (their
    /// transforms translate by it), or a setting of its polar projection
    /// is wild (the theta offset, the radial origin, the angle of the
    /// radial labels).
    fn has_wild_leaves(&self) -> bool {
        let wild_projection = self.polar().is_some_and(|polar| {
            let settings = [polar.theta_offset, polar.rlabel_position];
            !settings.into_iter().chain(polar.rorigin).all(is_tame)
        });
        wild_projection
            || [AxisName::X, AxisName::Y].into_iter().any(|name| {
                let axis = self.axis(name);
                let (low, high) = self.view_interval(name);
                let [low, high] = axis.known_scaled_ends([low, high]).unwrap_or([low, high]);
                !is_tame_span(low, high)
                    || [(&axis.major_style, false), (&axis.minor_style, true)]
                        .into_iter()
                        .any(|(style, minor)| {
                            let own = axis.overrides(minor).pads.iter().flatten();
                            !(is_tame(style.pad) && own.copied().all(is_tame))
                        })
            })
    }
}

impl FigureState {
    /// Whether the figure's size or resolution, which every transform of
    /// the graph rests on, is wild.
    fn has_wild_size(&self) -> bool {
        let (width, height) = self.size_inches;
        !(is_tame(self.dpi()) && is_tame(width) && is_tame(height))
    }

    /// Whether a leaf of the transforms of the axes `id` (`axes`) is
    /// wild: one of its own (see [`AxesState::has_wild_leaves`]) or its
    /// place in the figure.
    fn axes_is_wild(&self, id: ArtistId, axes: &AxesState) -> bool {
        axes.has_wild_leaves() || self.axes_bbox(id).is_some_and(|bbox| !is_tame_bbox(&bbox))
    }

    /// Whether a leaf of matplotlib's transform graph of the figure is
    /// wild, so that evaluating the graph may raise floating-point
    /// exceptions: the figure's size or resolution, the place of an axes,
    /// its view limits or the pad of the labels of its ticks.
    pub(crate) fn graph_is_wild(&self) -> bool {
        self.has_wild_size()
            || self.store.ids().any(|id| {
                let axes = self.store.get_as::<AxesState>(id);
                axes.is_some_and(|axes| self.axes_is_wild(id, axes))
            })
    }

    /// Whether matplotlib's evaluations of its transform graph for the
    /// artist `id` may raise floating-point exceptions: the artist's own
    /// transform is wild, or the figure's size is, or a leaf of the
    /// transforms of its axes (the nearest among the artist and its
    /// owners), or for an axes the transform of one of its artists. What
    /// the other axes of the figure rest on does not count: an artist
    /// goes through the transforms of its own.
    pub(crate) fn evaluates_wild(&self, id: ArtistId) -> bool {
        let wild = |id: ArtistId| self.artist(id).is_some_and(Artist::has_wild_transform);
        if wild(id) || self.has_wild_size() {
            return true;
        }
        let mut owner = Some(id);
        while let Some(at) = owner {
            if let Some(axes) = self.artist_as::<AxesState>(at) {
                return self.axes_is_wild(at, axes)
                    || (at == id && axes.children.iter().any(|&child| wild(child)));
            }
            owner = self.artist(at).and_then(|artist| artist.base().parent);
        }
        false
    }

    /// Whether a measurement of the axes `id` reaches the figure's
    /// evaluation hook (see [`measuring_evaluated`](Self::measuring_evaluated)):
    /// the figure's size or a leaf of the transforms of the axes is wild.
    /// Its artists' own transforms are not asked, which would cost every
    /// measurement a look at each of them: a tight box takes in only the
    /// artists that are not clipped to the axes.
    pub(crate) fn measurement_evaluated(&self, id: ArtistId) -> bool {
        self.evaluation_hook.is_some()
            && (self.has_wild_size()
                || self
                    .artist_as::<AxesState>(id)
                    .is_some_and(|axes| self.axes_is_wild(id, axes)))
    }

    /// Hands `evaluations`, which a measurement of the axes `id` stands
    /// for, to the figure's hook (see [`Figure::set_evaluation_hook`]),
    /// the figure lent meanwhile for the hook to read it, when
    /// [`measurement_evaluated`](Self::measurement_evaluated) says so;
    /// `evaluations` is asked only then. Nothing for a state that cannot
    /// be lent.
    pub(crate) fn measuring_evaluated(
        &mut self,
        id: ArtistId,
        evaluations: impl FnOnce(&Self) -> Vec<Evaluation>,
    ) {
        if !self.measurement_evaluated(id) {
            return;
        }
        let Some(hook) = self.evaluation_hook.clone() else {
            return;
        };
        let evaluations = evaluations(self);
        self.lend(|_| {
            for evaluation in &evaluations {
                hook.run(evaluation);
            }
        });
    }
}

impl RawHandle {
    /// Whether matplotlib's evaluations of its transform graph for this
    /// artist may raise floating-point exceptions, so that they are
    /// worth replaying where it joins an axes or is measured: its own
    /// transform (a rectangle's patch transform) is wild, or the
    /// figure's size, or a leaf of the transforms of its axes, or for an
    /// axes the transform of one of its artists.
    /// `false` while the figure is being changed (by code further up the
    /// call stack, which cannot be asked meanwhile).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::patches::Rectangle;
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// let tame = ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 2.0));
    /// let wild = ax.add_patch(Rectangle::new((f64::INFINITY, 0.0), 1.0, 2.0));
    /// assert!(!tame.raw().evaluates_wild() && wild.raw().evaluates_wild());
    /// ```
    pub fn evaluates_wild(&self) -> bool {
        (self.figure_state().try_borrow()).is_some_and(|state| state.evaluates_wild(self.id()))
    }
}

impl Figure {
    /// Hands `hook` each evaluation of matplotlib's transform graph that
    /// the draws of this figure stand for, while a leaf of the graph is
    /// wild (see [`Evaluation`]); `None` removes the
    /// hook. The hook may read the figure; it lives only while the figure
    /// does, and is not archived.
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::figure::Figure;
    /// use pltrs::transforms::{Evaluation, EvaluationHook};
    /// let fig = Figure::with_size(3.0, 2.0, 50.0);
    /// let ax = fig.subplot();
    /// let seen = Rc::new(RefCell::new(Vec::new()));
    /// let sink = Rc::clone(&seen);
    /// fig.set_evaluation_hook(Some(EvaluationHook::new(move |evaluation| {
    ///     sink.borrow_mut().push(evaluation.clone());
    /// })));
    /// fig.canvas().draw()?;
    /// assert!(seen.borrow().is_empty(), "every leaf is tame");
    /// ax.xaxis().set_view_interval(1.0, 1.0, true);
    /// fig.canvas().draw()?;
    /// assert!(seen.borrow().contains(&Evaluation::Axes(ax.id())));
    /// assert!(seen.borrow().iter().any(|e| matches!(e, Evaluation::Axis { .. })));
    /// # Ok::<(), pltrs::figure::CanvasError>(())
    /// ```
    pub fn set_evaluation_hook(&self, hook: Option<EvaluationHook>) {
        self.state_mut().evaluation_hook = hook;
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::artist::ArtistHandle;
    use crate::patches::Rectangle;

    /// The evaluations a draw of `fig` hands its hook.
    fn drawn(fig: &Figure) -> Vec<Evaluation> {
        handed(fig, || fig.canvas().draw().expect("drawn"))
    }

    /// The evaluations `work` on `fig` hands the figure's hook.
    fn handed(fig: &Figure, work: impl FnOnce()) -> Vec<Evaluation> {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&seen);
        fig.set_evaluation_hook(Some(EvaluationHook::new(move |evaluation| {
            sink.borrow_mut().push(evaluation.clone());
        })));
        work();
        fig.set_evaluation_hook(None);
        seen.take()
    }

    /// How many of `evaluations` measure the y axis of an axes.
    fn measured_y_axes(evaluations: &[Evaluation]) -> usize {
        let measures_y = |e: &&Evaluation| {
            matches!(
                e,
                Evaluation::Axis {
                    axis: AxisName::Y,
                    measured: true,
                    ..
                }
            )
        };
        evaluations.iter().filter(measures_y).count()
    }

    #[test]
    fn a_tight_box_hands_over_the_axes_it_measures_then_the_axes() {
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let ax = fig.subplot();
        ax.plot([0.0, 1.0], [1.0, 3.0]).expect("a line");
        let measured = |fig: &Figure| {
            handed(fig, || {
                ax.tight_bbox(false);
            })
        };
        assert_eq!(measured(&fig), [], "tame");
        ax.yaxis().set_view_interval(2.0, 2.0, true);
        let evaluations = measured(&fig);
        let last = Evaluation::Measured {
            axes: ax.id(),
            children: true,
        };
        assert_eq!(evaluations.last(), Some(&last));
        assert_eq!(evaluations.len(), 3, "both axes, then the axes");
        assert_eq!(measured_y_axes(&evaluations), 1);
        // A title is placed past the y axis, measured once more: in tight
        // boxes and in draws.
        ax.set_title("title");
        assert_eq!(measured_y_axes(&measured(&fig)), 2);
        let evaluations = drawn(&fig);
        let started = position(&evaluations, |e| *e == Evaluation::Axes(ax.id()));
        assert_eq!(measured_y_axes(&evaluations[started..=started + 1]), 1);
        // The axis alone.
        let axis = handed(&fig, || {
            ax.yaxis().tight_bbox();
        });
        assert_eq!((axis.len(), measured_y_axes(&axis)), (1, 1));
    }

    #[test]
    fn rectangles_answer_from_their_sides_as_from_their_transforms() {
        use crate::patches::PatchShape;
        let mut rectangle = Rectangle::new((1.0, 2.0), 3.0, 4.0);
        assert!(rectangle.has_tame_numbers());
        rectangle.angle = 30.0;
        assert!(!rectangle.has_tame_numbers(), "its transform is asked");
        let ax = Figure::new().subplot();
        for (wild, shape) in [
            (false, rectangle),
            (true, Rectangle::new((0.0, 0.0), f64::INFINITY, 1.0)),
            (true, Rectangle::new((f64::NAN, 0.0), 1.0, 1.0)),
        ] {
            let patch = ax.add_patch(shape);
            assert_eq!(patch.raw().evaluates_wild(), wild);
        }
    }

    #[test]
    fn an_artist_counts_the_leaves_of_its_own_axes_only() {
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let wild = fig.add_subplot(1, 2, 1).expect("a cell");
        let tame = fig.add_subplot(1, 2, 2).expect("a cell");
        let lines = [&wild, &tame].map(|ax| ax.plot([0.0, 1.0], [1.0, 3.0]).expect("a line"));
        wild.xaxis().set_view_interval(1.0, 1.0, true);
        assert!(wild.raw().evaluates_wild() && lines[0].raw().evaluates_wild());
        assert!(!tame.raw().evaluates_wild() && !lines[1].raw().evaluates_wild());
        assert!(fig.state().graph_is_wild());
    }

    /// Where the evaluation `wanted` chooses is among `evaluations`.
    fn position(evaluations: &[Evaluation], wanted: impl Fn(&Evaluation) -> bool) -> usize {
        evaluations
            .iter()
            .position(wanted)
            .expect("the evaluation is handed over")
    }

    #[test]
    fn a_wild_figure_hands_its_draw_over_in_drawing_order() {
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 3.0]).expect("a line");
        assert_eq!(drawn(&fig), [], "tame");
        ax.yaxis().set_view_interval(2.0, 2.0, true);
        let evaluations = drawn(&fig);
        let started = position(&evaluations, |e| *e == Evaluation::Axes(ax.id()));
        let background = position(&evaluations, |e| *e == Evaluation::Background(ax.id()));
        assert!(started < background);
        let x_axis = position(
            &evaluations,
            |e| matches!(e, Evaluation::Axis { axes, axis: AxisName::X, .. } if *axes == ax.id()),
        );
        let y_axis = position(&evaluations, |e| {
            matches!(
                e,
                Evaluation::Axis {
                    axis: AxisName::Y,
                    ..
                }
            )
        });
        let drawn_line = position(&evaluations, |e| *e == Evaluation::Draw(line.id()));
        assert!(background < x_axis && x_axis < y_axis && y_axis < drawn_line);
        let Evaluation::Axis { ticks, label, .. } = &evaluations[x_axis] else {
            unreachable!("an axis");
        };
        assert!(!label, "no label text");
        let tick = ticks.first().expect("the x axis has ticks");
        assert!(tick.text && tick.labels == [true, false] && tick.marks == [true, false]);
        assert!(!tick.gridline && !tick.minor);
        assert_eq!(tick.pad, 3.5 + 3.5, "the pad past the mark");
    }

    #[test]
    fn a_tame_figure_hands_over_its_wild_patches_only() {
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let ax = fig.subplot();
        ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 2.0));
        let wild = ax.add_patch(Rectangle::new((0.0, 0.0), f64::INFINITY, 2.0));
        assert_eq!(drawn(&fig), [Evaluation::Draw(wild.id())]);
        wild.set_visible(false);
        assert_eq!(drawn(&fig), [], "a hidden artist evaluates nothing");
    }

    #[test]
    fn a_hidden_axis_evaluates_nothing() {
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let ax = fig.subplot();
        ax.xaxis().set_view_interval(1.0, 1.0, true);
        ax.yaxis().set_visible(false);
        let evaluations = drawn(&fig);
        assert!(evaluations.iter().any(|e| matches!(
            e,
            Evaluation::Axis {
                axis: AxisName::X,
                ..
            }
        )));
        assert!(!evaluations.iter().any(|e| matches!(
            e,
            Evaluation::Axis {
                axis: AxisName::Y,
                ..
            }
        )));
    }

    #[test]
    fn tame_numbers_are_finite_and_moderate() {
        assert!(is_tame(0.0) && is_tame(-3.5e99) && is_tame(1e-300));
        assert!(!is_tame(f64::NAN) && !is_tame(f64::INFINITY) && !is_tame(1e101));
    }

    #[test]
    fn tame_spans_have_a_width_to_divide_by() {
        assert!(is_tame_span(0.0, 1.0) && is_tame_span(5.0, -5.0));
        assert!(!is_tame_span(1.0, 1.0), "no width");
        assert!(!is_tame_span(0.0, 1e-200), "a width whose inverse is huge");
        assert!(!is_tame_span(0.0, f64::INFINITY) && !is_tame_span(f64::NAN, 1.0));
    }

    #[test]
    fn a_figure_is_wild_while_a_view_has_no_width() {
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let ax = fig.subplot();
        assert!(!fig.state().graph_is_wild());
        ax.yaxis().set_view_interval(2.0, 2.0, true);
        assert!(fig.state().graph_is_wild());
        ax.yaxis().set_view_interval(2.0, 3.0, true);
        assert!(!fig.state().graph_is_wild());
    }

    #[test]
    fn a_tick_with_an_infinite_pad_makes_the_figure_wild() {
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let ax = fig.subplot();
        ax.xaxis().set_tick_pad(false, 1, f64::INFINITY);
        assert!(fig.state().graph_is_wild());
        ax.xaxis().set_tick_pad(false, 1, 4.0);
        assert!(!fig.state().graph_is_wild());
    }

    #[test]
    fn polar_settings_that_are_no_numbers_make_the_figure_wild() {
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let ax = fig.add_polar_subplot(1, 1, 1).expect("a subplot");
        assert!(!fig.state().graph_is_wild());
        ax.set_rlabel_position(f64::INFINITY);
        assert!(fig.state().graph_is_wild());
    }

    #[test]
    fn a_log_axis_is_judged_by_its_scaled_limits() {
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let ax = fig.subplot();
        ax.set_yscale("log").expect("a scale");
        ax.yaxis().set_view_interval(1e-250, 1e250, true);
        assert!(!fig.state().graph_is_wild(), "500 decades are tame");
        ax.yaxis().set_view_interval(1e-250, 1e-250, true);
        assert!(fig.state().graph_is_wild(), "no decades at all");
    }
}
