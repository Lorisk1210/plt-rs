//! Runtime configuration: matplotlib's `rcParams`, rc files and their values.
//!
//! [`RcParams`] holds every parameter as a typed field
//! (`params.lines_linewidth`; a parameter naming one of a few keywords
//! holds an enum, such as the [`TitleLocation`](crate::axes::TitleLocation)
//! of `params.axes_titlelocation`) and also offers matplotlib's
//! string-keyed access ([`RcParams::set`]`("lines.linewidth", "2")`) with
//! identical validation and spellings. The *current* parameters are per thread, like pyplot state;
//! threads start from [`RcParams::startup`], matplotlib's defaults with
//! the figure [look](crate::look)'s style on top and then the user's rc
//! file, `pltrc` (see [`config`](crate::config)):
//!
//! ```
//! use plt::rc;
//! let width = rc::params().lines_linewidth;
//! rc::context(|| {
//!     rc::set("lines.linewidth", "3").unwrap();
//!     assert_eq!(rc::params().lines_linewidth, 3.0);
//! });
//! assert_eq!(rc::params().lines_linewidth, width);
//! ```

#[macro_use]
mod macros;
mod cycler;
mod file;
mod keywords;
mod kinds;
mod literal;
mod table;
mod validate;
mod value;

use std::cell::RefCell;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

pub use keywords::UnknownName;
pub(crate) use keywords::keyword_names;
pub(crate) use literal::Expr;
pub(crate) use value::RcValue;

pub use cycler::{CycleValue, Cycler, CyclerError};
pub use file::{RcEntry, RcFile, RcWarning, RcWarningKind, parse_rc_text, strip_comment};
pub use kinds::{
    Aspect, AxisBelow, ColorOrAuto, ColorOrEdge, ColorOrInherit, Distiller, Dpi, FontType,
    HistBins, LabelColor, LegendLocParam, SavefigBbox, Whiskers,
};
pub use table::RcParams;

impl Default for RcParams {
    /// matplotlib's defaults (`rcParamsDefault`).
    fn default() -> Self {
        Self::matplotlib()
    }
}

impl RcParams {
    /// The parameters threads start from: matplotlib's defaults with the
    /// style of the current figure [look](crate::look) on top, and the
    /// settings of the user's rc file on top of that, as matplotlib's
    /// `matplotlibrc` applies over its defaults (see
    /// [`config`](crate::config); not while the user configuration is
    /// [ignored](crate::config::ignore_user_config)).
    pub fn startup() -> Self {
        crate::config::with_user_rc(crate::look::appearance().figures.params())
    }

    /// Applies every assignment of rc-file text (a style sheet), stopping at
    /// the first invalid line.
    pub fn apply_rc_text(&mut self, text: &str) -> Result<(), RcError> {
        for entry in parse_rc_text(text)? {
            self.set(&entry.key, &entry.value)
                .map_err(|err| RcError::AtLine {
                    line: entry.line,
                    source: Box::new(err),
                })?;
        }
        Ok(())
    }
}

/// An invalid parameter name, value or rc file.
#[derive(Clone, Debug, PartialEq)]
pub enum RcError {
    /// No such parameter.
    UnknownKey(String),
    /// The value failed validation.
    InvalidValue {
        /// Parameter name.
        key: String,
        /// Rejected text.
        value: String,
        /// Why it was rejected.
        reason: String,
    },
    /// A line of an rc file that is not `key: value`.
    Syntax {
        /// 1-based line number.
        line: usize,
        /// The offending line.
        text: String,
    },
    /// An error on a specific line of an rc file.
    AtLine {
        /// 1-based line number.
        line: usize,
        /// The underlying error.
        source: Box<RcError>,
    },
    /// No style sheet with this name, and no style file at this path.
    UnknownStyle(String),
    /// A double quote that is not closed, in this line.
    UnclosedQuote(String),
    /// An rc file or style file that cannot be read.
    Unreadable {
        /// The file.
        path: PathBuf,
        /// Why it cannot be read.
        reason: String,
    },
}

impl fmt::Display for RcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKey(key) => write!(f, "{key:?} is not a valid rc parameter"),
            Self::InvalidValue { key, value, reason } => {
                write!(f, "key {key:?}: {reason} (value {value:?})")
            }
            Self::Syntax { line, text } => write!(f, "line {line}: missing colon in {text:?}"),
            Self::AtLine { line, source } => write!(f, "line {line}: {source}"),
            Self::UnknownStyle(name) => write!(f, "{name:?} is not a known style"),
            Self::UnclosedQuote(text) => write!(f, "missing closing quote in {text:?}"),
            Self::Unreadable { path, reason } => {
                write!(f, "cannot read {}: {reason}", path.display())
            }
        }
    }
}

impl std::error::Error for RcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AtLine { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

thread_local! {
    static CURRENT: RefCell<Rc<RcParams>> = RefCell::new(Rc::new(RcParams::startup()));
}

/// The current parameters (a cheap shared snapshot).
pub fn params() -> Rc<RcParams> {
    CURRENT.with(|current| Rc::clone(&current.borrow()))
}

/// Sets one parameter from its rc-file text.
pub fn set(key: &str, value: &str) -> Result<(), RcError> {
    let mut updated = (*params()).clone();
    updated.set(key, value)?;
    replace(updated);
    Ok(())
}

/// Changes the current parameters with `f`, which works on a copy that
/// replaces them once it returns: meanwhile `f` may read the parameters,
/// as they were, and make figures.
pub fn update(f: impl FnOnce(&mut RcParams)) {
    let mut updated = (*params()).clone();
    f(&mut updated);
    replace(updated);
}

/// Replaces the current parameters, returning the previous ones.
pub fn replace(params: RcParams) -> Rc<RcParams> {
    CURRENT.with(|current| current.replace(Rc::new(params)))
}

/// Restores matplotlib's defaults (matplotlib's `rcdefaults()`).
pub fn reset() {
    replace(RcParams::default());
}

/// Restores the parameters threads start from, [`RcParams::startup`]: the
/// look's with the user's rc file on top (matplotlib's
/// `rc_file_defaults()`).
pub fn restore_startup() {
    replace(RcParams::startup());
}

/// Runs `f` and afterwards restores the parameters that were current
/// before it, even if `f` panics (matplotlib's `rc_context`).
pub fn context<R>(f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Rc<RcParams>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some(saved) = self.0.take() {
                CURRENT.with(|current| current.replace(saved));
            }
        }
    }
    let _restore = Restore(Some(params()));
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_matplotlib() {
        let rc = RcParams::matplotlib();
        // matplotlib's 338, then plt-rs' own `plt.*` parameters.
        let matplotlib_keys = RcParams::KEYS.iter().filter(|key| !key.starts_with("plt."));
        assert_eq!(matplotlib_keys.count(), 338);
        assert_eq!(rc.lines_linewidth, 1.5);
        assert_eq!(rc.figure_figsize, [6.4, 4.8]);
        assert_eq!(rc.font_size, 10.0);
        assert_eq!(rc.axes_prop_cycle.len(), 10);
        assert_eq!(rc.axes_formatter_limits, [-5, 6]);
        assert_eq!(rc.path_simplify_threshold, 0.111_111_111_111);
    }

    #[test]
    fn string_access_validates() {
        let mut rc = RcParams::matplotlib();
        rc.set("lines.linewidth", "2.5").expect("valid");
        assert_eq!(rc.get("lines.linewidth").as_deref(), Some("2.5"));
        assert!(matches!(
            rc.set("lines.linewidth", "thick"),
            Err(RcError::InvalidValue { .. })
        ));
        assert!(matches!(
            rc.set("lines.thickness", "2"),
            Err(RcError::UnknownKey(_))
        ));
        assert!(rc.set("image.origin", "middle").is_err());
        assert!(rc.set("axes.xmargin", "-0.6").is_err());
        assert!(rc.set("path.simplify_threshold", "1.5").is_err());
    }

    #[test]
    fn keywords_keep_matplotlib_spellings_and_messages() {
        let mut rc = RcParams::matplotlib();
        rc.set("ps.papersize", "A4").expect("valid");
        assert_eq!(rc.get("ps.papersize").as_deref(), Some("a4"));
        rc.set("toolbar", "NONE").expect("valid");
        assert_eq!(rc.toolbar, crate::gui::ToolbarKind::None);
        assert_eq!(rc.get("toolbar").as_deref(), Some("None"));
        rc.set("text.hinting", "either").expect("valid");
        assert_eq!(rc.get("text.hinting").as_deref(), Some("either"));
        rc.set("mathtext.fallback", "STIXSANS").expect("valid");
        assert_eq!(rc.get("mathtext.fallback").as_deref(), Some("stixsans"));
        assert!(rc.set("yaxis.labellocation", "left").is_err());
        assert!(rc.set("markers.fillstyle", "Full").is_err());
        let stage = rc
            .set("image.interpolation_stage", "antialiased")
            .unwrap_err();
        assert_eq!(
            stage.to_string(),
            r#"key "image.interpolation_stage": "antialiased" is not one of ["auto", "data", "rgba"] (value "antialiased")"#,
            "the stage parses more names than matplotlib's rc allows"
        );
    }

    #[test]
    fn every_default_round_trips() {
        let rc = RcParams::matplotlib();
        let mut copy = RcParams::matplotlib();
        for key in RcParams::KEYS {
            let text = rc.get(key).expect("known key");
            copy.set(key, &text)
                .unwrap_or_else(|err| panic!("{key}: {err}"));
        }
        assert_eq!(copy, rc);
    }

    #[test]
    fn context_restores_after_panic() {
        let result = std::panic::catch_unwind(|| {
            context(|| {
                update(|rc| rc.lines_linewidth = 9.0);
                panic!("boom");
            })
        });
        assert!(result.is_err());
        assert_eq!(params().lines_linewidth, 1.5);
    }

    #[test]
    fn rc_text_reports_lines() {
        let mut rc = RcParams::matplotlib();
        rc.apply_rc_text("lines.linewidth: 2\naxes.grid: True\n")
            .expect("valid");
        assert!(rc.axes_grid);
        let err = rc
            .apply_rc_text("lines.linewidth: 2\naxes.grid: perhaps\n")
            .unwrap_err();
        assert!(matches!(err, RcError::AtLine { line: 2, .. }));
    }
}
