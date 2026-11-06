//! Compose files (`Compose(5)`), parsed as libxkbcommon parses them:
//!
//! ```text
//! include "%L"
//! <dead_acute> <e>        : "é"   eacute   # a dead key, then e
//! <Multi_key> <o> <c>     : "©"   copyright
//! ```
//!
//! A production is a sequence of up to ten keysyms, each with optional
//! modifiers (parsed, then ignored, as libxkbcommon ignores them), a
//! colon, and a string, a keysym or both. Lines with a problem are
//! skipped, up to ten errors per file (an unknown keysym or modifier,
//! an unexpected token); more, or an include that cannot be read, make
//! the whole file fail. Includes nest five deep at most.

use std::path::{Path, PathBuf};

use super::lexer::{Expansions, Lexer, Token};
use super::table::{ComposeTable, Outcome};
use crate::keysym::{self, NO_SYMBOL};

/// The most keysyms of a sequence.
const MAX_SEQUENCE: usize = 10;
/// The longest string a production types, in bytes.
const MAX_TEXT: usize = 255;
/// The errors a file may have before it fails.
const MAX_ERRORS: usize = 10;
/// How deep includes nest.
const MAX_INCLUDE_DEPTH: usize = 5;
/// The modifiers sequences may name.
const MODIFIERS: [&str; 6] = ["Shift", "Ctrl", "Alt", "Meta", "Lock", "Caps"];

/// Why a Compose file was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refused {
    /// More than ten lines had errors.
    TooManyErrors,
    /// An included file could not be read.
    Unreadable(PathBuf),
    /// Includes nested more than five deep (an include loop?).
    TooDeep,
}

/// Where Compose files come from: the file system, or test data.
pub(crate) trait Files {
    /// The bytes of the file at `path`, if it can be read.
    fn read(&self, path: &Path) -> Option<Vec<u8>>;
}

/// What the file being parsed may include, and how deep it is.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Includes<'a> {
    /// What include paths expand.
    pub(crate) expansions: Expansions<'a>,
    /// Where included files are read.
    pub(crate) files: &'a dyn Files,
}

impl std::fmt::Debug for dyn Files + '_ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Files")
    }
}

/// The keysym named `name`, as libxkbcommon reads names (`NoSymbol`
/// names none).
fn named_keysym(name: &str) -> Option<u32> {
    keysym::from_name(name).filter(|&keysym| keysym != NO_SYMBOL)
}

/// How a line ended.
enum Line {
    /// It was done with: the next starts.
    Done,
    /// It had an error (counted), or something to skip: the rest of the
    /// line goes.
    Skip { error: bool },
}

/// What the left-hand side of a production takes next.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// A keysym, modifiers, or the colon after a keysym.
    Event,
    /// More modifiers or the keysym they go with (after a modifier or
    /// `!`).
    Modifiers,
    /// The modifier a `~` negates.
    Negated,
    /// The keysym `None` goes with.
    Keysym,
}

/// Parses the Compose file `bytes` into `table`, `depth` includes deep.
pub(crate) fn parse(
    bytes: &[u8],
    includes: Includes<'_>,
    depth: usize,
    table: &mut ComposeTable,
) -> Result<(), Refused> {
    let mut lexer = Lexer::new(bytes);
    let mut errors = 0;
    loop {
        let token = lexer.next_token();
        let line = match token {
            Token::EndOfLine => continue,
            Token::EndOfFile => return Ok(()),
            Token::Include => match lexer.include_path(includes.expansions) {
                Some(path) if lexer.next_token() == Token::EndOfLine => {
                    include(Path::new(&path), includes, depth, table)?;
                    Line::Done
                }
                _ => Line::Skip { error: true },
            },
            token => production(&mut lexer, token, table),
        };
        if let Line::Skip { error } = line {
            if error {
                errors += 1;
                if errors > MAX_ERRORS {
                    return Err(Refused::TooManyErrors);
                }
            }
            lexer.skip_line();
        }
    }
}

/// Parses the included file at `path`.
fn include(
    path: &Path,
    includes: Includes<'_>,
    depth: usize,
    table: &mut ComposeTable,
) -> Result<(), Refused> {
    if depth >= MAX_INCLUDE_DEPTH {
        return Err(Refused::TooDeep);
    }
    let bytes = includes
        .files
        .read(path)
        .ok_or_else(|| Refused::Unreadable(path.to_owned()))?;
    parse(&bytes, includes, depth + 1, table)
}

/// Parses a production starting with `token` and adds it to `table`.
fn production(lexer: &mut Lexer<'_>, mut token: Token, table: &mut ComposeTable) -> Line {
    let mut sequence = Vec::new();
    let mut expect = Expect::Event;
    loop {
        let is_modifier = |name: &str| MODIFIERS.contains(&name);
        expect = match (expect, token) {
            (Expect::Event, Token::Colon) if sequence.is_empty() => {
                return Line::Skip { error: false };
            }
            (Expect::Event, Token::Colon) => break,
            (Expect::Event, Token::Ident(name)) if name == "None" => Expect::Keysym,
            (Expect::Event, Token::Bang) => Expect::Modifiers,
            (Expect::Event | Expect::Modifiers, Token::Tilde) => Expect::Negated,
            (Expect::Event | Expect::Modifiers | Expect::Negated, Token::Ident(name))
                if is_modifier(&name) =>
            {
                Expect::Modifiers
            }
            (Expect::Event | Expect::Modifiers | Expect::Keysym, Token::Keysym(name)) => {
                let Some(keysym) = named_keysym(&name) else {
                    return Line::Skip { error: true };
                };
                if sequence.len() == MAX_SEQUENCE {
                    return Line::Skip { error: false };
                }
                sequence.push(keysym);
                Expect::Event
            }
            _ => return Line::Skip { error: true },
        };
        token = lexer.next_token();
    }
    outcome(lexer, &sequence, table)
}

/// Parses what the sequence `sequence` types, after its colon, and adds
/// the production.
fn outcome(lexer: &mut Lexer<'_>, sequence: &[u32], table: &mut ComposeTable) -> Line {
    let mut outcome = Outcome {
        text: None,
        keysym: None,
    };
    loop {
        match lexer.next_token() {
            Token::Text(text) => {
                if outcome.text.is_some() || text.is_empty() || text.len() > MAX_TEXT {
                    return Line::Skip { error: false };
                }
                outcome.text = Some(text);
            }
            Token::Ident(name) => {
                let Some(keysym) = named_keysym(&name) else {
                    return Line::Skip { error: true };
                };
                // libxkbcommon ends the production at its keysym: what
                // follows on the line starts another.
                outcome.keysym = Some(keysym);
                table.add(sequence, outcome);
                return Line::Done;
            }
            Token::EndOfLine | Token::EndOfFile if outcome.text.is_some() => {
                table.add(sequence, outcome);
                return Line::Done;
            }
            Token::EndOfLine | Token::EndOfFile => return Line::Skip { error: false },
            _ => return Line::Skip { error: true },
        }
    }
}

/// Files in memory, for the tests.
#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub(crate) struct MemoryFiles(std::collections::HashMap<PathBuf, Vec<u8>>);

#[cfg(test)]
impl MemoryFiles {
    /// The files `(path, text)`.
    pub(crate) fn new(files: &[(&str, &str)]) -> Self {
        let mut memory = Self::default();
        for (path, text) in files {
            memory.insert(path, text);
        }
        memory
    }

    /// Adds (or replaces) the file at `path`.
    pub(crate) fn insert(&mut self, path: &str, text: &str) {
        self.0.insert(PathBuf::from(path), text.as_bytes().to_vec());
    }
}

#[cfg(test)]
impl Files for MemoryFiles {
    fn read(&self, path: &Path) -> Option<Vec<u8>> {
        self.0.get(path).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::table::Step;

    type Memory = MemoryFiles;

    fn memory(files: &[(&str, &str)]) -> Memory {
        MemoryFiles::new(files)
    }

    fn parse_with(text: &str, files: &Memory) -> Result<ComposeTable, Refused> {
        let includes = Includes {
            expansions: Expansions {
                home: Some(Path::new("/home/u")),
                locale_file: Some(Path::new("/locale/en_US.UTF-8/Compose")),
                locale_dir: Path::new("/locale"),
            },
            files,
        };
        let mut table = ComposeTable::default();
        parse(text.as_bytes(), includes, 0, &mut table).map(|()| table)
    }

    fn parsed(text: &str) -> ComposeTable {
        parse_with(text, &memory(&[])).expect("the file parses")
    }

    /// What `names` type in `table`.
    fn typed(table: &ComposeTable, names: &[&str]) -> Option<String> {
        let mut node = ComposeTable::ROOT;
        for (index, name) in names.iter().enumerate() {
            match table.step(node, named_keysym(name)?) {
                Step::Branch(next) if index + 1 < names.len() => node = next,
                Step::Leaf(outcome) if index + 1 == names.len() => return outcome.typed(),
                _ => return None,
            }
        }
        None
    }

    #[test]
    fn productions_type_strings_and_keysyms() {
        let table = parsed(
            "<dead_acute> <e> : \"é\" eacute # LATIN SMALL LETTER E WITH ACUTE\n\
             <Multi_key> <o> <c> : copyright\n\
             <dead_acute> <space> : \"'\"\n\
             None <Multi_key> !Ctrl ~Shift <U2203> Alt <a> : \"x\"\n",
        );
        assert_eq!(typed(&table, &["dead_acute", "e"]).as_deref(), Some("é"));
        assert_eq!(
            typed(&table, &["Multi_key", "o", "c"]).as_deref(),
            Some("©")
        );
        assert_eq!(
            typed(&table, &["dead_acute", "space"]).as_deref(),
            Some("'")
        );
        assert_eq!(
            typed(&table, &["Multi_key", "U2203", "a"]).as_deref(),
            Some("x"),
            "modifiers are ignored"
        );
    }

    #[test]
    fn faulty_lines_are_skipped() {
        let table = parsed(
            "<dead_acute> <e> : \"é\" no_such_keysym\n\
             <no_such_keysym> <e> : \"x\"\n\
             : \"x\"\n\
             <a> <b> : \n\
             <a> <c> : \"x\" \"y\"\n\
             <a> <d> : \"\"\n\
             <a> <e> : bogus\n\
             Hyper <a> <f> : \"x\"\n\
             <a> <h> Ctrl : \"x\"\n\
             ~<a> <i> : \"x\"\n\
             <a> <b> <c> <d> <e> <f> <g> <h> <i> <j> <k> : \"x\"\n\
             <a> <g> : \"g\" eacute \"tail\"\n\
             <dead_grave> <e> : \"è\"\n",
        );
        assert_eq!(typed(&table, &["dead_acute", "e"]), None);
        assert_eq!(typed(&table, &["a", "b"]), None);
        assert_eq!(typed(&table, &["a", "c"]), None);
        assert_eq!(typed(&table, &["a", "f"]), None);
        assert_eq!(typed(&table, &["a", "h"]), None, "modifiers before keysyms");
        assert_eq!(typed(&table, &["a", "i"]), None);
        assert_eq!(
            typed(&table, &["a", "g"]).as_deref(),
            Some("g"),
            "the keysym ends the production"
        );
        assert_eq!(typed(&table, &["dead_grave", "e"]).as_deref(), Some("è"));
    }

    #[test]
    fn too_many_errors_refuse_the_file() {
        let ten = "<bogus> : \"x\"\n".repeat(MAX_ERRORS);
        assert!(parse_with(&ten, &memory(&[])).is_ok());
        let eleven = format!("{ten}<bogus> : \"x\"\n");
        assert_eq!(
            parse_with(&eleven, &memory(&[])),
            Err(Refused::TooManyErrors)
        );
    }

    #[test]
    fn includes_are_read_and_expanded() {
        let files = memory(&[
            ("/locale/en_US.UTF-8/Compose", "<dead_acute> <e> : \"é\"\n"),
            ("/home/u/more", "include \"%S/extra\"\n"),
            ("/locale/extra", "<dead_grave> <e> : \"è\"\n"),
        ]);
        let table = parse_with(
            "include \"%L\"\n<dead_acute> <e> : \"E\"\ninclude \"%H/more\"\n",
            &files,
        )
        .unwrap();
        assert_eq!(
            typed(&table, &["dead_acute", "e"]).as_deref(),
            Some("E"),
            "later lines win"
        );
        assert_eq!(typed(&table, &["dead_grave", "e"]).as_deref(), Some("è"));
        assert_eq!(
            parse_with("include \"/missing\"\n", &files),
            Err(Refused::Unreadable("/missing".into()))
        );
        let looping = memory(&[("/loop", "include \"/loop\"\n")]);
        assert_eq!(
            parse_with("include \"/loop\"\n", &looping),
            Err(Refused::TooDeep)
        );
        let table = parse_with("include \"%Q\"\ninclude <a>\n<a> : \"a\"\n", &files).unwrap();
        assert_eq!(
            typed(&table, &["a"]).as_deref(),
            Some("a"),
            "bad includes are errors"
        );
    }
}
