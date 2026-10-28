//! Finding files in the texmf tree (matplotlib's `dviread.find_tex_file`):
//! through a long-lived `luatex --luaonly` process running matplotlib's
//! `kpsewhich.lua` when LuaTeX is installed, else by running `kpsewhich`
//! for every file.

use std::collections::HashMap;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Mutex, PoisonError};

use super::error::TexError;
use super::files::FileFinder;

/// matplotlib's `mpl-data/kpsewhich.lua`: looks up every line it reads
/// with kpathsea (making PK fonts as needed) and prints the result.
const KPSEWHICH_LUA: &str = "-- see dviread._LuatexKpsewhich\n\
kpse.set_program_name(\"latex\")\n\
kpse.init_prog(\"\", 600, \"ljfour\")\n\
while true do print(kpse.lookup(io.read():gsub(\"\\r\", \"\"), {mktexpk=true})); io.flush(); end\n";

/// What the LuaTeX helper answered.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Answer {
    /// The file's path.
    Found(String),
    /// kpathsea has no such file (`nil`).
    Absent,
    /// The helper did not answer.
    Silent,
}

/// The running `luatex --luaonly kpsewhich.lua` (`_LuatexKpsewhich`).
#[derive(Debug)]
struct LuatexKpsewhich {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl LuatexKpsewhich {
    /// Starts the helper; `None` when LuaTeX cannot be run.
    fn start(cache_dir: &Path) -> Option<Self> {
        let script = cache_dir.join("kpsewhich.lua");
        if std::fs::read_to_string(&script).ok().as_deref() != Some(KPSEWHICH_LUA) {
            std::fs::create_dir_all(cache_dir).ok()?;
            std::fs::write(&script, KPSEWHICH_LUA).ok()?;
        }
        let mut child = Command::new("luatex")
            .arg("--luaonly")
            .arg(&script)
            .env("MT_VARTEXFONTS", cache_dir.join("vartexfonts"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdin = child.stdin.take()?;
        let stdout = BufReader::new(child.stdout.take()?);
        Some(Self {
            child,
            stdin,
            stdout,
        })
    }

    /// Whether the process has exited.
    fn is_dead(&mut self) -> bool {
        !matches!(self.child.try_wait(), Ok(None))
    }

    /// What kpathsea answers for `name`.
    fn search(&mut self, name: &str) -> Answer {
        let mut line = String::new();
        let answered = writeln!(self.stdin, "{name}").is_ok()
            && self.stdin.flush().is_ok()
            && self.stdout.read_line(&mut line).is_ok_and(|read| read > 0);
        match line.trim_end() {
            _ if !answered => Answer::Silent,
            "nil" => Answer::Absent,
            found => Answer::Found(found.to_owned()),
        }
    }
}

impl Drop for LuatexKpsewhich {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// How files are looked up: the LuaTeX helper, or `kpsewhich` when
/// LuaTeX is not available.
#[derive(Debug)]
enum Lookup {
    Unstarted,
    Luatex(LuatexKpsewhich),
    Kpsewhich,
}

/// matplotlib's `find_tex_file`, with its results kept (found or not) for
/// the life of the process.
#[derive(Debug)]
pub(crate) struct Kpathsea {
    cache_dir: PathBuf,
    lookup: Mutex<Lookup>,
    found: Mutex<HashMap<String, Option<PathBuf>>>,
}

impl Kpathsea {
    /// A lookup whose generated fonts go to `cache_dir/vartexfonts`
    /// (where the LuaTeX helper's script is written too).
    pub(crate) fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            lookup: Mutex::new(Lookup::Unstarted),
            found: Mutex::new(HashMap::new()),
        }
    }

    /// Runs `kpsewhich -mktex=pk name`.
    fn kpsewhich(&self, name: &str) -> Option<String> {
        let output = Command::new("kpsewhich")
            .arg("-mktex=pk")
            .arg(name)
            .env("MT_VARTEXFONTS", self.cache_dir.join("vartexfonts"))
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let path = text.strip_suffix('\n').unwrap_or(&text);
        (!path.is_empty()).then(|| path.to_owned())
    }

    /// Asks kpathsea for `name`, starting (or restarting) the LuaTeX
    /// helper as needed.
    fn search(&self, name: &str) -> Option<String> {
        let mut lookup = self.lookup.lock().unwrap_or_else(PoisonError::into_inner);
        if let Lookup::Luatex(helper) = &mut *lookup
            && helper.is_dead()
        {
            *lookup = Lookup::Unstarted;
        }
        if matches!(*lookup, Lookup::Unstarted) {
            *lookup =
                LuatexKpsewhich::start(&self.cache_dir).map_or(Lookup::Kpsewhich, Lookup::Luatex);
        }
        if let Lookup::Luatex(helper) = &mut *lookup {
            match helper.search(name) {
                Answer::Found(path) => return Some(path),
                Answer::Absent => return None,
                // A helper that stops answering is given up for kpsewhich.
                Answer::Silent => *lookup = Lookup::Kpsewhich,
            }
        }
        self.kpsewhich(name)
    }
}

impl FileFinder for Kpathsea {
    fn find(&self, name: &str) -> Result<PathBuf, TexError> {
        let known = self
            .found
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(name)
            .cloned();
        let found = if let Some(found) = known {
            found
        } else {
            // Names with line breaks would desynchronize the helper.
            let found = (!name.contains(['\n', '\r']))
                .then(|| self.search(name))
                .flatten()
                .map(PathBuf::from);
            self.found
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(name.to_owned(), found.clone());
            found
        };
        found.ok_or_else(|| TexError::NotFound {
            name: name.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_helper_script_is_matplotlibs() {
        assert!(KPSEWHICH_LUA.contains("kpse.init_prog(\"\", 600, \"ljfour\")"));
        assert!(KPSEWHICH_LUA.contains("{mktexpk=true}"));
        assert!(KPSEWHICH_LUA.ends_with("end\n"));
    }

    #[test]
    fn names_with_line_breaks_are_never_found() {
        let kpathsea = Kpathsea::new(std::env::temp_dir().join("plt-rs-kpathsea-test"));
        assert_eq!(
            kpathsea.find("cmr10\n.tfm"),
            Err(TexError::NotFound {
                name: "cmr10\n.tfm".into()
            })
        );
    }
}
