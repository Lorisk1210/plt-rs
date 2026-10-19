use plt_core::{Bbox, Point, Rgba};

use super::Figure;
use crate::artist::ArtistHandle;
use crate::testing::{Call, RecordingRenderer};

fn close(a: Point, b: Point) -> bool {
    (a - b).length() < 1e-9
}

#[test]
fn figure_geometry() {
    let fig = Figure::with_size(4.0, 3.0, 50.0);
    assert_eq!(fig.size_pixels(), (200.0, 150.0));
    fig.set_dpi(100.0);
    assert_eq!(fig.size_pixels(), (400.0, 300.0));
}

#[test]
fn draw_emits_figure_then_axes_backgrounds() {
    let fig = Figure::new();
    fig.set_facecolor(Rgba::opaque(0.9, 0.9, 0.9));
    let ax = fig.add_axes([0.125, 0.11, 0.775, 0.77]);
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).expect("drawable");
    assert!(!fig.is_stale());

    let [
        Call::Path {
            points: figure_rect,
            fill: figure_fill,
            ..
        },
        Call::Path {
            points: axes_rect, ..
        },
        ..,
    ] = renderer.calls.as_slice()
    else {
        panic!(
            "expected two background rectangles, got {:?}",
            renderer.calls
        );
    };
    assert_eq!(*figure_fill, Some(Rgba::opaque(0.9, 0.9, 0.9)));
    assert_eq!(
        Bbox::from_points(figure_rect.iter().copied()),
        Bbox::from_extents(0.0, 0.0, 640.0, 480.0)
    );
    let axes_box = Bbox::from_points(axes_rect.iter().copied());
    assert!(close(axes_box.p0, Point::new(80.0, 52.8)));
    assert!(close(axes_box.p1, Point::new(576.0, 422.4)));
    assert_eq!(ax.window_extent().map(|b| b.width()), Some(496.0));
}

#[test]
fn a_figure_without_frame_draws_no_background() {
    let fig = Figure::new();
    assert!(fig.frameon(), "drawn unless turned off");
    fig.set_frameon(false);
    assert!(!fig.frameon());
    let ax = fig.add_axes([0.125, 0.11, 0.775, 0.77]);
    ax.set_axis_off();
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).expect("drawable");
    assert!(renderer.calls.is_empty(), "drew {:?}", renderer.calls);
}

#[test]
fn data_transform_maps_view_limits_onto_axes_box() {
    let fig = Figure::new();
    let ax = fig.add_axes([0.125, 0.11, 0.775, 0.77]);
    ax.set_xlim(0.0, 10.0).set_ylim(1.0, -1.0);
    let state = fig.state.borrow();
    let data = ax.trans_data().resolve(&*state).expect("axes exists");
    assert!(close(
        data.transform_point(Point::new(5.0, 0.0)),
        Point::new(328.0, 237.6)
    ));
    assert!(
        close(
            data.transform_point(Point::new(0.0, 1.0)),
            Point::new(80.0, 52.8)
        ),
        "y is inverted"
    );
    let back = data
        .inverted()
        .expect("invertible")
        .transform_point(Point::new(576.0, 422.4));
    assert!(close(back, Point::new(10.0, -1.0)));
}

#[test]
fn removed_axes_disappear_from_figure() {
    let fig = Figure::new();
    let first = fig.add_axes([0.0, 0.0, 0.5, 1.0]);
    let second = fig.add_axes([0.5, 0.0, 0.5, 1.0]);
    assert_eq!(fig.axes(), vec![first.clone(), second.clone()]);
    first.remove();
    assert_eq!(fig.axes(), vec![second]);
    assert!(!first.raw().is_alive());
}

#[test]
fn hidden_artists_are_not_drawn() {
    let fig = Figure::new();
    fig.add_axes([0.1, 0.1, 0.8, 0.8]).set_visible(false);
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).expect("drawable");
    assert_eq!(
        renderer.calls.len(),
        1,
        "only the figure background is drawn"
    );
}

/// The figure of the animation tests: axes with a line and a figure-level
/// rectangle, both animated when `animated`.
fn animation_figure(animated: bool) -> Figure {
    let fig = Figure::new();
    let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
    ax.plot([0.0, 1.0], [0.0, 1.0])
        .expect("plottable")
        .set_animated(animated);
    fig.add_artist(crate::patches::Rectangle::new((0.0, 0.0), 0.5, 0.5))
        .set_animated(animated);
    fig
}

#[test]
fn animated_artists_are_drawn_only_into_saved_axes() {
    let draw_calls = |fig: &Figure| {
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        renderer.calls.len()
    };
    let (still, animated) = (animation_figure(false), animation_figure(true));
    assert_eq!(
        draw_calls(&animated),
        draw_calls(&still) - 2,
        "a normal draw leaves both out"
    );
    let saved = animated.to_svg().expect("savable");
    assert!(saved.contains("id=\"line2d_1\""), "the axes draw the line");
    let patches = |svg: &str| svg.matches("<g id=\"patch_").count();
    assert_eq!(
        patches(&saved),
        patches(&still.to_svg().expect("savable")) - 1,
        "the figure leaves the rectangle out"
    );
}

#[test]
fn subplots_follow_the_grid() {
    let fig = Figure::new();
    let axs = fig.subplots(2, 2).expect("valid grid");
    let (x, y, w, h) = axs[(1, 0)].position().bounds();
    assert!((x - 0.125).abs() < 1e-12 && (y - 0.11).abs() < 1e-12);
    assert!((w - 0.352_272_727_272_727_24).abs() < 1e-12 && (h - 0.35).abs() < 1e-12);
    assert_eq!(fig.axes().len(), 4);
}

#[test]
fn subplots_adjust_moves_grid_axes_only() {
    let fig = Figure::new();
    let grid_axes = fig.subplot();
    let free_axes = fig.add_axes([0.5, 0.5, 0.1, 0.1]);
    fig.subplots_adjust(&crate::gridspec::SubplotOverrides {
        left: Some(0.3),
        ..Default::default()
    })
    .expect("valid");
    assert!((grid_axes.position().x0() - 0.3).abs() < 1e-12);
    assert_eq!(free_axes.position().x0(), 0.5);
    assert!(
        fig.subplots_adjust(&crate::gridspec::SubplotOverrides {
            right: Some(0.2),
            ..Default::default()
        })
        .is_err()
    );
}
