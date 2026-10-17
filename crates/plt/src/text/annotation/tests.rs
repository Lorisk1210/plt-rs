use plt_core::{Bbox, Point, Rgba};

use super::*;
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::coords::CoordError;
use crate::figure::Figure;
use crate::patches::ArrowStyle;
use crate::testing::{Call, RecordingRenderer};

/// A 640 × 480 px figure with one axes over `(0, 10)²`.
fn axes() -> (Figure, Axes) {
    let fig = Figure::new();
    let ax = fig.add_axes([0.125, 0.11, 0.775, 0.77]);
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    (fig, ax)
}

/// The display position of a data point of `axes()`.
fn display(x: f64, y: f64) -> Point {
    Point::new(80.0 + x * 49.6, 52.8 + y * 36.96)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn arrow(style: &str) -> ArrowProps {
    style.parse::<ArrowStyle>().expect("valid style").into()
}

#[test]
fn offset_text_is_placed_from_the_point() {
    let (_, ax) = axes();
    let note = ax
        .annotate(
            "a",
            (5.0, 5.0),
            AnnotateOptions {
                xytext: Some(Point::new(10.0, 20.0)),
                textcoords: Some("offset points".parse().expect("valid coords")),
                ..AnnotateOptions::default()
            },
        )
        .expect("valid annotation");
    let extent = note.window_extent().expect("text has an extent");
    let anchor = display(5.0, 5.0) + Point::new(10.0, 20.0) * (100.0 / 72.0);
    assert!(close(extent.x0(), anchor.x), "{extent:?}");
    assert!(extent.y0() < anchor.y && extent.y1() > anchor.y);
    assert_eq!(note.xy(), Point::new(5.0, 5.0));
}

#[test]
fn the_annotated_point_cannot_be_an_offset() {
    let (_, ax) = axes();
    let result = ax.annotate(
        "a",
        (0.0, 0.0),
        AnnotateOptions {
            xycoords: "offset pixels".parse().expect("valid coords"),
            ..AnnotateOptions::default()
        },
    );
    assert_eq!(result.err(), Some(CoordError::NoOffset));
}

#[test]
fn points_outside_the_axes_hide_data_annotations() {
    let (_fig, ax) = axes();
    let options = || AnnotateOptions {
        xytext: Some(Point::new(5.0, 5.0)),
        ..AnnotateOptions::default()
    };
    let hidden = ax
        .annotate("hidden", (20.0, 5.0), options())
        .expect("valid");
    assert_eq!(hidden.window_extent(), Some(Bbox::UNIT));
    let shown = ax
        .annotate(
            "shown",
            (20.0, 5.0),
            AnnotateOptions {
                annotation_clip: Some(false),
                ..options()
            },
        )
        .expect("valid");
    assert_ne!(shown.window_extent(), Some(Bbox::UNIT));
    hidden.set_annotation_clip(Some(false));
    assert_ne!(hidden.window_extent(), Some(Bbox::UNIT));
}

#[test]
fn arrows_reach_the_point_and_widen_the_extent() {
    let (fig, ax) = axes();
    let note = ax
        .annotate(
            "text",
            (8.0, 8.0),
            AnnotateOptions {
                xytext: Some(Point::new(1.0, 1.0)),
                arrowprops: Some(arrow("-|>")),
                ..AnnotateOptions::default()
            },
        )
        .expect("valid");
    let extent = note.window_extent().expect("an extent");
    let point = display(8.0, 8.0);
    // The arrow stops 2 points short of the point.
    assert!(extent.x1() > point.x - 5.0 && extent.x1() < point.x);
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).expect("drawable");
    let filled_heads = renderer
        .calls
        .iter()
        .filter(
            |call| matches!(call, Call::Path { points, fill: Some(_), .. } if points.len() == 4),
        )
        .count();
    assert!(filled_heads >= 1, "the filled head is drawn");
}

#[test]
fn legacy_arrows_are_filled_in_their_face_colour() {
    let (fig, ax) = axes();
    let legacy = LegacyArrowProps {
        patch: crate::patches::PatchProps::default().facecolor("red"),
        ..LegacyArrowProps::default()
    };
    ax.annotate(
        "",
        (8.0, 8.0),
        AnnotateOptions {
            xytext: Some(Point::new(1.0, 1.0)),
            arrowprops: Some(ArrowProps::Legacy(legacy)),
            ..AnnotateOptions::default()
        },
    )
    .expect("valid");
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).expect("drawable");
    let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
    assert!(
        renderer
            .calls
            .iter()
            .any(|call| matches!(call, Call::Path { fill: Some(c), .. } if *c == red))
    );
}

#[test]
fn texts_can_be_annotated_points() {
    let (_, ax) = axes();
    let box_text = ax
        .annotate("box", (5.0, 5.0), AnnotateOptions::default())
        .expect("valid");
    let below = ax
        .annotate(
            "below",
            (0.0, 0.0),
            AnnotateOptions {
                xycoords: crate::coords::Coords::Artist(box_text.id()),
                ..AnnotateOptions::default()
            },
        )
        .expect("valid");
    let reference = box_text.window_extent().expect("an extent");
    let extent = below.window_extent().expect("an extent");
    assert!(close(extent.x0(), reference.x0()));
}
