//! Runtime configuration from pyplot (matplotlib's `rc`, `rc_context`,
//! `rc_file`, `rcdefaults` and `xkcd`).

use std::path::Path;

use super::interactive::draw_if_interactive;
use crate::rc::{RcError, RcFile, RcParams};
use crate::style::{STYLE_BLACKLIST, XkcdError, XkcdSketch};

/// The short names matplotlib's `rc` accepts for parameter names.
const ALIASES: [(&str, &str); 8] = [
    ("lw", "linewidth"),
    ("ls", "linestyle"),
    ("c", "color"),
    ("fc", "facecolor"),
    ("ec", "edgecolor"),
    ("mew", "markeredgewidth"),
    ("aa", "antialiased"),
    ("sans", "sans-serif"),
];

/// Sets the parameters `group.name` from their rc-file text, with
/// matplotlib's short names (`lw`, `c`, `fc`, ...) for `name`
/// (matplotlib's `rc(group, **kwargs)`). Either every setting applies or
/// none does.
///
/// ```
/// use plt::pyplot as plt;
/// plt::rc_context(&[], || {
///     plt::rc("lines", &[("lw", "3"), ("color", "red")])?;
///     assert_eq!(::plt::rc::params().lines_linewidth, 3.0);
///     Ok::<(), ::plt::rc::RcError>(())
/// })?;
/// # Ok::<(), ::plt::rc::RcError>(())
/// ```
///
/// # Errors
///
/// When a parameter is unknown or a value invalid.
pub fn rc(group: &str, settings: &[(&str, &str)]) -> Result<(), RcError> {
    let mut updated = (*crate::rc::params()).clone();
    for (name, value) in settings {
        let name = ALIASES
            .iter()
            .find(|(alias, _)| alias == name)
            .map_or(*name, |(_, full)| full);
        updated.set(&format!("{group}.{name}"), value)?;
    }
    crate::rc::replace(updated);
    Ok(())
}

/// Runs `f` with the parameters changed by `settings` (full names and
/// rc-file text), restoring the previous ones afterwards (matplotlib's
/// `with rc_context(rc):`). `f` returns a `Result` whose error a refused
/// setting converts into, as for [`style::context`](crate::style::context).
///
/// # Errors
///
/// When a parameter is unknown or a value invalid, without running `f`,
/// and `f`'s own errors.
pub fn rc_context<T, E: From<RcError>>(
    settings: &[(&str, &str)],
    f: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    crate::rc::context(|| {
        for (key, value) in settings {
            crate::rc::set(key, value)?;
        }
        f()
    })
}

/// Resets the parameters to matplotlib's defaults updated by the rc file
/// at `path` (matplotlib's `rc_file`): the parameters of the style
/// blacklist ([`STYLE_BLACKLIST`]) keep their current values instead.
/// Invalid lines of the file are skipped, as matplotlib skips them with a
/// warning.
///
/// ```
/// use plt::pyplot as plt;
/// let path = std::env::temp_dir().join(format!("plt-rc-file-{}.rc", std::process::id()));
/// std::fs::write(&path, "lines.linewidth: 33\n")?;
/// plt::rc_context(&[("lines.color", "red")], || {
///     plt::rc_file(&path)?;
///     let params = ::plt::rc::params();
///     assert_eq!(params.lines_linewidth, 33.0);
///     assert_eq!(params.lines_color, ::plt::rc::RcParams::matplotlib().lines_color);
///     Ok::<(), ::plt::rc::RcError>(())
/// })?;
/// std::fs::remove_file(&path)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the file cannot be read, or has a double quote that is not closed.
pub fn rc_file(path: impl AsRef<Path>) -> Result<(), RcError> {
    let file = RcFile::read(path)?;
    let current = crate::rc::params();
    let mut params = RcParams::matplotlib();
    for key in STYLE_BLACKLIST {
        if let Some(value) = current.get(key) {
            params.set(key, &value)?;
        }
    }
    for entry in &file.entries {
        if !STYLE_BLACKLIST.contains(&entry.key.as_str()) {
            params.set(&entry.key, &entry.value)?;
        }
    }
    crate::rc::replace(params);
    Ok(())
}

/// Runs `f` with the parameters of the rc file at `path`, as [`rc_file`]
/// sets them, changed further by `settings` (full names and rc-file
/// text), restoring the previous parameters afterwards (matplotlib's
/// `with rc_context(rc, fname):`), as [`rc_context`].
///
/// # Errors
///
/// As [`rc_file`], and when a setting names an unknown parameter or has an
/// invalid value, without running `f`; and `f`'s own errors.
pub fn rc_file_context<T, E: From<RcError>>(
    path: impl AsRef<Path>,
    settings: &[(&str, &str)],
    f: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    crate::rc::context(|| {
        rc_file(path)?;
        for (key, value) in settings {
            crate::rc::set(key, value)?;
        }
        f()
    })
}

/// Restores matplotlib's default parameters (matplotlib's `rcdefaults`)
/// and, in interactive mode, redraws the shown figures.
pub fn rcdefaults() {
    crate::rc::reset();
    draw_if_interactive();
}

/// Runs `f` in matplotlib's xkcd sketch style and restores the previous
/// rc parameters afterwards (matplotlib's `with plt.xkcd(...):`): every
/// artist made inside wobbles as `sketch` says and is outlined in white,
/// in a 14 pt comic font (`DejaVu Sans` when no xkcd font is installed).
/// `f` returns a `Result` whose error an [`XkcdError`] converts into, as
/// for [`style::context`](crate::style::context).
///
/// ```
/// use plt::pyplot as plt;
/// use ::plt::style::XkcdSketch;
/// let png = plt::xkcd(XkcdSketch::default(), || -> Result<_, Box<dyn std::error::Error>> {
///     let fig = plt::figure();
///     plt::plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.5])?;
///     Ok(fig.to_png()?)
/// })?;
/// assert!(!png.is_empty());
/// assert_eq!(::plt::rc::params().path_sketch, None, "the style ends with the scope");
/// plt::close_all();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// [`XkcdError::Usetex`] when `text.usetex` is on, without running `f`,
/// and `f`'s own errors.
pub fn xkcd<T, E: From<XkcdError>>(
    sketch: XkcdSketch,
    f: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    crate::rc::context(|| {
        let mut rc = (*crate::rc::params()).clone();
        crate::style::apply_xkcd(&mut rc, sketch)?;
        crate::rc::replace(rc);
        f()
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::artist::ArtistHandle;
    use crate::lines::Line2D;
    use crate::pyplot::{close_all, plot};

    #[test]
    fn rc_sets_a_group_with_aliases_all_or_nothing() {
        crate::rc::context(|| {
            rc("lines", &[("lw", "4"), ("ls", "--")]).unwrap();
            assert_eq!(crate::rc::params().lines_linewidth, 4.0);
            let unknown = rc("lines", &[("lw", "1"), ("nonsense", "1")]);
            assert_eq!(unknown, Err(RcError::UnknownKey("lines.nonsense".into())));
            assert_eq!(crate::rc::params().lines_linewidth, 4.0, "nothing applied");
            rcdefaults();
            assert_eq!(crate::rc::params().lines_linewidth, 1.5);
        });
    }

    #[test]
    fn rc_context_restores_and_validates() {
        let width = rc_context(&[("lines.linewidth", "7")], || {
            Ok::<_, RcError>(crate::rc::params().lines_linewidth)
        });
        assert_eq!(width, Ok(7.0));
        assert_eq!(crate::rc::params().lines_linewidth, 1.5);
        let refused = rc_context(&[("lines.linewidth", "wide")], || Ok::<_, RcError>(()));
        assert!(refused.is_err());
    }

    #[test]
    fn xkcd_styles_what_is_made_in_its_scope() {
        close_all();
        let sketched = xkcd(XkcdSketch::default(), || -> Result<_, Box<dyn Error>> {
            Ok(plot(vec![0.0, 1.0], vec![0.0, 1.0])?)
        })
        .expect("usetex is off and the data is valid");
        let plain = plot(vec![0.0, 1.0], vec![1.0, 0.0]).expect("valid data");
        let sketch = |line: &Line2D| line.raw().with_base(|base| base.sketch);
        let effects = |line: &Line2D| line.raw().with_base(|base| base.path_effects.len());
        assert_eq!(sketch(&sketched).map(|s| s.length), Some(100.0));
        assert_eq!(effects(&sketched), 1, "the white outline");
        assert_eq!((sketch(&plain), effects(&plain)), (None, 0));
        close_all();
    }
}
