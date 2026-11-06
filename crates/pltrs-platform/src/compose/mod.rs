//! Compose sequences on X11 and Wayland: dead keys and the Compose key
//! typing characters (´ then e types é, Compose o c types ©), from the
//! Compose files libxkbcommon reads, which applications on these
//! systems type with (neither X's core protocol nor Wayland's keyboard
//! does it for them).
//!
//! * [`locate`] finds the file that applies: the user's (`$XCOMPOSEFILE`,
//!   `~/.config/XCompose`, `~/.XCompose`), else the locale's, through
//!   the X locale directory's `locale.alias` and `compose.dir`.
//! * [`parser`] reads the file format (`Compose(5)`), includes and all,
//!   with the lexer in [`lexer`].
//! * [`table`] holds the sequences as a trie of keysyms.
//! * [`Composer`] follows a keyboard through them. The file is read the
//!   first time a key is pressed, once per process.
//!
//! As in Qt and other applications on libxkbcommon, a key that ends a
//! sequence types what the sequence gives, the keys before it type
//! nothing, and a key that matches no sequence ends it and types its own
//! text (´ then x types x). Modifier keys do not interrupt a sequence
//! (´, Shift+e types É); shortcuts (Control, Alt or Super held) end it.

#[cfg(test)]
mod fuzz_tests;
mod lexer;
mod locate;
mod parser;
mod table;

use std::sync::{Arc, OnceLock};

use crate::event::{ButtonState, KeyEvent};
use crate::keyboard::Modifiers;
use crate::keysym;
use locate::{Environment, SystemFiles};
use table::{ComposeTable, Step};

/// The sequences of this process's Compose file, if it has one.
fn system_table() -> Option<Arc<ComposeTable>> {
    static TABLE: OnceLock<Option<Arc<ComposeTable>>> = OnceLock::new();
    TABLE
        .get_or_init(|| {
            locate::load(&Environment::current(), &SystemFiles)
                .filter(|table| !table.is_empty())
                .map(Arc::new)
        })
        .clone()
}

/// The sequences a [`Composer`] follows.
#[derive(Debug, Clone)]
enum Source {
    /// The process's Compose file, read when first needed.
    System,
    /// These sequences, or none.
    Given(Option<Arc<ComposeTable>>),
}

/// What feeding a key press did.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Fed {
    /// The key is in no sequence: it types its own text.
    Passed,
    /// The key started or continued a sequence: it types nothing.
    Composing,
    /// The key completed a sequence, which types this.
    Composed(Option<String>),
}

/// A keyboard's progress through compose sequences.
#[derive(Debug, Clone)]
pub(crate) struct Composer {
    source: Source,
    /// The branch of the table reached, while a sequence is typed.
    node: Option<usize>,
}

impl Default for Composer {
    /// A composer on the process's Compose file.
    fn default() -> Self {
        Self {
            source: Source::System,
            node: None,
        }
    }
}

impl Composer {
    /// A composer following `table` (`None`: no sequences).
    #[cfg(test)]
    fn with_table(table: Option<ComposeTable>) -> Self {
        Self {
            source: Source::Given(table.map(Arc::new)),
            node: None,
        }
    }

    /// A composer that knows no sequence.
    #[cfg(test)]
    pub(crate) fn without_sequences() -> Self {
        Self::with_table(None)
    }

    /// A composer that knows the sequences of keysyms given, and what
    /// each types.
    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn with_sequences(sequences: &[(&[u32], &str)]) -> Self {
        let mut table = ComposeTable::default();
        for &(sequence, text) in sequences {
            let outcome = table::Outcome {
                text: Some(text.to_owned()),
                keysym: None,
            };
            table.add(sequence, outcome);
        }
        Self::with_table(Some(table))
    }

    /// The sequences followed, reading the process's file if needed.
    fn table(&mut self) -> Option<Arc<ComposeTable>> {
        let table = match &self.source {
            Source::System => system_table(),
            Source::Given(table) => return table.clone(),
        };
        self.source = Source::Given(table.clone());
        table
    }

    /// Ends the sequence being typed, if any (the keyboard's focus left).
    pub(crate) fn reset(&mut self) {
        self.node = None;
    }

    /// The event of a key producing `keysym` with `modifiers` held, as
    /// [`keysym::key_event`] makes it, with the text of the sequence it
    /// is part of.
    pub(crate) fn key_event(
        &mut self,
        keysym: u32,
        state: ButtonState,
        modifiers: Modifiers,
        repeat: bool,
        scancode: u32,
    ) -> KeyEvent {
        let mut event = keysym::key_event(keysym, state, modifiers, repeat, scancode);
        if state.is_pressed() {
            match self.feed(keysym, modifiers) {
                Fed::Passed => {}
                Fed::Composing => event.text = None,
                Fed::Composed(text) => {
                    event.text = text.filter(|text| !text.chars().any(char::is_control));
                }
            }
        }
        event
    }

    /// Feeds a key press producing `keysym`.
    fn feed(&mut self, keysym: u32, modifiers: Modifiers) -> Fed {
        if keysym::is_modifier(keysym) {
            return Fed::Passed;
        }
        if modifiers.control() || modifiers.alt() || modifiers.super_key() {
            self.reset();
            return Fed::Passed;
        }
        let Some(table) = self.table() else {
            return Fed::Passed;
        };
        let from = self.node.take().unwrap_or(ComposeTable::ROOT);
        match table.step(from, keysym) {
            Step::Nowhere => Fed::Passed,
            Step::Branch(node) => {
                self.node = Some(node);
                Fed::Composing
            }
            Step::Leaf(outcome) => Fed::Composed(outcome.typed()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::table::Outcome;
    use super::*;
    use crate::keyboard::Key;

    const DEAD_ACUTE: u32 = 0xfe51;
    const MULTI_KEY: u32 = 0xff20;
    const SHIFT_L: u32 = 0xffe1;

    fn composer() -> Composer {
        let mut table = ComposeTable::default();
        let text = |text: &str| Outcome {
            text: Some(text.to_owned()),
            keysym: None,
        };
        table.add(&[DEAD_ACUTE, 0x65], text("é"));
        table.add(&[DEAD_ACUTE, 0x45], text("É"));
        table.add(&[DEAD_ACUTE, DEAD_ACUTE], text("´"));
        table.add(&[MULTI_KEY, 0x6f, 0x63], text("©"));
        table.add(
            &[MULTI_KEY, 0x2d, 0x2d],
            Outcome {
                text: None,
                keysym: Some(0x0100_2013),
            },
        );
        table.add(&[MULTI_KEY, 0x6e, 0x6c], text("\n"));
        Composer::with_table(Some(table))
    }

    /// The text of pressing each keysym in turn, with no modifier but
    /// Shift for capitals.
    fn type_keys(composer: &mut Composer, keysyms: &[u32]) -> Vec<Option<String>> {
        keysyms
            .iter()
            .map(|&keysym| {
                let shift = keysym == SHIFT_L || (0x41..=0x5a).contains(&keysym);
                let modifiers = if shift {
                    Modifiers::SHIFT
                } else {
                    Modifiers::NONE
                };
                let event = composer.key_event(keysym, ButtonState::Pressed, modifiers, false, 0);
                event.text
            })
            .collect()
    }

    #[test]
    fn dead_keys_put_their_accent_on_the_next_letter() {
        let mut composer = composer();
        assert_eq!(
            type_keys(&mut composer, &[DEAD_ACUTE, 0x65, 0x65]),
            [None, Some("é".into()), Some("e".into())]
        );
        assert_eq!(
            type_keys(&mut composer, &[DEAD_ACUTE, SHIFT_L, 0x45]),
            [None, None, Some("É".into())],
            "Shift does not interrupt"
        );
        assert_eq!(
            type_keys(&mut composer, &[DEAD_ACUTE, DEAD_ACUTE]),
            [None, Some("´".into())]
        );
        let event =
            composer.key_event(DEAD_ACUTE, ButtonState::Pressed, Modifiers::NONE, false, 21);
        assert_eq!((event.key, event.text), (Key::Dead('´'), None));
    }

    #[test]
    fn the_compose_key_starts_longer_sequences() {
        let mut composer = composer();
        assert_eq!(
            type_keys(&mut composer, &[MULTI_KEY, 0x6f, 0x63]),
            [None, None, Some("©".into())]
        );
        assert_eq!(
            type_keys(&mut composer, &[MULTI_KEY, 0x2d, 0x2d]),
            [None, None, Some("–".into())],
            "the keysym's character"
        );
        assert_eq!(
            type_keys(&mut composer, &[MULTI_KEY, 0x6e, 0x6c]),
            [None, None, None],
            "control characters type nothing"
        );
    }

    #[test]
    fn keys_outside_sequences_type_themselves() {
        let mut composer = composer();
        assert_eq!(
            type_keys(&mut composer, &[DEAD_ACUTE, 0x78, 0x65]),
            [None, Some("x".into()), Some("e".into())],
            "x ends the sequence"
        );
        assert_eq!(
            type_keys(&mut composer, &[MULTI_KEY, 0x6f, 0x78]),
            [None, None, Some("x".into())]
        );
        type_keys(&mut composer, &[DEAD_ACUTE]);
        let ctrl = composer.key_event(0x65, ButtonState::Pressed, Modifiers::CONTROL, false, 0);
        assert_eq!(ctrl.text, None, "a shortcut");
        assert_eq!(
            type_keys(&mut composer, &[0x65]),
            [Some("e".into())],
            "ended it"
        );
        type_keys(&mut composer, &[DEAD_ACUTE]);
        composer.reset();
        assert_eq!(type_keys(&mut composer, &[0x65]), [Some("e".into())]);
        let release =
            composer.key_event(DEAD_ACUTE, ButtonState::Released, Modifiers::NONE, false, 0);
        assert_eq!(release.text, None);
        assert_eq!(
            type_keys(&mut composer, &[0x65]),
            [Some("e".into())],
            "releases are not fed"
        );
    }

    #[test]
    fn without_a_table_keys_type_as_they_are() {
        let mut composer = Composer::without_sequences();
        assert_eq!(
            type_keys(&mut composer, &[DEAD_ACUTE, 0x65]),
            [None, Some("e".into())]
        );
    }

    #[test]
    fn this_systems_file_types_accents() {
        // Only where X's locale data is installed.
        let env = Environment {
            locale: "de_DE.UTF-8".to_owned(),
            compose_file: None,
            config_home: None,
            home: None,
            ..Environment::current()
        };
        if !env.locale_dir.join("compose.dir").is_file() {
            return;
        }
        let table = locate::load(&env, &SystemFiles).expect("the system's file parses");
        let mut composer = Composer::with_table(Some(table));
        assert_eq!(
            type_keys(&mut composer, &[DEAD_ACUTE, 0x65, MULTI_KEY, 0x6f, 0x63]),
            [None, Some("é".into()), None, None, Some("©".into())]
        );
    }
}
