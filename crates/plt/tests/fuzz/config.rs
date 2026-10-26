//! The user's configuration files: rc files read from disk (random
//! settings, invalid UTF-8, byte-order marks, line ends of every kind),
//! whose settings must apply over every figure look as threads apply
//! them at start-up, and user style libraries of random sheets (names
//! that shadow bundled sheets, odd file names, directories named like
//! sheets, unreadable files) used by name, path and `file:` URL.

use std::fs;
use std::path::{Path, PathBuf};

use plt::figure::SaveFormat;
use plt::look::FigureLook;
use plt::rc::{RcFile, RcParams};
use plt::style::{self, Style};

use crate::harness::{Corruption, Gen, fuzz};
use crate::rc::{draw_standard_figure, rc_text};

/// A directory of its own for a target, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("plt-fuzz-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch directory");
        Self(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The bytes of a configuration file: rc text, maybe after a byte-order
/// mark or with Windows line ends, maybe corrupted as bytes.
#[derive(Debug)]
struct FileBytes {
    text: String,
    bom: bool,
    crlf: bool,
    corruption: Option<Corruption>,
}

impl FileBytes {
    fn generate(g: &mut Gen) -> Self {
        let text = rc_text(g);
        let corruption = g.chance(0.2).then(|| g.corrupt(text.as_bytes()));
        Self {
            text,
            bom: g.chance(0.1),
            crlf: g.chance(0.2),
            corruption,
        }
    }

    fn bytes(&self) -> Vec<u8> {
        let mut text = if self.crlf {
            self.text.replace('\n', "\r\n")
        } else {
            self.text.clone()
        };
        if self.bom {
            text.insert(0, '\u{FEFF}');
        }
        match &self.corruption {
            Some(corruption) => corruption.apply(text.as_bytes()),
            None => text.into_bytes(),
        }
    }
}

/// Reads the rc file at `path` and checks that its settings apply over
/// every figure look, as threads start from the look with the user's rc
/// file on top (a setting that fails there would fail at start-up).
fn read_and_apply(path: &Path) {
    let Ok(file) = RcFile::read(path) else {
        return;
    };
    assert_eq!(file.path.as_deref(), Some(path));
    for look in FigureLook::ALL {
        let mut params = look.params();
        for entry in &file.entries {
            if let Err(err) = params.set(&entry.key, &entry.value) {
                panic!("{entry:?} is read but fails over the {look:?} look: {err}");
            }
        }
    }
}

#[test]
fn rc_files_on_disk() {
    let scratch = Scratch::new("rc-files");
    let path = scratch.path("pltrc");
    fuzz("rc files on disk", 1_500, FileBytes::generate, |file| {
        fs::write(&path, file.bytes()).expect("the file is written");
        read_and_apply(&path);
    });
}

/// Names of style sheets: bundled ones (which user sheets add to), the
/// special style names, and odd file names.
const SHEET_NAMES: [&str; 12] = [
    "classic",
    "ggplot",
    "plt-modern",
    "default",
    "mpl15",
    "mine",
    "a.b",
    "é ü",
    ".hidden",
    "_private",
    "file:x",
    "%41",
];

/// One file or directory of a user style library.
#[derive(Debug)]
enum LibraryItem {
    /// A sheet: its name, extension and contents.
    Sheet(String, &'static str, FileBytes),
    /// A directory named like a sheet.
    Directory(String),
}

/// A user style library: its directories and what they hold.
#[derive(Debug)]
struct Library {
    items: Vec<(usize, LibraryItem)>,
    /// The directories listed, by number (a missing one included).
    listed: Vec<usize>,
    /// The styles used afterwards, by sheet name or by number.
    used: Vec<StyleUse>,
    draw: Option<SaveFormat>,
}

/// How a style is used.
#[derive(Debug)]
enum StyleUse {
    Named(String),
    File(usize),
    Url(usize),
}

impl Library {
    fn generate(g: &mut Gen) -> Self {
        let name = |g: &mut Gen| {
            if g.chance(0.8) {
                g.pick(&SHEET_NAMES).to_owned()
            } else {
                g.soup(&["s", ".", " ", "é"], 4)
            }
        };
        let items = (0..g.len(6))
            .map(|_| {
                let directory = g.below(3);
                let item = if g.chance(0.9) {
                    let extension = g.pick(&["mplstyle", "mplstyle", "MPLSTYLE", "txt", ""]);
                    LibraryItem::Sheet(name(g), extension, FileBytes::generate(g))
                } else {
                    LibraryItem::Directory(name(g))
                };
                (directory, item)
            })
            .collect();
        let used = (0..g.len(4))
            .map(|_| match g.below(4) {
                0 => StyleUse::File(g.below(6)),
                1 => StyleUse::Url(g.below(6)),
                _ => StyleUse::Named(name(g)),
            })
            .collect();
        Self {
            items,
            listed: (0..=g.below(3)).map(|_| g.below(4)).collect(),
            used,
            draw: g
                .chance(0.2)
                .then(|| g.pick(&[SaveFormat::Png, SaveFormat::Svg])),
        }
    }

    /// Writes the library into fresh directories under `root`; returns
    /// the paths of its files.
    fn write(&self, root: &Path) -> Vec<PathBuf> {
        let _ = fs::remove_dir_all(root);
        let mut files = Vec::new();
        for directory in 0..3 {
            fs::create_dir_all(root.join(directory.to_string())).expect("a library directory");
        }
        for (directory, item) in &self.items {
            let dir = root.join(directory.to_string());
            match item {
                LibraryItem::Sheet(name, extension, bytes) => {
                    let path = dir.join(format!("{name}.{extension}"));
                    if fs::write(&path, bytes.bytes()).is_ok() {
                        files.push(path);
                    }
                }
                LibraryItem::Directory(name) => {
                    let _ = fs::create_dir_all(dir.join(format!("{name}.mplstyle")));
                }
            }
        }
        files
    }
}

/// A `file:` URL of `path`, its bytes percent-escaped where URLs need it.
fn file_url(path: &Path) -> String {
    let escaped: String = path
        .to_string_lossy()
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"/-_.".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    format!("file://{escaped}")
}

#[test]
fn style_libraries() {
    let scratch = Scratch::new("style-libraries");
    fuzz("style libraries", 300, Library::generate, |library| {
        let files = library.write(&scratch.0);
        let listed = library
            .listed
            .iter()
            .map(|directory| scratch.0.join(directory.to_string()))
            .collect();
        style::set_user_library_paths(listed);
        let _ = style::reload_library();
        for name in style::available() {
            if let Some(sheet) = style::sheet(&name) {
                let _ = sheet.apply(&mut RcParams::matplotlib());
            }
        }
        let file = |index: &usize| files.get(index % files.len().max(1)).cloned();
        let styles: Vec<Style> = library
            .used
            .iter()
            .filter_map(|used| match used {
                StyleUse::Named(name) => Some(Style::Named(name.clone())),
                StyleUse::File(index) => file(index).map(Style::File),
                StyleUse::Url(index) => file(index).map(|path| Style::Named(file_url(&path))),
            })
            .collect();
        let _ = style::context_with(&styles, || {
            if let Some(format) = library.draw {
                draw_standard_figure(format);
            }
            Ok::<_, plt::rc::RcError>(())
        });
        for path in &files {
            read_and_apply(path);
        }
        style::set_user_library_paths(Vec::new());
        let _ = style::reload_library();
    });
}
