//! Which Compose file applies, found as libxkbcommon finds it
//! (`xkb_compose_table_new_from_locale`), the first that can be read of:
//!
//! 1. `$XCOMPOSEFILE`;
//! 2. `$XDG_CONFIG_HOME/XCompose` (`~/.config/XCompose`);
//! 3. `~/.XCompose`;
//! 4. the locale's file. The locale (`$LC_ALL`, `$LC_CTYPE` or `$LANG`,
//!    else `C`) is resolved through the X locale directory's
//!    `locale.alias` (`de_DE.utf8` is `de_DE.UTF-8`), then its file
//!    looked up in `compose.dir` (`en_US.UTF-8/Compose` for
//!    `de_DE.UTF-8`); `C` stands for `en_US.UTF-8` there.
//!
//! The file found is parsed with its includes; if that fails, there are
//! no sequences.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::lexer::Expansions;
use super::parser::{self, Files, Includes};
use super::table::ComposeTable;

/// The largest Compose file read (the system's are about 300 KiB).
const MAX_FILE_LEN: u64 = 8 << 20;

/// Where X keeps its locale data, by system: Linux and the BSDs'
/// packages, `XQuartz` on macOS.
const LOCALE_DIRS: [&str; 3] = [
    "/usr/share/X11/locale",
    "/usr/local/share/X11/locale",
    "/opt/X11/share/X11/locale",
];

/// What the lookup depends on: the environment, and the X locale
/// directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Environment {
    /// `$XCOMPOSEFILE`.
    pub(crate) compose_file: Option<PathBuf>,
    /// `$XDG_CONFIG_HOME`, if absolute.
    pub(crate) config_home: Option<PathBuf>,
    /// `$HOME`.
    pub(crate) home: Option<PathBuf>,
    /// The X locale directory: `$XLOCALEDIR`, else the system's.
    pub(crate) locale_dir: PathBuf,
    /// The locale: `$LC_ALL`, `$LC_CTYPE` or `$LANG`, else `C`.
    pub(crate) locale: String,
}

impl Environment {
    /// The process's environment.
    pub(crate) fn current() -> Self {
        let set = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
        let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
            .into_iter()
            .find_map(set)
            .map_or_else(
                || "C".to_owned(),
                |value| value.to_string_lossy().into_owned(),
            );
        let locale_dir = set("XLOCALEDIR").map_or_else(
            || {
                let dirs = LOCALE_DIRS.map(PathBuf::from);
                let found = dirs.iter().find(|dir| dir.is_dir());
                found.unwrap_or(&dirs[0]).clone()
            },
            PathBuf::from,
        );
        Self {
            compose_file: set("XCOMPOSEFILE").map(PathBuf::from),
            config_home: set("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute()),
            home: set("HOME").map(PathBuf::from),
            locale_dir,
            locale,
        }
    }

    /// The locale, resolved through `locale.alias`.
    fn resolved_locale(&self, files: &dyn Files) -> String {
        let aliases = self.locale_dir.join("locale.alias");
        lookup(files, &aliases, Direction::LeftToRight, &self.locale)
            .unwrap_or_else(|| self.locale.clone())
    }

    /// The Compose file of the locale `resolved`, from `compose.dir`.
    fn locale_file(&self, files: &dyn Files, resolved: &str) -> Option<PathBuf> {
        // libxkbcommon reads only UTF-8 files, and `C` is mostly a
        // misconfigured UTF-8 locale; `compose.dir` would give it
        // ISO 8859-1's file.
        let locale = if resolved == "C" {
            "en_US.UTF-8"
        } else {
            resolved
        };
        let directory = self.locale_dir.join("compose.dir");
        let file = lookup(files, &directory, Direction::RightToLeft, locale)?;
        Some(self.locale_dir.join(file))
    }

    /// The user's own files, in the order they are tried.
    fn user_files(&self) -> impl Iterator<Item = PathBuf> {
        let config = self
            .config_home
            .clone()
            .or_else(|| self.home.as_ref().map(|home| home.join(".config")))
            .map(|config| config.join("XCompose"));
        let home = self.home.as_ref().map(|home| home.join(".XCompose"));
        [self.compose_file.clone(), config, home]
            .into_iter()
            .flatten()
    }
}

/// Which column of a `locale.alias` or `compose.dir` line is looked up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    /// The first gives the second (`locale.alias`).
    LeftToRight,
    /// The second gives the first (`compose.dir`).
    RightToLeft,
}

/// The other column of the first line of the file at `path` with `name`
/// in the column `direction` looks in. Lines are two words, the first
/// optionally followed by a colon; `#` starts comment lines.
fn lookup(files: &dyn Files, path: &Path, direction: Direction, name: &str) -> Option<String> {
    let bytes = files.read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    text.lines().find_map(|line| {
        let line = line.trim_start();
        if line.starts_with('#') {
            return None;
        }
        let end = line
            .find(|c: char| c.is_whitespace() || c == ':')
            .unwrap_or(line.len());
        let (left, rest) = line.split_at(end);
        let rest = rest.strip_prefix(':').unwrap_or(rest).trim_start();
        let right = rest.split_whitespace().next().unwrap_or("");
        let (key, value) = match direction {
            Direction::LeftToRight => (left, right),
            Direction::RightToLeft => (right, left),
        };
        (key == name).then(|| value.to_owned())
    })
}

/// The sequences of the Compose file that applies in `environment`, if
/// one is found and parses.
pub(crate) fn load(environment: &Environment, files: &dyn Files) -> Option<ComposeTable> {
    let resolved = environment.resolved_locale(files);
    let locale_file = environment.locale_file(files, &resolved);
    let (bytes, _) = environment
        .user_files()
        .chain(locale_file.clone())
        .find_map(|path| files.read(&path).map(|bytes| (bytes, path)))?;
    let includes = Includes {
        expansions: Expansions {
            home: environment.home.as_deref(),
            locale_file: locale_file.as_deref(),
            locale_dir: &environment.locale_dir,
        },
        files,
    };
    let mut table = ComposeTable::default();
    parser::parse(&bytes, includes, 0, &mut table).ok()?;
    Some(table)
}

/// The file system, read with a limit on file sizes.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemFiles;

impl Files for SystemFiles {
    fn read(&self, path: &Path) -> Option<Vec<u8>> {
        let mut bytes = Vec::new();
        File::open(path)
            .ok()?
            .take(MAX_FILE_LEN + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        (bytes.len() as u64 <= MAX_FILE_LEN).then_some(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::parser::MemoryFiles;
    use crate::compose::table::Step;

    const ALIASES: &str = "# locale.alias\n\
        POSIX\t\t\tC\n\
        de_DE.utf8:\t\tde_DE.UTF-8\n\
        de_DE\t\t\tde_DE.ISO8859-1\n";
    const DIRECTORY: &str = "# compose.dir\n\
        iso8859-1/Compose\t\tde_DE.ISO8859-1\n\
        en_US.UTF-8/Compose:\t\ten_US.UTF-8\n\
        en_US.UTF-8/Compose:\t\tde_DE.UTF-8\n\
        el_GR.UTF-8/Compose\t\tel_GR.UTF-8\n";

    fn system() -> MemoryFiles {
        MemoryFiles::new(&[
            ("/x/locale.alias", ALIASES),
            ("/x/compose.dir", DIRECTORY),
            ("/x/en_US.UTF-8/Compose", "<dead_acute> <e> : \"é\"\n"),
            (
                "/x/el_GR.UTF-8/Compose",
                "include \"%S/en_US.UTF-8/Compose\"\n<dead_acute> <Greek_alpha> : \"ά\"\n",
            ),
        ])
    }

    fn environment(locale: &str) -> Environment {
        Environment {
            compose_file: None,
            config_home: None,
            home: Some(PathBuf::from("/home/u")),
            locale_dir: PathBuf::from("/x"),
            locale: locale.to_owned(),
        }
    }

    fn types(table: &ComposeTable, first: u32, second: u32) -> Option<String> {
        let Step::Branch(node) = table.step(ComposeTable::ROOT, first) else {
            return None;
        };
        match table.step(node, second) {
            Step::Leaf(outcome) => outcome.typed(),
            _ => None,
        }
    }

    const DEAD_ACUTE: u32 = 0xfe51;

    #[test]
    fn locales_find_their_file_through_the_alias_and_the_directory() {
        let files = system();
        let env = environment("de_DE.utf8");
        assert_eq!(env.resolved_locale(&files), "de_DE.UTF-8");
        assert_eq!(
            env.locale_file(&files, "de_DE.UTF-8"),
            Some(PathBuf::from("/x/en_US.UTF-8/Compose"))
        );
        assert_eq!(
            env.locale_file(&files, "C"),
            Some(PathBuf::from("/x/en_US.UTF-8/Compose")),
            "C reads UTF-8"
        );
        assert_eq!(environment("POSIX").resolved_locale(&files), "C");
        assert_eq!(env.locale_file(&files, "xx_XX"), None);
        let table = load(&env, &files).expect("a table");
        assert_eq!(types(&table, DEAD_ACUTE, 0x65).as_deref(), Some("é"));
    }

    #[test]
    fn locale_files_include_others() {
        let mut files = system();
        let table = load(&environment("el_GR.UTF-8"), &files).expect("a table");
        assert_eq!(types(&table, DEAD_ACUTE, 0x7e1).as_deref(), Some("ά"));
        assert_eq!(types(&table, DEAD_ACUTE, 0x65).as_deref(), Some("é"));
        files.insert("/x/el_GR.UTF-8/Compose", "include \"%L\"\n");
        assert_eq!(
            load(&environment("el_GR.UTF-8"), &files),
            None,
            "%L is el_GR's own file: a loop"
        );
    }

    #[test]
    fn user_files_come_first() {
        let mut system = system();
        system.insert(
            "/home/u/.XCompose",
            "include \"%L\"\n<dead_acute> <e> : \"E\"\n",
        );
        let env = environment("de_DE.UTF-8");
        let table = load(&env, &system).unwrap();
        assert_eq!(types(&table, DEAD_ACUTE, 0x65).as_deref(), Some("E"));
        system.insert("/home/u/.config/XCompose", "<dead_acute> <e> : \"3\"\n");
        let table = load(&env, &system).unwrap();
        assert_eq!(types(&table, DEAD_ACUTE, 0x65).as_deref(), Some("3"));
        system.insert("/c/XCompose", "<dead_acute> <e> : \"2\"\n");
        let config = Environment {
            config_home: Some(PathBuf::from("/c")),
            ..env.clone()
        };
        let table = load(&config, &system).unwrap();
        assert_eq!(types(&table, DEAD_ACUTE, 0x65).as_deref(), Some("2"));
        system.insert("/f", "<dead_acute> <e> : \"1\"\n");
        let chosen = Environment {
            compose_file: Some(PathBuf::from("/f")),
            ..config
        };
        let table = load(&chosen, &system).unwrap();
        assert_eq!(types(&table, DEAD_ACUTE, 0x65).as_deref(), Some("1"));
    }

    #[test]
    fn nothing_is_found_without_files_and_bad_files_give_nothing() {
        assert_eq!(load(&environment("C"), &MemoryFiles::default()), None);
        let mut system = system();
        system.insert("/x/iso8859-1/Compose", "<dead_acute> <e> : \"\\351\"\n");
        let table = load(&environment("de_DE"), &system).expect("parses");
        assert!(table.is_empty(), "ISO 8859-1 strings are no UTF-8");
        system.insert("/x/en_US.UTF-8/Compose", "include \"/nowhere\"\n");
        assert_eq!(load(&environment("C"), &system), None);
    }

    #[test]
    fn the_system_files_are_read_with_a_limit() {
        assert_eq!(SystemFiles.read(Path::new("/nonexistent/pltrs")), None);
        let env = Environment::current();
        assert!(env.locale_dir.is_absolute());
        assert_ne!(env.locale, "");
    }
}
