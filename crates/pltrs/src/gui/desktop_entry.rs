//! pltrs's desktop entry, for Linux desktops that take a window's icon
//! from the application it belongs to rather than from the window.
//!
//! GNOME's shell is one: mutter offers windows neither
//! `xdg-toplevel-icon-v1` nor `_NET_WM_ICON` for its dash and overview,
//! and finds a window's icon in the `.desktop` file named after its app
//! id (Wayland) or `WM_CLASS` (X11), `pltrs`: without one, pltrs's
//! windows show a generic icon there. pltrs never writes into the user's
//! home unasked; [`install_desktop_entry`] does it when called:
//!
//! * `applications/pltrs.desktop`, an entry the application lists leave
//!   out (`NoDisplay`), which names the icon and the windows' class;
//! * `icons/hicolor/<side>x<side>/apps/pltrs.png`, pltrs's
//!   [`app_icon`](super::app_icon) drawn at each of [`ICON_SIDES`].
//!
//! Icon themes (GTK's, GNOME's shell's) look for new icons only when the
//! theme's directory changes, not the size directories under it, so the
//! installer marks `icons/hicolor` changed, as `xdg-icon-resource` does.
//! GNOME's shell notices a new entry within some seconds; windows opened
//! after that show the icon. [`remove_desktop_entry`] takes the files
//! away again.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use pltrs_codec::png::ImageRef;
use pltrs_platform::IconLayout;

use super::app_icon;

/// The sides of the icons written, in pixels: the hicolor theme's sizes,
/// up to what a large application grid shows on a dense screen.
pub const ICON_SIDES: [u32; 8] = [16, 24, 32, 48, 64, 128, 256, 512];

/// The name of the entry and of its icon, which is the app id and the
/// `WM_CLASS` of pltrs's windows.
const NAME: &str = "pltrs";
/// The icon theme the icons go into, which every theme falls back to.
const THEME: &str = "icons/hicolor";

/// The desktop entry's text.
const ENTRY: &str = "[Desktop Entry]
Type=Application
Name=pltrs
Comment=Figure windows of pltrs, matplotlib rebuilt in Rust
Icon=pltrs
StartupWMClass=pltrs
NoDisplay=true
";

/// The files of pltrs's desktop entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    /// `applications/pltrs.desktop`.
    pub entry: PathBuf,
    /// `icons/hicolor/<side>x<side>/apps/pltrs.png`, by side.
    pub icons: Vec<PathBuf>,
}

impl DesktopEntry {
    /// Where the entry's files go in the data directory `data_dir` (such
    /// as `~/.local/share` or `/usr/share`).
    pub fn in_dir(data_dir: &Path) -> Self {
        let icon = |side: u32| {
            data_dir
                .join(THEME)
                .join(format!("{side}x{side}"))
                .join("apps")
                .join(format!("{NAME}.png"))
        };
        Self {
            entry: data_dir
                .join("applications")
                .join(format!("{NAME}.desktop")),
            icons: ICON_SIDES.into_iter().map(icon).collect(),
        }
    }

    /// The files, the entry first.
    fn files(&self) -> impl Iterator<Item = &PathBuf> {
        std::iter::once(&self.entry).chain(&self.icons)
    }
}

/// The user's data directory: `$XDG_DATA_HOME` if it is an absolute
/// path, else `~/.local/share`, as the XDG Base Directory specification
/// says.
///
/// # Errors
///
/// [`io::ErrorKind::NotFound`] when neither `$XDG_DATA_HOME` nor `$HOME`
/// is set.
pub fn user_data_dir() -> io::Result<PathBuf> {
    let set = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    if let Some(data) = set("XDG_DATA_HOME").map(PathBuf::from)
        && data.is_absolute()
    {
        return Ok(data);
    }
    set("HOME")
        .map(|home| PathBuf::from(home).join(".local/share"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "neither $XDG_DATA_HOME nor $HOME is set",
            )
        })
}

/// The PNG file of pltrs's icon `side` pixels wide, as desktops other
/// than macOS show it (full bleed).
fn icon_png(side: u32) -> io::Result<Vec<u8>> {
    let image = app_icon()
        .images(&[side], IconLayout::FullBleed)
        .into_iter()
        .next()
        .ok_or_else(|| io::Error::other(format!("the icon cannot be drawn at {side} pixels")))?;
    let size = image.size();
    let pixels =
        ImageRef::rgba8(size.width, size.height, image.rgba()).map_err(io::Error::other)?;
    pltrs_codec::png::encode(pixels).map_err(io::Error::other)
}

/// Writes pltrs's desktop entry and icons into the user's data directory
/// ([`user_data_dir`]), so that GNOME (and other desktops finding icons
/// through applications) shows pltrs's icon for its windows. Files there
/// already are replaced. Returns the files written.
///
/// The same, without Rust, from pltrs (Python):
///
/// ```text
/// python -c "import pltrs.backends.backend_pltrs as b; b.install_desktop_entry()"
/// ```
///
/// # Errors
///
/// When there is no data directory, or a file cannot be written.
pub fn install_desktop_entry() -> io::Result<DesktopEntry> {
    install_desktop_entry_in(&user_data_dir()?)
}

/// Writes pltrs's desktop entry and icons into the data directory
/// `data_dir` (`/usr/share` for every user, or a package's): see
/// [`install_desktop_entry`].
///
/// # Errors
///
/// When a directory cannot be made or a file written.
pub fn install_desktop_entry_in(data_dir: &Path) -> io::Result<DesktopEntry> {
    let files = DesktopEntry::in_dir(data_dir);
    write(&files.entry, ENTRY.as_bytes())?;
    for (&side, path) in ICON_SIDES.iter().zip(&files.icons) {
        write(path, &icon_png(side)?)?;
    }
    // Only the desktops of Unix systems read `hicolor` themes.
    #[cfg(unix)]
    mark_theme_changed(data_dir)?;
    Ok(files)
}

/// Marks the icon theme of `data_dir` changed: its directory's
/// modification time is now, so that icon themes look at it again.
#[cfg(unix)]
fn mark_theme_changed(data_dir: &Path) -> io::Result<()> {
    fs::File::open(data_dir.join(THEME))?.set_modified(std::time::SystemTime::now())
}

/// Writes `bytes` to `path`, making its directory.
fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(path, bytes)
}

/// Removes the files [`install_desktop_entry`] writes from the user's
/// data directory. Returns those that were there.
///
/// # Errors
///
/// When there is no data directory, or a file cannot be removed.
pub fn remove_desktop_entry() -> io::Result<Vec<PathBuf>> {
    remove_desktop_entry_from(&user_data_dir()?)
}

/// Removes the files [`install_desktop_entry_in`] writes from `data_dir`.
/// Returns those that were there.
///
/// # Errors
///
/// When a file cannot be removed.
pub fn remove_desktop_entry_from(data_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let files = DesktopEntry::in_dir(data_dir);
    let mut removed = Vec::new();
    for path in files.files() {
        match fs::remove_file(path) {
            Ok(()) => removed.push(path.clone()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    // The size directories left empty (`<side>x<side>/apps` and
    // `<side>x<side>`) go too: removing a directory that is not empty
    // fails, which leaves it.
    for icon in &files.icons {
        for directory in icon.ancestors().skip(1).take(2) {
            let _ = fs::remove_dir(directory);
        }
    }
    #[cfg(unix)]
    if !removed.is_empty() && data_dir.join(THEME).is_dir() {
        mark_theme_changed(data_dir)?;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch data directory in the temporary directory, removed when
    /// dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("pltrs-desktop-entry-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_entry_and_its_icons_are_written_and_removed() {
        let scratch = Scratch::new("install");
        let files = install_desktop_entry_in(&scratch.0).unwrap();
        assert_eq!(files.entry, scratch.0.join("applications/pltrs.desktop"));
        let entry = fs::read_to_string(&files.entry).unwrap();
        assert!(entry.starts_with("[Desktop Entry]\n"));
        for line in [
            "Type=Application",
            "Icon=pltrs",
            "StartupWMClass=pltrs",
            "NoDisplay=true",
        ] {
            assert!(entry.lines().any(|l| l == line), "{line}");
        }
        assert_eq!(files.icons.len(), ICON_SIDES.len());
        for (side, path) in ICON_SIDES.iter().zip(&files.icons) {
            assert_eq!(
                *path,
                scratch
                    .0
                    .join(format!("icons/hicolor/{side}x{side}/apps/pltrs.png"))
            );
            let png = pltrs_codec::png::decode(&fs::read(path).unwrap()).unwrap();
            assert_eq!((png.width(), png.height()), (*side, *side));
        }
        // Again: the files are replaced.
        assert_eq!(install_desktop_entry_in(&scratch.0).unwrap(), files);
        // Someone else's icon keeps its size directory.
        let other = scratch.0.join("icons/hicolor/48x48/apps/other.png");
        fs::write(&other, b"").unwrap();
        let removed = remove_desktop_entry_from(&scratch.0).unwrap();
        assert_eq!(removed.len(), 1 + ICON_SIDES.len());
        assert!(!files.entry.exists());
        assert!(
            !scratch.0.join("icons/hicolor/16x16").exists(),
            "emptied, removed"
        );
        assert!(other.exists());
        assert_eq!(
            remove_desktop_entry_from(&scratch.0).unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    #[cfg(unix)]
    fn the_icon_theme_is_marked_changed() {
        use std::time::{Duration, SystemTime};
        let scratch = Scratch::new("theme");
        install_desktop_entry_in(&scratch.0).unwrap();
        // Writing into size directories that are there already changes
        // no directory of the theme: the installer marks it changed.
        let theme = scratch.0.join(THEME);
        let past = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
        let age = |theme: &Path| fs::metadata(theme).unwrap().modified().unwrap();
        fs::File::open(&theme).unwrap().set_modified(past).unwrap();
        install_desktop_entry_in(&scratch.0).unwrap();
        assert!(age(&theme) > past);
        fs::File::open(&theme).unwrap().set_modified(past).unwrap();
        remove_desktop_entry_from(&scratch.0).unwrap();
        assert!(age(&theme) > past, "and when its icons go");
    }

    #[test]
    fn the_icons_are_the_app_icons_full_bleed() {
        let png = pltrs_codec::png::decode(&icon_png(64).unwrap()).unwrap();
        let icon = app_icon().images(&[64], IconLayout::FullBleed).remove(0);
        assert_eq!(png.to_rgba8(), icon.rgba());
    }

    #[test]
    fn failures_are_errors() {
        let scratch = Scratch::new("blocked");
        fs::create_dir_all(&scratch.0).unwrap();
        // A file where the directory of the entry should be.
        fs::write(scratch.0.join("applications"), b"").unwrap();
        assert!(install_desktop_entry_in(&scratch.0).is_err());
        if std::env::var_os("HOME").is_some() {
            assert!(user_data_dir().unwrap().is_absolute());
        }
    }
}
