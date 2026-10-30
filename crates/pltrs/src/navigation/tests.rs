use super::*;
use crate::navigation::PointerButton;

/// Pans from the middle of the figure 50 pixels to the right with two
/// axes stacked on the same rectangle; returns the x limits of both.
fn pan_stacked(forward: Option<bool>) -> ((f64, f64), (f64, f64)) {
    let fig = Figure::new();
    let below = fig
        .add_axes([0.1, 0.1, 0.8, 0.8])
        .expect("a valid rectangle");
    let above = fig
        .add_axes([0.1, 0.1, 0.8, 0.8])
        .expect("a valid rectangle");
    for ax in [&below, &above] {
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
    }
    above.set_forward_navigation_events(forward);
    let mut navigation = Navigation::new();
    navigation.toggle(&fig, NavMode::Pan).unwrap();
    let start = Point::new(320.0, 240.0);
    navigation.press(&fig, start, PointerButton::Left);
    navigation.drag(Point::new(370.0, 240.0), DragKey::None);
    navigation.release(&fig, Point::new(370.0, 240.0), DragKey::None);
    (below.xlim(), above.xlim())
}

#[test]
fn opaque_axes_keep_navigation_unless_they_forward_it() {
    let (below, above) = pan_stacked(None);
    assert_eq!(
        below,
        (0.0, 10.0),
        "the opaque axes on top captures the pan"
    );
    assert!(above.0 < 0.0);
    let (below, above) = pan_stacked(Some(true));
    assert!(
        below.0 < 0.0 && above.0 < 0.0,
        "forwarded to the axes below"
    );
    assert_eq!(pan_stacked(Some(false)).0, (0.0, 10.0));
}

/// An image coloured from 0 to 10 with a vertical colorbar, laid out, and
/// the display points of the colorbar values 4 and 6.
fn image_with_colorbar() -> (Figure, crate::image::AxesImage, Point, Point) {
    use crate::axes::ImshowOptions;
    use crate::colorbar::ColorbarOptions;
    use crate::colors::Colorize;
    use crate::numeric::Grid;

    let fig = Figure::new();
    let data = Grid::from_fn(4, 3, |row, column| (row * 3 + column) as f64);
    let image = fig
        .subplot()
        .imshow(data, ImshowOptions::default().limits(Some(0.0), Some(10.0)))
        .unwrap();
    let colorbar = fig.colorbar(&image, ColorbarOptions::default()).unwrap();
    fig.layout();
    let to_display = fig.resolve_transform(&colorbar.ax().trans_data()).unwrap();
    let at = |value: f64| to_display.transform_point(Point::new(0.5, value));
    (fig, image, at(4.0), at(6.0))
}

#[test]
fn zooming_and_panning_a_colorbar_change_its_norm() {
    let (fig, image, four, six) = image_with_colorbar();
    let mut navigation = Navigation::new();
    navigation.toggle(&fig, NavMode::Zoom).unwrap();
    navigation.press(&fig, four, PointerButton::Left);
    navigation.release(&fig, six, DragKey::None);
    let (Some(vmin), Some(vmax)) = image.clim() else {
        panic!("a zoomed colorbar sets both limits");
    };
    assert!((vmin - 4.0).abs() < 1e-9 && (vmax - 6.0).abs() < 1e-9);
    navigation.home(&fig);
    assert_eq!(
        image.clim(),
        (Some(0.0), Some(10.0)),
        "home restores the norm"
    );

    navigation.toggle(&fig, NavMode::Pan).unwrap();
    navigation.press(&fig, six, PointerButton::Left);
    navigation.drag(four, DragKey::None);
    navigation.release(&fig, four, DragKey::None);
    let (Some(vmin), Some(vmax)) = image.clim() else {
        panic!("a panned colorbar sets both limits");
    };
    assert!((vmin - 2.0).abs() < 1e-9 && (vmax - 12.0).abs() < 1e-9);
}

#[test]
fn colorbars_of_contours_do_not_navigate() {
    use crate::colorbar::ColorbarOptions;
    use crate::contour::{ContourOptions, QuadField};
    use crate::numeric::Grid;

    let fig = Figure::new();
    let field = QuadField::new(Grid::from_fn(4, 3, |row, column| (row + column) as f64));
    let contours = fig
        .subplot()
        .contourf(&field, ContourOptions::default())
        .unwrap();
    let colorbar = fig.colorbar(&contours, ColorbarOptions::default()).unwrap();
    assert!(!colorbar.ax().navigate());
}

/// A subplot showing `0..10` with an equal aspect, never laid out.
fn equal_aspect_plot() -> (Figure, Axes) {
    use crate::axes::Aspect;

    let fig = Figure::new();
    let ax = fig.subplot();
    ax.plot_y((0..10).map(f64::from));
    ax.set_aspect(Aspect::Equal).unwrap();
    (fig, ax)
}

/// The centre, in display pixels of the default 640 × 480 canvas, of a box
/// in figure fractions.
fn centre_pixels(fraction: Bbox) -> Point {
    Point::new(
        (fraction.x0() + fraction.width() / 2.0) * 640.0,
        (fraction.y0() + fraction.height() / 2.0) * 480.0,
    )
}

/// Whether two display points are the same up to rounding.
fn close(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
}

#[test]
fn transforms_resolved_as_is_wait_for_autoscaling_and_the_aspect() {
    let (fig, ax) = equal_aspect_plot();
    let to_display = || fig.resolve_transform_as_is(&ax.trans_data()).unwrap();
    let at = to_display().transform_point(Point::new(0.5, 0.5));
    assert!(
        close(at, centre_pixels(ax.original_position())),
        "the view of a fresh axes on its original position"
    );
    let active = ax.position();
    assert!(
        active.width() < ax.original_position().width(),
        "reading the position applies the aspect"
    );
    let middle = Point::new(4.5, 4.5);
    let at = to_display().transform_point(middle);
    assert!(close(at, centre_pixels(active)), "the autoscaled view");
    let laid_out = fig.resolve_transform(&ax.trans_data()).unwrap();
    assert!(close(laid_out.transform_point(middle), at));
}

#[test]
fn the_axes_under_the_pointer_is_where_it_is_now() {
    let (fig, ax) = equal_aspect_plot();
    let near_left = Point::new((ax.original_position().x0() + 0.01) * 640.0, 240.0);
    assert!(
        ax.contains_point(near_left),
        "the aspect is not applied yet"
    );
    fig.layout();
    assert!(
        !ax.contains_point(near_left),
        "the box narrowed to the aspect"
    );
}

#[test]
fn applying_the_aspect_leaves_the_layout_engine_to_the_next_layout() {
    use crate::layout::LayoutEngine;

    let fig = Figure::new();
    fig.set_layout_engine(LayoutEngine::constrained()).unwrap();
    let ax = fig.subplot();
    let before = ax.position();
    assert_eq!(before, ax.original_position());
    fig.layout();
    assert_ne!(ax.position(), before, "constrained layout placed the axes");
}

#[test]
fn removed_axes_keep_their_transforms() {
    let (fig, ax) = equal_aspect_plot();
    fig.layout();
    let before = fig.resolve_transform_as_is(&ax.trans_axes()).unwrap();
    ax.remove();
    let after = fig.resolve_transform_as_is(&ax.trans_axes()).unwrap();
    assert_eq!(before.affine_part(), after.affine_part());
}
