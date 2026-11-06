//! Benchmarking a build on a machine that cannot build it. `cargo xtask
//! bench --pack DIR [--revision NAME]` gathers into `DIR` what the
//! benchmark runs: this executable (pltrs (Rust) and the harness), the
//! package of pltrs (Python) with its extension built as the wheel builds
//! it, the scripts and their Python harness, and a manifest naming the
//! commit and the compiler of the build.
//!
//! Copied to another machine and given the two venvs there
//! (`DIR/.venv` with matplotlib, `DIR/.venv-pltrs` without), `DIR/xtask
//! bench ...` runs as it does in the workspace: an executable with a
//! manifest beside it takes its files from its own directory (the
//! [`Tree`] it runs in), builds nothing, and reports the commit and the
//! compiler the manifest names rather than the machine's.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use pltrs_core::os::Os;

/// The manifest's file name in a pack.
const MANIFEST: &str = "bench-pack.txt";

/// The files of the tree a pack copies, beside the executable: the
/// scripts, their harness and the package of pltrs (Python).
const COPIED: [&str; 3] = ["benchmarks/python", "spec/reference/bench", "python/pltrs"];

/// What built a pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Build {
    /// The commit, as `git describe` names it or as `--revision` gave it.
    pub(super) revision: String,
    /// The Rust compiler's version.
    pub(super) rustc: String,
}

impl Build {
    /// The manifest's text: one tab-separated `key value` line per field.
    fn to_text(&self) -> String {
        format!("revision\t{}\nrustc\t{}\n", self.revision, self.rustc)
    }

    /// The build a manifest's `text` describes; what it does not name is
    /// `unknown`.
    fn from_text(text: &str) -> Self {
        let field = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key)?.strip_prefix('\t'))
                .map_or_else(unknown, str::to_owned)
        };
        Self {
            revision: field("revision"),
            rustc: field("rustc"),
        }
    }
}

/// Where the benchmark finds what it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Tree {
    /// The workspace, or the pack's directory.
    pub(super) root: PathBuf,
    /// What built the pack; `None` in the workspace.
    pub(super) build: Option<Build>,
}

impl Tree {
    /// The pack in `dir`.
    pub(super) fn pack(dir: &Path) -> Result<Self, Box<dyn Error>> {
        let manifest = dir.join(MANIFEST);
        let text = fs::read_to_string(&manifest)
            .map_err(|err| format!("{} is no pack: {err}", manifest.display()))?;
        Ok(Self {
            root: dir.to_owned(),
            build: Some(Build::from_text(&text)),
        })
    }

    /// The executable of this tree's pltrs (Rust): the one running in the
    /// workspace, `xtask` in a pack.
    pub(super) fn executable(&self) -> Result<PathBuf, Box<dyn Error>> {
        Ok(if self.build.is_some() {
            self.root.join(executable_name())
        } else {
            std::env::current_exe()?
        })
    }

    /// The Python harness of the scripts.
    pub(super) fn harness(&self) -> PathBuf {
        self.root.join("spec/reference/bench/bench.py")
    }
}

/// The tree this executable runs in: its pack, if it has a manifest
/// beside it, else the workspace.
pub(super) fn current() -> &'static Tree {
    static TREE: OnceLock<Tree> = OnceLock::new();
    TREE.get_or_init(|| {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_owned))
            .and_then(|dir| Tree::pack(&dir).ok())
            .unwrap_or_else(|| Tree {
                root: crate::python::root(),
                build: None,
            })
    })
}

/// The root of the tree this executable runs in.
pub(super) fn root() -> &'static Path {
    &current().root
}

/// The interpreter of the reference venv (`.venv`, matplotlib) of the
/// tree this executable runs in.
pub(super) fn reference_python() -> PathBuf {
    crate::python::interpreter(&root().join(".venv"), Os::CURRENT)
}

/// The interpreter of the pltrs venv (`.venv-pltrs`) of the tree this
/// executable runs in, or why there is none.
pub(super) fn pltrs_python() -> Result<PathBuf, String> {
    let python = crate::python::interpreter(&root().join(".venv-pltrs"), Os::CURRENT);
    if python.exists() {
        Ok(python)
    } else {
        Err(format!(
            "{} does not exist: run `cargo xtask python setup`, or make the venv in the pack",
            python.display()
        ))
    }
}

/// The executable's file name in a pack.
fn executable_name() -> String {
    format!("xtask{}", std::env::consts::EXE_SUFFIX)
}

/// Builds the extension of pltrs (Python) as the wheel builds it and
/// writes the pack of this build into `dir`, naming the commit
/// `revision`, or what `git describe` says of the workspace.
pub(super) fn write(dir: &Path, revision: Option<&str>) -> Result<(), Box<dyn Error>> {
    if current().build.is_some() {
        return Err("a pack cannot pack itself again: run --pack in the workspace".into());
    }
    let root = crate::python::root();
    crate::python::build(crate::python::Profile::Release)?;
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    for path in COPIED {
        copy_tree(&root.join(path), &dir.join(path))?;
    }
    fs::copy(std::env::current_exe()?, dir.join(executable_name()))?;
    let build = Build {
        revision: revision.map_or_else(|| first_line(&root, "git", &describe()), str::to_owned),
        rustc: first_line(&root, "rustc", &["--version"]),
    };
    fs::write(dir.join(MANIFEST), build.to_text())?;
    println!(
        "packed {} ({}, {}) into {}",
        root.display(),
        build.revision,
        build.rustc,
        dir.display()
    );
    Ok(())
}

/// `git describe`'s arguments naming the commit, `-dirty` when the tree
/// has changes.
pub(super) fn describe() -> [&'static str; 4] {
    ["describe", "--always", "--dirty", "--abbrev=10"]
}

/// The first line `program args` prints, run in `dir`, or `unknown`.
pub(super) fn first_line(dir: &Path, program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| text.lines().next().map(str::to_owned))
        .unwrap_or_else(unknown)
}

/// What stands for a value that cannot be read.
pub(super) fn unknown() -> String {
    "unknown".to_owned()
}

/// Copies the directory `source` to `target`, but for Python's caches.
fn copy_tree(source: &Path, target: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let to = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            if entry.file_name() != "__pycache__" {
                copy_tree(&path, &to)?;
            }
        } else {
            fs::copy(&path, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_reads_back() {
        let build = Build {
            revision: "0f901fc9a1-dirty".to_owned(),
            rustc: "rustc 1.99.1 (58cf8f9c3 2026-10-13)".to_owned(),
        };
        assert_eq!(Build::from_text(&build.to_text()), build);
        assert_eq!(Build::from_text("revision\tabc\n").rustc, "unknown");
    }

    #[test]
    fn a_pack_is_its_own_tree() {
        let dir = crate::python::target_dir().join("bench-pack-test");
        fs::create_dir_all(&dir).expect("a scratch directory");
        assert!(Tree::pack(&dir.join("missing")).is_err());
        fs::write(dir.join(MANIFEST), "revision\tabc\nrustc\trustc 1\n").expect("a manifest");
        let tree = Tree::pack(&dir).expect("a pack");
        assert_eq!(tree.root, dir);
        assert_eq!(
            tree.executable().expect("its executable"),
            dir.join(executable_name())
        );
        assert_eq!(tree.harness(), dir.join("spec/reference/bench/bench.py"));
        fs::remove_dir_all(&dir).expect("cleaned up");
    }

    #[test]
    fn the_workspace_is_the_tree_of_tests() {
        assert_eq!(current().root, crate::python::root());
        assert!(current().build.is_none());
    }
}
