//! Installs pltrs's desktop entry and icons, so that GNOME's dash and
//! overview show pltrs's icon for its windows (see
//! `pltrs::gui::install_desktop_entry`): into the user's data directory
//! (`~/.local/share`), or into the data directory given; `--remove` takes
//! them away again.
//!
//! ```text
//! cargo run -p pltrs --example install_desktop_entry
//! cargo run -p pltrs --example install_desktop_entry -- --remove
//! cargo run -p pltrs --example install_desktop_entry -- python/share
//! ```
//!
//! The last writes the files pltrs (Python)'s Linux wheels install, which
//! the tree keeps in `python/share`.

#![expect(
    clippy::print_stdout,
    reason = "the example reports the files it wrote or removed"
)]

use std::path::PathBuf;

use pltrs::gui::{
    install_desktop_entry, install_desktop_entry_in, remove_desktop_entry,
    remove_desktop_entry_from,
};

fn main() -> std::io::Result<()> {
    let mut remove = false;
    let mut data_dir = None;
    for argument in std::env::args().skip(1) {
        if argument == "--remove" {
            remove = true;
        } else {
            data_dir = Some(PathBuf::from(argument));
        }
    }
    let files: Vec<PathBuf> = match (remove, data_dir.as_deref()) {
        (false, None) => written(install_desktop_entry()?),
        (false, Some(dir)) => written(install_desktop_entry_in(dir)?),
        (true, None) => remove_desktop_entry()?,
        (true, Some(dir)) => remove_desktop_entry_from(dir)?,
    };
    let verb = if remove { "removed" } else { "wrote" };
    for file in &files {
        println!("{verb} {}", file.display());
    }
    if files.is_empty() {
        println!("nothing to do");
    }
    Ok(())
}

/// The files of an installed entry, the entry first.
fn written(entry: pltrs::gui::DesktopEntry) -> Vec<PathBuf> {
    std::iter::once(entry.entry).chain(entry.icons).collect()
}
