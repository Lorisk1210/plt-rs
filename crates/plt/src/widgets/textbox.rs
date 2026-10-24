//! Text entry boxes (matplotlib's `widgets.TextBox`).

use std::cell::Cell;
use std::rc::Rc;

use plt_core::Point;
use plt_text::{HAlign, VAlign};

use super::{CallbackId, Connections, Observers, left_click};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::collections::Collection;
use crate::colors::Color;
use crate::events::{Event, EventKind};
use crate::text::Text;
use crate::transforms::Transform;

/// Look of a [`TextBox`].
#[derive(Clone, Debug, PartialEq)]
pub struct TextBoxOptions {
    /// Background when the pointer is elsewhere.
    pub color: Color,
    /// Background under the pointer.
    pub hovercolor: Color,
    /// Gap between the label and the box, in axes fractions.
    pub label_pad: f64,
    /// Where the text sits in the box.
    pub textalignment: HAlign,
}

impl Default for TextBoxOptions {
    /// matplotlib's near-white box, white when hovered, left-aligned text.
    fn default() -> Self {
        Self {
            color: Color::from(".95"),
            hovercolor: Color::from("1"),
            label_pad: 0.01,
            textalignment: HAlign::Left,
        }
    }
}

/// The parts of a text box its event handlers change.
#[derive(Debug)]
struct Entry {
    ax: Axes,
    text: Text,
    cursor: Collection,
    /// Character position of the cursor.
    cursor_index: Cell<usize>,
    typing: Cell<bool>,
    changed: Observers<String>,
    submitted: Observers<String>,
}

impl Entry {
    fn value(&self) -> String {
        self.text.text()
    }

    /// The width in pixels of the first `chars` characters of the text.
    fn prefix_width(&self, chars: usize) -> f64 {
        let full = self.value();
        let prefix: String = full.chars().take(chars).collect();
        self.text.set_text(if prefix.is_empty() {
            ",".to_owned()
        } else {
            prefix.clone()
        });
        let width = self.text.window_extent().map_or(0.0, |b| b.width());
        self.text.set_text(full);
        if prefix.is_empty() { 0.0 } else { width }
    }

    /// The character position closest to display x (matplotlib's
    /// `_char_index_at`).
    fn index_at(&self, x: f64) -> usize {
        let Some(start) = self.text.window_extent().map(|b| b.x0()) else {
            return 0;
        };
        let count = self.value().chars().count();
        (0..=count)
            .min_by(|&a, &b| {
                let distance = |i| (start + self.prefix_width(i) - x).abs();
                distance(a).total_cmp(&distance(b))
            })
            .unwrap_or(0)
    }

    /// Shows the cursor after the first `cursor_index` characters
    /// (matplotlib's `_rendercursor`).
    fn render_cursor(&self) {
        self.ax.figure().layout();
        let Some(bbox) = self.text.window_extent() else {
            return;
        };
        let x = bbox.x0() + self.prefix_width(self.cursor_index.get());
        let (bottom, top) = if bbox.height() > 0.0 {
            (bbox.y0(), bbox.y1())
        } else {
            let middle = bbox.y0();
            (middle - 6.0, middle + 6.0)
        };
        self.cursor
            .set_segments(vec![vec![Point::new(x, bottom), Point::new(x, top)]]);
        self.cursor.set_visible(true);
    }

    fn begin_typing(&self) {
        self.typing.set(true);
        self.ax.figure().set_capture_keys(true);
    }

    /// Stops editing and, if it was editing, submits the text.
    fn stop_typing(&self) {
        if self.typing.replace(false) {
            self.ax.figure().set_capture_keys(false);
            self.cursor.set_visible(false);
            self.submitted.process(&self.value());
        }
    }

    /// One key press while editing (matplotlib's `_keypress`).
    fn key(&self, key: &str) {
        let mut chars: Vec<char> = self.value().chars().collect();
        let index = self.cursor_index.get().min(chars.len());
        let mut single = key.chars();
        match (single.next(), single.next(), key) {
            (Some(c), None, _) => {
                chars.insert(index, c);
                self.cursor_index.set(index + 1);
            }
            (_, _, "right") => self.cursor_index.set((index + 1).min(chars.len())),
            (_, _, "left") => self.cursor_index.set(index.saturating_sub(1)),
            (_, _, "home") => self.cursor_index.set(0),
            (_, _, "end") => self.cursor_index.set(chars.len()),
            (_, _, "backspace") if index > 0 => {
                chars.remove(index - 1);
                self.cursor_index.set(index - 1);
            }
            (_, _, "delete") if index < chars.len() => {
                chars.remove(index);
            }
            _ => {}
        }
        self.text.set_text(chars.into_iter().collect::<String>());
        self.render_cursor();
        self.changed.process(&self.value());
        if key == "enter" {
            self.submitted.process(&self.value());
        }
    }
}

/// A one-line text entry with a label (matplotlib's `TextBox`): click to
/// edit, type, and press Enter or click elsewhere to submit.
#[derive(Debug)]
pub struct TextBox {
    entry: Rc<Entry>,
    label: Text,
    _connections: Connections,
}

impl TextBox {
    /// Turns `ax` into a text box labelled `label` holding `initial`.
    pub fn new(ax: &Axes, label: &str, initial: &str, options: TextBoxOptions) -> Self {
        let caption = ax.text(-options.label_pad, 0.5, label);
        caption.set_transform(ax.trans_axes());
        caption.set_valign(VAlign::Center).set_halign(HAlign::Right);
        let x = match options.textalignment {
            HAlign::Left => 0.05,
            HAlign::Center => 0.5,
            HAlign::Right => 0.95,
        };
        let text = ax.text(x, 0.5, initial);
        text.set_transform(ax.trans_axes());
        text.set_valign(VAlign::Center)
            .set_halign(options.textalignment)
            .set_parse_math(false);
        ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
        ax.set_navigate(false);
        ax.set_facecolor(options.color.clone());
        ax.set_xticks(Vec::<f64>::new());
        ax.set_yticks(Vec::<f64>::new());
        let cursor = ax
            .vlines_in([0.0], [0.0], [0.0], Transform::Display)
            .expect("one segment from one position and one span");
        cursor
            .set_edgecolor("k")
            .set_linewidths(vec![1.0])
            .set_visible(false);
        let entry = Rc::new(Entry {
            ax: ax.clone(),
            text,
            cursor,
            cursor_index: Cell::new(0),
            typing: Cell::new(false),
            changed: Observers::default(),
            submitted: Observers::default(),
        });
        let mut connections = Connections::new(ax.figure());
        {
            let entry = Rc::clone(&entry);
            connections.connect(EventKind::ButtonPress, move |event| {
                let Some(at) = left_click(event) else {
                    return;
                };
                if entry.ax.contains_point(at) {
                    entry.begin_typing();
                    entry.cursor_index.set(entry.index_at(at.x));
                    entry.render_cursor();
                } else {
                    entry.stop_typing();
                }
            });
        }
        {
            let entry = Rc::clone(&entry);
            connections.connect(EventKind::KeyPress, move |event| {
                if let Event::KeyPress(key) = event
                    && entry.typing.get()
                {
                    entry.key(&key.key);
                }
            });
        }
        {
            let entry = Rc::clone(&entry);
            connections.connect(EventKind::Motion, move |event| {
                if let Event::Motion(mouse) = event {
                    let at = Point::new(mouse.location.x, mouse.location.y);
                    let color = if entry.ax.contains_point(at) {
                        &options.hovercolor
                    } else {
                        &options.color
                    };
                    if entry.ax.facecolor() != *color {
                        entry.ax.set_facecolor(color.clone());
                    }
                }
            });
        }
        Self {
            entry,
            label: caption,
            _connections: connections,
        }
    }

    /// The text entered.
    pub fn text(&self) -> String {
        self.entry.value()
    }

    /// Replaces the text, shows the cursor where it stands in the new text
    /// and runs the change and submit callbacks (matplotlib's `set_val`);
    /// does nothing when the text is already `value`.
    pub fn set_val(&self, value: &str) {
        if self.entry.value() == value {
            return;
        }
        self.entry.text.set_text(value);
        self.entry.render_cursor();
        self.entry.changed.process(&value.to_owned());
        self.entry.submitted.process(&value.to_owned());
    }

    /// Starts editing: key presses go to the box (matplotlib's
    /// `begin_typing`).
    pub fn begin_typing(&self) {
        self.entry.begin_typing();
    }

    /// Stops editing; when it was editing, hides the cursor and runs the
    /// submit callbacks (matplotlib's `stop_typing`).
    pub fn stop_typing(&self) {
        self.entry.stop_typing();
    }

    /// The label.
    pub fn label(&self) -> &Text {
        &self.label
    }

    /// Runs `callback` with the text after every edit (matplotlib's
    /// `on_text_change`); returns its id for [`TextBox::disconnect`].
    pub fn on_text_change(&self, mut callback: impl FnMut(&str) + 'static) -> CallbackId {
        self.entry
            .changed
            .connect(move |text: &String| callback(text))
    }

    /// Runs `callback` with the text when it is submitted (matplotlib's
    /// `on_submit`); returns its id for [`TextBox::disconnect`].
    pub fn on_submit(&self, mut callback: impl FnMut(&str) + 'static) -> CallbackId {
        self.entry
            .submitted
            .connect(move |text: &String| callback(text))
    }

    /// Disconnects the callback with id `id`, whichever signal it was
    /// connected to (matplotlib's `disconnect`); returns whether it was
    /// connected.
    pub fn disconnect(&self, id: CallbackId) -> bool {
        self.entry.changed.disconnect(id) || self.entry.submitted.disconnect(id)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::events::EventSource;
    use crate::figure::Figure;
    use crate::navigation::PointerButton;

    #[test]
    fn typing_edits_and_submits() {
        let fig = Figure::with_size(6.0, 3.0, 100.0);
        let ax = fig.add_axes([0.78, 0.45, 0.18, 0.1]);
        let entry = TextBox::new(&ax, "Name ", "plt-rs", TextBoxOptions::default());
        let submitted = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&submitted);
        entry.on_submit(move |text| log.borrow_mut().push(text.to_owned()));
        let mut source = EventSource::new();
        // Click at the far right of the box: the cursor goes after the text.
        source.button_press(&fig, 0.95 * 600.0, 0.5 * 300.0, PointerButton::Left, false);
        assert!(fig.captures_keys());
        for key in ["!", "left", "backspace", "enter"] {
            source.key_press(&fig, key, 0.0, 0.0);
        }
        assert_eq!(entry.text(), "plt-r!");
        source.button_press(&fig, 1.0, 1.0, PointerButton::Left, false);
        assert!(!fig.captures_keys());
        assert_eq!(*submitted.borrow(), ["plt-r!", "plt-r!"]);
    }

    #[test]
    fn disconnect_finds_the_callback_whichever_signal_it_listens_to() {
        let fig = Figure::with_size(6.0, 3.0, 100.0);
        let ax = fig.add_axes([0.78, 0.45, 0.18, 0.1]);
        let entry = TextBox::new(&ax, "Name ", "", TextBoxOptions::default());
        let log = Rc::new(RefCell::new(Vec::new()));
        let (changes, submits) = (Rc::clone(&log), Rc::clone(&log));
        let change =
            entry.on_text_change(move |text| changes.borrow_mut().push(format!("c {text}")));
        entry.on_submit(move |text| submits.borrow_mut().push(format!("s {text}")));
        assert!(entry.disconnect(change));
        entry.set_val("x");
        assert_eq!(*log.borrow(), ["s x"]);
    }

    #[test]
    fn setting_a_new_value_shows_the_cursor_before_the_callbacks() {
        let fig = Figure::with_size(6.0, 3.0, 100.0);
        let entry = TextBox::new(
            &fig.add_axes([0.1, 0.05, 0.8, 0.075]),
            "Evaluate",
            "",
            TextBoxOptions::default(),
        );
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&seen);
        let cursor = entry.entry.cursor.clone();
        entry.on_submit(move |text| log.borrow_mut().push((text.to_owned(), cursor.visible())));
        entry.set_val("t ** 2");
        entry.set_val("t ** 2");
        assert_eq!(*seen.borrow(), [("t ** 2".to_owned(), true)]);
        let text = entry.entry.text.window_extent().expect("laid out");
        let x = entry.entry.cursor.paths()[0].vertices()[0].x;
        assert!(
            (x - text.x0()).abs() < 1e-9,
            "the cursor stays at the start"
        );
    }

    #[test]
    fn stopping_to_type_submits_once() {
        let fig = Figure::new();
        let tool = TextBox::new(&fig.subplot(), "", "x", TextBoxOptions::default());
        let submits = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&submits);
        tool.on_submit(move |text| seen.borrow_mut().push(text.to_owned()));
        tool.stop_typing();
        assert!(submits.borrow().is_empty(), "not typing: nothing to submit");
        tool.begin_typing();
        tool.stop_typing();
        tool.stop_typing();
        assert_eq!(*submits.borrow(), ["x"]);
    }
}
