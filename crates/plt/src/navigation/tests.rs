use super::*;
use crate::navigation::PointerButton;

/// Pans from the middle of the figure 50 pixels to the right with two
/// axes stacked on the same rectangle; returns the x limits of both.
fn pan_stacked(forward: Option<bool>) -> ((f64, f64), (f64, f64)) {
    let fig = Figure::new();
    let below = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
    let above = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
    for ax in [&below, &above] {
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
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
    use crate::image::ColorizerOptions;
    use crate::numeric::Grid;

    let fig = Figure::new();
    let colors = ColorizerOptions {
        vmin: Some(0.0),
        vmax: Some(10.0),
        ..ColorizerOptions::default()
    };
    let data = Grid::from_fn(4, 3, |row, column| (row * 3 + column) as f64);
    let image = fig
        .subplot()
        .imshow(data, ImshowOptions::default().colors(colors))
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
