use plt_core::{Bbox, Point, Rgba};

use super::*;
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::coords::{CoordError, Coords};
use crate::patches::ArrowStyle;
use crate::testing::{Call, RecordingRenderer, close, data_axes, data_to_display};

fn arrow(style: &str) -> ArrowProps {
    style.parse::<ArrowStyle>().expect("valid style").into()
}

#[test]
fn offset_text_is_placed_from_the_point() {
    let (_, ax) = data_axes();
    let note = ax
        .annotate(
            "a",
            (5.0, 5.0),
            AnnotateOptions {
                xytext: Some(Point::new(10.0, 20.0).into()),
                textcoords: Some("offset points".parse().expect("valid coords")),
                ..AnnotateOptions::default()
            },
        )
        .expect("valid annotation");
    let extent = note.window_extent().expect("text has an extent");
    let anchor = data_to_display(5.0, 5.0) + Point::new(10.0, 20.0) * (100.0 / 72.0);
    assert!(close(extent.x0(), anchor.x), "{extent:?}");
    assert!(extent.y0() < anchor.y && extent.y1() > anchor.y);
    assert_eq!(note.xy(), Point::new(5.0, 5.0));
}

#[test]
fn the_annotated_point_cannot_be_an_offset() {
    let (_, ax) = data_axes();
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
    let (_fig, ax) = data_axes();
    let options = || AnnotateOptions {
        xytext: Some(Point::new(5.0, 5.0).into()),
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
    let (fig, ax) = data_axes();
    let note = ax
        .annotate(
            "text",
            (8.0, 8.0),
            AnnotateOptions {
                xytext: Some(Point::new(1.0, 1.0).into()),
                arrowprops: Some(arrow("-|>")),
                ..AnnotateOptions::default()
            },
        )
        .expect("valid");
    let extent = note.window_extent().expect("an extent");
    let point = data_to_display(8.0, 8.0);
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
    let (fig, ax) = data_axes();
    let legacy = LegacyArrowProps {
        patch: crate::patches::PatchProps::default().facecolor("red"),
        ..LegacyArrowProps::default()
    };
    ax.annotate(
        "",
        (8.0, 8.0),
        AnnotateOptions {
            xytext: Some(Point::new(1.0, 1.0).into()),
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
    let (_, ax) = data_axes();
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

/// The right edge of an arrowed annotation from the empty text at `(1, 5)`
/// to `(9, 5)`, clipped at its end by `patch_b`.
fn arrow_end(ax: &Axes, patch_b: Option<crate::artist::ArtistId>) -> f64 {
    let props = FancyArrowProps {
        shrink_b: 0.0,
        patch_b,
        ..FancyArrowProps::new("-".parse().expect("valid style"))
    };
    let note = ax
        .annotate(
            "",
            (9.0, 5.0),
            AnnotateOptions {
                xytext: Some(Point::new(1.0, 5.0).into()),
                arrowprops: Some(ArrowProps::Fancy(props)),
                ..AnnotateOptions::default()
            },
        )
        .expect("valid");
    note.window_extent().expect("an extent").x1()
}

#[test]
fn patch_b_clips_the_arrow_at_the_point() {
    let (_, ax) = data_axes();
    let unclipped = arrow_end(&ax, None);
    assert!(close(unclipped, data_to_display(9.0, 5.0).x));
    // An unstroked circle of radius 1 around the point.
    let circle = ax.add_patch(crate::patches::Circle::new((9.0, 5.0), 1.0));
    let clipped = arrow_end(&ax, Some(circle.id()));
    assert!(
        (clipped - data_to_display(8.0, 5.0).x).abs() < 0.5,
        "{clipped}"
    );
}

#[test]
fn a_text_box_clips_like_its_bbox_patch() {
    let (_, ax) = data_axes();
    let target = ax
        .annotate("target", (9.0, 5.0), AnnotateOptions::default())
        .expect("valid");
    target.set_halign(plt_text::HAlign::Center);
    assert!(
        close(
            arrow_end(&ax, Some(target.id())),
            data_to_display(9.0, 5.0).x
        ),
        "a text without a box does not clip"
    );
    target.set_bbox(Some(crate::text::TextBoxProps::default()));
    let clipped = arrow_end(&ax, Some(target.id()));
    let extent = target.window_extent().expect("an extent");
    assert!(clipped < extent.x0(), "stops at the padded box");
}

#[test]
fn text_can_be_offset_from_another_artist() {
    let (_, ax) = data_axes();
    let anchor = ax
        .annotate("anchor", (5.0, 5.0), AnnotateOptions::default())
        .expect("annotated");
    let options = AnnotateOptions {
        xytext: Some(Point::new(0.0, 0.0).into()),
        textcoords: Some(crate::coords::Coords::OffsetFrom {
            artist: anchor.id(),
            reference: Point::new(0.5, 0.0),
            unit: crate::coords::CoordUnit::Pixels,
        }),
        ..AnnotateOptions::default()
    };
    let below = ax
        .annotate("below", (1.0, 1.0), options)
        .expect("annotated");
    below
        .set_halign(crate::text::HAlign::Center)
        .set_valign(crate::text::VAlign::Top);
    let (above, under) = (
        anchor.window_extent().expect("laid out"),
        below.window_extent().expect("laid out"),
    );
    assert!(close(
        f64::midpoint(under.x0(), under.x1()),
        f64::midpoint(above.x0(), above.x1())
    ));
    assert!(
        close(under.y1(), above.y0()),
        "{} vs {}",
        under.y1(),
        above.y0()
    );
}

#[test]
fn arrows_are_measured_at_their_last_drawn_resolution() {
    let (fig, ax) = data_axes();
    let note = ax
        .annotate(
            "a",
            (5.0, 5.0),
            AnnotateOptions {
                xytext: Some(Point::new(1.0, 1.0).into()),
                arrowprops: Some(arrow("simple")),
                ..AnnotateOptions::default()
            },
        )
        .expect("valid annotation");
    // Before any draw, matplotlib's arrow (its heads and the shrinking at
    // its ends) is one pixel per point; afterwards 100/72.
    let measured = note.window_extent().expect("an extent");
    fig.draw(&mut RecordingRenderer::new(640.0, 480.0, 100.0))
        .expect("draws");
    let drawn = note.window_extent().expect("an extent");
    assert!(
        !close(drawn.x1(), measured.x1()),
        "{measured:?} then {drawn:?}"
    );
}

/// How far right of its boxed text at `(1, 5)` the arrow of an
/// annotation of `(9, 5)` starts, when `patch_a` clips it.
fn arrow_start(ax: &Axes, patch_a: ArrowStart) -> f64 {
    let props = FancyArrowProps {
        shrink_a: 0.0,
        shrink_b: 0.0,
        patch_a,
        ..FancyArrowProps::new("-".parse().expect("valid style"))
    };
    let note = ax
        .annotate(
            "start",
            (9.0, 5.0),
            AnnotateOptions {
                xytext: Some(Point::new(1.0, 5.0).into()),
                arrowprops: Some(ArrowProps::Fancy(props)),
                ..AnnotateOptions::default()
            },
        )
        .expect("valid");
    note.set_halign(plt_text::HAlign::Center)
        .set_valign(plt_text::VAlign::Center)
        .set_bbox(Some(crate::text::TextBoxProps::default()));
    let state = note.raw().figure_state().borrow();
    let layout = LayoutContext::new(&state);
    let start = note.raw().with::<TextArtist, _>(|t| {
        let annotation = t.annotation.as_deref().expect("an annotation");
        let point = annotation.point(t, layout).expect("a point");
        let anchor = annotation.anchor(t, layout, point).expect("an anchor");
        let target = AnnotationState::target(t, layout, point, anchor).expect("a target");
        let paths = annotation
            .arrow
            .as_ref()
            .expect("an arrow")
            .measured_paths(layout, &t.rc, target)
            .expect("arrow paths");
        let displayed: Vec<plt_core::Path> = paths.iter().map(DisplayPath::to_display).collect();
        plt_core::Path::compound(&displayed).extents().x0()
    });
    start - data_to_display(1.0, 5.0).x
}

#[test]
fn patch_a_chooses_where_the_arrow_leaves_the_text() {
    let (_, ax) = data_axes();
    let boxed = arrow_start(&ax, ArrowStart::TextBox);
    let unclipped = arrow_start(&ax, ArrowStart::Unclipped);
    assert!(
        unclipped.abs() < 1e-6,
        "unclipped arrows start at the anchor: {unclipped}"
    );
    assert!(boxed > 10.0, "the box clips the start: {boxed}");
    // An unstroked circle of radius 2 (99.2 px) around the text.
    let circle = ax.add_patch(crate::patches::Circle::new((1.0, 5.0), 2.0));
    let clipped = arrow_start(&ax, ArrowStart::Artist(circle.id()));
    assert!((clipped - 99.2).abs() < 0.5, "{clipped}");
}

#[test]
fn lines_clip_arrows_within_their_pick_radius() {
    // `patchB=line` clips where `Line2D.contains` starts to hold: within
    // the pick radius (5 points) of a marker of a line without a style.
    let (_, ax) = data_axes();
    let marker = ax.plot_fmt([9.0], [5.0], "o").expect("a line");
    let props = FancyArrowProps {
        shrink_a: 0.0,
        shrink_b: 0.0,
        patch_b: Some(marker.id()),
        ..FancyArrowProps::new("-".parse().expect("valid style"))
    };
    let note = ax
        .annotate(
            "start",
            (9.0, 5.0),
            AnnotateOptions {
                xytext: Some(Point::new(1.0, 5.0).into()),
                arrowprops: Some(ArrowProps::Fancy(props)),
                ..AnnotateOptions::default()
            },
        )
        .expect("valid annotation");
    let end = note.window_extent().expect("an extent").x1();
    let clipped = data_to_display(9.0, 5.0).x - 5.0 * 100.0 / 72.0;
    assert!((end - clipped).abs() < 0.05, "{end} {clipped}");
}

#[test]
fn coordinate_systems_and_clipping_read_back_as_set() {
    let (_fig, ax) = data_axes();
    let note = ax
        .annotate("note", (5.0, 5.0), AnnotateOptions::default())
        .expect("valid annotation");
    assert_eq!(
        (note.xycoords(), note.textcoords()),
        (Coords::Data, Coords::Data)
    );
    assert_eq!(note.annotation_clip(), None);
    let fraction: Coords = "axes fraction".parse().expect("valid coords");
    let offset: Coords = "offset points".parse().expect("valid coords");
    note.set_textcoords(offset.clone())
        .set_annotation_clip(Some(false));
    note.set_xycoords(fraction.clone()).expect("not an offset");
    assert_eq!(
        (note.xycoords(), note.textcoords()),
        (fraction.clone(), offset.clone())
    );
    assert_eq!(note.annotation_clip(), Some(false));
    assert_eq!(note.set_xycoords(offset).err(), Some(CoordError::NoOffset));
    assert_eq!(note.xycoords(), fraction, "a rejected system leaves it");
}
