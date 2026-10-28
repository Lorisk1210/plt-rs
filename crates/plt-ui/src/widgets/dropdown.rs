//! Drop-down lists: a button showing the chosen option, opening a list
//! of all the options.

use std::fmt;
use std::rc::Rc;
use std::time::Instant;

use plt_core::{Path, Point};
use plt_platform::{ButtonState, Key, KeyEvent, MouseButton};

use super::frame::{paint_edge, paint_frame};
use super::type_ahead::TypeAhead;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::icon::Icon;
use crate::text::TextLayout;
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// Space between the edges and the text, in the button and the rows.
const PADDING: Spacing = Spacing::Sm;
/// Space between the list's outline and its rows, and between the list
/// and the button (or the window's edges).
const INSET: Spacing = Spacing::Xs;

/// Makes the message sent for an option.
type OptionMessage<M> = Rc<dyn Fn(usize) -> M>;

/// The chevron after the chosen option.
fn chevron() -> Icon {
    Icon::new(24.0).stroke(Path::polyline(vec![
        Point::new(7.0, 10.0),
        Point::new(12.0, 15.0),
        Point::new(17.0, 10.0),
    ]))
}

/// A button showing the chosen option; pressing it opens the list of
/// options over the window, below the button (above when there is no
/// room). Clicking an option, or Enter on the one highlighted with the
/// arrow keys, chooses it and sends a message with its index; a click
/// elsewhere or Escape closes the list.
///
/// From the keyboard, as in Qt and GTK: while the drop-down has the
/// focus, Space, Enter, Alt+Down and F4 open the list, and the arrow
/// keys, Home, End and typing the start of an option choose another
/// without opening it; in the open list they highlight an option, which
/// Enter (or Space) chooses, while Escape, Alt+Up and F4 close it.
pub struct Dropdown<M> {
    options: Vec<String>,
    chosen: usize,
    on_choose: OptionMessage<M>,
    chevron: Icon,
    /// The options laid out by the last measure.
    layouts: Vec<TextLayout>,
    open: bool,
    /// The option under the pointer or picked with the arrow keys.
    hot: Option<usize>,
    /// The start of an option being typed.
    type_ahead: TypeAhead,
    /// The list's rectangle in the window, from the last arrangement.
    popup: Rect,
    /// Lengths in pixels from the last measure.
    row_height: f64,
    inset: f64,
    padding: f64,
}

impl<M> fmt::Debug for Dropdown<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dropdown")
            .field("options", &self.options)
            .field("chosen", &self.chosen)
            .field("open", &self.open)
            .finish_non_exhaustive()
    }
}

impl<M> Dropdown<M> {
    /// A drop-down of `options` with the first chosen, sending
    /// `on_choose(index)` when the user picks another.
    pub fn new(options: Vec<String>, on_choose: impl Fn(usize) -> M + 'static) -> Self {
        Self {
            options,
            chosen: 0,
            on_choose: Rc::new(on_choose),
            chevron: chevron(),
            layouts: Vec::new(),
            open: false,
            hot: None,
            type_ahead: TypeAhead::default(),
            popup: Rect::default(),
            row_height: 0.0,
            inset: 0.0,
            padding: 0.0,
        }
    }

    /// The options.
    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// The chosen option.
    pub fn chosen(&self) -> usize {
        self.chosen
    }

    /// Chooses option `index` (ignored when out of range); no message is
    /// sent.
    pub fn set_chosen(&mut self, index: usize) {
        if index < self.options.len() {
            self.chosen = index;
        }
    }

    /// Whether the list of options is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The option under `point`, when the list is open.
    fn option_at(&self, point: Point) -> Option<usize> {
        if !self.open || !self.popup.contains(point) || self.row_height <= 0.0 {
            return None;
        }
        let row = ((point.y - self.popup.y - self.inset) / self.row_height).floor();
        (row >= 0.0 && (row as usize) < self.options.len()).then_some(row as usize)
    }

    /// The rectangle of row `index` of the open list.
    fn row(&self, index: usize) -> Rect {
        Rect::new(
            self.popup.x + self.inset,
            self.popup.y + self.inset + index as f64 * self.row_height,
            self.popup.width - 2.0 * self.inset,
            self.row_height,
        )
    }

    fn set_open(&mut self, cx: &mut EventCx<'_, M>, open: bool) {
        self.open = open;
        if open {
            self.hot = Some(self.chosen);
            cx.open_popup();
        } else {
            self.hot = None;
            cx.close_popup();
        }
    }

    /// Chooses option `index`, sending a message if it is another.
    fn pick(&mut self, cx: &mut EventCx<'_, M>, index: usize) {
        if index != self.chosen {
            self.chosen = index;
            cx.emit((self.on_choose)(index));
        }
        cx.request_paint();
    }

    /// Chooses option `index` and closes the list.
    fn choose(&mut self, cx: &mut EventCx<'_, M>, index: usize) {
        self.pick(cx, index);
        self.set_open(cx, false);
    }

    /// The option a navigation key leads to from `from`: Up and Down step,
    /// Home and Page Up go to the first and End and Page Down to the last
    /// (the open list shows every option, so a page is all of them).
    fn navigated(&self, key: &KeyEvent, from: usize) -> Option<usize> {
        let last = self.options.len().checked_sub(1)?;
        if key.modifiers.control() || key.modifiers.alt() {
            return None;
        }
        match key.key {
            Key::Up => Some(from.saturating_sub(1)),
            Key::Down => Some((from + 1).min(last)),
            Key::Home | Key::PageUp => Some(0),
            Key::End | Key::PageDown => Some(last),
            _ => None,
        }
    }

    /// The option the key press `key` leads to from `from` by navigation
    /// or type-ahead.
    fn reached(&mut self, key: &KeyEvent, now: Instant, from: usize) -> Option<usize> {
        self.navigated(key, from).or_else(|| {
            let labels = self.options.iter().map(String::as_str);
            self.type_ahead.search(key, now, labels, Some(from))
        })
    }

    /// Whether `key` toggles the list: Alt+Down or Alt+Up, or F4.
    fn toggles(key: &KeyEvent) -> bool {
        let alt_arrow = key.modifiers.alt() && matches!(key.key, Key::Up | Key::Down);
        alt_arrow || key.key == Key::F(4)
    }

    /// Handles a key press while the list is closed.
    fn key_closed(&mut self, cx: &mut EventCx<'_, M>, key: &KeyEvent) {
        let plain = !key.modifiers.control() && !key.modifiers.alt();
        let opens = plain && matches!(key.key, Key::Character(' ') | Key::Enter);
        if opens || Self::toggles(key) {
            self.set_open(cx, true);
            cx.set_handled();
        } else if let Some(index) = self.reached(key, Instant::now(), self.chosen) {
            self.pick(cx, index);
            cx.set_handled();
        }
    }

    /// Handles a key press while the list is open: every key is the
    /// list's.
    fn key_open(&mut self, cx: &mut EventCx<'_, M>, key: &KeyEvent) {
        let hot = self.hot.unwrap_or(self.chosen);
        let now = Instant::now();
        let space = key.key == Key::Character(' ') && !self.type_ahead.is_typing(now);
        if key.key == Key::Escape || Self::toggles(key) {
            self.set_open(cx, false);
        } else if key.key == Key::Enter || space {
            self.choose(cx, hot);
        } else if let Some(index) = self.reached(key, now, hot) {
            self.hot = Some(index);
        }
        cx.set_handled();
        cx.request_paint();
    }

    /// The width of the widest option, in pixels.
    fn widest(&self) -> f64 {
        self.layouts
            .iter()
            .map(|layout| layout.size().width)
            .fold(0.0, f64::max)
    }
}

impl<M: 'static> Widget<M> for Dropdown<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let style = theme.text_style();
        self.layouts = self
            .options
            .iter()
            .map(|option| TextLayout::new(option, &style, scale))
            .collect::<Result<_, _>>()?;
        self.padding = theme.space(PADDING) * scale;
        self.inset = theme.space(INSET) * scale;
        self.row_height = theme.control_height * scale;
        let chevron = theme.small_icon_size * scale;
        let width = self.widest() + 3.0 * self.padding + chevron;
        Ok(constraints.constrain(Size::new(width, self.row_height)))
    }

    fn arrange(&mut self, cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        let window = cx.window_size();
        let height = self.options.len() as f64 * self.row_height + 2.0 * self.inset;
        let width = rect
            .width
            .max(self.widest() + 2.0 * (self.padding + self.inset));
        let below = rect.max_y() + self.inset;
        let above = rect.y - self.inset - height;
        let y = if below + height <= window.height - self.inset || above < self.inset {
            below
        } else {
            above
        };
        let x = rect
            .x
            .min(window.width - self.inset - width)
            .max(self.inset);
        self.popup = Rect::new(x, y, width, height).snapped();
        Ok(())
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let status = cx.status();
        let face = if self.open || status.active {
            theme.button_active
        } else if status.hot {
            theme.button_hot
        } else {
            theme.button
        };
        let radius = theme.corner_radius * scale;
        paint_frame(cx, rect, radius, face, status.focus_visible())?;
        if !status.focus_visible() {
            paint_edge(cx, rect, radius)?;
        }
        let side = theme.small_icon_size * scale;
        let icon = Rect::new(
            rect.max_x() - self.padding - side,
            rect.center().y - side / 2.0,
            side,
            side,
        );
        self.chevron
            .paint(cx.canvas(), icon, theme.text_secondary)?;
        if let Some(layout) = self.layouts.get(self.chosen) {
            let text = Rect::new(rect.x, rect.y, icon.x - rect.x, rect.height);
            let origin = Point::new(
                rect.x + self.padding,
                rect.center().y - layout.size().height / 2.0,
            );
            cx.canvas()
                .clipped(text)
                .draw_text(layout, origin, theme.text)?;
        }
        Ok(())
    }

    fn paint_popup(&self, cx: &mut PaintCx<'_>, _rect: Rect) -> Result<(), UiError> {
        if !self.open {
            return Ok(());
        }
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let radius = theme.corner_radius * scale;
        let shadow = Rect::new(
            self.popup.x,
            self.popup.y + 2.0 * scale,
            self.popup.width,
            self.popup.height,
        );
        cx.canvas()
            .fill_rounded_rect(shadow.inset(-scale), radius + scale, theme.shadow)?;
        cx.canvas()
            .fill_rounded_rect(self.popup, radius, theme.surface)?;
        cx.canvas()
            .stroke_rounded_rect(self.popup, radius, scale, theme.border)?;
        for (index, layout) in self.layouts.iter().enumerate() {
            let row = self.row(index).snapped();
            if self.hot == Some(index) {
                cx.canvas()
                    .fill_rounded_rect(row, radius - scale, theme.selection)?;
            }
            let color = if index == self.chosen {
                theme.accent_text
            } else {
                theme.text
            };
            let origin = Point::new(
                row.x + self.padding,
                row.center().y - layout.size().height / 2.0,
            );
            cx.canvas().draw_text(layout, origin, color)?;
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
                if !self.open {
                    self.set_open(cx, true);
                } else if !self.popup.contains(*position) {
                    // A click elsewhere, the button included, closes.
                    self.set_open(cx, false);
                }
            }
            UiEvent::PointerUp {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if let Some(index) = self.option_at(*position) {
                    self.choose(cx, index);
                } else if self.open && !rect.contains(*position) && !self.popup.contains(*position)
                {
                    // Pressing the button and releasing elsewhere drops
                    // the list.
                    self.set_open(cx, false);
                }
            }
            UiEvent::PointerEnter(position) | UiEvent::PointerMove { position, .. }
                if self.open =>
            {
                let hot = self.option_at(*position);
                if hot.is_some() && hot != self.hot {
                    self.hot = hot;
                    cx.request_paint();
                }
            }
            UiEvent::Key(key) if key.state == ButtonState::Pressed => {
                if self.open {
                    self.key_open(cx, key);
                } else {
                    self.key_closed(cx, key);
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
    use plt_platform::Modifiers;

    use super::*;

    fn dropdown() -> Dropdown<usize> {
        let mut dropdown = Dropdown::new(
            vec!["Axes 1".into(), "Axes 2".into(), "Axes 3".into()],
            |i| i,
        );
        dropdown.row_height = 20.0;
        dropdown.popup = Rect::new(0.0, 30.0, 100.0, 60.0);
        dropdown
    }

    fn pointer(down: bool, x: f64, y: f64) -> UiEvent {
        let position = Point::new(x, y);
        if down {
            UiEvent::PointerDown {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::NONE,
                click_count: 1,
            }
        } else {
            UiEvent::PointerUp {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::NONE,
            }
        }
    }

    #[test]
    fn clicking_opens_the_list_and_an_option_chooses() {
        let mut dropdown = dropdown();
        let button = Rect::new(0.0, 0.0, 100.0, 28.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        dropdown.event(&mut cx, &pointer(true, 10.0, 10.0), button);
        dropdown.event(&mut cx, &pointer(false, 10.0, 10.0), button);
        assert!(dropdown.is_open(), "the click on the button opens the list");
        assert_eq!(cx.popup, Some(true));
        assert_eq!(dropdown.option_at(Point::new(50.0, 75.0)), Some(2));
        dropdown.event(&mut cx, &pointer(true, 50.0, 75.0), button);
        dropdown.event(&mut cx, &pointer(false, 50.0, 75.0), button);
        assert!(!dropdown.is_open());
        assert_eq!(cx.popup, Some(false));
        assert_eq!((dropdown.chosen(), messages), (2, vec![2]));
    }

    #[test]
    fn clicks_elsewhere_and_escape_close_the_list() {
        let mut dropdown = dropdown();
        let button = Rect::new(0.0, 0.0, 100.0, 28.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        dropdown.event(&mut cx, &pointer(true, 10.0, 10.0), button);
        dropdown.event(&mut cx, &pointer(true, 300.0, 300.0), button);
        assert!(!dropdown.is_open());
        dropdown.event(&mut cx, &pointer(true, 10.0, 10.0), button);
        let key = |key| {
            UiEvent::Key(KeyEvent {
                key,
                state: ButtonState::Pressed,
                repeat: false,
                text: None,
                modifiers: Modifiers::NONE,
                scancode: 0,
            })
        };
        dropdown.event(&mut cx, &key(Key::Down), button);
        dropdown.event(&mut cx, &key(Key::Escape), button);
        assert!(!dropdown.is_open());
        assert!(cx.messages.is_empty(), "nothing was chosen");
        dropdown.event(&mut cx, &pointer(true, 10.0, 10.0), button);
        dropdown.event(&mut cx, &key(Key::Down), button);
        dropdown.event(&mut cx, &key(Key::Enter), button);
        assert_eq!(*cx.messages, [1]);
    }

    fn press(key: Key, text: Option<&str>, modifiers: Modifiers) -> UiEvent {
        UiEvent::Key(KeyEvent {
            key,
            state: ButtonState::Pressed,
            repeat: false,
            text: text.map(str::to_owned),
            modifiers,
            scancode: 0,
        })
    }

    #[test]
    fn keys_open_the_list_and_choose_without_it() {
        let mut dropdown = dropdown();
        let button = Rect::new(0.0, 0.0, 100.0, 28.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        dropdown.event(&mut cx, &press(Key::Down, None, Modifiers::NONE), button);
        dropdown.event(&mut cx, &press(Key::End, None, Modifiers::NONE), button);
        dropdown.event(
            &mut cx,
            &press(Key::Character('a'), Some("a"), Modifiers::NONE),
            button,
        );
        assert!(!dropdown.is_open(), "arrows and typing choose in place");
        assert!(cx.handled);
        for opening in [
            press(Key::Character(' '), Some(" "), Modifiers::NONE),
            press(Key::Enter, None, Modifiers::NONE),
            press(Key::Down, None, Modifiers::ALT),
            press(Key::F(4), None, Modifiers::NONE),
        ] {
            dropdown.event(&mut cx, &opening, button);
            assert!(dropdown.is_open());
            dropdown.event(&mut cx, &press(Key::Up, None, Modifiers::ALT), button);
            assert!(!dropdown.is_open());
        }
        let mut other = EventCx::new(cx.messages);
        dropdown.event(&mut other, &press(Key::Tab, None, Modifiers::NONE), button);
        assert!(!other.handled, "Tab is not the drop-down's");
        assert_eq!(messages, [1, 2, 0], "Down, End, then 'a' for Axes 1");
    }

    #[test]
    fn the_open_list_follows_the_keys() {
        let mut dropdown = dropdown();
        let button = Rect::new(0.0, 0.0, 100.0, 28.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        dropdown.event(&mut cx, &press(Key::Enter, None, Modifiers::NONE), button);
        dropdown.event(
            &mut cx,
            &press(Key::PageDown, None, Modifiers::NONE),
            button,
        );
        assert_eq!(dropdown.hot, Some(2));
        dropdown.event(&mut cx, &press(Key::Home, None, Modifiers::NONE), button);
        for c in "axes 3".chars() {
            let text = c.to_string();
            dropdown.event(
                &mut cx,
                &press(Key::Character(c), Some(&text), Modifiers::NONE),
                button,
            );
        }
        assert_eq!(dropdown.hot, Some(2), "the space is part of what was typed");
        assert!(dropdown.is_open());
        dropdown.event(&mut cx, &press(Key::Enter, None, Modifiers::NONE), button);
        assert!(!dropdown.is_open());
        dropdown.event(
            &mut cx,
            &press(Key::Character(' '), Some(" "), Modifiers::NONE),
            button,
        );
        dropdown.event(&mut cx, &press(Key::Up, None, Modifiers::NONE), button);
        dropdown.event(&mut cx, &press(Key::Escape, None, Modifiers::NONE), button);
        assert!(!dropdown.is_open(), "Escape closes without choosing");
        assert_eq!(messages, [2]);
    }
}
