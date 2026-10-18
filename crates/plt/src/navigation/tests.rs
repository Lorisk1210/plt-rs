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
