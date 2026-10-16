use plt_core::Rgba;

use super::PlotError;
use crate::artist::ArtistHandle;
use crate::figure::Figure;
use crate::lines::DrawStyle;
use crate::testing::{Call, RecordingRenderer};

fn axes() -> (Figure, super::Axes) {
    let fig = Figure::new();
    let ax = fig.add_axes([0.125, 0.11, 0.775, 0.77]);
    (fig, ax)
}

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-12 && (a.1 - b.1).abs() < 1e-12
}

#[test]
fn plot_autoscales_like_matplotlib() {
    // spec/goldens/figures/basic/line.json
    let (_, ax) = axes();
    ax.plot([1.0, 2.0, 3.0, 4.0], [1.0, 4.0, 2.0, 3.0]).expect("same length");
    assert!(close(ax.xlim(), (0.85, 4.15)));
    assert!(close(ax.ylim(), (0.85, 4.15)));
}

#[test]
fn empty_axes_keep_unit_limits() {
    let (_, ax) = axes();
    assert_eq!(ax.xlim(), (0.0, 1.0));
}

#[test]
fn explicit_limits_stop_autoscaling() {
    let (_, ax) = axes();
    ax.set_xlim(10.0, 0.0);
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
    assert_eq!(second.rgba(), Ok(Rgba::from_rgba8([0xff, 0x7f, 0x0e, 0xff])));
    assert_eq!(first.zorder(), 2.0);
}

#[test]
fn mismatched_lengths_are_rejected() {
    let (_, ax) = axes();
    assert_eq!(ax.plot([1.0], [1.0, 2.0]), Err(PlotError::LengthMismatch { x: 1, y: 2 }));
}

#[test]
fn drawing_maps_data_into_the_axes_box() {
    let (fig, ax) = axes();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    ax.plot([0.0, 10.0], [0.0, 10.0]).expect("same length");
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).expect("drawable");
    let Some(Call::Path { points, fill: None, linewidth }) = renderer.calls.last() else {
        panic!("expected the line last, got {:?}", renderer.calls);
    };
    assert_eq!(*linewidth, 1.5);
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
    let line = ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.0]).expect("same length");
    line.set_drawstyle(DrawStyle::StepsPost);
    assert_eq!(line.path().map(|p| p.len()), Ok(5));
}
