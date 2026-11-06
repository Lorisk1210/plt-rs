//! Where the decorations are and what the pointer is over: the title
//! bar and its buttons, and the resize border's edges and corners. All
//! lengths are logical pixels.

use crate::decoration::{ButtonLayout, TitleBarButton, buttons};
use crate::geometry::PhysicalRect;

/// Height of the title bar, in logical pixels.
pub(crate) const TITLE_BAR_HEIGHT: u32 = 34;
/// Width of the invisible resize border around the window, in logical
/// pixels.
pub(crate) const RESIZE_MARGIN: u32 = 10;
/// How far along an edge the resize corners reach, in logical pixels.
const CORNER: u32 = 24;

/// The edges an interactive resize moves (`xdg_toplevel.resize_edge`
/// bits: top 1, bottom 2, left 4, right 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Edges(pub(crate) u32);

impl Edges {
    /// The top edge.
    pub(crate) const TOP: u32 = 1;
    /// The bottom edge.
    pub(crate) const BOTTOM: u32 = 2;
    /// The left edge.
    pub(crate) const LEFT: u32 = 4;
    /// The right edge.
    pub(crate) const RIGHT: u32 = 8;
}

/// What the pointer is over in the decorations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hit {
    /// The title bar outside its buttons: drag to move.
    Title,
    /// A title bar button.
    Button(TitleBarButton),
    /// A resize border.
    Resize(Edges),
}

/// What is at `(x, y)` of a title bar `width` logical pixels wide with
/// the buttons of `layout`.
pub(crate) fn title_bar_hit(x: f64, y: f64, width: u32, layout: &ButtonLayout) -> Hit {
    let inside = |rect: PhysicalRect| {
        x >= f64::from(rect.x)
            && x < f64::from(rect.x + rect.width)
            && y >= f64::from(rect.y)
            && y < f64::from(rect.y + rect.height)
    };
    buttons(layout, width, TITLE_BAR_HEIGHT)
        .into_iter()
        .find(|&(_, rect)| inside(rect))
        .map_or(Hit::Title, |(button, _)| Hit::Button(button))
}

/// The edges at `(x, y)` of the resize border, a frame `width` × `height`
/// logical pixels around the window with [`RESIZE_MARGIN`] on each side;
/// `None` inside the window.
pub(crate) fn border_hit(x: f64, y: f64, width: u32, height: u32) -> Option<Edges> {
    let (width, height) = (f64::from(width), f64::from(height));
    let margin = f64::from(RESIZE_MARGIN);
    let corner = f64::from(CORNER);
    let near = |position: f64, low: f64, high: f64| (position < low, position >= high);
    let (left, right) = near(x, margin, width - margin);
    let (top, bottom) = near(y, margin, height - margin);
    let (left_corner, right_corner) = near(x, corner, width - corner);
    let (top_corner, bottom_corner) = near(y, corner, height - corner);
    let mut edges = 0;
    if top || ((left || right) && top_corner) {
        edges |= Edges::TOP;
    }
    if bottom || ((left || right) && bottom_corner) {
        edges |= Edges::BOTTOM;
    }
    if left || ((top || bottom) && left_corner) {
        edges |= Edges::LEFT;
    }
    if right || ((top || bottom) && right_corner) {
        edges |= Edges::RIGHT;
    }
    (edges != 0).then_some(Edges(edges))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_bar_hits() {
        let h = f64::from(TITLE_BAR_HEIGHT);
        let layout = ButtonLayout::default();
        assert_eq!(title_bar_hit(5.0, 5.0, 300, &layout), Hit::Title);
        assert_eq!(
            title_bar_hit(299.0, h - 1.0, 300, &layout),
            Hit::Button(TitleBarButton::Close)
        );
        assert_eq!(
            title_bar_hit(300.0 - h - 1.0, 1.0, 300, &layout),
            Hit::Button(TitleBarButton::Maximize)
        );
        assert_eq!(
            title_bar_hit(300.0 - 2.0 * h - 1.0, 1.0, 300, &layout),
            Hit::Button(TitleBarButton::Minimize)
        );
        let close_only = ButtonLayout::parse("appmenu:close");
        assert_eq!(
            title_bar_hit(300.0 - h - 1.0, 1.0, 300, &close_only),
            Hit::Title
        );
    }

    #[test]
    fn border_hits_find_edges_and_corners() {
        let (w, h) = (400, 300);
        assert_eq!(border_hit(200.0, 150.0, w, h), None);
        assert_eq!(border_hit(2.0, 150.0, w, h), Some(Edges(Edges::LEFT)));
        assert_eq!(border_hit(398.0, 150.0, w, h), Some(Edges(Edges::RIGHT)));
        assert_eq!(border_hit(200.0, 1.0, w, h), Some(Edges(Edges::TOP)));
        assert_eq!(border_hit(200.0, 299.0, w, h), Some(Edges(Edges::BOTTOM)));
        assert_eq!(
            border_hit(2.0, 20.0, w, h),
            Some(Edges(Edges::LEFT | Edges::TOP)),
            "the corner reaches along the edge"
        );
        assert_eq!(
            border_hit(390.0, 298.0, w, h),
            Some(Edges(Edges::RIGHT | Edges::BOTTOM))
        );
        assert_eq!(border_hit(f64::NAN, f64::NAN, w, h), None);
    }
}
