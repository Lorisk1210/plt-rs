use crate::artist::ArtistHandle;
use crate::events::EventSource;
use crate::figure::Figure;
use crate::legend::{Legend, LegendOptions};
use crate::lines::LineProps;
use crate::navigation::PointerButton;
use crate::offsetbox::AnchorLoc;

/// A figure with a labelled line and an upper-left legend.
fn legend() -> (Figure, Legend) {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("line"))
        .unwrap();
    let legend = ax
        .legend(LegendOptions::default().loc(AnchorLoc::UpperLeft))
        .unwrap();
    (fig, legend)
}

fn corner(legend: &Legend) -> (f64, f64) {
    let extent = legend.window_extent().unwrap();
    (extent.x0(), extent.y0())
}

fn drag(fig: &Figure, from: (f64, f64), to: (f64, f64)) {
    let mut source = EventSource::new();
    source.button_press(fig, from.0, from.1, PointerButton::Left, false);
    source.motion(fig, to.0, to.1);
    source.button_release(fig, to.0, to.1, PointerButton::Left);
}

#[test]
fn draggable_legends_follow_the_pointer() {
    let (fig, legend) = legend();
    let (x0, y0) = corner(&legend);
    let inside = (x0 + 5.0, y0 + 5.0);
    drag(&fig, inside, (inside.0 + 50.0, inside.1 - 30.0));
    assert_eq!(corner(&legend), (x0, y0), "not draggable yet");

    legend.set_draggable(true);
    assert!(legend.draggable() && legend.pickable());
    drag(&fig, inside, (inside.0 + 50.0, inside.1 - 30.0));
    let (x1, y1) = corner(&legend);
    assert!((x1 - (x0 + 50.0)).abs() < 1e-9 && (y1 - (y0 - 30.0)).abs() < 1e-9);

    // Further motion without a press leaves it alone.
    EventSource::new().motion(&fig, 10.0, 10.0);
    assert_eq!(corner(&legend), (x1, y1));
}

#[test]
fn the_wheel_and_misses_do_not_drag() {
    let (fig, legend) = legend();
    legend.set_draggable(true);
    let (x0, y0) = corner(&legend);
    let mut source = EventSource::new();
    source.scroll(&fig, x0 + 5.0, y0 + 5.0, 1.0);
    source.motion(&fig, x0 + 60.0, y0 + 5.0);
    assert_eq!(corner(&legend), (x0, y0), "scrolling picks nothing up");
    drag(&fig, (x0 - 30.0, y0 - 30.0), (x0, y0));
    assert_eq!(corner(&legend), (x0, y0), "a press beside the legend");
}

#[test]
fn legends_stop_being_draggable() {
    let (fig, _) = legend();
    let options = LegendOptions {
        draggable: true,
        ..LegendOptions::default()
    };
    let legend = fig.axes()[0].legend(options).unwrap();
    assert!(legend.draggable(), "made draggable");
    legend.set_draggable(false);
    assert!(!legend.draggable());
    let (x0, y0) = corner(&legend);
    drag(&fig, (x0 + 5.0, y0 + 5.0), (x0 + 40.0, y0 + 40.0));
    assert_eq!(corner(&legend), (x0, y0));
}

#[test]
fn draggable_annotations_move_their_text() {
    use crate::coords::Coords;
    use crate::text::AnnotateOptions;

    let fig = Figure::new();
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 10.0))
        .expect("finite limits");
    let options = AnnotateOptions {
        xytext: Some((20.0, 20.0).into()),
        textcoords: Some("offset points".parse::<Coords>().unwrap()),
        ..AnnotateOptions::default()
    };
    let note = ax.annotate("note", (5.0, 5.0), options).unwrap();
    note.set_draggable(true);
    assert!(note.draggable());
    let before = note.window_extent().unwrap();
    let inside = (before.x0() + 2.0, before.y0() + 2.0);
    drag(&fig, inside, (inside.0 + 40.0, inside.1 + 20.0));
    let after = note.window_extent().unwrap();
    assert!((after.x0() - before.x0() - 40.0).abs() < 1e-9);
    assert!((after.y0() - before.y0() - 20.0).abs() < 1e-9);
    assert_eq!(
        note.xy(),
        pltrs_core::Point::new(5.0, 5.0),
        "the point stays"
    );
    // 40 × 20 pixels at 100 dpi are 28.8 × 14.4 points more offset.
    let moved = note.position();
    assert!((moved.x - 48.8).abs() < 1e-9 && (moved.y - 34.4).abs() < 1e-9);
}

#[test]
fn draggable_annotation_boxes_move_in_their_coordinates() {
    use crate::artist::ArtistHandle;
    use crate::offsetbox::{AnnotationBboxOptions, TextArea};
    use crate::text::TextProps;

    let fig = Figure::new();
    let ax = fig
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 640.0)
        .and_then(|ax| ax.set_ylim(0.0, 480.0))
        .expect("finite limits");
    let options = AnnotationBboxOptions {
        xybox: Some((300.0, 200.0).into()),
        ..AnnotationBboxOptions::default()
    };
    let note = ax
        .add_annotation_bbox(
            TextArea::from_text("box", TextProps::default()),
            (100.0, 100.0),
            options,
        )
        .unwrap();
    note.set_draggable(true);
    assert!(note.draggable());
    let before = note.window_extent().unwrap();
    let inside = (before.center().x, before.center().y);
    drag(&fig, inside, (inside.0 + 30.0, inside.1 - 20.0));
    // Data coordinates are pixels here: the box moved by the drag.
    let moved = note.xybox();
    assert!((moved.x - 330.0).abs() < 1e-9 && (moved.y - 180.0).abs() < 1e-9);
    assert_eq!(note.xy(), pltrs_core::Point::new(100.0, 100.0));
}

#[test]
fn dragged_anchored_boxes_stay_where_dropped() {
    use crate::artist::ArtistHandle;
    use crate::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions};
    use crate::text::TextProps;

    let fig = Figure::new();
    let ax = fig.subplot();
    let label = AnchoredBox::text(
        "label",
        TextProps::default(),
        AnchoredOptions::new(AnchorLoc::UpperLeft),
    )
    .unwrap();
    let anchored = ax.add_anchored_box(label);
    anchored.set_draggable(true);
    assert!(anchored.draggable());
    let before = anchored.window_extent().unwrap();
    let text = anchored.texts()[0].window_extent().unwrap();
    let inside = (text.center().x, text.center().y);
    drag(&fig, inside, (inside.0 + 25.0, inside.1 - 15.0));
    let after = anchored.window_extent().unwrap();
    assert!((after.x0() - before.x0() - 25.0).abs() < 1e-9);
    assert!((after.y0() - before.y0() + 15.0).abs() < 1e-9);
    // It no longer follows the axes.
    fig.set_size_inches(8.0, 6.0).unwrap();
    assert_eq!(anchored.window_extent().unwrap(), after);
}
