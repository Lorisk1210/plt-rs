//! Scrollable lists of rows.

use std::fmt;
use std::rc::Rc;

use plt_core::Point;
use plt_platform::{ButtonState, Key, MouseButton, ScrollDelta};

use super::frame::paint_frame;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::icon::Icon;
use crate::text::{TextLayout, TextStyle};
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// Space between the list's outline and its rows.
const INSET: Spacing = Spacing::Xs;
/// Space between a row's edges and its icon and text.
const ROW_PADDING: Spacing = Spacing::Sm;
/// How many rows a list shows unless told otherwise.
const DEFAULT_VISIBLE_ROWS: usize = 8;
/// Rows scrolled per wheel notch.
const ROWS_PER_NOTCH: f64 = 3.0;

/// One row of a [`List`].
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// The text shown.
    pub label: String,
    /// An icon before the text.
    pub icon: Option<Icon>,
}

impl ListItem {
    /// A row showing `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
        }
    }

    /// The row with `icon` before its text.
    #[must_use]
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// Makes the message sent for a row.
type RowMessage<M> = Rc<dyn Fn(usize) -> M>;

/// Pixel lengths of a list from the last layout.
#[derive(Debug, Clone, Copy, Default)]
struct Metrics {
    row_height: f64,
    /// Space between the outline and the rows.
    inset: f64,
    /// Space between a row's edges and its content.
    padding: f64,
    /// The height of the visible rows.
    viewport: f64,
    /// Width of the scroll bar.
    scrollbar_width: f64,
    /// Shortest scroll bar thumb.
    min_thumb: f64,
}

/// A list of rows with at most one selected: a click selects a row, a
/// double click or Enter activates it, the arrow keys move the selection,
/// and the wheel or the scroll bar (shown when the rows overflow)
/// scrolls.
pub struct List<M> {
    items: Vec<ListItem>,
    selected: Option<usize>,
    /// Scroll offset in pixels.
    scroll: f64,
    on_select: RowMessage<M>,
    on_activate: Option<RowMessage<M>>,
    visible_rows: usize,
    /// The labels laid out by the last measure.
    layouts: Vec<TextLayout>,
    metrics: Metrics,
    /// The row under the pointer.
    hot: Option<usize>,
    /// While the scroll bar's thumb is dragged: the pointer's distance
    /// from the thumb's top.
    dragging: Option<f64>,
}

impl<M> fmt::Debug for List<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("List")
            .field("items", &self.items.len())
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl<M> List<M> {
    /// A list of `items` sending `on_select(row)` when a row is selected.
    pub fn new(items: Vec<ListItem>, on_select: impl Fn(usize) -> M + 'static) -> Self {
        Self {
            items,
            selected: None,
            scroll: 0.0,
            on_select: Rc::new(on_select),
            on_activate: None,
            visible_rows: DEFAULT_VISIBLE_ROWS,
            layouts: Vec::new(),
            metrics: Metrics::default(),
            hot: None,
            dragging: None,
        }
    }

    /// Also sends `on_activate(row)` when a row is double-clicked or Enter
    /// is pressed on it.
    #[must_use]
    pub fn on_activate(mut self, on_activate: impl Fn(usize) -> M + 'static) -> Self {
        self.on_activate = Some(Rc::new(on_activate));
        self
    }

    /// The list asking for room for `rows` rows (8 by default).
    #[must_use]
    pub fn with_visible_rows(mut self, rows: usize) -> Self {
        self.visible_rows = rows;
        self
    }

    /// The rows.
    pub fn items(&self) -> &[ListItem] {
        &self.items
    }

    /// Replaces the rows, scrolled to the top with nothing selected.
    pub fn set_items(&mut self, items: Vec<ListItem>) {
        self.items = items;
        self.selected = None;
        self.hot = None;
        self.scroll = 0.0;
        self.layouts.clear();
    }

    /// The selected row.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Selects `row` (nothing when `None` or out of range) and scrolls it
    /// into view.
    pub fn set_selected(&mut self, row: Option<usize>) {
        self.selected = row.filter(|&row| row < self.items.len());
        self.scroll_to_selection();
    }

    fn scroll_to_selection(&mut self) {
        let Some(row) = self.selected else {
            return;
        };
        let Metrics {
            row_height,
            viewport,
            ..
        } = self.metrics;
        let top = row as f64 * row_height;
        if top < self.scroll {
            self.scroll = top;
        } else if top + row_height > self.scroll + viewport {
            self.scroll = top + row_height - viewport;
        }
    }

    /// The height of all the rows.
    fn content_height(&self) -> f64 {
        self.items.len() as f64 * self.metrics.row_height
    }

    /// The largest useful scroll offset.
    fn max_scroll(&self) -> f64 {
        (self.content_height() - self.metrics.viewport).max(0.0)
    }

    /// The row at height `y` within the visible rows.
    fn row_at(&self, y: f64) -> Option<usize> {
        let row_height = self.metrics.row_height;
        if row_height <= 0.0 || y < 0.0 || y >= self.metrics.viewport {
            return None;
        }
        let row = ((y + self.scroll) / row_height).floor() as usize;
        (row < self.items.len()).then_some(row)
    }

    /// The area of the rows inside the outline of a list at `rect`.
    fn rows_area(&self, rect: Rect) -> Rect {
        rect.inset(self.metrics.inset)
    }

    /// The scroll bar's track and thumb, when the rows overflow.
    fn scroll_bar(&self, rect: Rect) -> Option<(Rect, Rect)> {
        let max_scroll = self.max_scroll();
        if max_scroll <= 0.0 {
            return None;
        }
        let area = self.rows_area(rect);
        let width = self.metrics.scrollbar_width;
        let track = Rect::new(area.max_x() - width, area.y, width, area.height);
        let height = (track.height * self.metrics.viewport / self.content_height())
            .max(self.metrics.min_thumb)
            .min(track.height);
        let top = track.y + (track.height - height) * self.scroll / max_scroll;
        Some((track, Rect::new(track.x, top, width, height)))
    }

    /// Scrolls so that the scroll bar's thumb starts at height `top`.
    fn drag_thumb_to(&mut self, cx: &mut EventCx<'_, M>, rect: Rect, top: f64) {
        if let Some((track, thumb)) = self.scroll_bar(rect) {
            let travel = track.height - thumb.height;
            if travel > 0.0 {
                self.scroll_to(cx, (top - track.y) / travel * self.max_scroll());
            }
        }
    }

    fn select(&mut self, cx: &mut EventCx<'_, M>, row: usize) {
        self.set_selected(Some(row));
        cx.emit((self.on_select)(row));
        cx.request_paint();
    }

    fn activate(&self, cx: &mut EventCx<'_, M>, row: usize) {
        if let Some(on_activate) = &self.on_activate {
            cx.emit(on_activate(row));
        }
    }

    fn scroll_to(&mut self, cx: &mut EventCx<'_, M>, scroll: f64) {
        self.scroll = scroll.clamp(0.0, self.max_scroll());
        cx.request_paint();
    }

    fn paint_row(
        &self,
        cx: &mut PaintCx<'_>,
        area: Rect,
        row: usize,
        focused: bool,
    ) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let Metrics {
            row_height,
            padding,
            ..
        } = self.metrics;
        let top = area.y + row as f64 * row_height - self.scroll;
        let band = Rect::new(area.x, top, area.width, row_height).snapped();
        let radius = theme.corner_radius * scale;
        let wash = if self.selected == Some(row) {
            Some(if focused {
                theme.selection
            } else {
                theme.selection_inactive
            })
        } else if self.hot == Some(row) && self.dragging.is_none() {
            Some(theme.hover)
        } else {
            None
        };
        let mut canvas = cx.canvas().clipped(area);
        if let Some(wash) = wash {
            canvas.fill_rounded_rect(band, radius, wash)?;
        }
        let mut x = band.x + padding;
        if let Some(icon) = &self.items[row].icon {
            let side = theme.small_icon_size * scale;
            let at = Rect::new(x, band.center().y - side / 2.0, side, side);
            icon.paint(&mut canvas, at, theme.text_secondary)?;
            x += side + padding;
        }
        let layout = &self.layouts[row];
        let text_top = band.center().y - layout.size().height / 2.0;
        canvas.draw_text(layout, Point::new(x, text_top), theme.text)
    }
}

impl<M: 'static> Widget<M> for List<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let style = TextStyle::new(theme.font_size);
        self.layouts = self
            .items
            .iter()
            .map(|item| TextLayout::new(&item.label, &style, scale))
            .collect::<Result<_, _>>()?;
        let padding = theme.space(ROW_PADDING) * scale;
        let inset = theme.space(INSET) * scale;
        self.metrics.row_height = theme.control_height * scale;
        self.metrics.padding = padding;
        self.metrics.inset = inset;
        self.metrics.scrollbar_width = theme.scrollbar_width * scale;
        self.metrics.min_thumb = theme.control_height * scale;
        let icon = theme.small_icon_size * scale + padding;
        let widest = self
            .items
            .iter()
            .zip(&self.layouts)
            .map(|(item, layout)| {
                layout.size().width + if item.icon.is_some() { icon } else { 0.0 }
            })
            .fold(0.0, f64::max);
        let scroll_bar = (theme.scrollbar_width + theme.space(Spacing::Xs)) * scale;
        let width =
            (widest + 2.0 * (padding + inset) + scroll_bar).max(theme.list_min_width * scale);
        let height = self.visible_rows as f64 * self.metrics.row_height + 2.0 * inset;
        Ok(constraints.constrain(Size::new(width, height)))
    }

    fn arrange(&mut self, _cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        self.metrics.viewport = self.rows_area(rect).height;
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
        Ok(())
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let status = cx.status();
        paint_frame(cx, rect, theme.card_radius * scale, theme.surface)?;
        let area = self.rows_area(rect);
        let row_height = self.metrics.row_height;
        if row_height > 0.0 {
            let first = (self.scroll / row_height).floor() as usize;
            let shown = (area.height / row_height).ceil() as usize + 1;
            for row in (first..self.items.len()).take(shown) {
                self.paint_row(cx, area, row, status.focused)?;
            }
        }
        if let Some((_, thumb)) = self.scroll_bar(rect) {
            let color = if status.hot || self.dragging.is_some() {
                theme.border_hot
            } else {
                theme.border
            };
            cx.canvas()
                .fill_rounded_rect(thumb, thumb.width / 2.0, color)?;
        }
        Ok(())
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        let area = self.rows_area(rect);
        match event {
            UiEvent::PointerDown {
                button: MouseButton::Left,
                position,
                click_count,
                ..
            } => {
                if let Some((track, thumb)) = self.scroll_bar(rect)
                    && position.x >= track.x
                {
                    // A press beside the thumb moves it under the pointer.
                    let anchor = if thumb.contains(*position) {
                        position.y - thumb.y
                    } else {
                        thumb.height / 2.0
                    };
                    self.dragging = Some(anchor);
                    self.drag_thumb_to(cx, rect, position.y - anchor);
                } else if let Some(row) = self.row_at(position.y - area.y) {
                    self.select(cx, row);
                    if *click_count >= 2 {
                        self.activate(cx, row);
                    }
                }
            }
            UiEvent::PointerUp { .. } => {
                self.dragging = None;
                cx.request_paint();
            }
            UiEvent::PointerMove { position, .. } if self.dragging.is_some() => {
                let anchor = self.dragging.unwrap_or_default();
                self.drag_thumb_to(cx, rect, position.y - anchor);
            }
            UiEvent::PointerEnter(position) | UiEvent::PointerMove { position, .. } => {
                let hot = self.row_at(position.y - area.y);
                if hot != self.hot {
                    self.hot = hot;
                    cx.request_paint();
                }
            }
            UiEvent::PointerLeave => {
                self.hot = None;
                cx.request_paint();
            }
            UiEvent::Wheel { delta, .. } => {
                let pixels = match *delta {
                    ScrollDelta::Lines { y, .. } => y * ROWS_PER_NOTCH * self.metrics.row_height,
                    ScrollDelta::Pixels { y, .. } => y,
                };
                self.scroll_to(cx, self.scroll - pixels);
            }
            UiEvent::Key(key) if key.state == ButtonState::Pressed => {
                let last = self.items.len().checked_sub(1);
                let row = match (&key.key, self.selected) {
                    (Key::Down, None) | (Key::Home, _) => last.map(|_| 0),
                    (Key::Down, Some(row)) => last.map(|last| (row + 1).min(last)),
                    (Key::Up, Some(row)) => Some(row.saturating_sub(1)),
                    (Key::End, _) => last,
                    (Key::Enter, Some(row)) => {
                        self.activate(cx, row);
                        None
                    }
                    _ => None,
                };
                if let Some(row) = row {
                    self.select(cx, row);
                }
            }
            _ => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(rows: usize) -> List<usize> {
        let items = (0..rows)
            .map(|i| ListItem::new(format!("row {i}")))
            .collect();
        let mut list = List::new(items, |row| row);
        list.metrics = Metrics {
            row_height: 10.0,
            inset: 0.0,
            padding: 0.0,
            viewport: 50.0,
            scrollbar_width: 6.0,
            min_thumb: 20.0,
        };
        list
    }

    #[test]
    fn rows_under_the_pointer_and_scrolling() {
        let mut list = list(20);
        assert_eq!(list.row_at(25.0), Some(2));
        assert_eq!(list.row_at(55.0), None, "below the visible rows");
        assert_eq!(list.max_scroll(), 150.0);
        list.set_selected(Some(9));
        assert_eq!(list.scroll, 50.0, "the selection scrolls into view");
        assert_eq!(list.row_at(0.0), Some(5));
        list.set_selected(Some(99));
        assert_eq!(list.selected(), None);
        list.set_items(vec![ListItem::new("only")]);
        assert_eq!((list.items().len(), list.scroll), (1, 0.0));
        assert_eq!(list.row_at(15.0), None);
    }

    #[test]
    fn the_scroll_bar_shows_where_the_rows_are() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert!(list(5).scroll_bar(rect).is_none(), "all rows fit");
        let mut long = list(100);
        let (track, top) = long.scroll_bar(rect).unwrap();
        assert_eq!((track.x, track.height), (94.0, 50.0));
        assert_eq!(top.y, 0.0);
        assert_eq!(top.height, 20.0, "never shorter than the minimum");
        long.scroll = long.max_scroll();
        let (_, bottom) = long.scroll_bar(rect).unwrap();
        assert_eq!(bottom.max_y(), track.max_y());
    }

    #[test]
    fn dragging_the_thumb_scrolls() {
        let mut long = list(100);
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        let press = |y| UiEvent::PointerDown {
            button: MouseButton::Left,
            position: Point::new(97.0, y),
            modifiers: plt_platform::Modifiers::NONE,
            click_count: 1,
        };
        long.event(&mut cx, &press(5.0), rect);
        let drag = UiEvent::PointerMove {
            position: Point::new(97.0, 35.0),
            modifiers: plt_platform::Modifiers::NONE,
        };
        long.event(&mut cx, &drag, rect);
        assert_eq!(long.scroll, long.max_scroll(), "dragged to the bottom");
        long.event(&mut cx, &press(0.0), rect);
        assert!(long.scroll < 50.0, "a press above the thumb pages up");
        assert!(messages.is_empty(), "the scroll bar selects nothing");
    }
}
