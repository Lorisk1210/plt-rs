//! The text of a field being edited: the caret, the selection, moving by
//! characters and words, and undo.

use super::history::{EditKind, History, Snapshot};

/// What a character is, for moving and selecting by words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharClass {
    /// Letters, digits and underscores.
    Word,
    /// White space.
    Space,
    /// Everything else: punctuation and symbols.
    Other,
}

impl CharClass {
    fn of(c: char) -> Self {
        if c.is_alphanumeric() || c == '_' {
            Self::Word
        } else if c.is_whitespace() {
            Self::Space
        } else {
            Self::Other
        }
    }
}

/// One line of text with a caret and maybe a selection, edited the way
/// Qt and GTK fields are, with undo and redo.
#[derive(Debug, Clone, Default)]
pub(super) struct Editor {
    text: String,
    /// The caret, as a byte offset at a character boundary.
    caret: usize,
    /// The other end of the selection, when there is one.
    anchor: Option<usize>,
    history: History,
}

impl Editor {
    /// `text`, with the caret at its end.
    pub(super) fn new(text: String) -> Self {
        Self {
            caret: text.len(),
            text,
            anchor: None,
            history: History::default(),
        }
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    pub(super) fn caret(&self) -> usize {
        self.caret
    }

    /// Replaces the text, with the caret at its end, no selection and no
    /// history.
    pub(super) fn set_text(&mut self, text: String) {
        *self = Self::new(text);
    }

    /// Selects the bytes from `start` to `end` (moved back to character
    /// boundaries), with the caret at `end`.
    pub(super) fn select(&mut self, start: usize, end: usize) {
        self.anchor = Some(self.boundary_at_or_before(start));
        self.caret = self.boundary_at_or_before(end);
        self.history.break_typing();
    }

    /// Selects all the text, with the caret at its end.
    pub(super) fn select_all(&mut self) {
        self.select(0, self.text.len());
    }

    /// Selects the word (or the run of spaces or punctuation) around
    /// byte `index`.
    pub(super) fn select_word_at(&mut self, index: usize) {
        let index = self.boundary_at_or_before(index);
        let class = self.text[index..]
            .chars()
            .next()
            .or_else(|| self.text[..index].chars().next_back())
            .map(CharClass::of);
        let Some(class) = class else {
            return;
        };
        let start = self.text[..index]
            .char_indices()
            .rev()
            .take_while(|&(_, c)| CharClass::of(c) == class)
            .last()
            .map_or(index, |(i, _)| i);
        let end = self.text[index..]
            .char_indices()
            .find(|&(_, c)| CharClass::of(c) != class)
            .map_or(self.text.len(), |(i, _)| index + i);
        self.select(start, end);
    }

    /// The selected byte range, if any characters are selected.
    pub(super) fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?;
        (anchor != self.caret).then(|| (anchor.min(self.caret), anchor.max(self.caret)))
    }

    /// The selected text, if any.
    pub(super) fn selected_text(&self) -> Option<&str> {
        self.selection().map(|(start, end)| &self.text[start..end])
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

    /// The start of the word before `index`: back over spaces and
    /// punctuation, then over the word.
    fn word_start_before(&self, index: usize) -> usize {
        let mut chars = self.text[..index].char_indices().rev().peekable();
        while chars
            .next_if(|&(_, c)| CharClass::of(c) != CharClass::Word)
            .is_some()
        {}
        let mut start = chars.peek().map_or(0, |&(i, c)| i + c.len_utf8());
        while let Some((i, _)) = chars.next_if(|&(_, c)| CharClass::of(c) == CharClass::Word) {
            start = i;
        }
        start
    }

    /// The end of the word after `index`: on over spaces and punctuation,
    /// then over the word.
    fn word_end_after(&self, index: usize) -> usize {
        let mut chars = self.text[index..].char_indices().peekable();
        while chars
            .next_if(|&(_, c)| CharClass::of(c) != CharClass::Word)
            .is_some()
        {}
        while chars
            .next_if(|&(_, c)| CharClass::of(c) == CharClass::Word)
            .is_some()
        {}
        chars.peek().map_or(self.text.len(), |&(i, _)| index + i)
    }

    /// Moves the caret to `to`, extending the selection when `extend`.
    pub(super) fn move_caret(&mut self, to: usize, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.caret);
        } else {
            self.anchor = None;
        }
        self.caret = to;
        self.history.break_typing();
    }

    /// Places the caret at `index` for a click, extending the selection
    /// when `extend`; otherwise a selection starts there, empty, for a
    /// drag to extend.
    pub(super) fn click(&mut self, index: usize, extend: bool) {
        self.move_caret(index, extend);
        self.anchor.get_or_insert(index);
    }

    /// Moves the caret a character left (`forward` false) or right, by
    /// words when `by_word`, extending the selection when `extend`;
    /// without `extend`, a selection collapses to its end on that side.
    pub(super) fn step(&mut self, forward: bool, by_word: bool, extend: bool) {
        let to = match (self.selection(), forward) {
            (Some((start, _)), false) if !extend && !by_word => start,
            (Some((_, end)), true) if !extend && !by_word => end,
            (_, false) if by_word => self.word_start_before(self.caret),
            (_, true) if by_word => self.word_end_after(self.caret),
            (_, false) => self.previous(self.caret),
            (_, true) => self.next(self.caret),
        };
        self.move_caret(to, extend);
    }

    /// Moves the caret to the start (`end` false) or the end of the text.
    pub(super) fn jump(&mut self, end: bool, extend: bool) {
        let to = if end { self.text.len() } else { 0 };
        self.move_caret(to, extend);
    }

    /// Records the text before an edit of `kind`, for undo.
    fn record(&mut self, kind: EditKind) {
        let snapshot = Snapshot {
            text: self.text.clone(),
            caret: self.caret,
            anchor: self.anchor,
        };
        self.history.record(snapshot, kind);
    }

    /// Replaces the selection (or inserts at the caret) with `text`.
    pub(super) fn insert(&mut self, text: &str, kind: EditKind) {
        self.record(kind);
        let (start, end) = self.selection().unwrap_or((self.caret, self.caret));
        self.text.replace_range(start..end, text);
        self.caret = start + text.len();
        self.anchor = None;
    }

    /// Deletes the selection, or the character (or word, when `by_word`)
    /// before the caret (`forward` false) or after it; returns whether
    /// anything was deleted.
    pub(super) fn delete(&mut self, forward: bool, by_word: bool) -> bool {
        let range = self
            .selection()
            .unwrap_or_else(|| match (forward, by_word) {
                (false, false) => (self.previous(self.caret), self.caret),
                (false, true) => (self.word_start_before(self.caret), self.caret),
                (true, false) => (self.caret, self.next(self.caret)),
                (true, true) => (self.caret, self.word_end_after(self.caret)),
            });
        // A click leaves an empty selection behind: drop it with the
        // edit, or it would span the changed text.
        if range.0 == range.1 {
            self.anchor = None;
            return false;
        }
        let kind = if by_word || self.selection().is_some() {
            EditKind::Other
        } else {
            EditKind::Delete
        };
        self.record(kind);
        self.text.replace_range(range.0..range.1, "");
        self.caret = range.0;
        self.anchor = None;
        true
    }

    /// Undoes the last edit; returns whether there was one.
    pub(super) fn undo(&mut self) -> bool {
        let current = Snapshot {
            text: self.text.clone(),
            caret: self.caret,
            anchor: self.anchor,
        };
        self.history.undo(current).is_some_and(|s| self.restore(s))
    }

    /// Redoes the last edit undone; returns whether there was one.
    pub(super) fn redo(&mut self) -> bool {
        let current = Snapshot {
            text: self.text.clone(),
            caret: self.caret,
            anchor: self.anchor,
        };
        self.history.redo(current).is_some_and(|s| self.restore(s))
    }

    fn restore(&mut self, snapshot: Snapshot) -> bool {
        self.text = snapshot.text;
        self.caret = snapshot.caret;
        self.anchor = snapshot.anchor;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_by_characters_and_words() {
        let mut editor = Editor::new("x = 0.125 é".into());
        editor.step(false, true, false);
        assert_eq!(editor.caret(), 10, "before é");
        editor.step(false, true, false);
        assert_eq!(editor.caret(), 6, "before 125");
        editor.step(false, true, true);
        assert_eq!(editor.selection(), Some((4, 6)), "0. selected");
        editor.step(true, false, false);
        assert_eq!(editor.caret(), 6, "the selection collapses to its end");
        editor.step(true, true, false);
        assert_eq!(editor.caret(), 9);
        editor.step(true, true, false);
        assert_eq!(editor.caret(), 12, "after é");
        editor.step(true, true, false);
        assert_eq!(editor.caret(), 12);
        editor.jump(false, true);
        assert_eq!(editor.selected_text(), Some("x = 0.125 é"));
        editor.step(false, false, false);
        assert_eq!(editor.caret(), 0);
        editor.step(false, true, false);
        assert_eq!(editor.caret(), 0);
    }

    #[test]
    fn selecting_words() {
        let mut editor = Editor::new("plot  data.png".into());
        editor.select_word_at(2);
        assert_eq!(editor.selected_text(), Some("plot"));
        editor.select_word_at(5);
        assert_eq!(editor.selected_text(), Some("  "));
        editor.select_word_at(14);
        assert_eq!(editor.selected_text(), Some("png"), "at the end");
        editor.select_word_at(10);
        assert_eq!(editor.selected_text(), Some("."));
        let mut empty = Editor::default();
        empty.select_word_at(0);
        assert_eq!(empty.selection(), None);
    }

    #[test]
    fn deleting_characters_and_words() {
        let mut editor = Editor::new("figure 1.png".into());
        assert!(editor.delete(false, true));
        assert_eq!(editor.text(), "figure 1.");
        assert!(editor.delete(false, true));
        assert_eq!(editor.text(), "figure ");
        editor.jump(false, false);
        assert!(editor.delete(true, true));
        assert_eq!(editor.text(), " ");
        assert!(editor.delete(true, false));
        assert!(!editor.delete(true, false), "nothing after the caret");
        assert!(!editor.delete(false, true));
        let mut accented = Editor::new("aé".into());
        accented.delete(false, false);
        assert_eq!(accented.text(), "a");
    }

    #[test]
    fn editing_after_a_click_keeps_every_character() {
        let mut editor = Editor::new("#1f77b4ff".into());
        // What a click at the end leaves: an empty selection there.
        editor.anchor = Some(editor.caret);
        editor.delete(false, false);
        editor.delete(false, false);
        assert_eq!(editor.text(), "#1f77b4");
        assert_eq!(editor.selection(), None);
        editor.anchor = Some(editor.caret);
        editor.insert("a", EditKind::Insert);
        editor.insert("b", EditKind::Insert);
        assert_eq!(editor.text(), "#1f77b4ab");
        editor.anchor = Some(0);
        editor.caret = 0;
        editor.delete(true, false);
        editor.delete(true, false);
        assert_eq!(editor.text(), "f77b4ab");
    }

    #[test]
    fn undo_takes_back_typing_in_runs() {
        let mut editor = Editor::new("0.1".into());
        for c in ["2", "5"] {
            editor.insert(c, EditKind::Insert);
        }
        editor.delete(false, false);
        editor.select_all();
        editor.insert("x", EditKind::Insert);
        assert_eq!(editor.text(), "x");
        assert!(editor.undo());
        assert_eq!(editor.text(), "0.12");
        assert_eq!(editor.selection(), Some((0, 4)), "the selection comes back");
        assert!(editor.undo());
        assert_eq!(editor.text(), "0.125");
        assert!(editor.undo());
        assert_eq!(editor.text(), "0.1", "typed characters go together");
        assert!(!editor.undo());
        assert!(editor.redo());
        assert!(editor.redo());
        assert_eq!(editor.text(), "0.12");
        editor.insert("9", EditKind::Insert);
        assert!(!editor.redo(), "a new edit drops what was undone");
        editor.set_text("fresh".into());
        assert!(!editor.undo(), "replaced text starts a new history");
    }
}
