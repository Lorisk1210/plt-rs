//! Style sheets: named sets of rcParams that restyle every plot
//! (`matplotlib.style`).
//!
//! The [`library`] bundles matplotlib's built-in sheets, applied with
//! [`use_style`] (matplotlib's `style.use`) or for a scope with
//! [`context`] (`style.context`):
//!
//! ```
//! use plt::{rc, style};
//! style::context("ggplot", || {
//!     assert!(rc::params().axes_grid);
//! })?;
//! assert!(!rc::params().axes_grid);
//! # Ok::<(), plt::rc::RcError>(())
//! ```
//!
//! [`apply_xkcd`] sets the parameters of matplotlib's xkcd sketch style;
//! [`pyplot::xkcd`](crate::pyplot::xkcd) applies them for a scope.
//!
//! The sheets in `style/sheets/` are copied verbatim from matplotlib 3.11
//! (`mpl-data/stylelib`), copyright the Matplotlib Development Team and
//! distributed under the Matplotlib license; the comments of several
//! sheets credit the projects their palettes come from (seaborn,
//! fivethirtyeight, Tableau, Solarized, Petroff's accessible colour
//! sequences, ...).

mod xkcd;

pub use xkcd::{XkcdError, XkcdSketch, apply_xkcd};

use crate::rc::{RcEntry, RcError, RcParams, parse_rc_text};

/// A bundled style sheet: an entry of matplotlib's `style.library`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StyleSheet {
    name: &'static str,
    text: &'static str,
}

/// rcParams that are not about style, which [`use_style`] ignores
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

/// Declares the bundled sheets from `sheets/<name>.mplstyle`.
macro_rules! sheets {
    ($($name:literal),* $(,)?) => {
        /// Every bundled sheet, sorted by name.
        const LIBRARY: &[StyleSheet] = &[
            $(StyleSheet {
                name: $name,
                text: include_str!(concat!("sheets/", $name, ".mplstyle")),
            },)*
        ];
    };
}

sheets![
    "Solarize_Light2",
    "bmh",
    "classic",
    "dark_background",
    "fast",
    "fivethirtyeight",
    "ggplot",
    "grayscale",
    "petroff10",
    "petroff6",
    "petroff8",
    "seaborn-v0_8",
    "seaborn-v0_8-bright",
    "seaborn-v0_8-colorblind",
    "seaborn-v0_8-dark",
    "seaborn-v0_8-dark-palette",
    "seaborn-v0_8-darkgrid",
    "seaborn-v0_8-deep",
    "seaborn-v0_8-muted",
    "seaborn-v0_8-notebook",
    "seaborn-v0_8-paper",
    "seaborn-v0_8-pastel",
    "seaborn-v0_8-poster",
    "seaborn-v0_8-talk",
    "seaborn-v0_8-ticks",
    "seaborn-v0_8-white",
    "seaborn-v0_8-whitegrid",
    "tableau-colorblind10",
];

impl StyleSheet {
    /// The sheet's name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The sheet's rc-file text.
    pub fn text(&self) -> &'static str {
        self.text
    }

    /// The sheet's settings in file order (matplotlib's
    /// `rc_params_from_file(..., use_default_template=False)`).
    pub fn entries(&self) -> Result<Vec<RcEntry>, RcError> {
        parse_rc_text(self.text)
    }

    /// Applies the sheet's settings to `rc`, all of them, as matplotlib's
    /// `rcParams.update(library[name])` does.
    pub fn apply(&self, rc: &mut RcParams) -> Result<(), RcError> {
        rc.apply_rc_text(self.text)
    }
}

/// Every bundled sheet, sorted by name (matplotlib's `style.library`).
pub fn library() -> &'static [StyleSheet] {
    LIBRARY
}

/// The bundled sheet called `name`.
pub fn sheet(name: &str) -> Option<StyleSheet> {
    LIBRARY.iter().copied().find(|sheet| sheet.name == name)
}

/// The names of the public sheets (matplotlib's `style.available`).
pub fn available() -> Vec<&'static str> {
    LIBRARY
        .iter()
        .map(|sheet| sheet.name)
        .filter(|name| !name.starts_with('_'))
        .collect()
}

/// Applies a style to the current parameters (matplotlib's `style.use`):
/// a sheet of the [`library`], `"default"` (or `"mpl20"`) for matplotlib's
/// defaults, or `"mpl15"` for `"classic"`. Parameters of the
/// [`STYLE_BLACKLIST`] are left alone.
pub fn use_style(name: &str) -> Result<(), RcError> {
    let name = match name {
        "mpl20" => "default",
        "mpl15" => "classic",
        other => other,
    };
    let entries: Vec<(String, String)> = if name == "default" {
        let defaults = RcParams::matplotlib();
        RcParams::KEYS
            .iter()
            .filter_map(|&key| Some((key.to_owned(), defaults.get(key)?)))
            .collect()
    } else {
        sheet(name)
            .ok_or_else(|| RcError::UnknownStyle(name.to_owned()))?
            .entries()?
            .into_iter()
            .map(|entry| (entry.key, entry.value))
            .collect()
    };
    let mut updated = (*crate::rc::params()).clone();
    for (key, value) in entries {
        if !STYLE_BLACKLIST.contains(&key.as_str()) {
            updated.set(&key, &value)?;
        }
    }
    crate::rc::replace(updated);
    Ok(())
}

/// Runs `f` with a style applied and restores the parameters afterwards
/// (matplotlib's `style.context`).
pub fn context<R>(name: &str, f: impl FnOnce() -> R) -> Result<R, RcError> {
    crate::rc::context(|| {
        use_style(name)?;
        Ok(f())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_sheet_is_valid() {
        for sheet in library() {
            let mut rc = RcParams::matplotlib();
            sheet
                .apply(&mut rc)
                .unwrap_or_else(|err| panic!("{}: {err}", sheet.name()));
            assert!(!sheet.entries().unwrap().is_empty(), "{}", sheet.name());
        }
        assert_eq!(available().len(), 28);
        let mut names: Vec<&str> = available();
        names.sort_unstable();
        assert_eq!(names, available(), "the library is sorted");
    }

    #[test]
    fn sheets_change_only_what_they_list() {
        let mut rc = RcParams::matplotlib();
        sheet("ggplot").unwrap().apply(&mut rc).unwrap();
        assert!(rc.axes_grid);
        assert_eq!(rc.patch_linewidth, 0.5);
        assert_eq!(rc.lines_linewidth, 1.5, "not in the sheet");
    }

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
}
