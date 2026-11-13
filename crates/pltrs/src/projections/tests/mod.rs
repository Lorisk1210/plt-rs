//! Axes drawn through projections defined in the tests: a sheared
//! rectangle redefining its ticks and spines, a disc refusing changes, and
//! a projection whose set-up fails; [`creation`] makes them with options
//! wherever axes are made, [`navigation`] pans and zooms projections that
//! do it themselves.

mod creation;
mod navigation;

use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use pltrs_core::{Bbox, Path, Point};
use pltrs_text::{HAlign, VAlign};

use super::*;
use crate::artist::ArtistHandle;
use crate::axes::{Adjustable, AspectError, Axes, AxesState, AxisName, ProjectionKind, Sides};
use crate::figure::Figure;
use crate::layout::{ConstrainedLayout, LayoutEngine, TightLayout};
use crate::navigation::{DragKey, NavMode, Navigation, PointerButton};
use crate::rc::Aspect;
use crate::scale::ScaleName;
use crate::spines::SpineSide;
use crate::testing::{RecordingRenderer, close_points};
use crate::transforms::{Affine2D, PointMap, ResolvedTransform, Step};

/// A shear of 45° to the right, in axes fractions.
fn shear() -> ResolvedTransform {
    ResolvedTransform::affine(Affine2D::identity().skew_deg(45.0, 0.0))
}

/// Rectilinear axes sheared to the right, ticking x over twice the view
/// interval, drawing negative x ticks on the second side only, with its
/// top spine spanning half the view; it counts its set-ups.
#[derive(Clone, Debug, Default)]
struct Sheared {
    set_ups: usize,
}

impl Projection for Sheared {
    fn name(&self) -> &'static str {
        "sheared"
    }

    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        view.rectilinear().then(shear())
    }

    fn set_up(&mut self, axes: &Axes) -> Result<(), Box<dyn Error>> {
        self.set_ups += 1;
        axes.set_xlim(0.0, 10.0)?.set_ylim(0.0, 10.0)?;
        Ok(())
    }

    fn axis_transform(
        &self,
        view: &ProjectionView<'_>,
        axis: AxisName,
    ) -> Option<ResolvedTransform> {
        let across = ResolvedTransform::blend(view.rectilinear(), ResolvedTransform::identity());
        (axis == AxisName::X).then(|| across.then(shear()))
    }

    fn tick_interval(&self, view: &ProjectionView<'_>, axis: AxisName) -> (f64, f64) {
        let (v0, v1) = view.interval(axis);
        match axis {
            AxisName::X => (2.0 * v0 - v1, v1),
            AxisName::Y => (v0, v1),
        }
    }

    fn tick_sides(&self, _: &ProjectionView<'_>, _: AxisName, loc: f64) -> Sides {
        Sides {
            first: loc >= 0.0,
            second: true,
        }
    }

    fn spine_order(&self) -> [SpineSide; 4] {
        [
            SpineSide::Top,
            SpineSide::Top,
            SpineSide::Left,
            SpineSide::Bottom,
        ]
    }

    fn spine_interval(&self, view: &ProjectionView<'_>, side: SpineSide) -> Option<(f64, f64)> {
        let (v0, v1) = view.interval(AxisName::X);
        (side == SpineSide::Top).then_some((f64::midpoint(v0, v1), v1))
    }
}

/// The unit disc around the origin: fixed limits and scales, no
/// navigation, a fixed data ratio, all labels at the centre and its own
/// status-bar text.
#[derive(Clone, Debug)]
struct Disc;

impl Projection for Disc {
    fn name(&self) -> &'static str {
        "disc"
    }

    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        view.rectilinear()
    }

    fn set_up(&mut self, axes: &Axes) -> Result<(), Box<dyn Error>> {
        axes.set_xlim(-1.0, 1.0)?.set_ylim(-1.0, 1.0)?;
        Ok(())
    }

    fn tick_label(
        &self,
        _: &ProjectionView<'_>,
        _: SpineSide,
        _: f64,
    ) -> Option<TickLabelPlacement> {
        Some(TickLabelPlacement {
            anchor: Point::new(0.5, 0.5),
            offset: Point::new(3.0, 0.0),
            halign: HAlign::Left,
            valign: VAlign::Center,
        })
    }

    fn outline(&self) -> AxesOutline {
        AxesOutline::Path(Path::circle(Point::new(0.5, 0.5), 0.5))
    }

    fn fixed_limits(&self) -> bool {
        true
    }

    fn linear_only(&self) -> bool {
        true
    }

    fn data_ratio(&self, _: &ProjectionView<'_>) -> Option<f64> {
        Some(1.0)
    }

    fn fixes_data_ratio(&self) -> bool {
        true
    }

    fn can_zoom(&self) -> bool {
        false
    }

    fn can_pan(&self) -> bool {
        false
    }

    fn format_coord(&self, x: f64, y: f64) -> Option<String> {
        Some(format!("{x:.1} {y:.1}"))
    }
}

/// A projection that cannot set its axes up.
#[derive(Clone, Debug)]
struct Broken;

impl Projection for Broken {
    fn name(&self) -> &'static str {
        "broken"
    }

    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        view.rectilinear()
    }

    fn set_up(&mut self, _: &Axes) -> Result<(), Box<dyn Error>> {
        Err("no room".into())
    }
}

/// The plane as it is, noting the interpolation steps of the lines (the
/// paths of two vertices: gridlines and spines, not the marks of ticks)
/// it is asked to map (a transform of a program's bends a path by them).
#[derive(Debug)]
struct NotingSteps(Arc<Mutex<Vec<usize>>>);

impl PointMap for NotingSteps {
    fn forward(&self, point: Point) -> Point {
        point
    }

    fn inverse(&self, point: Point) -> Point {
        point
    }

    fn transform_path(&self, path: &Path) -> Path {
        if path.vertices().len() == 2 {
            self.0.lock().unwrap().push(path.interpolation_steps());
        }
        path.clone()
    }
}

/// Rectilinear axes whose class made one spine, around its outline (the
/// `'geo'` spine of matplotlib's geographic axes, drawn as the top
/// spine), whose x axis maps its gridlines through a map of the plane and
/// whose bottom side lies a whole axes height below the axes.
#[derive(Clone, Debug, Default)]
struct OneSpine {
    steps: Arc<Mutex<Vec<usize>>>,
}

impl Projection for OneSpine {
    fn name(&self) -> &'static str {
        "one spine"
    }

    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        view.rectilinear()
    }

    fn axis_transform(
        &self,
        view: &ProjectionView<'_>,
        axis: AxisName,
    ) -> Option<ResolvedTransform> {
        let noting = Step::Plane(Rc::new(NotingSteps(Arc::clone(&self.steps))));
        let lowered = Affine2D::identity().scale(1.0, 2.0).translate(0.0, -1.0);
        let across = ResolvedTransform::blend(view.rectilinear(), ResolvedTransform::identity());
        (axis == AxisName::X).then(|| ResolvedTransform::with_step(noting, lowered).then(across))
    }

    fn spines(&self) -> Vec<SpineSide> {
        vec![SpineSide::Top]
    }
}

/// Rectilinear axes giving their own inverse of the data transform: the
/// rectilinear one, landing a hundred to the right of where the inverse
/// of the whole transform lands.
#[derive(Clone, Debug)]
struct OwnInverse;

impl Projection for OwnInverse {
    fn name(&self) -> &'static str {
        "own inverse"
    }

    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        view.rectilinear()
    }

    fn axes_to_data(&self, view: &ProjectionView<'_>) -> Option<ResolvedTransform> {
        let shifted = ResolvedTransform::affine(Affine2D::identity().translate(100.0, 0.0));
        Some(view.rectilinear().inverted()?.then(shifted))
    }
}

/// Rectilinear axes noting where their hooks are told the axes is.
#[derive(Clone, Debug, Default)]
struct NotingPosition {
    seen: Arc<Mutex<Vec<Bbox>>>,
}

impl Projection for NotingPosition {
    fn name(&self) -> &'static str {
        "noting position"
    }

    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        self.seen.lock().unwrap().push(view.position());
        view.rectilinear()
    }
}

/// A figure with sheared axes filling its default subplot area.
fn sheared() -> (Figure, Axes) {
    let kind = register(Sheared::default).unwrap();
    let fig = Figure::new();
    let ax = fig.add_subplot_with_projection(1, 1, 1, kind).unwrap();
    (fig, ax)
}

/// A figure with disc axes filling its default subplot area.
fn disc() -> (Figure, Axes) {
    let kind = register(|| Disc).unwrap();
    let fig = Figure::new();
    let ax = fig.subplot_with_projection(kind).unwrap();
    (fig, ax)
}

/// The display point of `point` in `transform` of `fig`.
fn display(fig: &Figure, transform: &crate::transforms::Transform, point: Point) -> Point {
    fig.resolve_transform(transform)
        .unwrap()
        .transform_point(point)
}

#[test]
fn data_and_gridlines_go_through_the_projection() {
    let (fig, ax) = sheared();
    assert_eq!(ax.projection(), ProjectionKind::Registered("sheared"));
    assert_eq!(ax.with_projection(|p: &Sheared| p.set_ups), Some(1));
    // The top-left data corner is sheared onto the top-right axes corner,
    // and so is the top of the x gridline at 0 (an `axvline`).
    let corner = display(&fig, &ax.trans_axes(), Point::new(1.0, 1.0));
    let data = display(&fig, &ax.trans_data(), Point::new(0.0, 10.0));
    let gridline = display(&fig, &ax.trans_xaxis(), Point::new(0.0, 1.0));
    assert!(close_points(data, corner) && close_points(gridline, corner));
    // The y axis keeps its default: level gridlines.
    let level = display(&fig, &ax.trans_yaxis(), Point::new(0.0, 10.0));
    let top_left = display(&fig, &ax.trans_axes(), Point::new(0.0, 1.0));
    assert!(close_points(level, top_left));
    for document in [
        fig.to_svg().unwrap(),
        fig.to_ps().unwrap(),
        fig.to_pgf().unwrap(),
    ] {
        assert_ne!(document, "");
    }
    assert!(!fig.to_pdf().unwrap().is_empty() && !fig.to_png().unwrap().is_empty());
}

#[test]
fn ticks_follow_the_tick_interval_and_sides() {
    let (_, ax) = sheared();
    let locs: Vec<f64> = (ax.ticks(AxisName::X).unwrap().major.iter())
        .map(|tick| tick.loc)
        .collect();
    assert_eq!(locs.first(), Some(&-10.0));
    assert_eq!(locs.last(), Some(&10.0));
    let sides = |loc| {
        ax.raw()
            .with::<AxesState, _>(|state| state.drawn_tick_sides(AxisName::X, loc))
    };
    assert_eq!(
        sides(-4.0),
        Sides {
            first: false,
            second: true
        }
    );
    assert!(sides(4.0).first);
    let rectilinear = Figure::new().subplot();
    let both = rectilinear
        .raw()
        .with::<AxesState, _>(|state| state.drawn_tick_sides(AxisName::X, -4.0));
    assert!(both.first && both.second);
}

#[test]
fn spines_span_and_stack_as_the_projection_says() {
    let (fig, ax) = sheared();
    let width = fig
        .resolve_transform(&ax.trans_axes())
        .unwrap()
        .transform_bbox(&pltrs_core::Bbox::UNIT)
        .width();
    let left = display(&fig, &ax.trans_axes(), Point::new(0.0, 0.0)).x;
    let top = ax.spines().top.line_extent().unwrap();
    // From x = 5 to 10 at the top, sheared one axes width to the right.
    assert!((top.x0() - (left + 1.5 * width)).abs() < 1e-6);
    assert!((top.x1() - (left + 2.0 * width)).abs() < 1e-6);
    let bottom = ax.spines().bottom.line_extent().unwrap();
    assert!((bottom.x0() - left).abs() < 1e-6);
    let order = ax
        .raw()
        .with::<AxesState, _>(AxesState::spines_in_children_order);
    let sides: Vec<SpineSide> = order
        .iter()
        .map(|&id| {
            ax.spines()
                .iter()
                .find(|spine| spine.id() == id)
                .unwrap()
                .side()
        })
        .collect();
    assert_eq!(
        sides,
        [
            SpineSide::Top,
            SpineSide::Left,
            SpineSide::Bottom,
            SpineSide::Right
        ]
    );
}

#[test]
fn outlines_clip_pick_and_frame_the_axes() {
    let (fig, ax) = disc();
    let centre = display(&fig, &ax.trans_axes(), Point::new(0.5, 0.5));
    let corner = display(&fig, &ax.trans_axes(), Point::new(0.02, 0.02));
    assert!(ax.contains_point(centre) && !ax.contains_point(corner));
    assert_eq!(crate::navigation::axes_at(&fig, centre), Some(ax.clone()));
    assert!(ax.spines().top.line_extent().is_some());
    assert!(ax.spines().left.line_extent().is_none());
    assert!(ax.spines().top.visible());
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    assert!(fig.draw(&mut renderer).is_ok());
}

#[test]
fn gridlines_ask_for_the_steps_matplotlib_gives_them() {
    let steps = Arc::new(Mutex::new(Vec::new()));
    let noted = Arc::clone(&steps);
    let kind = register(move || OneSpine {
        steps: Arc::clone(&noted),
    })
    .unwrap();
    let fig = Figure::new();
    let ax = fig.subplot_with_projection(kind).unwrap();
    ax.grid(true);
    ax.set_xticks([0.25, 0.75]);
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).unwrap();
    // matplotlib's `GRIDLINE_INTERPOLATION_STEPS` on the two gridlines;
    // the spines are lines of one step.
    let steps = steps.lock().unwrap();
    let asked: Vec<usize> = (steps.iter().copied())
        .filter(|&steps| steps != 1)
        .collect();
    assert_eq!(asked, [180, 180], "{steps:?}");
}

#[test]
fn the_hooks_see_the_axes_where_its_aspect_puts_it() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let noted = Arc::clone(&seen);
    let kind = register(move || NotingPosition {
        seen: Arc::clone(&noted),
    })
    .unwrap();
    let fig = Figure::new();
    let ax = fig.subplot_with_projection(kind).unwrap();
    let whole = ax.position();
    // A third as high as wide: the box shrinks before anything is drawn
    // through the projection.
    ax.set_aspect(Aspect::Ratio(1.0 / 3.0)).unwrap();
    seen.lock().unwrap().clear();
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).unwrap();
    let seen = seen.lock().unwrap();
    let last = seen.last().expect("asked while drawing");
    assert_eq!(*last, ax.position());
    assert!(last.height() < whole.height() && (last.width() - whole.width()).abs() < 1e-12);
}

#[test]
fn a_label_goes_beyond_the_axes_on_a_side_without_a_spine() {
    let kind = register(OneSpine::default).unwrap();
    let fig = Figure::new();
    let ax = fig.subplot_with_projection(kind).unwrap();
    ax.set_xticks(Vec::<f64>::new());
    ax.set_xlabel("below");
    let bottom = display(&fig, &ax.trans_axes(), Point::new(0.0, 0.0)).y;
    let pad = ax.xaxis().label_pad() * fig.dpi() / 72.0;
    let label = ax.xaxis().label_extent().unwrap();
    // matplotlib's `spines.get("bottom", self.axes)`: no spine of the class
    // frames the bottom, so the label hangs below the axes, not below
    // where the x axis draws its side (a whole axes height further down).
    assert!(
        (label.y1() - (bottom - pad)).abs() < 1.0,
        "{label:?} {bottom} {pad}"
    );
}

#[test]
fn labels_go_where_the_projection_puts_them() {
    let (fig, ax) = disc();
    let centre = display(&fig, &ax.trans_axes(), Point::new(0.5, 0.5));
    let labels = ax.xaxis().ticklabel_extents();
    assert_eq!(labels.len(), ax.ticks(AxisName::X).unwrap().major.len());
    for label in labels {
        assert!((label.x0() - (centre.x + 3.0)).abs() < 1e-6);
    }
}

#[test]
fn fixed_projections_refuse_changes() {
    let (fig, ax) = disc();
    assert_eq!(ax.xlim(), (-1.0, 1.0));
    ax.set_xlim(0.0, 1.0).unwrap();
    ax.set_yscale("log").unwrap();
    assert_eq!((ax.xlim(), ax.yscale()), ((-1.0, 1.0), ScaleName::Linear));
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    let errors = fig.draw(&mut renderer).unwrap_err().0;
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|error| error.message.contains("disc")));
    assert_eq!(
        ax.set_adjustable(Adjustable::Datalim).err(),
        Some(AspectError::FixedDataRatio(ProjectionKind::Registered(
            "disc"
        )))
    );
    assert_eq!(ax.data_ratio(), 1.0);
    assert!(!ax.can_zoom() && !ax.can_pan());
    assert_eq!(ax.format_coord(0.5, 0.25), "0.5 0.2");
    let (_, sheared) = sheared();
    assert!(sheared.can_zoom() && sheared.can_pan());
    sheared.set_adjustable(Adjustable::Datalim).unwrap();
}

#[test]
fn fixed_limits_let_the_axis_change_them() {
    let (fig, ax) = disc();
    ax.xaxis().set_inverted(true);
    let limits = crate::axes::Limits::new(-0.5, 0.5);
    assert_eq!(ax.yaxis().set_limits(limits, true), Ok((-0.5, 0.5)));
    assert_eq!((ax.xlim(), ax.ylim()), ((1.0, -1.0), (-0.5, 0.5)));
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).unwrap();
    ax.set_xlim(0.0, 1.0).unwrap();
    let errors = fig.draw(&mut renderer).unwrap_err().0;
    assert_eq!(errors[0].kind, crate::artist::DrawErrorKind::InvalidType);
    assert_eq!(
        errors[0].message,
        "Changing axes limits of a disc projection is not supported."
    );
}

#[test]
fn panning_moves_the_view_through_the_inverse() {
    let pan = |fig: &Figure, ax: &Axes| {
        let start = display(fig, &ax.trans_axes(), Point::new(0.5, 0.5));
        let end = start + Point::new(50.0, 0.0);
        let mut navigation = Navigation::new();
        navigation.toggle(fig, NavMode::Pan).unwrap();
        navigation.press(fig, start, PointerButton::Left);
        navigation.drag(end, DragKey::None);
        navigation.release(fig, end, DragKey::None);
        ax.xlim()
    };
    let (fig, ax) = sheared();
    let (x0, x1) = pan(&fig, &ax);
    assert!(x0 < 0.0 && (x1 - x0 - 10.0).abs() < 1e-9);
    let (fig, ax) = disc();
    assert_eq!(pan(&fig, &ax), (-1.0, 1.0));
    let readout =
        Navigation::new().readout(&fig, display(&fig, &ax.trans_data(), Point::new(0.5, 0.25)));
    assert!(readout.contains("0.5 0.2"), "{readout}");
    // A projection inverting its transform itself is panned through its
    // own inverse (the frozen one of the start of the pan, there and
    // back: its shift cancels out of the limits' width, not of where a
    // box lands).
    let fig = Figure::new();
    let ax = fig
        .subplot_with_projection(register(|| OwnInverse).unwrap())
        .unwrap();
    let (x0, x1) = pan(&fig, &ax);
    assert!((x1 - x0 - 1.0).abs() < 1e-9 && x0 > 99.0, "{x0} {x1}");
}

#[test]
fn layouts_make_room_for_the_labels() {
    let (fig, ax) = sheared();
    // The top spine, sheared a whole axes width to the right, would count
    // as the horizontal line it is (matplotlib's `Bbox._is_finite`) and
    // leave the axes no room.
    ax.spines().top.set_visible(false);
    ax.set_ylabel("height");
    let before = ax.position();
    fig.tight_layout(TightLayout::default()).unwrap();
    assert_ne!(ax.position(), before);
    let tight = ax.tight_bbox(true).unwrap();
    let box_extent = fig
        .resolve_transform(&ax.trans_axes())
        .unwrap()
        .transform_bbox(&pltrs_core::Bbox::UNIT);
    assert!(tight.x0() < box_extent.x0());

    let (fig, ax) = disc();
    let before = ax.position();
    let engine = LayoutEngine::Constrained(ConstrainedLayout::default());
    fig.set_layout_engine(Some(engine)).unwrap();
    fig.layout();
    assert_ne!(ax.position(), before);
}

#[test]
fn clearing_sets_the_axes_up_again() {
    let (_, ax) = sheared();
    ax.with_projection_mut(|p: &mut Sheared| p.set_ups = 10);
    ax.set_xlim(2.0, 3.0).unwrap();
    ax.clear();
    assert_eq!(ax.projection(), ProjectionKind::Registered("sheared"));
    assert_eq!(ax.with_projection(|p: &Sheared| p.set_ups), Some(11));
    assert_eq!(ax.xlim(), (0.0, 10.0));
}

#[test]
fn projections_are_reached_by_their_type() {
    let (fig, ax) = sheared();
    assert_eq!(ax.with_projection(|_: &Disc| ()), None);
    assert_eq!(ax.with_projection_mut(|_: &mut Disc| ()), None);
    assert_eq!(fig.subplot().with_projection(|_: &Sheared| ()), None);
}

#[test]
fn failures_add_no_axes() {
    let fig = Figure::new();
    assert_eq!(
        fig.subplot_with_projection(ProjectionKind::Registered("unheard-of")),
        Err(ProjectionError::Unknown("unheard-of".into()))
    );
    let kind = register(|| Broken).unwrap();
    assert_eq!(
        fig.add_axes_with_projection([0.1, 0.1, 0.5, 0.5], kind),
        Err(ProjectionError::SetUp {
            projection: "broken",
            message: "no room".into()
        })
    );
    assert!(matches!(
        fig.add_subplot_with_projection(1, 1, 2, kind),
        Err(ProjectionError::Grid(_))
    ));
    assert!(matches!(
        fig.add_axes_with_projection([0.1, 0.1, -0.5, 0.5], ProjectionKind::Polar),
        Err(ProjectionError::Rect(_))
    ));
    assert_eq!(fig.axes(), []);
}

#[test]
fn built_in_kinds_are_made_alike() {
    let fig = Figure::new();
    let spec = fig.figure_grid(1, 2).unwrap().numbered(2).unwrap();
    for kind in ProjectionKind::BUILTIN {
        let ax = fig.add_subplot_spec_with_projection(&spec, kind).unwrap();
        assert_eq!(ax.projection(), kind);
    }
    assert!(fig.axes()[2].as_geo().is_some());
}

#[test]
fn pyplot_reuses_registered_axes() {
    use crate::pyplot;

    let kind = register(Sheared::default).unwrap();
    let ax = pyplot::subplot_with_projection(1, 2, 1, kind).unwrap();
    assert_eq!(pyplot::subplot_with_projection(1, 2, 1, kind).unwrap(), ax);
    let plain = pyplot::subplot(1, 2, 1).unwrap();
    assert_eq!(plain, ax);
    let other = pyplot::axes_with_projection(Some([0.6, 0.1, 0.3, 0.3]), kind).unwrap();
    assert_eq!(other.projection(), kind);
    pyplot::close_all();
}
