use plt_core::Rgba;

use super::PlotError;
use crate::artist::ArtistHandle;
use crate::figure::Figure;
use crate::lines::DrawStyle;
use crate::testing::{Call, RecordingRenderer};

fn axes() -> (Figure, super::Axes) {
    let fig = Figure::new();
    let ax = fig
        .add_axes([0.125, 0.11, 0.775, 0.77])
        .expect("a valid rectangle");
    (fig, ax)
}

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-12 && (a.1 - b.1).abs() < 1e-12
}

#[test]
fn plot_autoscales_like_matplotlib() {
    // spec/goldens/figures/basic/line.json
    let (_, ax) = axes();
    ax.plot([1.0, 2.0, 3.0, 4.0], [1.0, 4.0, 2.0, 3.0])
        .expect("same length");
    assert!(close(ax.xlim(), (0.85, 4.15)));
    assert!(close(ax.ylim(), (0.85, 4.15)));
}

#[test]
fn round_number_limits_fit_the_ticks_the_axis_has_room_for() {
    // spec/goldens/figures/rc/axes_autolimit_mode.json: on a 3 in wide
    // axes five x ticks fit, so the limits round to steps of 2.
    crate::rc::context(|| {
        crate::rc::set("axes.autolimit_mode", "round_numbers").expect("valid");
        let fig = Figure::with_size(3.0, 2.0, 100.0);
        let ax = fig.subplot();
        ax.plot([0.0, 6.0], [-1.0, 1.0]).expect("same length");
        assert_eq!(ax.xlim(), (-2.0, 8.0));
        assert_eq!(ax.ylim(), (-2.0, 2.0));
    });
}

#[test]
fn empty_axes_keep_unit_limits() {
    let (_, ax) = axes();
    assert_eq!(ax.xlim(), (0.0, 1.0));
}

#[test]
fn explicit_limits_stop_autoscaling() {
    let (_, ax) = axes();
    ax.set_xlim(10.0, 0.0).expect("finite limits");
    ax.plot([1.0, 2.0], [3.0, 4.0]).expect("same length");
    assert_eq!(ax.xlim(), (10.0, 0.0));
    assert!(close(ax.ylim(), (2.95, 4.05)));
}

#[test]
fn lines_take_colours_from_the_cycle() {
    let (_, ax) = axes();
    let first = ax.plot_y([1.0, 2.0]);
    let second = ax.plot_y([2.0, 1.0]);
    assert_eq!(first.rgba(), Ok(Rgba::from_rgba8([0x1f, 0x77, 0xb4, 0xff])));
    assert_eq!(
        second.rgba(),
        Ok(Rgba::from_rgba8([0xff, 0x7f, 0x0e, 0xff]))
    );
    assert_eq!(first.zorder(), 2.0);
}

#[test]
fn mismatched_lengths_are_rejected() {
    let (_, ax) = axes();
    assert_eq!(
        ax.plot([1.0], [1.0, 2.0]),
        Err(PlotError::LengthMismatch { x: 1, y: 2 })
    );
}

#[test]
fn drawing_maps_data_into_the_axes_box() {
    let (fig, ax) = axes();
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 10.0))
        .expect("finite limits");
    ax.plot([0.0, 10.0], [0.0, 10.0]).expect("same length");
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).expect("drawable");
    // The line (z-order 2) is drawn before the spines (z-order 2.5).
    let points = renderer
        .calls
        .iter()
        .find_map(|call| match call {
            Call::Path {
                points,
                fill: None,
                linewidth,
            } if (*linewidth - 1.5).abs() < 1e-12 => Some(points),
            _ => None,
        })
        .expect("the line is drawn");
    assert!((points[0].x - 80.0).abs() < 1e-9 && (points[1].y - 422.4).abs() < 1e-9);
}

#[test]
fn invalid_colours_are_reported_not_drawn() {
    let (fig, ax) = axes();
    let line = ax.plot_y([1.0, 2.0]);
    line.set_color("not-a-colour");
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    let errors = fig.draw(&mut renderer).expect_err("bad colour");
    assert_eq!(errors.0.len(), 1);
    assert_eq!(errors.0[0].artist, line.id());
}

#[test]
fn draw_style_changes_the_path() {
    let (_, ax) = axes();
    let line = ax
        .plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.0])
        .expect("same length");
    line.set_drawstyle(DrawStyle::StepsPost);
    assert_eq!(line.path().map(|p| p.len()), Ok(5));
}

#[test]
fn bar_containers_report_bottoms_tops_and_centres() {
    use super::{BarAlign, BarOptions};

    let (_fig, ax) = axes();
    let options = BarOptions {
        base: vec![0.0, 1.0, 0.0].into(),
        thickness: 0.6.into(),
        align: BarAlign::Edge,
        ..BarOptions::default()
    };
    let bars = ax.bar([1.0, 2.0, 3.0], [0.5, -0.4, 0.8], options).unwrap();
    assert_eq!(bars.bottoms(), [0.0, 1.0, 0.0]);
    assert_eq!(bars.tops(), [0.5, 0.6, 0.8]);
    assert_eq!(bars.position_centers(), [1.3, 2.3, 3.3]);
    let options = BarOptions {
        base: vec![1.0, 0.0].into(),
        ..BarOptions::default()
    };
    let horizontal = ax.barh([1.0, 2.0], [2.0, -3.0], options).unwrap();
    assert_eq!(horizontal.bottoms(), [1.0, 0.0]);
    assert_eq!(horizontal.tops(), [3.0, -3.0]);
    assert_eq!(horizontal.position_centers(), [1.0, 2.0]);
}

#[test]
fn segments_with_a_missing_coordinate_are_missing_entirely() {
    // matplotlib's `_combine_masks` in `vlines` and `hlines`.
    let (_fig, ax) = axes();
    let low = [0.0, 0.0, f64::NAN, 0.0];
    ax.vlines([1.0, 2.0, 3.0, f64::NAN], low, [1.0, 9.0, 20.0, 30.0])
        .unwrap();
    let limits = ax.data_lim().bbox;
    assert_eq!((limits.x0(), limits.x1()), (1.0, 2.0));
    assert_eq!((limits.y0(), limits.y1()), (0.0, 9.0));
}

#[test]
fn decreasing_function_scales_keep_the_axis_orientation() {
    // matplotlib's `TransformedBbox` keeps the orientation of the view.
    use crate::scale::{FuncScale, FuncTransform};
    use crate::transforms::Point;

    let fig = Figure::with_size(2.0, 2.0, 72.0);
    let ax = fig
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    let inverse = FuncTransform::new(f64::recip, f64::recip);
    ax.set_scale(
        super::AxisName::X,
        Box::new(FuncScale::from_transform(inverse)),
    );
    ax.set_xlim(0.1, 0.5).expect("finite limits");
    let data = fig.resolve_transform(&ax.trans_data()).unwrap();
    let x = |value: f64| data.transform_point(Point::new(value, 0.0)).x;
    assert!(x(0.5).abs() < 1e-9, "the larger value on the left");
    assert!((x(0.1) - 144.0).abs() < 1e-9);
}

#[test]
fn only_artists_without_a_clip_path_clip_to_the_axes_box() {
    use crate::artist::ClipPathSpec;
    use crate::patches::{PatchProps, Rectangle};
    use plt_core::Path;

    let (_, ax) = axes();
    let unit = || Rectangle::new((0.0, 0.0), 1.0, 1.0);
    let boxed = ax.add_patch(unit());
    assert_eq!(boxed.raw().with_base(|b| b.clip_box), Some(ax.id()));
    let clip = ClipPathSpec::Path {
        path: Path::unit_rectangle(),
        transform: ax.trans_data(),
    };
    let clipped = ax.add_patch_with(unit(), PatchProps::default().clip_path(clip));
    assert_eq!(clipped.raw().with_base(|b| b.clip_box), None);
    assert!(clipped.raw().with_base(|b| b.clip_path.is_some()));
}
