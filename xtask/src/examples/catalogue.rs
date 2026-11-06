//! The examples on disk: the pairs of a Rust binary
//! (`examples/rust/src/bin/<name>.rs`) and a Python script
//! (`examples/python/<name>.py`) drawing the same figure, the images they
//! write, and the gallery (`examples/README.md`) that shows them; and the
//! tutorials (`examples/rust/tutorials/<name>.rs`), matplotlib's
//! tutorials as cargo examples of the `pltrs-examples` crate.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Modules of `examples/python/` that the scripts share rather than
/// examples of their own.
const PYTHON_HELPERS: &[&str] = &["samples"];

/// The extensions of the images examples write: a figure as a PNG, an
/// animation as a GIF.
const IMAGE_EXTENSIONS: &[&str] = &["png", "gif"];

/// Where the examples and their outputs live, under the workspace root.
#[derive(Debug)]
pub(super) struct Layout {
    root: PathBuf,
}

impl Layout {
    /// The layout of the workspace at `root`.
    pub(super) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// The workspace root.
    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    /// The sources of the Rust examples.
    pub(super) fn rust_sources(&self) -> PathBuf {
        self.root.join("examples/rust/src/bin")
    }

    /// The tutorials.
    pub(super) fn tutorial_sources(&self) -> PathBuf {
        self.root.join("examples/rust/tutorials")
    }

    /// The manifest of the `pltrs-examples` crate, which registers the
    /// tutorials.
    pub(super) fn rust_manifest(&self) -> PathBuf {
        self.root.join("examples/rust/Cargo.toml")
    }

    /// The Python examples.
    pub(super) fn python_sources(&self) -> PathBuf {
        self.root.join("examples/python")
    }

    /// The committed images of the gallery.
    pub(super) fn images(&self) -> PathBuf {
        self.root.join("examples/images")
    }

    /// The gallery page.
    pub(super) fn readme(&self) -> PathBuf {
        self.root.join("examples/README.md")
    }

    /// A directory for one kind of output, under `target/examples`.
    pub(super) fn output(&self, kind: &str) -> PathBuf {
        self.root.join("target/examples").join(kind)
    }
}

/// The names of the files in `dir` with extension `extension`, sorted.
fn stems(dir: &Path, extension: &str) -> Result<BTreeSet<String>, String> {
    let entries =
        fs::read_dir(dir).map_err(|err| format!("cannot list {}: {err}", dir.display()))?;
    Ok(entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == extension))
        .filter_map(|path| Some(path.file_stem()?.to_str()?.to_owned()))
        .collect())
}

/// Every example, by name: the Rust binaries, each of which must have
/// its Python script, and no Python script without its binary.
pub(super) fn discover(layout: &Layout) -> Result<Vec<String>, String> {
    let rust = stems(&layout.rust_sources(), "rs")?;
    let mut python = stems(&layout.python_sources(), "py")?;
    python.retain(|name| !PYTHON_HELPERS.contains(&name.as_str()));
    let unpaired: Vec<String> = rust
        .symmetric_difference(&python)
        .map(|name| {
            let (has, lacks) = if rust.contains(name) {
                ("a Rust", "Python")
            } else {
                ("a Python", "Rust")
            };
            format!("{name} has {has} version but no {lacks} one")
        })
        .collect();
    if unpaired.is_empty() {
        Ok(rust.into_iter().collect())
    } else {
        Err(unpaired.join("\n"))
    }
}

/// Every tutorial, by name, each of which the crate's manifest must
/// register as a cargo example (`path = "tutorials/<name>.rs"`).
pub(super) fn tutorials(layout: &Layout) -> Result<Vec<String>, String> {
    let names = stems(&layout.tutorial_sources(), "rs")?;
    let manifest_path = layout.rust_manifest();
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("cannot read {}: {err}", manifest_path.display()))?;
    let unregistered = unregistered_tutorials(&manifest, &names);
    if unregistered.is_empty() {
        Ok(names.into_iter().collect())
    } else {
        Err(unregistered.join("\n"))
    }
}

/// What `manifest` lacks for the tutorials `names`: the `[[example]]`
/// entry pointing at each.
fn unregistered_tutorials(manifest: &str, names: &BTreeSet<String>) -> Vec<String> {
    names
        .iter()
        .filter(|name| !manifest.contains(&format!("path = \"tutorials/{name}.rs\"")))
        .map(|name| format!("examples/rust/Cargo.toml has no [[example]] for tutorials/{name}.rs"))
        .collect()
}

/// The image the example `name` wrote to `dir`, if it wrote one.
pub(super) fn image(dir: &Path, name: &str) -> Option<PathBuf> {
    IMAGE_EXTENSIONS
        .iter()
        .map(|extension| dir.join(format!("{name}.{extension}")))
        .find(|path| path.is_file())
}

/// Removes the images in `dir` of the examples `names`, so that an
/// example failing to write its image cannot leave an old one behind.
pub(super) fn clear_images(dir: &Path, names: &[String]) -> Result<(), String> {
    for path in names.iter().filter_map(|name| image(dir, name)) {
        fs::remove_file(&path).map_err(|err| format!("cannot remove {}: {err}", path.display()))?;
    }
    Ok(())
}

/// The images in `dir` that belong to none of the examples `names`.
pub(super) fn strays(dir: &Path, names: &[String]) -> Result<Vec<PathBuf>, String> {
    let entries =
        fs::read_dir(dir).map_err(|err| format!("cannot list {}: {err}", dir.display()))?;
    Ok(entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .is_none_or(|stem| !names.iter().any(|name| name == stem))
        })
        .collect())
}

/// What the gallery page lacks for the example `name` whose image is
/// `image`: a link to the image and to both sources, as paths relative
/// to the page.
pub(super) fn readme_gaps(readme: &str, name: &str, image: &Path) -> Vec<String> {
    let image = image
        .file_name()
        .map_or_else(String::new, |file| file.to_string_lossy().into_owned());
    [
        format!("images/{image}"),
        format!("rust/src/bin/{name}.rs"),
        format!("python/{name}.py"),
    ]
    .into_iter()
    .filter(|link| !readme.contains(&format!("({link})")))
    .map(|link| format!("examples/README.md does not link {link}"))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tutorial_must_be_a_cargo_example() {
        let manifest = "[[example]]\nname = \"layouts\"\npath = \"tutorials/layouts.rs\"\n";
        let names: BTreeSet<String> = ["layouts", "saving"].map(String::from).into();
        assert_eq!(
            unregistered_tutorials(manifest, &names),
            ["examples/rust/Cargo.toml has no [[example]] for tutorials/saving.rs"]
        );
    }

    #[test]
    fn the_readme_must_link_the_image_and_both_sources() {
        let readme = "![a](images/line_plot.png) [Rust](rust/src/bin/line_plot.rs)";
        let gaps = readme_gaps(readme, "line_plot", Path::new("/x/line_plot.png"));
        assert_eq!(
            gaps,
            ["examples/README.md does not link python/line_plot.py"]
        );
        let complete = format!("{readme} [Python](python/line_plot.py)");
        assert_eq!(
            readme_gaps(&complete, "line_plot", Path::new("line_plot.png")),
            Vec::<String>::new()
        );
    }
}
