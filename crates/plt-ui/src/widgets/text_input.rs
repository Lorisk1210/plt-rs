//! Single-line text fields.

use std::fmt;
use std::rc::Rc;

use plt_core::Point;
use plt_platform::{ButtonState, Key, KeyEvent, MouseButton};

use super::frame::paint_frame;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::{TextLayout, TextStyle};
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// Makes the message sent with the field's text.
type TextMessage<M> = Rc<dyn Fn(String) -> M>;

/// A field holding one line of editable text: click to focus and place
/// the caret, drag or shift with the arrows to select, type to replace
/// the selection; Home/End and Ctrl+A work as usual. It sends a message
/// whenever the text changes, and another when Enter is pressed.
pub struct TextInput<M> {
    text: String,
    /// The caret, as a byte offset at a character boundary.
    caret: usize,
    /// The other end of the selection, when there is one.
    anchor: Option<usize>,
    changed: TextMessage<M>,
    submitted: Option<TextMessage<M>>,
    /// Horizontal scroll keeping the caret visible, in pixels.
    scroll: f64,
    /// The text laid out by the last measure, with the scale factor.
    layout: Option<(TextLayout, TextStyle, f64)>,
    /// The padding inside the border by the last measure, in pixels.
    padding: f64,
    /// Whether the left button went down here and is held.
    selecting: bool,
}

impl<M> fmt::Debug for TextInput<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextInput")
            .field("text", &self.text)
            .field("caret", &self.caret)
            .field("anchor", &self.anchor)
            .finish_non_exhaustive()
    }
}

impl<M> TextInput<M> {
    /// A field showing `text`, sending `changed(text)` after each edit.
    pub fn new(text: impl Into<String>, changed: impl Fn(String) -> M + 'static) -> Self {
        let text = text.into();
        Self {
            caret: text.len(),
            text,
            anchor: None,
            changed: Rc::new(changed),
            submitted: None,
            scroll: 0.0,
            layout: None,
            padding: 0.0,
            selecting: false,
        }
    }

    /// Also sends `submitted(text)` when Enter is pressed.
    #[must_use]
    pub fn on_submit(mut self, submitted: impl Fn(String) -> M + 'static) -> Self {
        self.submitted = Some(Rc::new(submitted));
        self
    }

    /// The text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text, with the caret at its end and no selection; no
    /// message is sent.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.caret = self.text.len();
        self.anchor = None;
        self.layout = None;
    }

    /// Selects the bytes from `start` to `end` of the text (moved back to
    /// character boundaries), with the caret at `end`.
    pub fn select(&mut self, start: usize, end: usize) {
        self.anchor = Some(self.boundary_at_or_before(start));
        self.caret = self.boundary_at_or_before(end);
    }

    /// The selected byte range, if any characters are selected.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?;
        (anchor != self.caret).then(|| (anchor.min(self.caret), anchor.max(self.caret)))
    }

    fn boundary_at_or_before(&self, index: usize) -> usize {
        let mut index = index.min(self.text.len());
        while !self.text.is_char_boundary(index) {
            index -= 1;
        }
        index
    }

    /// The character boundary before `index`.
    fn previous(&self, index: usize) -> usize {
        self.text[..index]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }

    /// The character boundary after `index`.
    fn next(&self, index: usize) -> usize {
        self.text[index..]
            .chars()
            .next()
            .map_or(index, |c| index + c.len_utf8())
    }

    /// Removes the selection; returns whether there was one.
    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection() else {
            return false;
        };
        self.text.replace_range(start..end, "");
        self.caret = start;
        self.anchor = None;
        true
    }

    /// Moves the caret to `to`, extending the selection when `extend`.
    fn move_caret(&mut self, to: usize, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.caret);
        } else {
            self.anchor = None;
        }
        self.caret = to;
    }

    /// Applies a key press; returns whether the text changed.
    fn key(&mut self, event: &KeyEvent) -> KeyOutcome {
        let shift = event.modifiers.shift();
        match &event.key {
            Key::Enter => return KeyOutcome::Submitted,
            Key::Left => {
                let to = match self.selection() {
                    Some((start, _)) if !shift => start,
                    _ => self.previous(self.caret),
                };
                self.move_caret(to, shift);
            }
            Key::Right => {
                let to = match self.selection() {
                    Some((_, end)) if !shift => end,
                    _ => self.next(self.caret),
                };
                self.move_caret(to, shift);
            }
            Key::Home => self.move_caret(0, shift),
            Key::End => self.move_caret(self.text.len(), shift),
            Key::Backspace => {
                if !self.delete_selection() && self.caret > 0 {
                    let start = self.previous(self.caret);
                    self.text.replace_range(start..self.caret, "");
                    self.caret = start;
                }
                // A click leaves an empty selection behind: drop it with
                // the edit, or it would span the changed text.
                self.anchor = None;
                return KeyOutcome::Changed;
            }
            Key::Delete => {
                if !self.delete_selection() && self.caret < self.text.len() {
                    let end = self.next(self.caret);
                    self.text.replace_range(self.caret..end, "");
                }
                self.anchor = None;
                return KeyOutcome::Changed;
            }
            Key::Character('a' | 'A') if event.modifiers.control() => {
                self.anchor = Some(0);
                self.caret = self.text.len();
            }
            _ if event.modifiers.control() || event.modifiers.alt() => {
                return KeyOutcome::Ignored;
            }
            _ => {
                let Some(text) = event
                    .text
                    .as_deref()
                    .filter(|t| !t.chars().any(char::is_control))
                else {
                    return KeyOutcome::Ignored;
                };
                self.delete_selection();
                self.text.insert_str(self.caret, text);
                self.caret += text.len();
                self.anchor = None;
                return KeyOutcome::Changed;
            }
        }
        KeyOutcome::Moved
    }
}

/// What a key did to the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyOutcome {
    Ignored,
    Moved,
    Changed,
    Submitted,
}

impl<M: 'static> TextInput<M> {
    /// The width of the text before byte `index`, in pixels.
    fn x_of(&self, index: usize) -> f64 {
        self.layout.as_ref().map_or(0.0, |(_, style, scale)| {
            TextLayout::new(&self.text[..index], style, *scale)
                .map_or(0.0, |prefix| prefix.size().width)
        })
    }

    /// The character boundary closest to `x` pixels into the text.
    fn index_at(&self, x: f64) -> usize {
        let boundaries = self
            .text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(self.text.len()));
        boundaries
            .map(|i| (i, (self.x_of(i) - x).abs()))
            .reduce(|best, next| if next.1 < best.1 { next } else { best })
            .map_or(0, |(i, _)| i)
    }

    /// Scrolls so that the caret stays within `width` pixels of text.
    fn keep_caret_visible(&mut self, width: f64) {
        let x = self.x_of(self.caret);
        if x - self.scroll > width {
            self.scroll = x - width;
        } else if x < self.scroll {
            self.scroll = x;
        }
    }

    /// The text area inside the field's border and padding.
    fn text_area(rect: Rect, padding: f64) -> Rect {
        Rect::new(
            rect.x + padding,
            rect.y,
            (rect.width - 2.0 * padding).max(0.0),
            rect.height,
        )
    }
}

impl<M: 'static> Widget<M> for TextInput<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let style = theme.text_style();
        let scale = cx.scale_factor();
        let layout = TextLayout::new(&self.text, &style, scale)?;
        self.layout = Some((layout, style, scale));
        self.padding = theme.space(Spacing::Sm) * scale;
        // A field asks for the theme's field width and takes what it gets.
        let natural = Size::new(theme.field_width * scale, theme.control_height * scale);
        Ok(constraints.constrain(natural))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let focused = cx.status().focused;
        paint_frame(cx, rect, theme.corner_radius * scale, theme.surface)?;
        let Some((layout, _, _)) = &self.layout else {
            return Ok(());
        };
        let area = Self::text_area(rect, self.padding);
        let top = area.y + (area.height - layout.size().height) / 2.0;
        let left = area.x - self.scroll;
        if let (true, Some((start, end))) = (focused, self.selection()) {
            let (x0, x1) = (self.x_of(start), self.x_of(end));
            let band = Rect::new(left + x0, top, x1 - x0, layout.size().height);
            cx.canvas()
                .fill_rect(band.intersect(area), theme.selection)?;
        }
        cx.canvas()
            .clipped(area)
            .draw_text(layout, Point::new(left, top), theme.text)?;
        if focused {
            let x = (left + self.x_of(self.caret)).round();
            let caret = Rect::new(x, top, scale.max(1.0), layout.size().height);
            cx.canvas().fill_rect(caret.intersect(area), theme.text)?;
        }
        Ok(())
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        let area = Self::text_area(rect, self.padding);
        match event {
            UiEvent::PointerDown {
                button: MouseButton::Left,
                position,
                modifiers,
                click_count,
            } => {
                let index = self.index_at(position.x - area.x + self.scroll);
                if *click_count >= 2 {
                    self.anchor = Some(0);
                    self.caret = self.text.len();
                } else {
                    self.move_caret(index, modifiers.shift());
                    self.anchor.get_or_insert(index);
                    self.selecting = true;
                }
                cx.request_paint();
            }
            UiEvent::PointerMove { position, .. } if self.selecting => {
                self.caret = self.index_at(position.x - area.x + self.scroll);
                self.keep_caret_visible(area.width);
                cx.request_paint();
            }
            UiEvent::PointerUp {
                button: MouseButton::Left,
                ..
            } => self.selecting = false,
            UiEvent::Key(key) if key.state == ButtonState::Pressed => match self.key(key) {
                KeyOutcome::Ignored => {}
                KeyOutcome::Moved => {
                    self.keep_caret_visible(area.width);
                    cx.request_paint();
                }
                KeyOutcome::Changed => {
                    cx.emit((self.changed)(self.text.clone()));
                    cx.request_layout();
                }
                KeyOutcome::Submitted => {
                    if let Some(submitted) = &self.submitted {
                        cx.emit(submitted(self.text.clone()));
                    }
                }
            },
            _ => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn focused_by_keyboard(&mut self) {
        self.anchor = Some(0);
        self.caret = self.text.len();
    }
}

#[cfg(test)]
mod tests {
    use plt_platform::Modifiers;

    use super::*;

    fn press(
        input: &mut TextInput<String>,
        key: Key,
        text: Option<&str>,
        modifiers: Modifiers,
    ) -> KeyOutcome {
        input.key(&KeyEvent {
            key,
            state: ButtonState::Pressed,
            repeat: false,
            text: text.map(str::to_owned),
            modifiers,
            scancode: 0,
        })
    }

    #[test]
    fn typing_moving_and_deleting() {
        let mut input = TextInput::new("figure.png", |text| text);
        assert_eq!(input.caret, 10);
        press(&mut input, Key::Home, None, Modifiers::NONE);
        press(&mut input, Key::Character('é'), Some("é"), Modifiers::NONE);
        assert_eq!(input.text(), "éfigure.png");
        press(&mut input, Key::Right, None, Modifiers::SHIFT);
        press(&mut input, Key::Right, None, Modifiers::SHIFT);
        assert_eq!(input.selection(), Some((2, 4)));
        assert_eq!(
            press(&mut input, Key::Backspace, None, Modifiers::NONE),
            KeyOutcome::Changed
        );
        assert_eq!(input.text(), "égure.png");
        press(&mut input, Key::Left, None, Modifiers::NONE);
        press(&mut input, Key::Delete, None, Modifiers::NONE);
        assert_eq!(input.text(), "gure.png");
        press(
            &mut input,
            Key::Character('a'),
            Some("a"),
            Modifiers::CONTROL,
        );
        assert_eq!(input.selection(), Some((0, 8)));
        press(&mut input, Key::Character('x'), Some("x"), Modifiers::NONE);
        assert_eq!(input.text(), "x");
        assert_eq!(
            press(&mut input, Key::Enter, None, Modifiers::NONE),
            KeyOutcome::Submitted
        );
    }

    #[test]
    fn editing_after_a_click_keeps_every_character() {
        let mut input = TextInput::new("#1f77b4ff", |text| text);
        // What a click at the end leaves: an empty selection there.
        input.anchor = Some(input.caret);
        press(&mut input, Key::Backspace, None, Modifiers::NONE);
        press(&mut input, Key::Backspace, None, Modifiers::NONE);
        assert_eq!(input.text(), "#1f77b4");
        assert_eq!(input.selection(), None);
        input.anchor = Some(input.caret);
        press(&mut input, Key::Character('a'), Some("a"), Modifiers::NONE);
        press(&mut input, Key::Character('b'), Some("b"), Modifiers::NONE);
        assert_eq!(input.text(), "#1f77b4ab");
        input.anchor = Some(0);
        input.caret = 0;
        press(&mut input, Key::Delete, None, Modifiers::NONE);
        press(&mut input, Key::Delete, None, Modifiers::NONE);
        assert_eq!(input.text(), "f77b4ab");
    }

    #[test]
    fn selecting_a_range() {
        let mut input = TextInput::new("plot.svg", |text| text);
        input.select(0, 4);
        assert_eq!(input.selection(), Some((0, 4)));
        input.set_text("a.pdf");
        assert_eq!(input.selection(), None);
        assert_eq!(input.caret, 5);
    }
}
