//! Tab bars: the names of the pages of a [`Stack`](crate::Stack), one of
//! them selected.

use std::fmt;
use std::rc::Rc;

use plt_platform::{ButtonState, Key, KeyEvent, MouseButton};

use super::frame::paint_focus_ring;
use super::segments::Segments;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// Space left and right of a tab's name.
const PADDING: Spacing = Spacing::Md;
/// How much taller than controls the bar is.
const EXTRA_HEIGHT: Spacing = Spacing::Sm;
/// Space between the top and bottom of a tab and its hover wash.
const WASH_INSET: Spacing = Spacing::Xs;

/// Makes the message sent for a tab.
type TabMessage<M> = Rc<dyn Fn(usize) -> M>;

/// A row of tabs over a hairline spanning the bar; the selected tab's
/// name is in the accent colour and underlined. Pressing a tab selects it
/// and sends a message with its index.
///
/// From the keyboard, as in Qt and GTK: Ctrl+Tab and Ctrl+PageDown select
/// the next tab and Ctrl+Shift+Tab and Ctrl+PageUp the previous one
/// (wrapping around) wherever the focus is, unless the focused widget
/// uses those keys; and when Tab gives the bar the focus (clicks do not),
/// the arrow keys, Home and End select tabs.
pub struct Tabs<M> {
    labels: Vec<String>,
    selected: usize,
    on_select: TabMessage<M>,
    segments: Segments,
}

impl<M> fmt::Debug for Tabs<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tabs")
            .field("labels", &self.labels)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl<M> Tabs<M> {
    /// Tabs named `labels` with the first selected, sending
    /// `on_select(index)` when the user selects one.
    pub fn new(labels: Vec<String>, on_select: impl Fn(usize) -> M + 'static) -> Self {
        Self {
            labels,
            selected: 0,
            on_select: Rc::new(on_select),
            segments: Segments::default(),
        }
    }

    /// The tab names.
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// The selected tab.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Selects tab `index` (ignored when out of range); no message is
    /// sent.
    pub fn set_selected(&mut self, index: usize) {
        if index < self.labels.len() {
            self.selected = index;
        }
    }

    /// Selects tab `index` for the user, sending a message if it is
    /// another.
    fn select(&mut self, cx: &mut EventCx<'_, M>, index: usize) {
        if index != self.selected {
            self.selected = index;
            cx.emit((self.on_select)(index));
            cx.request_paint();
        }
    }

    /// The tab a Ctrl+Tab-like key press selects, wrapping around.
    fn cycled(&self, key: &KeyEvent) -> Option<usize> {
        let count = self.labels.len();
        if count == 0 || key.state != ButtonState::Pressed || !key.modifiers.control() {
            return None;
        }
        let forward = match key.key {
            Key::Tab => !key.modifiers.shift(),
            Key::PageDown => true,
            Key::PageUp => false,
            _ => return None,
        };
        let step = if forward { 1 } else { count - 1 };
        Some((self.selected + step) % count)
    }
}

impl<M: 'static> Widget<M> for Tabs<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        self.segments.measure(
            &self.labels,
            &theme.text_style(),
            scale,
            theme.space(PADDING) * scale,
            0.0,
        )?;
        let height = (theme.control_height + theme.space(EXTRA_HEIGHT)) * scale;
        Ok(constraints.constrain(Size::new(self.segments.total_width(), height)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let hairline = Rect::new(rect.x, rect.max_y() - scale, rect.width, scale);
        cx.canvas().fill_rect(hairline, theme.separator)?;
        let indicator = theme.indicator_width * scale;
        let inset = theme.space(WASH_INSET) * scale;
        let radius = theme.corner_radius * scale;
        let hot = self.segments.hot;
        for (index, x, width) in self.segments.placed(rect.x) {
            let tab = Rect::new(x, rect.y, width, rect.height - indicator).snapped();
            let selected = index == self.selected;
            let wash = Rect::new(tab.x, tab.y + inset, tab.width, tab.height - 2.0 * inset);
            if selected {
                let line = Rect::new(tab.x, rect.max_y() - indicator, tab.width, indicator);
                cx.canvas()
                    .fill_rounded_rect(line.snapped(), indicator / 2.0, theme.accent)?;
                if cx.status().focus_visible() {
                    paint_focus_ring(cx, wash, radius)?;
                }
            } else if hot == Some(index) {
                cx.canvas().fill_rounded_rect(wash, radius, theme.hover)?;
            }
            let color = if selected {
                theme.accent
            } else if hot == Some(index) {
                theme.text
            } else {
                theme.text_secondary
            };
            self.segments.draw_label(cx.canvas(), index, tab, color)?;
        }
        Ok(())
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        match event {
            UiEvent::PointerDown {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if let Some(index) = self.segments.at(position.x - rect.x) {
                    self.select(cx, index);
                }
            }
            UiEvent::Key(key) => {
                let target = self
                    .cycled(key)
                    .or_else(|| self.segments.key_target(key, self.selected, false));
                if let Some(index) = target {
                    self.select(cx, index);
                    cx.set_handled();
                }
            }
            _ => {}
        }
        if self.segments.track_hover(event, rect) {
            cx.request_paint();
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn focus_on_click(&self) -> bool {
        false
    }

    fn handles_shortcut(&self, key: &KeyEvent) -> bool {
        self.cycled(key).is_some()
    }
}

#[cfg(test)]
mod tests {
    use plt_core::Point;
    use plt_platform::Modifiers;

    use super::*;

    #[test]
    fn pressing_a_tab_selects_it() {
        let mut tabs = Tabs::new(vec!["Axes".into(), "Curves".into()], |i| i);
        tabs.segments.widths = vec![50.0, 70.0];
        let rect = Rect::new(10.0, 0.0, 300.0, 36.0);
        let press = |x| UiEvent::PointerDown {
            button: MouseButton::Left,
            position: Point::new(x, 10.0),
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        tabs.event(&mut cx, &press(100.0), rect);
        tabs.event(&mut cx, &press(100.0), rect);
        tabs.event(&mut cx, &press(200.0), rect);
        assert_eq!(messages, [1], "only a change of tab is reported");
        assert_eq!(tabs.selected(), 1);
        tabs.set_selected(9);
        assert_eq!(tabs.selected(), 1);
    }

    #[test]
    fn keys_select_tabs() {
        let mut tabs = Tabs::new(vec!["Axes".into(), "Curves".into(), "Images".into()], |i| i);
        tabs.segments.widths = vec![50.0, 70.0, 70.0];
        let key = |key, modifiers| KeyEvent {
            key,
            state: ButtonState::Pressed,
            repeat: false,
            text: None,
            modifiers,
            scancode: 0,
        };
        let next = key(Key::Tab, Modifiers::CONTROL);
        let previous = key(Key::Tab, Modifiers::CONTROL | Modifiers::SHIFT);
        assert!(tabs.handles_shortcut(&next));
        assert!(tabs.handles_shortcut(&key(Key::PageUp, Modifiers::CONTROL)));
        assert!(!tabs.handles_shortcut(&key(Key::Tab, Modifiers::NONE)));
        assert!(!tabs.handles_shortcut(&key(Key::Right, Modifiers::NONE)));
        let rect = Rect::new(0.0, 0.0, 300.0, 36.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        let presses = [
            previous,
            next.clone(),
            key(Key::PageDown, Modifiers::CONTROL),
            key(Key::Left, Modifiers::NONE),
            key(Key::End, Modifiers::NONE),
        ];
        for press in presses {
            tabs.event(&mut cx, &UiEvent::Key(press), rect);
        }
        assert_eq!(messages, [2, 0, 1, 0, 2], "Ctrl+Tab wraps around");
        assert!(!Tabs::new(Vec::new(), |i: usize| i).handles_shortcut(&next));
    }
}
