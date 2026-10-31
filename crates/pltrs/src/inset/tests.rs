use pltrs_core::Bbox;

use super::geometry::visible_connectors;
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::figure::Figure;
use crate::lines::LineStyle;
use crate::patches::PatchProps;

/// Whether `a` and `b` agree to 1e-9.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// An axes over `[0, 10]²` filling most of `fig`, with an inset showing
/// `[1, 2]²`.
fn zoomed(fig: &Figure) -> (Axes, Axes) {
    let ax = fig
        .add_axes([0.1, 0.1, 0.8, 0.8])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 10.0))
        .expect("finite limits");
    let inset = ax.inset_axes([0.6, 0.6, 0.3, 0.3]);
    inset
        .set_xlim(1.0, 2.0)
        .and_then(|ax| ax.set_ylim(1.0, 2.0))
        .expect("finite limits");
    (ax, inset)
}

#[test]
fn zoom_indicators_follow_the_inset_limits() {
    let fig = Figure::new();
    let (ax, inset) = zoomed(&fig);
    let marker = ax.indicate_inset_zoom(&inset, PatchProps::default());
    let extent = marker.window_extent().expect("drawn");
    // The rectangle starts at data (1, 1) = pixels (115.2, 86.4); the
    // connectors reach the inset's right edge at x = 0.82 * 640.
    assert!(
        close(extent.x0(), 115.2) && close(extent.y0(), 86.4),
        "{extent:?}"
    );
    assert!(close(extent.x1(), 524.8), "{extent:?}");
    inset.set_xlim(0.0, 2.0).expect("finite limits");
    let moved = marker.window_extent().expect("drawn");
    assert!(close(moved.x0(), 64.0), "{moved:?}");
    assert_eq!(marker.rectangle().shape().bbox().x0(), 0.0);
}

#[test]
fn connectors_avoid_the_rectangle() {
    let inset = Bbox::from_extents(60.0, 60.0, 90.0, 90.0);
    // Rectangle below left of the inset: upper-left and lower-right.
    let below_left = Bbox::from_extents(10.0, 10.0, 30.0, 30.0);
    assert_eq!(
        visible_connectors(&below_left, &inset),
        [false, true, true, false]
    );
    // Rectangle above left: lower-left and upper-right.
    let above_left = Bbox::from_extents(10.0, 95.0, 30.0, 99.0);
    assert_eq!(
        visible_connectors(&above_left, &inset),
        [true, false, false, true]
    );
}

#[test]
fn connectors_are_chosen_from_the_rectangle_mapped_twice() {
    // matplotlib 3.11 (test_streamplot.py's test_integration_options):
    // the rectangle (-1.2, -0.8, 0.5, 0.4) reaches right of the inset's
    // right edge, but measured as (-1.8, -1.12)..(-1.55, -0.96) it does
    // not, so the lower-right connector is drawn, not the upper-right.
    let fig = Figure::new();
    let ax = fig
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    ax.set_xlim(-2.0, 2.0)
        .and_then(|ax| ax.set_ylim(-1.5, 1.5))
        .expect("finite limits");
    let inset = ax.inset_axes([0.0, 0.7, 0.18, 0.3]);
    inset
        .set_xlim(-1.2, -0.7)
        .and_then(|ax| ax.set_ylim(-0.8, -0.4))
        .expect("finite limits");
    let marker = ax.indicate_inset_zoom(&inset, PatchProps::default());
    let extent = marker.window_extent().expect("drawn");
    // The lower-right connector ends at the rectangle's bottom; the
    // upper-right one would reach the inset's top at y = 480.
    assert!(extent.y1() < 480.0 - 1.0, "{extent:?}");
    // The lower-left and lower-right connectors, as matplotlib shows them.
    let shown: Vec<bool> = marker
        .connectors()
        .expect("an inset")
        .iter()
        .map(ArtistHandle::visible)
        .collect();
    assert_eq!(shown, [true, false, true, false]);
}

#[test]
fn indicators_without_an_inset_are_a_plain_rectangle() {
    let fig = Figure::new();
    let ax = fig
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 1.0)
        .and_then(|ax| ax.set_ylim(1.0, 0.0))
        .expect("finite limits");
    let marker = ax.indicate_inset(
        [0.25, 0.25, 0.5, 0.5],
        None,
        PatchProps::default().alpha(1.0),
    );
    let extent = marker.window_extent().expect("drawn");
    assert_eq!(
        (extent.x0(), extent.x1(), extent.y0(), extent.y1()),
        (160.0, 480.0, 120.0, 360.0)
    );
    assert_eq!(marker.alpha(), Some(1.0));
    assert_eq!(marker.connectors(), None);
}

#[test]
fn connectors_keep_styles_of_their_own() {
    let fig = Figure::new();
    let (ax, inset) = zoomed(&fig);
    let marker = ax.indicate_inset_zoom(&inset, PatchProps::default().linewidth(2.0));
    let connectors = marker.connectors().expect("an inset");
    assert_eq!(connectors.len(), 4);
    marker
        .rectangle()
        .update_props(PatchProps::default().linewidth(42.0));
    assert!(connectors.iter().all(|c| close(c.linewidth(), 2.0)));
    marker.update_props(
        PatchProps::default()
            .linewidth(7.0)
            .linestyle(LineStyle::Dotted)
            .alpha(0.8),
    );
    let rectangle = marker.rectangle();
    assert_eq!((rectangle.linewidth(), rectangle.alpha()), (7.0, Some(0.8)));
    for patch in &connectors {
        assert_eq!((patch.linewidth(), patch.alpha()), (7.0, Some(0.8)));
    }
    assert_eq!(marker.alpha(), Some(0.8));
    connectors[1].update_props(PatchProps::default().linestyle(LineStyle::Dashed));
    connectors[1].set_visible(true);
    fig.to_png()
        .expect("a connector of its own style draws apart");
}

#[test]
fn visibility_is_chosen_once() {
    let fig = Figure::new();
    let (ax, inset) = zoomed(&fig);
    let marker = ax.indicate_inset_zoom(&inset, PatchProps::default());
    fig.to_png().expect("drawn");
    let shown = |marker: &super::InsetIndicator| -> Vec<bool> {
        marker
            .connectors()
            .expect("an inset")
            .iter()
            .map(ArtistHandle::visible)
            .collect()
    };
    let chosen = shown(&marker);
    assert_eq!(chosen.iter().filter(|&&v| v).count(), 2);
    // Moving the inset below left of the rectangle keeps the choice.
    inset
        .set_xlim(8.0, 9.0)
        .and_then(|ax| ax.set_ylim(8.0, 9.0))
        .expect("finite limits");
    assert_eq!(shown(&marker), chosen);
}
