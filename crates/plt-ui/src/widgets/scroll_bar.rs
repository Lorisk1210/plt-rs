//! The vertical scroll bar of lists and scrolled views: how far content
//! taller than its viewport is scrolled, the thumb showing where, and
//! dragging it.

use plt_core::Point;
use plt_platform::ScrollDelta;

use crate::error::UiError;
use crate::geometry::{Rect, Size};
use crate::theme::Theme;
use crate::widget::PaintCx;

/// Lines a wheel notch scrolls.
const LINES_PER_NOTCH: f64 = 3.0;

/// The scroll offset into content taller than its viewport, and the scroll
/// bar beside it. Lengths are in pixels.
///
/// The bar's track is a strip beside the viewport, as long as it and as
/// wide as the pointer's target (the widget chooses it); the thumb drawn
/// in it is slim, and as much shorter than the track as the viewport is
/// than the content.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct ScrollBar {
    offset: f64,
    content: f64,
    viewport: f64,
    /// Width of the slim thumb drawn.
    thumb_width: f64,
    /// Shortest thumb.
    min_thumb: f64,
    /// While the thumb is dragged: the pointer's distance from its top.
    dragging: Option<f64>,
}

impl ScrollBar {
    /// Takes the thumb's width and shortest length from `theme` at
    /// `scale`.
    pub(crate) fn set_metrics(&mut self, theme: &Theme, scale: f64) {
        self.thumb_width = theme.scrollbar_width * scale;
        self.min_thumb = theme.control_height * scale;
    }

    /// Sets the heights of the content and of the viewport showing it,
    /// keeping the offset within reach.
    pub(crate) fn set_extent(&mut self, content: f64, viewport: f64) {
        self.content = content;
        self.viewport = viewport;
        self.offset = self.offset.clamp(0.0, self.max_offset());
    }

    /// How far the content is scrolled.
    pub(crate) fn offset(&self) -> f64 {
        self.offset
    }

    /// The height of the viewport.
    pub(crate) fn viewport(&self) -> f64 {
        self.viewport
    }

    /// The largest useful offset.
    pub(crate) fn max_offset(&self) -> f64 {
        (self.content - self.viewport).max(0.0)
    }

    /// Whether the content is taller than the viewport, so that the bar
    /// shows.
    pub(crate) fn overflows(&self) -> bool {
        self.max_offset() > 0.0
    }

    /// Scrolls to `offset`, within reach; returns whether it moved.
    pub(crate) fn scroll_to(&mut self, offset: f64) -> bool {
        let offset = offset.clamp(0.0, self.max_offset());
        #[expect(clippy::float_cmp, reason = "an offset kept is the very same value")]
        let moved = offset != self.offset;
        self.offset = offset;
        moved
    }

    /// Scrolls back to the top and lets go of the thumb.
    pub(crate) fn reset(&mut self) {
        self.offset = 0.0;
        self.dragging = None;
    }

    /// Whether the thumb is being dragged.
    pub(crate) fn dragging(&self) -> bool {
        self.dragging.is_some()
    }

    /// The thumb in `track`, across the track's width, while the content
    /// overflows.
    pub(crate) fn thumb(&self, track: Rect) -> Option<Rect> {
        let max_offset = self.max_offset();
        if max_offset <= 0.0 {
            return None;
        }
        let height = (track.height * self.viewport / self.content)
            .max(self.min_thumb)
            .min(track.height);
        let top = track.y + (track.height - height) * self.offset / max_offset;
        Some(Rect::new(track.x, top, track.width, height))
    }

    /// Starts dragging the thumb when the left button is pressed at
    /// `position` on `track` (a press beside the thumb moves it under the
    /// pointer); returns whether the press was on the bar.
    pub(crate) fn press(&mut self, track: Rect, position: Point) -> bool {
        let Some(thumb) = self.thumb(track).filter(|_| track.contains(position)) else {
            return false;
        };
        let anchor = if thumb.contains(position) {
            position.y - thumb.y
        } else {
            thumb.height / 2.0
        };
        self.dragging = Some(anchor);
        self.drag_thumb_to(track, position.y - anchor);
        true
    }

    /// Follows the pointer at height `y` while the thumb is dragged;
    /// returns whether it is.
    pub(crate) fn drag(&mut self, track: Rect, y: f64) -> bool {
        let Some(anchor) = self.dragging else {
            return false;
        };
        self.drag_thumb_to(track, y - anchor);
        true
    }

    /// Lets go of the thumb.
    pub(crate) fn release(&mut self) {
        self.dragging = None;
    }

    /// Scrolls so that the thumb starts at height `top`.
    fn drag_thumb_to(&mut self, track: Rect, top: f64) {
        if let Some(thumb) = self.thumb(track) {
            let travel = track.height - thumb.height;
            if travel > 0.0 {
                self.scroll_to((top - track.y) / travel * self.max_offset());
            }
        }
    }

    /// Paints the slim thumb, level with its place in `track` and centred
    /// across `lane`, darker under the pointer (`hot`) or while dragged.
    pub(crate) fn paint(
        &self,
        cx: &mut PaintCx<'_>,
        track: Rect,
        lane: Rect,
        hot: bool,
    ) -> Result<(), UiError> {
        let Some(thumb) = self.thumb(track) else {
            return Ok(());
        };
        let theme = cx.theme();
        let color = if hot || self.dragging() {
            theme.border_hot
        } else {
            theme.border
        };
        let reach = Rect::new(lane.x, thumb.y, lane.width, thumb.height);
        let width = self.thumb_width;
        let slim = reach.centered(Size::new(width, thumb.height)).snapped();
        cx.canvas().fill_rounded_rect(slim, width / 2.0, color)
    }
}

/// The pixels a wheel `delta` scrolls content up by, lines being `line`
/// pixels high.
pub(crate) fn wheel_pixels(delta: ScrollDelta, line: f64) -> f64 {
    match delta {
        ScrollDelta::Lines { y, .. } => y * LINES_PER_NOTCH * line,
        ScrollDelta::Pixels { y, .. } => y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(content: f64) -> ScrollBar {
        let mut bar = ScrollBar {
            min_thumb: 20.0,
            thumb_width: 6.0,
            ..ScrollBar::default()
        };
        bar.set_extent(content, 50.0);
        bar
    }

    const TRACK: Rect = Rect::new(76.0, 0.0, 24.0, 50.0);

    #[test]
    fn the_thumb_shows_where_the_viewport_is() {
        assert!(bar(40.0).thumb(TRACK).is_none(), "everything fits");
        let mut long = bar(1000.0);
        let top = long.thumb(TRACK).unwrap();
        assert_eq!((top.x, top.y, top.width), (76.0, 0.0, 24.0));
        assert_eq!(top.height, 20.0, "never shorter than the minimum");
        assert!(long.scroll_to(f64::INFINITY));
        assert_eq!(long.offset(), 950.0);
        assert_eq!(long.thumb(TRACK).unwrap().max_y(), TRACK.max_y());
        assert!(!long.scroll_to(2000.0), "already at the bottom");
        long.set_extent(500.0, 50.0);
        assert_eq!(long.offset(), 450.0, "kept within reach");
    }

    #[test]
    fn dragging_the_thumb_scrolls() {
        let mut long = bar(200.0);
        assert!(!long.press(TRACK, Point::new(10.0, 5.0)), "beside the bar");
        assert!(long.press(TRACK, Point::new(90.0, 5.0)));
        assert!(long.drag(TRACK, 35.0));
        assert_eq!(long.offset(), long.max_offset(), "dragged to the bottom");
        long.release();
        assert!(!long.drag(TRACK, 0.0), "let go");
        assert!(long.press(TRACK, Point::new(90.0, 0.0)));
        assert!(long.offset() < 50.0, "a press above the thumb pages up");
    }

    #[test]
    fn wheels_scroll_three_lines_a_notch() {
        let notch = ScrollDelta::Lines { x: 0.0, y: -1.0 };
        assert_eq!(wheel_pixels(notch, 10.0), -30.0);
        let swipe = ScrollDelta::Pixels { x: 0.0, y: 12.0 };
        assert_eq!(wheel_pixels(swipe, 10.0), 12.0);
    }
}
