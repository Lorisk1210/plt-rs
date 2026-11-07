//! Scrollable lists of rows.

use std::fmt;
use std::rc::Rc;
use std::time::Instant;

use pltrs_core::Point;
use pltrs_platform::{ButtonState, Key, KeyEvent, MouseButton};

use super::frame::paint_frame;
use super::scroll_bar::{ScrollBar, wheel_pixels};
use super::type_ahead::TypeAhead;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::icon::Icon;
use crate::text::{Cut, TextLayout};
use crate::theme::{Spacing, Theme};
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// Space between the list's outline and its rows.
const INSET: Spacing = Spacing::Xs;
/// Space between a row's edges and its icon and text.
const ROW_PADDING: Spacing = Spacing::Sm;
/// How many rows a list shows unless told otherwise.
const DEFAULT_VISIBLE_ROWS: usize = 8;

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

    /// The width the row's icon and the `padding` after it take, in
    /// pixels.
    fn icon_width(&self, theme: &Theme, scale: f64, padding: f64) -> f64 {
        if self.icon.is_some() {
            theme.small_icon_size * scale + padding
        } else {
            0.0
        }
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
    /// Width of the strip beside the rows that the scroll bar takes when
    /// the rows overflow: with the inset, a pointer target of the theme's
    /// [`target_size`](crate::Theme::target_size).
    gutter: f64,
}

/// A list of rows with at most one selected: a click selects a row, a
/// double click or Enter activates it, and the wheel or the scroll bar
/// (shown when the rows overflow) scrolls. While the list has the focus,
/// the arrow keys move the selection, Page Up and Page Down by the rows
/// shown, Home and End to the ends, and typing the start of a row's label
/// selects it; the selection scrolls into view.
pub struct List<M> {
    items: Vec<ListItem>,
    selected: Option<usize>,
    /// How far the rows are scrolled, beside the scroll bar.
    bar: ScrollBar,
    on_select: RowMessage<M>,
    on_activate: Option<RowMessage<M>>,
    visible_rows: usize,
    /// The labels laid out by the last measure.
    layouts: Vec<TextLayout>,
    metrics: Metrics,
    /// The row under the pointer.
    hot: Option<usize>,
    /// The start of a label being typed.
    type_ahead: TypeAhead,
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
            bar: ScrollBar::default(),
            on_select: Rc::new(on_select),
            on_activate: None,
            visible_rows: DEFAULT_VISIBLE_ROWS,
            layouts: Vec::new(),
            metrics: Metrics::default(),
            hot: None,
            type_ahead: TypeAhead::default(),
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
        self.bar.reset();
        self.sync_extent();
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
        self.sync_extent();
        let row_height = self.metrics.row_height;
        let (scroll, viewport) = (self.bar.offset(), self.bar.viewport());
        let top = row as f64 * row_height;
        if top < scroll {
            self.bar.scroll_to(top);
        } else if top + row_height > scroll + viewport {
            self.bar.scroll_to(top + row_height - viewport);
        }
    }

    /// The height of all the rows.
    fn content_height(&self) -> f64 {
        self.items.len() as f64 * self.metrics.row_height
    }

    /// Tells the scroll bar the height of all the rows.
    fn sync_extent(&mut self) {
        self.bar
            .set_extent(self.content_height(), self.bar.viewport());
    }

    /// The row at height `y` within the visible rows.
    fn row_at(&self, y: f64) -> Option<usize> {
        let row_height = self.metrics.row_height;
        if row_height <= 0.0 || y < 0.0 || y >= self.bar.viewport() {
            return None;
        }
        let row = ((y + self.bar.offset()) / row_height).floor() as usize;
        (row < self.items.len()).then_some(row)
    }

    /// The area of the rows inside the outline of a list at `rect`.
    fn rows_area(&self, rect: Rect) -> Rect {
        rect.inset(self.metrics.inset)
    }

    /// The width of the rows in `area`: all of it, but the scroll bar's
    /// gutter when the rows overflow.
    fn row_width(&self, area: Rect) -> f64 {
        if self.bar.overflows() {
            (area.width - self.metrics.gutter).max(0.0)
        } else {
            area.width
        }
    }

    /// The scroll bar's track, as wide as its gutter (the pointer's
    /// target), beside the rows of a list at `rect`.
    fn track(&self, rect: Rect) -> Rect {
        let area = self.rows_area(rect);
        let width = self.metrics.gutter;
        Rect::new(area.max_x() - width, area.y, width, area.height)
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

    /// How many rows a page of Page Up and Page Down moves: one fewer than
    /// the rows shown, and at least one.
    fn page(&self) -> usize {
        let row_height = self.metrics.row_height;
        let viewport = self.bar.viewport();
        if row_height > 0.0 {
            // Whole rows of a finite viewport.
            ((viewport / row_height).floor() as usize)
                .saturating_sub(1)
                .max(1)
        } else {
            1
        }
    }

    /// The row a navigation key leads to from the selection.
    fn navigated(&self, key: &KeyEvent) -> Option<usize> {
        let last = self.items.len().checked_sub(1)?;
        if key.modifiers.control() || key.modifiers.alt() {
            return None;
        }
        let Some(row) = self.selected else {
            return match key.key {
                Key::End => Some(last),
                Key::Up | Key::Down | Key::PageUp | Key::PageDown | Key::Home => Some(0),
                _ => None,
            };
        };
        match key.key {
            Key::Up => Some(row.saturating_sub(1)),
            Key::Down => Some((row + 1).min(last)),
            Key::PageUp => Some(row.saturating_sub(self.page())),
            // Pages of rows a hair high saturate the count.
            Key::PageDown => Some(row.saturating_add(self.page()).min(last)),
            Key::Home => Some(0),
            Key::End => Some(last),
            _ => None,
        }
    }

    /// Handles a key press: navigation, type-ahead, and Enter activating
    /// the selected row (when the list has rows to activate).
    fn key(&mut self, cx: &mut EventCx<'_, M>, key: &KeyEvent) {
        if key.key == Key::Enter {
            if let (Some(row), Some(_)) = (self.selected, &self.on_activate) {
                self.activate(cx, row);
                cx.set_handled();
            }
            return;
        }
        let reached = self.navigated(key).or_else(|| {
            let labels = self.items.iter().map(|item| item.label.as_str());
            self.type_ahead
                .search(key, Instant::now(), labels, self.selected)
        });
        if let Some(row) = reached {
            if self.selected != Some(row) {
                self.select(cx, row);
            }
            cx.set_handled();
        }
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
        let top = area.y + row as f64 * row_height - self.bar.offset();
        let band = Rect::new(area.x, top, self.row_width(area), row_height).snapped();
        let radius = theme.corner_radius * scale;
        let wash = if self.selected == Some(row) {
            Some(if focused {
                theme.selection
            } else {
                theme.selection_inactive
            })
        } else if self.hot == Some(row) && !self.bar.dragging() {
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
        let style = theme.text_style();
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
        self.metrics.gutter = (theme.target_size * scale - inset).max(0.0);
        self.bar.set_metrics(theme, scale);
        let widest = self
            .items
            .iter()
            .zip(&self.layouts)
            .map(|(item, layout)| layout.size().width + item.icon_width(theme, scale, padding))
            .fold(0.0, f64::max);
        let width = (widest + 2.0 * (padding + inset) + self.metrics.gutter)
            .max(theme.list_min_width * scale);
        let height = self.visible_rows as f64 * self.metrics.row_height + 2.0 * inset;
        Ok(constraints.constrain(Size::new(width, height)))
    }

    fn arrange(&mut self, cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        let area = self.rows_area(rect);
        self.bar.set_extent(self.content_height(), area.height);
        // Labels too long for their row end in an ellipsis.
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let style = theme.text_style();
        let padding = self.metrics.padding;
        let room = self.row_width(area) - 2.0 * padding;
        for (item, layout) in self.items.iter().zip(&mut self.layouts) {
            let width = room - item.icon_width(theme, scale, padding);
            if layout.size().width > width {
                *layout = TextLayout::fitted(&item.label, &style, scale, width, Cut::End)?;
            }
        }
        Ok(())
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let status = cx.status();
        let radius = theme.card_radius * scale;
        paint_frame(cx, rect, radius, theme.surface, status.focused())?;
        let area = self.rows_area(rect);
        let row_height = self.metrics.row_height;
        if row_height > 0.0 {
            let first = (self.bar.offset() / row_height).floor() as usize;
            // Rows a hair high (at a vanishing scale factor) count beyond
            // `usize`: the cast saturates and every row is a candidate.
            let shown = ((area.height / row_height).ceil() as usize).saturating_add(1);
            for row in (first..self.items.len()).take(shown) {
                self.paint_row(cx, area, row, status.focused())?;
            }
        }
        // The thumb is centred in its target, which reaches the outline.
        let track = self.track(rect);
        let lane = Rect::new(
            track.x,
            track.y,
            track.width + self.metrics.inset,
            track.height,
        );
        self.bar.paint(cx, track, lane, status.hot)
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
                if self.bar.press(self.track(rect), *position) {
                    cx.request_paint();
                } else if let Some(row) = self.row_at(position.y - area.y) {
                    self.select(cx, row);
                    if *click_count >= 2 {
                        self.activate(cx, row);
                    }
                }
            }
            UiEvent::PointerUp { .. } => {
                self.bar.release();
                cx.request_paint();
            }
            UiEvent::PointerMove { position, .. } if self.bar.dragging() => {
                self.bar.drag(self.track(rect), position.y);
                cx.request_paint();
            }
            UiEvent::PointerEnter(position) | UiEvent::PointerMove { position, .. } => {
                let hot = self.row_at(position.y - area.y);
                if hot != self.hot {
                    self.hot = hot;
                    cx.request_paint();
                }
            }
            UiEvent::PointerLeave(_) => {
                self.hot = None;
                cx.request_paint();
            }
            UiEvent::Wheel { delta, .. } => {
                let pixels = wheel_pixels(*delta, self.metrics.row_height);
                self.bar.scroll_to(self.bar.offset() - pixels);
                cx.request_paint();
                cx.set_handled();
            }
            UiEvent::Key(key) if key.state == ButtonState::Pressed => self.key(cx, key),
            _ => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests;
