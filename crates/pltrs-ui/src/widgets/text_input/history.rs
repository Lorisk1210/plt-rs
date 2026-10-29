//! The undo and redo history of a text field.

/// How many edits a field can undo.
const MAX_UNDO: usize = 100;

/// The text of a field with its caret and selection, as it was before an
/// edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub(super) text: String,
    pub(super) caret: usize,
    pub(super) anchor: Option<usize>,
}

/// What an edit did, so that a run of typed (or deleted) characters
/// undoes in one step, as in Qt and GTK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditKind {
    /// Characters typed.
    Insert,
    /// Characters deleted one at a time.
    Delete,
    /// Anything else (paste, cut, deleting words or a selection): a step
    /// of its own.
    Other,
}

/// The states to go back and forth to.
#[derive(Debug, Clone, Default)]
pub(super) struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// The kind of the last edit while the caret has not moved since, to
    /// group runs of typing.
    typing: Option<EditKind>,
}

impl History {
    /// Remembers `before`, the state before an edit of `kind`, unless the
    /// edit continues a run of the same typing; forgets what was undone.
    pub(super) fn record(&mut self, before: Snapshot, kind: EditKind) {
        let continues = kind != EditKind::Other && self.typing == Some(kind);
        if !continues {
            if self.undo.len() == MAX_UNDO {
                self.undo.remove(0);
            }
            self.undo.push(before);
        }
        self.typing = (kind != EditKind::Other).then_some(kind);
        self.redo.clear();
    }

    /// Ends the current run of typing (the caret moved).
    pub(super) fn break_typing(&mut self) {
        self.typing = None;
    }

    /// The state before the last edit, if any; `current` becomes the state
    /// to redo.
    pub(super) fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let previous = self.undo.pop()?;
        self.redo.push(current);
        self.typing = None;
        Some(previous)
    }

    /// The state the last undo left, if any; `current` becomes the state
    /// to undo again.
    pub(super) fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        self.typing = None;
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(text: &str) -> Snapshot {
        Snapshot {
            text: text.to_owned(),
            caret: text.len(),
            anchor: None,
        }
    }

    #[test]
    fn the_history_keeps_its_last_hundred_steps() {
        let mut history = History::default();
        for step in 0..=MAX_UNDO {
            history.record(snapshot(&step.to_string()), EditKind::Other);
        }
        let mut oldest = None;
        while let Some(previous) = history.undo(snapshot("now")) {
            oldest = Some(previous);
        }
        assert_eq!(oldest, Some(snapshot("1")), "the first step was dropped");
    }

    #[test]
    fn runs_of_typing_are_one_step() {
        let mut history = History::default();
        history.record(snapshot(""), EditKind::Insert);
        history.record(snapshot("a"), EditKind::Insert);
        history.break_typing();
        history.record(snapshot("ab"), EditKind::Insert);
        history.record(snapshot("abc"), EditKind::Delete);
        assert_eq!(history.undo(snapshot("ab")), Some(snapshot("abc")));
        assert_eq!(history.undo(snapshot("abc")), Some(snapshot("ab")));
        assert_eq!(history.undo(snapshot("ab")), Some(snapshot("")));
        assert_eq!(history.undo(snapshot("")), None);
    }
}
