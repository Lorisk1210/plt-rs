//! Axes drawn through projections defined in the tests: a sheared
//! rectangle redefining its ticks and spines, a disc refusing changes, and
//! a projection whose set-up fails; [`creation`] makes them with options
//! wherever axes are made, [`navigation`] pans and zooms projections that
//! do it themselves.

mod creation;
mod navigation;

use std::error::Error;

use plt_core::{Path, Point};
use plt_text::{HAlign, VAlign};

use super::*;
use crate::artist::ArtistHandle;
use crate::axes::{Adjustable, AspectError, Axes, AxesState, AxisName, ProjectionKind, Sides};
use crate::figure::Figure;
use crate::layout::{ConstrainedLayout, LayoutEngine, TightLayout};
use crate::navigation::{DragKey, NavMode, Navigation, PointerButton};
use crate::scale::ScaleName;
use crate::spines::SpineSide;
use crate::testing::{RecordingRenderer, close_points};
use crate::transforms::{Affine2D, ResolvedTransform};

/// A shear of 45° to the right, in axes fractions.
fn shear() -> ResolvedTransform {
    ResolvedTransform::affine(Affine2D::identity().skew_deg(45.0, 0.0))
}

/// Rectilinear axes sheared to the right, ticking x over twice the view
/// interval, drawing negative x ticks on the second side only, with its
/// top spine spanning half the view; it counts its set-ups.
#[derive(Debug, Default)]
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
#[derive(Debug)]
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

    fn data_ratio(&self) -> Option<f64> {
        Some(1.0)
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
#[derive(Debug)]
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
        .transform_bbox(&plt_core::Bbox::UNIT)
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
}

#[test]
fn layouts_make_room_for_the_labels() {
    let (fig, ax) = sheared();
    ax.set_ylabel("height");
    let before = ax.position();
    fig.tight_layout(&TightLayout::default()).unwrap();
    assert_ne!(ax.position(), before);
    let tight = ax.tight_bbox(true).unwrap();
    let box_extent = fig
        .resolve_transform(&ax.trans_axes())
        .unwrap()
        .transform_bbox(&plt_core::Bbox::UNIT);
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
