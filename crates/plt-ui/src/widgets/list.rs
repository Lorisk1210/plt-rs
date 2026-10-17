//! Scrollable lists of rows.

use std::fmt;
use std::rc::Rc;

use plt_core::{Point, Rgba};
use plt_platform::{ButtonState, Key, MouseButton, ScrollDelta};

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::icon::Icon;
use crate::text::{TextLayout, TextStyle};
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

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

/// A list of rows with at most one selected: a click selects a row, a
/// double click or Enter activates it, the arrow keys move the selection
/// and the wheel scrolls.
pub struct List<M> {
    items: Vec<ListItem>,
    selected: Option<usize>,
    /// Scroll offset in pixels.
    scroll: f64,
    on_select: RowMessage<M>,
    on_activate: Option<RowMessage<M>>,
    /// The labels laid out by the last measure.
    layouts: Vec<TextLayout>,
    /// Row height and padding in pixels, from the last measure.
    row_height: f64,
    padding: f64,
    /// The height of the visible rows, from the last arrangement.
    viewport: f64,
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
            layouts: Vec::new(),
            row_height: 0.0,
            padding: 0.0,
            viewport: 0.0,
        }
    }

    /// Also sends `on_activate(row)` when a row is double-clicked or Enter
    /// is pressed on it.
    #[must_use]
    pub fn on_activate(mut self, on_activate: impl Fn(usize) -> M + 'static) -> Self {
        self.on_activate = Some(Rc::new(on_activate));
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
        #[expect(clippy::cast_precision_loss, reason = "row counts are small")]
        let top = row as f64 * self.row_height;
        if top < self.scroll {
            self.scroll = top;
        } else if top + self.row_height > self.scroll + self.viewport {
            self.scroll = top + self.row_height - self.viewport;
        }
    }

    /// The largest useful scroll offset.
    fn max_scroll(&self) -> f64 {
        #[expect(clippy::cast_precision_loss, reason = "row counts are small")]
        let content = self.items.len() as f64 * self.row_height;
        (content - self.viewport).max(0.0)
    }

    /// The row at height `y` within the list.
    fn row_at(&self, y: f64) -> Option<usize> {
        if self.row_height <= 0.0 || y < 0.0 {
            return None;
        }
        let row = ((y + self.scroll) / self.row_height).floor();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a non-negative row index well within usize"
        )]
        let row = row as usize;
        (row < self.items.len()).then_some(row)
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
}

impl<M: 'static> Widget<M> for List<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let style = TextStyle::new(cx.theme().font_size);
        let scale = cx.scale_factor();
        self.layouts = self
            .items
            .iter()
            .map(|item| TextLayout::new(&item.label, &style, scale))
            .collect::<Result<_, _>>()?;
        let line = TextLayout::new("", &style, scale)?.size().height;
        self.padding = cx.theme().padding * scale;
        self.row_height = line + self.padding;
        Ok(constraints.constrain(Size::new(320.0 * scale, 8.0 * self.row_height)))
    }

    fn arrange(&mut self, _cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        self.viewport = rect.height;
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
        Ok(())
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let focused = cx.status().focused;
        let radius = theme.corner_radius * scale;
        let border = if focused {
            theme.accent
        } else {
            theme.separator
        };
        cx.canvas().fill_rounded_rect(rect, radius, border)?;
        let inner = rect.inset(scale);
        cx.canvas()
            .fill_rounded_rect(inner, radius - scale, theme.surface)?;
        let icon_size = self.row_height - self.padding;
        for (row, (item, layout)) in self.items.iter().zip(&self.layouts).enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "row counts are small")]
            let top = inner.y + row as f64 * self.row_height - self.scroll;
            if top + self.row_height < inner.y || top > inner.max_y() {
                continue;
            }
            let band = Rect::new(inner.x, top, inner.width, self.row_height);
            if self.selected == Some(row) {
                let accent = theme.accent;
                let alpha = if focused { 0.25 } else { 0.12 };
                let highlight = Rgba::new(accent.r, accent.g, accent.b, alpha);
                cx.canvas().fill_rect(band.intersect(inner), highlight)?;
            }
            let mut x = inner.x + self.padding;
            if let Some(icon) = &item.icon {
                let at = Rect::new(x, top + self.padding / 2.0, icon_size, icon_size);
                icon.paint(cx.canvas(), at, theme.text)?;
                x += icon_size + self.padding;
            }
            let text_top = top + (self.row_height - layout.size().height) / 2.0;
            cx.canvas()
                .draw_text(layout, Point::new(x, text_top), theme.text)?;
        }
        Ok(())
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        match event {
            UiEvent::PointerDown {
                button: MouseButton::Left,
                position,
                click_count,
                ..
            } => {
                if let Some(row) = self.row_at(position.y - rect.y) {
                    self.select(cx, row);
                    if *click_count >= 2 {
                        self.activate(cx, row);
                    }
                }
            }
            UiEvent::Wheel { delta, .. } => {
                let lines = match *delta {
                    ScrollDelta::Lines { y, .. } => y * 3.0 * self.row_height,
                    ScrollDelta::Pixels { y, .. } => y,
                };
                self.scroll = (self.scroll - lines).clamp(0.0, self.max_scroll());
                cx.request_paint();
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

    #[test]
    fn rows_under_the_pointer_and_scrolling() {
        let items = (0..20).map(|i| ListItem::new(format!("row {i}"))).collect();
        let mut list = List::new(items, |row| row);
        list.row_height = 10.0;
        list.viewport = 50.0;
        assert_eq!(list.row_at(25.0), Some(2));
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
}
