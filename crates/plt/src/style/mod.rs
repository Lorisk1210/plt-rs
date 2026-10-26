//! Style sheets: named sets of rcParams that restyle every plot
//! (`matplotlib.style`).
//!
//! The [`library`] bundles matplotlib's built-in sheets and plt-rs' own
//! `plt-modern` and `plt-modern-dark` (the [looks](crate::look) of
//! figures), plus the sheets of the user library directories: `stylelib`
//! in the [configuration directory](crate::config) unless
//! [`set_user_library_paths`] names others ([`reload_library`]). A [`Style`] names a
//! library sheet, a style file or a set of parameters; [`use_style`] and
//! [`use_styles`] apply styles (matplotlib's `style.use`), [`context`] and
//! [`context_with`] for a scope (`style.context`):
//!
//! ```
//! use plt::{rc, style};
//! use plt::style::Style;
//! rc::reset(); // matplotlib's defaults, without grids
//! style::context("ggplot", || {
//!     assert!(rc::params().axes_grid);
//!     Ok(())
//! })?;
//! assert!(!rc::params().axes_grid);
//! let styles = [Style::from("ggplot"), Style::params(&[("axes.grid", "False")])];
//! let grid = style::context_with(&styles, || Ok(rc::params().axes_grid))?;
//! assert!(!grid);
//! # Ok::<(), plt::rc::RcError>(())
//! ```
//!
//! [`apply_xkcd`] sets the parameters of matplotlib's xkcd sketch style;
//! [`pyplot::xkcd`](crate::pyplot::xkcd) applies them for a scope.
//!
//! The sheets in `style/sheets/` other than the `plt-*` ones are copied
//! verbatim from matplotlib 3.11 (`mpl-data/stylelib`), copyright the
//! Matplotlib Development Team and distributed under the Matplotlib
//! license; the comments of several
//! sheets credit the projects their palettes come from (seaborn,
//! fivethirtyeight, Tableau, Solarized, Petroff's accessible colour
//! sequences, ...).

mod library;
mod xkcd;

use std::path::{Path, PathBuf};

pub(crate) use library::bundled_sheet;
pub use library::{
    StyleSheet, available, library, reload_library, set_user_library_paths, sheet,
    user_library_paths,
};
pub use xkcd::{XkcdError, XkcdSketch, apply_xkcd};

use crate::rc::{RcEntry, RcError, RcFile, RcParams};

/// rcParams that are not about style, which [`use_styles`] ignores
/// (matplotlib's `STYLE_BLACKLIST`).
pub const STYLE_BLACKLIST: [&str; 15] = [
    "interactive",
    "backend",
    "webagg.port",
    "webagg.address",
    "webagg.port_retries",
    "webagg.open_in_browser",
    "backend_fallback",
    "toolbar",
    "timezone",
    "figure.max_open_warning",
    "figure.raise_window",
    "savefig.directory",
    "tk.window_focus",
    "docstring.hardcopy",
    "date.epoch",
];

/// One style of [`use_styles`] (an item of matplotlib's `style.use`
/// argument).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Style {
    /// A library sheet; `"default"` (or `"mpl20"`) for matplotlib's
    /// defaults and `"mpl15"` for `"classic"`. Any other name is the path
    /// or `file:` URL of a style file.
    Named(String),
    /// A style file, read as matplotlib's `rc_params_from_file` does
    /// (lines it cannot use are skipped).
    File(PathBuf),
    /// Parameters in rc-file syntax, applied in order.
    Params(Vec<(String, String)>),
}

impl Style {
    /// [`Style::Params`] from `(key, value)` pairs.
    pub fn params(pairs: &[(&str, &str)]) -> Self {
        Self::Params(
            pairs
                .iter()
                .map(|&(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
        )
    }

    /// The parameters the style sets, in order.
    fn settings(&self) -> Result<Vec<(String, String)>, RcError> {
        let pairs = |entries: Vec<RcEntry>| {
            entries
                .into_iter()
                .map(|entry| (entry.key, entry.value))
                .collect()
        };
        match self {
            Self::Named(name) => match name.as_str() {
                "default" | "mpl20" => Ok(defaults()),
                "mpl15" => Self::Named("classic".into()).settings(),
                name => match sheet(name) {
                    Some(sheet) => sheet.entries().map(pairs),
                    None => RcFile::read(style_file(name))
                        .map(|file| pairs(file.entries))
                        .map_err(|_| RcError::UnknownStyle(name.to_owned())),
                },
            },
            Self::File(path) => RcFile::read(path).map(|file| pairs(file.entries)),
            Self::Params(pairs) => Ok(pairs.clone()),
        }
    }
}

impl From<&str> for Style {
    fn from(name: &str) -> Self {
        Self::Named(name.to_owned())
    }
}

impl From<&Path> for Style {
    fn from(path: &Path) -> Self {
        Self::File(path.to_owned())
    }
}

impl From<PathBuf> for Style {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}

/// Every parameter at matplotlib's default (the `"default"` style).
fn defaults() -> Vec<(String, String)> {
    let defaults = RcParams::matplotlib();
    RcParams::KEYS
        .iter()
        .filter_map(|&key| Some((key.to_owned(), defaults.get(key)?)))
        .collect()
}

/// The file a style name that is no library sheet stands for: the path
/// itself, or the path of a `file:` URL (`file:/dir/a.mplstyle`,
/// `file:///dir/a.mplstyle`), percent-escapes decoded.
fn style_file(name: &str) -> PathBuf {
    let Some(url) = name.strip_prefix("file:") else {
        return PathBuf::from(name);
    };
    let path = match url.strip_prefix("//") {
        Some(authority_and_path) => authority_and_path
            .find('/')
            .map_or("", |slash| &authority_and_path[slash..]),
        None => url,
    };
    PathBuf::from(percent_decode(path))
}

/// `text` with `%XX` escapes replaced by the bytes they stand for.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%')
            .then(|| text.get(i + 1..i + 3))
            .flatten()
            .filter(|hex| hex.bytes().all(|b| b.is_ascii_hexdigit()))
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        if let Some(byte) = escaped {
            decoded.push(byte);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// Applies a style to the current parameters (matplotlib's `style.use`
/// with one name); see [`Style::Named`].
///
/// # Errors
///
/// As [`use_styles`].
pub fn use_style(name: &str) -> Result<(), RcError> {
    use_styles(&[Style::from(name)])
}

/// Applies styles to the current parameters, first to last (matplotlib's
/// `style.use`). Parameters of the [`STYLE_BLACKLIST`] are left alone. As
/// in matplotlib, the parameters set before an invalid one stay set.
///
/// # Errors
///
/// [`RcError::UnknownStyle`] for a name that is neither a library sheet
/// nor a readable style file, the read error of a [`Style::File`], and
/// the first unknown key or invalid value of a style.
pub fn use_styles(styles: &[Style]) -> Result<(), RcError> {
    for style in styles {
        let settings = style.settings()?;
        let mut updated = (*crate::rc::params()).clone();
        let applied = settings
            .iter()
            .filter(|(key, _)| !STYLE_BLACKLIST.contains(&key.as_str()))
            .try_for_each(|(key, value)| updated.set(key, value));
        crate::rc::replace(updated);
        applied?;
    }
    Ok(())
}

/// Runs `f` with a style applied and restores the parameters afterwards
/// (matplotlib's `style.context` with one name). `f` returns a `Result`
/// whose error a failure to apply the style converts into, so the call
/// returns one `Result`.
///
/// ```
/// use std::error::Error;
///
/// use plt::prelude::*;
/// use plt::style;
///
/// let png = style::context("ggplot", || -> Result<_, Box<dyn Error>> {
///     let fig = Figure::new();
///     fig.subplot().plot([0.0, 1.0], [1.0, 0.0])?;
///     Ok(fig.to_png()?)
/// })?;
/// assert!(png.starts_with(b"\x89PNG"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// As [`use_styles`], without running `f`, and `f`'s own errors.
pub fn context<T, E: From<RcError>>(name: &str, f: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    context_with(&[Style::from(name)], f)
}

/// Runs `f` with styles applied and restores the parameters afterwards,
/// also when applying them fails (matplotlib's `style.context`); as
/// [`context`].
///
/// # Errors
///
/// As [`use_styles`], without running `f`, and `f`'s own errors.
pub fn context_with<T, E: From<RcError>>(
    styles: &[Style],
    f: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    crate::rc::context(|| {
        use_styles(styles)?;
        f()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_apply_and_reset() {
        crate::rc::context(|| {
            use_style("dark_background").unwrap();
            assert_eq!(crate::rc::params().figure_facecolor, "black".into());
            use_style("default").unwrap();
            assert_eq!(*crate::rc::params(), RcParams::matplotlib());
            assert_eq!(
                use_style("no-such-style"),
                Err(RcError::UnknownStyle("no-such-style".into()))
            );
        });
    }

    #[test]
    fn later_styles_win_and_blacklisted_keys_are_ignored() {
        crate::rc::context(|| {
            crate::rc::reset();
            let styles = [
                Style::params(&[("image.cmap", "pink"), ("timezone", "Europe/Paris")]),
                Style::params(&[("image.cmap", "gray")]),
            ];
            use_styles(&styles).unwrap();
            let rc = crate::rc::params();
            assert_eq!(
                (rc.image_cmap.as_str(), rc.timezone.as_str()),
                ("gray", "UTC")
            );
        });
    }

    #[test]
    fn a_failing_style_keeps_what_it_set_before_but_contexts_restore() {
        crate::rc::context(|| {
            crate::rc::reset();
            let bad = [Style::params(&[("image.cmap", "pink"), ("badparam", "1")])];
            assert_eq!(
                use_styles(&bad),
                Err(RcError::UnknownKey("badparam".into()))
            );
            assert_eq!(crate::rc::params().image_cmap, "pink");
            crate::rc::reset();
            assert!(context_with(&bad, || Ok::<_, RcError>(())).is_err());
            assert_eq!(crate::rc::params().image_cmap, "viridis");
        });
    }

    #[test]
    fn style_files_by_path_and_url() {
        let dir = std::env::temp_dir().join(format!("plt-style-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("my style.mplstyle");
        std::fs::write(&path, "axes.facecolor: adeade\nfoo: bar\n").unwrap();
        let url = format!("file://{}", path.display()).replace(' ', "%20");
        for style in [Style::from(path.as_path()), Style::Named(url)] {
            let color = context_with(&[style], || {
                Ok::<_, RcError>(crate::rc::params().axes_facecolor.clone())
            });
            assert_eq!(color, Ok("#adeade".into()));
        }
        assert_eq!(
            style_file("file:/a/b.mplstyle"),
            PathBuf::from("/a/b.mplstyle")
        );
        assert_eq!(
            style_file("file://host/a%2Fb"),
            PathBuf::from("/a/b"),
            "the authority is dropped"
        );
        assert_eq!(style_file("file:/a%+1%4"), PathBuf::from("/a%+1%4"));
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(matches!(
            use_styles(&[Style::from(path.as_path())]),
            Err(RcError::Unreadable { .. })
        ));
    }

    #[test]
    fn aliases_match_their_styles() {
        let snapshot = |name: &str| {
            context(name, || Ok::<_, RcError>((*crate::rc::params()).clone())).unwrap()
        };
        assert_eq!(snapshot("mpl20"), snapshot("default"));
        assert_eq!(snapshot("mpl15"), snapshot("classic"));
    }

    #[test]
    fn the_blacklist_names_parameters() {
        // `backend` is pyplot's business in plt-rs, not an rc parameter;
        // style files may still name it, as they may in matplotlib.
        for key in STYLE_BLACKLIST.into_iter().filter(|&key| key != "backend") {
            assert!(RcParams::KEYS.contains(&key), "{key}");
        }
        crate::rc::context(|| {
            use_styles(&[Style::params(&[("backend", "agg")])]).expect("ignored");
        });
    }
}
