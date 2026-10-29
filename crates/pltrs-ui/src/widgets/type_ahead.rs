//! Type-ahead: reaching an option of a list or a drop-down by typing the
//! start of its label, as in Qt and GTK.

use std::time::{Duration, Instant};

use pltrs_platform::{ButtonState, KeyEvent};

/// How long after a keystroke the next one still extends the search.
const TIMEOUT: Duration = Duration::from_secs(1);

/// What was typed lately, and when.
#[derive(Debug, Default)]
pub(crate) struct TypeAhead {
    typed: String,
    last: Option<Instant>,
}

impl TypeAhead {
    /// Whether text typed now (at `now`) extends the search rather than
    /// starting a new one, so that a space is part of it.
    pub(crate) fn is_typing(&self, now: Instant) -> bool {
        !self.typed.is_empty() && self.last.is_some_and(|last| now - last < TIMEOUT)
    }

    /// Adds what the key press `key` typed at `now` and returns the option
    /// to go to among `labels`: the first from `current` on (wrapping
    /// around) whose label starts with the text typed lately, ignoring
    /// case. Typing one letter again and again goes through the options
    /// starting with it. `None` when the key typed nothing (or only a
    /// space starting a search) or nothing matches.
    pub(crate) fn search<'a>(
        &mut self,
        key: &KeyEvent,
        now: Instant,
        labels: impl IntoIterator<Item = &'a str>,
        current: Option<usize>,
    ) -> Option<usize> {
        let text = typed_text(key)?;
        if !self.is_typing(now) {
            self.typed.clear();
            if text.trim().is_empty() {
                return None;
            }
        }
        self.last = Some(now);
        self.typed.push_str(&text.to_lowercase());
        let mut letters = self.typed.chars();
        let first = letters.next()?;
        // One letter typed again and again: the next option with it.
        let (prefix, skip) = if letters.all(|c| c == first) {
            (first.to_string(), 1)
        } else {
            (self.typed.clone(), 0)
        };
        let labels: Vec<&str> = labels.into_iter().collect();
        let count = labels.len();
        let start = current.map_or(0, |index| index + skip);
        (0..count)
            .map(|offset| (start + offset) % count)
            .find(|&index| labels[index].to_lowercase().starts_with(&prefix))
    }
}

/// The text a key press typed, if it is printable and was not typed with
/// Control or Alt.
fn typed_text(key: &KeyEvent) -> Option<&str> {
    if key.state != ButtonState::Pressed || key.modifiers.control() || key.modifiers.alt() {
        return None;
    }
    key.text
        .as_deref()
        .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
}

#[cfg(test)]
mod tests {
    use pltrs_platform::{Key, Modifiers};

    use super::*;

    fn typed(text: &str) -> KeyEvent {
        KeyEvent {
            key: text.chars().next().map_or(Key::Enter, Key::Character),
            state: ButtonState::Pressed,
            repeat: false,
            text: Some(text.to_owned()),
            modifiers: Modifiers::NONE,
            scancode: 0,
        }
    }

    const LABELS: [&str; 5] = ["Linear", "log", "Logit", "Symlog", "Axes 2"];

    fn search(ahead: &mut TypeAhead, text: &str, at: Instant, current: usize) -> Option<usize> {
        ahead.search(&typed(text), at, LABELS.iter().copied(), Some(current))
    }

    #[test]
    fn typing_finds_options_by_their_start() {
        let mut ahead = TypeAhead::default();
        let start = Instant::now();
        let soon = |ms| start + Duration::from_millis(ms);
        assert_eq!(
            search(&mut ahead, "L", start, 0),
            Some(1),
            "the next with L"
        );
        assert_eq!(
            search(&mut ahead, "l", soon(100), 1),
            Some(2),
            "again: next"
        );
        assert_eq!(search(&mut ahead, "l", soon(200), 2), Some(0), "wrapping");
        let mut ahead = TypeAhead::default();
        assert_eq!(search(&mut ahead, "l", start, 3), Some(0));
        assert_eq!(search(&mut ahead, "o", soon(100), 0), Some(1), "prefix lo");
        assert_eq!(search(&mut ahead, "g", soon(200), 1), Some(1));
        assert_eq!(search(&mut ahead, "i", soon(300), 1), Some(2), "logi");
        assert_eq!(
            search(&mut ahead, "s", soon(2000), 2),
            Some(3),
            "a new search"
        );
        assert_eq!(search(&mut ahead, "x", soon(2100), 3), None, "no 'sx'");
    }

    #[test]
    fn spaces_count_only_within_a_search() {
        let mut ahead = TypeAhead::default();
        let start = Instant::now();
        assert_eq!(search(&mut ahead, " ", start, 0), None);
        assert!(!ahead.is_typing(start));
        assert_eq!(search(&mut ahead, "a", start, 0), Some(4));
        assert!(ahead.is_typing(start + Duration::from_millis(10)));
        assert_eq!(search(&mut ahead, "x", start, 4), Some(4));
        assert_eq!(search(&mut ahead, "e", start, 4), Some(4));
        assert_eq!(search(&mut ahead, "s", start, 4), Some(4));
        assert_eq!(search(&mut ahead, " ", start, 4), Some(4), "'axes '");
        let mut control = typed("a");
        control.modifiers = Modifiers::CONTROL;
        assert_eq!(
            ahead.search(&control, start, LABELS.iter().copied(), None),
            None
        );
    }
}
