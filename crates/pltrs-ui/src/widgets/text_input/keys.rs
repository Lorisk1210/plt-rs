//! The keys of text fields, as in Qt and GTK on Linux; on macOS, Command
//! makes the editing shortcuts too, and Option types characters and
//! moves by words.

use pltrs_platform::{ButtonState, Key, KeyEvent};

/// What a key press asks a text field to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Command {
    /// Move the caret a character or a word (Left, Right, with Ctrl, or
    /// Option on macOS, by words), extending the selection with Shift.
    Step {
        forward: bool,
        by_word: bool,
        extend: bool,
    },
    /// Move the caret to the start (Home) or the end (End), extending the
    /// selection with Shift.
    Jump { end: bool, extend: bool },
    /// Delete the selection, or the character (or with Ctrl, or Option on
    /// macOS, the word) before the caret (Backspace) or after it (Delete).
    Delete { forward: bool, by_word: bool },
    /// Select everything (Ctrl+A).
    SelectAll,
    /// Copy the selection (Ctrl+C, Ctrl+Insert).
    Copy,
    /// Cut the selection (Ctrl+X, Shift+Delete).
    Cut,
    /// Paste the clipboard's text (Ctrl+V, Shift+Insert).
    Paste,
    /// Undo the last edit (Ctrl+Z).
    Undo,
    /// Redo the last edit undone (Ctrl+Shift+Z, Ctrl+Y).
    Redo,
    /// Submit the text (Enter).
    Submit,
    /// Type this text over the selection.
    Type(String),
}

/// What the key press `key` asks of a text field, if anything.
pub(super) fn command(key: &KeyEvent) -> Option<Command> {
    if key.state != ButtonState::Pressed {
        return None;
    }
    let (ctrl, shift, alt) = (
        key.modifiers.control(),
        key.modifiers.shift(),
        key.modifiers.alt(),
    );
    // On macOS, Command makes the editing shortcuts (⌘C, ⌘V, ...), and
    // Control still does.
    let shortcut = ctrl || (cfg!(target_os = "macos") && key.modifiers.super_key());
    // On macOS, Option alone types the layout's other characters (`@` on
    // German layouts) and moves and deletes by words; elsewhere, and with
    // other modifiers, Alt makes no command.
    let option = alt && cfg!(target_os = "macos") && !shortcut;
    if alt && !option {
        return None;
    }
    let by_word = ctrl || option;
    let command = match key.key {
        Key::Left | Key::Right => Command::Step {
            forward: key.key == Key::Right,
            by_word,
            extend: shift,
        },
        Key::Home | Key::End => Command::Jump {
            end: key.key == Key::End,
            extend: shift,
        },
        Key::Backspace => Command::Delete {
            forward: false,
            by_word,
        },
        Key::Delete if shift && !ctrl => Command::Cut,
        Key::Delete => Command::Delete {
            forward: true,
            by_word,
        },
        Key::Insert if ctrl => Command::Copy,
        Key::Insert if shift => Command::Paste,
        Key::Enter => Command::Submit,
        Key::Character(c) if shortcut => match c.to_ascii_lowercase() {
            'a' => Command::SelectAll,
            'c' => Command::Copy,
            'x' => Command::Cut,
            'v' => Command::Paste,
            'z' if shift => Command::Redo,
            'z' => Command::Undo,
            'y' => Command::Redo,
            _ => return None,
        },
        _ if shortcut => return None,
        _ => {
            let text = key
                .text
                .as_deref()
                .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))?;
            Command::Type(text.to_owned())
        }
    };
    Some(command)
}

#[cfg(test)]
mod tests {
    use pltrs_platform::Modifiers;

    use super::*;

    fn press(key: Key, text: Option<&str>, modifiers: Modifiers) -> Option<Command> {
        command(&KeyEvent {
            key,
            state: ButtonState::Pressed,
            repeat: false,
            text: text.map(str::to_owned),
            modifiers,
            scancode: 0,
        })
    }

    #[test]
    fn keys_follow_the_desktop_conventions() {
        let ctrl = Modifiers::CONTROL;
        let shift = Modifiers::SHIFT;
        assert_eq!(
            press(Key::Left, None, ctrl | shift),
            Some(Command::Step {
                forward: false,
                by_word: true,
                extend: true
            })
        );
        assert_eq!(
            press(Key::End, None, shift),
            Some(Command::Jump {
                end: true,
                extend: true
            })
        );
        assert_eq!(
            press(Key::Backspace, None, ctrl),
            Some(Command::Delete {
                forward: false,
                by_word: true
            })
        );
        assert_eq!(press(Key::Delete, None, shift), Some(Command::Cut));
        assert_eq!(press(Key::Insert, None, ctrl), Some(Command::Copy));
        assert_eq!(press(Key::Insert, None, shift), Some(Command::Paste));
        assert_eq!(press(Key::Character('V'), None, ctrl), Some(Command::Paste));
        assert_eq!(
            press(Key::Character('Z'), None, ctrl | shift),
            Some(Command::Redo)
        );
        assert_eq!(press(Key::Character('z'), None, ctrl), Some(Command::Undo));
        assert_eq!(press(Key::Character('y'), None, ctrl), Some(Command::Redo));
        assert_eq!(press(Key::Character('q'), None, ctrl), None);
        assert_eq!(
            press(Key::Character('é'), Some("é"), Modifiers::NONE),
            Some(Command::Type("é".into()))
        );
        assert_eq!(press(Key::Escape, Some("\u{1b}"), Modifiers::NONE), None);
        assert_eq!(press(Key::Up, None, Modifiers::NONE), None);
        assert_eq!(
            press(Key::Enter, None, Modifiers::NONE),
            Some(Command::Submit)
        );
    }

    #[test]
    fn option_types_and_goes_by_words_on_macos() {
        let at = press(Key::Character('l'), Some("@"), Modifiers::ALT);
        let word = press(Key::Left, None, Modifiers::ALT);
        let erase = press(Key::Backspace, None, Modifiers::ALT);
        if cfg!(target_os = "macos") {
            assert_eq!(at, Some(Command::Type("@".into())));
            assert_eq!(
                word,
                Some(Command::Step {
                    forward: false,
                    by_word: true,
                    extend: false
                })
            );
            assert_eq!(
                erase,
                Some(Command::Delete {
                    forward: false,
                    by_word: true
                })
            );
        } else {
            assert_eq!((at, word, erase), (None, None, None), "Alt does nothing");
        }
        let with_control = Modifiers::CONTROL | Modifiers::ALT;
        assert_eq!(press(Key::Character('a'), Some("a"), with_control), None);
    }

    #[test]
    fn command_edits_on_macos() {
        let paste = press(Key::Character('v'), None, Modifiers::SUPER);
        if cfg!(target_os = "macos") {
            assert_eq!(paste, Some(Command::Paste));
        } else {
            assert_eq!(paste, None, "Super+V types nothing elsewhere");
        }
    }
}
