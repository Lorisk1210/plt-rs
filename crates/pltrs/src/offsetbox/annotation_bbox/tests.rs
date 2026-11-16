//! Placement, arrows and visibility of annotation boxes.

use super::*;
use crate::offsetbox::{DrawingArea, TextArea};
use crate::patches::{ConnectionEnd, ConnectionPatch, Rectangle};
use crate::testing::{Call, RecordingRenderer, close_points, data_axes, data_to_display};
use crate::text::TextProps;

fn draw(fig: &Figure) -> Vec<Call> {
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).expect("drawable");
    renderer.calls
}

#[test]
fn boxes_are_aligned_on_their_offset_position() {
    let (fig, ax) = data_axes();
    let note = ax
        .add_annotation_bbox(
            TextArea::from_text("note", TextProps::default()),
            (5.0, 5.0),
            AnnotationBboxOptions {
                xybox: Some((10.0, 20.0).into()),
                boxcoords: Some("offset points".parse().expect("valid coords")),
                box_alignment: (0.0, 0.0),
                ..AnnotationBboxOptions::default()
            },
        )
        .expect("valid annotation");
    draw(&fig);
    let anchor = data_to_display(5.0, 5.0) + Point::new(10.0, 20.0) * (100.0 / 72.0);
    assert!(close_points(note.patch().shape().xy, anchor));
    assert_eq!(note.texts()[0].text(), "note");
    note.set_xybox((0.0, 0.0));
    draw(&fig);
    assert!(close_points(
        note.patch().shape().xy,
        data_to_display(5.0, 5.0)
    ));
    assert_eq!(note.xybox(), Point::ORIGIN);
}

#[test]
fn boxes_default_to_the_point_and_its_coordinates() {
    // matplotlib's test_annotationbbox_properties.
    let (_, ax) = data_axes();
    let area = || DrawingArea::<BoxItem>::new(20.0, 20.0, 0.0, 0.0);
    let plain = ax
        .add_annotation_bbox(area(), (0.5, 0.5), AnnotationBboxOptions::default())
        .expect("valid annotation");
    assert_eq!(plain.xybox(), Point::new(0.5, 0.5));
    assert_eq!(plain.boxcoords(), Coords::Data);
    let fraction: Coords = "axes fraction".parse().expect("valid coords");
    let placed = ax
        .add_annotation_bbox(
            area(),
            (0.5, 0.5),
            AnnotationBboxOptions {
                xybox: Some(Point::new(-0.2, 0.4)),
                boxcoords: Some(fraction.clone()),
                ..AnnotationBboxOptions::default()
            },
        )
        .expect("valid annotation");
    assert_eq!(placed.xybox(), Point::new(-0.2, 0.4));
    assert_eq!(placed.boxcoords(), fraction);
}

#[test]
fn arrows_reach_the_point_and_hidden_points_hide_the_box() {
    let (fig, ax) = data_axes();
    let area = || {
        DrawingArea::new(10.0, 10.0, 0.0, 0.0).with_artist(BoxItem::patch(
            Rectangle::new((0.0, 0.0), 10.0, 10.0),
            PatchProps::default().facecolor("r"),
        ))
    };
    let arrow = FancyArrowProps::new("-|>".parse().expect("valid style"));
    let note = ax
        .add_annotation_bbox(
            area(),
            (8.0, 8.0),
            AnnotationBboxOptions {
                xybox: Some((2.0, 2.0).into()),
                arrowprops: Some(arrow),
                ..AnnotationBboxOptions::default()
            },
        )
        .expect("valid annotation");
    let calls = draw(&fig);
    let extent = note.window_extent().expect("an extent");
    let point = data_to_display(8.0, 8.0);
    assert!(
        extent.x1() < point.x && extent.x1() > point.x - 5.0,
        "{extent:?}"
    );
    let red = pltrs_core::Rgba::new(1.0, 0.0, 0.0, 1.0);
    let reds = |calls: &[Call]| {
        calls
            .iter()
            .filter(|c| matches!(c, Call::Path { fill: Some(f), .. } if *f == red))
            .count()
    };
    assert_eq!(reds(&calls), 1);
    note.set_xy((20.0, 8.0));
    assert_eq!(
        reds(&draw(&fig)),
        0,
        "data points outside the axes hide the box"
    );
    let result = ax.add_annotation_bbox(
        area(),
        (0.0, 0.0),
        AnnotationBboxOptions {
            xycoords: "offset points".parse().expect("valid coords"),
            ..AnnotationBboxOptions::default()
        },
    );
    assert_eq!(result.err(), Some(CoordError::NoOffset));
}

#[test]
fn figure_boxes_annotate_other_artists() {
    let fig = Figure::new();
    let target = fig.text(0.5, 0.5, "target");
    let red = PatchProps::default().facecolor("r");
    let area = DrawingArea::new(10.0, 10.0, 0.0, 0.0)
        .with_artist(BoxItem::patch(Rectangle::new((0.0, 0.0), 10.0, 10.0), red));
    let note = fig
        .add_annotation_bbox(
            area,
            (1.0, 1.0),
            AnnotationBboxOptions {
                xycoords: Coords::Artist(target.id()),
                xybox: Some((10.0, 20.0).into()),
                boxcoords: Some("offset points".parse().expect("valid coords")),
                box_alignment: (0.0, 0.0),
                ..AnnotationBboxOptions::default()
            },
        )
        .expect("valid annotation");
    let calls = draw(&fig);
    let corner = target.window_extent().expect("an extent").p1;
    let anchor = corner + Point::new(10.0, 20.0) * (100.0 / 72.0);
    assert!(close_points(note.patch().shape().xy, anchor));
    let red = pltrs_core::Rgba::new(1.0, 0.0, 0.0, 1.0);
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, Call::Path { fill: Some(f), .. } if *f == red)),
        "a figure's box is drawn whatever the annotated point"
    );
}

#[test]
fn boxes_place_others_relative_to_the_artists_they_draw() {
    let (fig, ax) = data_axes();
    let square = Rectangle::new((0.0, 0.0), 10.0, 10.0);
    let area = DrawingArea::new(10.0, 10.0, 0.0, 0.0)
        .with_artist(BoxItem::patch(square, PatchProps::default()));
    let first = ax
        .add_annotation_bbox(area, (2.0, 2.0), AnnotationBboxOptions::default())
        .expect("valid annotation");
    let artists = first.artist_ids();
    assert_eq!(artists.len(), 1, "the square only");
    let second = ax
        .add_annotation_bbox(
            TextArea::from_text("beside", TextProps::default()),
            (5.0, 5.0),
            AnnotationBboxOptions {
                xybox: Some((1.0, 0.0).into()),
                boxcoords: Some(Coords::Artist(artists[0])),
                box_alignment: (0.0, 0.0),
                ..AnnotationBboxOptions::default()
            },
        )
        .expect("valid annotation");
    assert_eq!(first.texts(), Vec::new());
    assert_eq!(second.artist_ids().len(), 1);
    draw(&fig);
    let px = 100.0 / 72.0;
    let corner = data_to_display(2.0, 2.0) + Point::new(5.0, -5.0) * px;
    assert!(
        close_points(second.patch().shape().xy, corner),
        "{corner:?}"
    );
}

/// The lower-left corner of a box placed by its lower-left corner at the
/// upper-right corner of a `->` connection across `fig`, whose z-order
/// puts it before or after the box; with the arrow's extent once drawn.
fn box_on_arrow(arrow_zorder: f64) -> (Point, Point) {
    let fig = Figure::new();
    let fraction: Coords = "figure fraction".parse().expect("valid coords");
    let mut link = ConnectionPatch::new(
        ConnectionEnd::new((0.1, 0.5), fraction.clone(), None),
        ConnectionEnd::new((0.9, 0.5), fraction, None),
    );
    link.spec.arrowstyle = "->".parse().expect("valid style");
    let arrow = fig.add_artist(link);
    arrow.set_zorder(arrow_zorder);
    let note = fig
        .add_annotation_bbox(
            TextArea::from_text("note", TextProps::default()),
            (1.0, 1.0),
            AnnotationBboxOptions {
                xycoords: Coords::Artist(arrow.id()),
                box_alignment: (0.0, 0.0),
                ..AnnotationBboxOptions::default()
            },
        )
        .expect("valid annotation");
    draw(&fig);
    // Read first: measuring lays the figure out again, as drawn.
    let placed = note.patch().shape().xy;
    (placed, arrow.window_extent().expect("an extent").p1)
}

#[test]
fn arrows_drawn_before_a_box_place_it_as_drawn() {
    // matplotlib places the box as it draws: an arrow drawn earlier in
    // the same (first) draw has its head sized at the canvas resolution,
    // one drawn later still at one pixel per point.
    let (placed, drawn) = box_on_arrow(1.0);
    assert!(close_points(placed, drawn), "{placed:?} {drawn:?}");
    let (placed, drawn) = box_on_arrow(5.0);
    assert!((placed - drawn).length() > 0.1, "{placed:?} {drawn:?}");
}

#[test]
fn an_unlisted_box_measures_and_draws_where_its_figure_would() {
    let fig = Figure::new();
    let red = PatchProps::default().facecolor("r");
    let area = DrawingArea::new(10.0, 10.0, 0.0, 0.0)
        .with_artist(BoxItem::patch(Rectangle::new((0.0, 0.0), 10.0, 10.0), red));
    let note = fig
        .add_annotation_bbox(
            area,
            (0.5, 0.5),
            AnnotationBboxOptions {
                xycoords: "figure fraction".parse().expect("valid coords"),
                frameon: false,
                ..AnnotationBboxOptions::default()
            },
        )
        .expect("valid annotation");
    let listed = note.window_extent().expect("an extent");
    note.unlist();
    let red = pltrs_core::Rgba::new(1.0, 0.0, 0.0, 1.0);
    let is_red = |c: &Call| matches!(c, Call::Path { fill: Some(f), .. } if *f == red);
    assert!(fig.artists().is_empty() && note.window_extent().is_none());
    assert!(
        !draw(&fig).iter().any(is_red),
        "the figure no longer draws it"
    );
    let measured = note
        .raw()
        .measured_extent()
        .expect("measured in its figure");
    assert!(close_points(measured.p0, listed.p0) && close_points(measured.p1, listed.p1));
    let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
    note.raw()
        .draw_onto(&mut renderer)
        .expect("drawn by its own draw");
    let square = renderer
        .calls
        .iter()
        .find_map(|c| match c {
            Call::Path {
                points,
                fill: Some(f),
                ..
            } if *f == red => Some(points.clone()),
            _ => None,
        })
        .expect("its square drawn");
    assert!(
        square.iter().all(|&p| measured.contains(p)),
        "drawn where it measures"
    );
}
