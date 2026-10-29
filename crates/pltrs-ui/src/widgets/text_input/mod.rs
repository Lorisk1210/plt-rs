//! Single-line text fields.

mod editor;
mod history;
mod keys;

use std::fmt;
use std::rc::Rc;

use pltrs_core::Point;
use pltrs_platform::{KeyEvent, MouseButton};

use super::frame::{paint_edge, paint_frame, paint_invalid};
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::{TextLayout, TextStyle};
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};
use editor::Editor;
use history::EditKind;
use keys::Command;

/// Makes the message sent with the field's text.
type TextMessage<M> = Rc<dyn Fn(String) -> M>;

/// A field holding one line of editable text, edited as in Qt and GTK:
///
/// * click to focus and place the caret, drag to select, double-click to
///   select a word and triple-click to select everything;
/// * the arrows move by characters (with Ctrl by words), Home and End to
///   the ends, and Shift with any of them selects;
/// * Backspace and Delete delete characters (with Ctrl words), Ctrl+A
///   selects everything, and typing replaces the selection;
/// * Ctrl+C, Ctrl+X and Ctrl+V (or Ctrl+Insert, Shift+Delete and
///   Shift+Insert) copy, cut and paste through the clipboard;
/// * Ctrl+Z undoes, and Ctrl+Shift+Z or Ctrl+Y redoes.
///
/// It sends a message whenever the text changes. Enter sends another when
/// the field has [`on_submit`](TextInput::on_submit); otherwise it goes
/// on to the window's default button. A field whose text the application
/// refuses can be [marked](TextInput::set_invalid) until it is edited.
pub struct TextInput<M> {
    editor: Editor,
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
    /// Whether the application refused the text, until it is edited.
    invalid: bool,
}

impl<M> fmt::Debug for TextInput<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextInput")
            .field("editor", &self.editor)
            .finish_non_exhaustive()
    }
}

impl<M> TextInput<M> {
    /// A field showing `text`, sending `changed(text)` after each edit.
    pub fn new(text: impl Into<String>, changed: impl Fn(String) -> M + 'static) -> Self {
        Self {
            editor: Editor::new(text.into()),
            changed: Rc::new(changed),
            submitted: None,
            scroll: 0.0,
            layout: None,
            padding: 0.0,
            selecting: false,
            invalid: false,
        }
    }

    /// Marks the text as refused, outlining the field in the danger
    /// colour until the user edits it, or clears the mark.
    pub fn set_invalid(&mut self, invalid: bool) {
        self.invalid = invalid;
    }

    /// Whether the text is marked as refused.
    pub fn invalid(&self) -> bool {
        self.invalid
    }

    /// Also sends `submitted(text)` when Enter is pressed (which then
    /// does not press the window's default button).
    #[must_use]
    pub fn on_submit(mut self, submitted: impl Fn(String) -> M + 'static) -> Self {
        self.submitted = Some(Rc::new(submitted));
        self
    }

    /// The text.
    pub fn text(&self) -> &str {
        self.editor.text()
    }

    /// Replaces the text, with the caret at its end, no selection and
    /// nothing to undo; no message is sent.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.editor.set_text(text.into());
        self.layout = None;
    }

    /// Selects the bytes from `start` to `end` of the text (moved back to
    /// character boundaries), with the caret at `end`.
    pub fn select(&mut self, start: usize, end: usize) {
        self.editor.select(start, end);
    }

    /// The selected byte range, if any characters are selected.
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.editor.selection()
    }
}

/// What a key press did to the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    /// Nothing: the key goes on.
    Ignored,
    /// The caret or the selection moved (or nothing could change).
    Moved,
    /// The text changed.
    Changed,
}

impl Outcome {
    /// The outcome of an edit that may have found nothing to change.
    fn of_edit(changed: bool) -> Self {
        if changed { Self::Changed } else { Self::Moved }
    }
}

impl<M: 'static> TextInput<M> {
    /// The width of the text before byte `index`, in pixels.
    fn x_of(&self, index: usize) -> f64 {
        self.layout.as_ref().map_or(0.0, |(_, style, scale)| {
            TextLayout::new(&self.text()[..index], style, *scale)
                .map_or(0.0, |prefix| prefix.size().width)
        })
    }

    /// The character boundary closest to `x` pixels into the text, the
    /// earliest of equally close ones. The text before a boundary only
    /// grows with it, so binary searches find the boundaries either side
    /// of `x` with a few layouts, where trying every boundary would lay
    /// out the text as many times as it has characters.
    fn index_at(&self, x: f64) -> usize {
        let text = self.text();
        let boundaries: Vec<usize> = text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        // The first boundary at `width` or beyond.
        let first_at = |width: f64| boundaries.partition_point(|&i| self.x_of(i) < width);
        let after = boundaries.partition_point(|&i| self.x_of(i) <= x);
        let Some(before) = after.checked_sub(1) else {
            return boundaries[0];
        };
        let before_x = self.x_of(boundaries[before]);
        match boundaries.get(after) {
            Some(&next) if self.x_of(next) - x < x - before_x => next,
            _ => boundaries[first_at(before_x)],
        }
    }

    /// Scrolls so that the caret stays within `width` pixels of text.
    fn keep_caret_visible(&mut self, width: f64) {
        let x = self.x_of(self.editor.caret());
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

    /// Carries out what a key press asks.
    fn key(&mut self, cx: &mut EventCx<'_, M>, key: &KeyEvent) -> Outcome {
        let Some(command) = keys::command(key) else {
            return Outcome::Ignored;
        };
        let editor = &mut self.editor;
        match command {
            Command::Step {
                forward,
                by_word,
                extend,
            } => editor.step(forward, by_word, extend),
            Command::Jump { end, extend } => editor.jump(end, extend),
            Command::Delete { forward, by_word } => {
                return Outcome::of_edit(editor.delete(forward, by_word));
            }
            Command::SelectAll => editor.select_all(),
            Command::Copy => {
                if let Some(text) = editor.selected_text() {
                    cx.copy_text(text.to_owned());
                }
            }
            Command::Cut => {
                let Some(text) = editor.selected_text() else {
                    return Outcome::Moved;
                };
                cx.copy_text(text.to_owned());
                editor.delete(false, false);
                return Outcome::Changed;
            }
            Command::Paste => cx.request_paste(),
            Command::Undo => return Outcome::of_edit(editor.undo()),
            Command::Redo => return Outcome::of_edit(editor.redo()),
            Command::Submit => {
                let Some(submitted) = &self.submitted else {
                    return Outcome::Ignored;
                };
                cx.emit(submitted(self.editor.text().to_owned()));
            }
            Command::Type(text) => {
                editor.insert(&text, EditKind::Insert);
                return Outcome::Changed;
            }
        }
        Outcome::Moved
    }

    /// Reports an outcome: a repaint for a move, the new text for a
    /// change.
    fn report(&mut self, cx: &mut EventCx<'_, M>, outcome: Outcome, area: Rect) {
        match outcome {
            Outcome::Ignored => return,
            Outcome::Moved => cx.request_paint(),
            Outcome::Changed => {
                self.invalid = false;
                cx.emit((self.changed)(self.text().to_owned()));
                // The text is laid out again before the caret is placed.
                cx.request_layout();
            }
        }
        self.keep_caret_visible(area.width);
        cx.set_handled();
    }

    /// Inserts pasted `text` over the selection: a single line, with line
    /// breaks and other control characters as spaces.
    fn paste(&mut self, text: &str) -> Outcome {
        let line: String = text
            .trim_end_matches(['\r', '\n'])
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        if line.is_empty() && self.editor.selection().is_none() {
            return Outcome::Moved;
        }
        self.editor.insert(&line, EditKind::Other);
        Outcome::Changed
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
        let layout = TextLayout::new(self.text(), &style, scale)?;
        self.layout = Some((layout, style, scale));
        self.padding = theme.space(Spacing::Sm) * scale;
        // A field asks for the theme's field width and takes what it gets.
        let natural = Size::new(theme.field_width * scale, theme.control_height * scale);
        Ok(constraints.constrain(natural))
    }

    /// Scrolls the text to suit the field's new width: the caret in view,
    /// and no further than the text needs (keys may have been typed
    /// before the field was first laid out).
    fn arrange(&mut self, _cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        let width = Self::text_area(rect, self.padding).width;
        self.keep_caret_visible(width);
        let text = self
            .layout
            .as_ref()
            .map_or(0.0, |(layout, _, _)| layout.size().width);
        self.scroll = self.scroll.clamp(0.0, (text - width).max(0.0));
        Ok(())
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let focused = cx.status().focused();
        let radius = theme.corner_radius * scale;
        paint_frame(cx, rect, radius, theme.surface, focused && !self.invalid)?;
        if self.invalid {
            paint_invalid(cx, rect, radius, focused)?;
        } else if !focused {
            paint_edge(cx, rect, radius)?;
        }
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
            let x = (left + self.x_of(self.editor.caret())).round();
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
                match click_count {
                    2 => self.editor.select_word_at(index),
                    3.. => self.editor.select_all(),
                    _ => {
                        self.editor.click(index, modifiers.shift());
                        self.selecting = true;
                    }
                }
                cx.request_paint();
            }
            UiEvent::PointerMove { position, .. } if self.selecting => {
                let index = self.index_at(position.x - area.x + self.scroll);
                self.editor.move_caret(index, true);
                self.keep_caret_visible(area.width);
                cx.request_paint();
            }
            UiEvent::PointerUp {
                button: MouseButton::Left,
                ..
            } => self.selecting = false,
            UiEvent::Key(key) => {
                let outcome = self.key(cx, key);
                self.report(cx, outcome, area);
            }
            UiEvent::Paste(text) => {
                let outcome = self.paste(text);
                self.report(cx, outcome, area);
            }
            _ => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn focused_by_keyboard(&mut self) {
        self.editor.select_all();
    }
}

#[cfg(test)]
mod tests;
