//! Randomised robustness tests of Compose files: a user's `~/.XCompose`
//! can hold anything. Files made of Compose tokens, mutated sample
//! files and corrupted bytes, including each other, are parsed; whatever
//! parses is then typed on with random keysyms. Nothing may panic, hang
//! or allocate without bound.
//!
//! The targets run on pltrs's fuzz harness ([`crate::fuzz`]), so
//! `PLTRS_FUZZ_SCALE`, `PLTRS_FUZZ_SEED` and `PLTRS_FUZZ_REPLAY` apply.

use std::path::Path;

use super::lexer::Expansions;
use super::parser::{self, Includes, MemoryFiles};
use super::{ComposeTable, Composer};
use crate::event::ButtonState;
use crate::fuzz::{Corruption, Gen, fuzz};
use crate::keyboard::Modifiers;

/// A Compose file of the kinds systems ship.
const SAMPLE: &str = "include \"%L\"\n\
# a comment\n\
<dead_acute> <e>\t\t: \"é\"\teacute # LATIN SMALL LETTER E WITH ACUTE\n\
<dead_acute> <space>\t: \"'\"\tapostrophe\n\
<Multi_key> <o> <c>\t: \"©\"\tcopyright\n\
<Multi_key> <minus> <minus> <period> : U2013\n\
!Ctrl ~Shift <Multi_key> <U2203> <a> : \"\\x41\\101\\\"\"\n\
None <dead_grave> <a> : \"à\"\n";

/// Pieces of Compose files.
const PIECES: [&str; 30] = [
    "<dead_acute>",
    "<Multi_key>",
    "<e>",
    "<U2203>",
    "<0x1008ff11>",
    "<NoSymbol>",
    "<",
    ">",
    ":",
    "\"",
    "\"é\"",
    "\\x",
    "\\777",
    "\\",
    "!",
    "~",
    "None",
    "Ctrl",
    "Shift",
    "include",
    "include \"%L\"",
    "include \"%H/a\"",
    "include \"/a\"",
    "include \"/self\"",
    "%",
    "#",
    "\n",
    " ",
    "eacute",
    "U2dd",
];

/// How the file parsed first is made.
#[derive(Debug)]
enum Made {
    /// The sample, its bytes corrupted.
    Corrupted(Corruption),
    /// The sample, mutated, or a soup of pieces.
    Text(String),
}

/// A file, what it may include, and the keysyms typed after.
#[derive(Debug)]
struct Case {
    made: Made,
    included: String,
    keysyms: Vec<u32>,
}

/// Keysyms the files name, and others.
const KEYSYMS: [u32; 9] = [0xfe51, 0xfe50, 0xff20, 0x65, 0x6f, 0x63, 0x20, 0xffe1, 0x2d];

fn generate(g: &mut Gen) -> Case {
    let made = if g.chance(0.3) {
        Made::Corrupted(g.corrupt(SAMPLE.as_bytes()))
    } else {
        Made::Text(g.text(&[SAMPLE], &PIECES))
    };
    let included = g.text(&[SAMPLE], &PIECES);
    let keysyms = (0..g.len(12))
        .map(|_| {
            if g.chance(0.8) {
                g.pick(&KEYSYMS)
            } else {
                g.below(1 << 30) as u32
            }
        })
        .collect();
    Case {
        made,
        included,
        keysyms,
    }
}

#[test]
fn compose_files() {
    fuzz("compose files", 600, generate, |case| {
        let bytes = match &case.made {
            Made::Corrupted(corruption) => corruption.apply(SAMPLE.as_bytes()),
            Made::Text(text) => text.as_bytes().to_vec(),
        };
        let mut files = MemoryFiles::new(&[
            ("/a", &case.included),
            ("/home/a", &case.included),
            ("/self", "include \"/self\"\n"),
        ]);
        files.insert("/x/en_US.UTF-8/Compose", &case.included);
        let includes = Includes {
            expansions: Expansions {
                home: Some(Path::new("/home")),
                locale_file: Some(Path::new("/x/en_US.UTF-8/Compose")),
                locale_dir: Path::new("/x"),
            },
            files: &files,
        };
        let mut table = ComposeTable::default();
        if parser::parse(&bytes, includes, 0, &mut table).is_err() {
            return;
        }
        let mut composer = Composer::with_table(Some(table));
        for &keysym in &case.keysyms {
            let _ = composer.key_event(keysym, ButtonState::Pressed, Modifiers::NONE, false, 0);
        }
    });
}
