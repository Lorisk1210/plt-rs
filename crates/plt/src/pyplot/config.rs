//! Runtime configuration from pyplot (matplotlib's `rc`, `rc_context`,
//! `rcdefaults` and `xkcd`).

use super::interactive::draw_if_interactive;
use crate::rc::RcError;
use crate::style::{XkcdError, XkcdSketch};

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
/// })??;
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
/// `with rc_context(rc):`).
///
/// # Errors
///
/// When a parameter is unknown or a value invalid; `f` does not run then.
pub fn rc_context<R>(settings: &[(&str, &str)], f: impl FnOnce() -> R) -> Result<R, RcError> {
    crate::rc::context(|| {
        for (key, value) in settings {
            crate::rc::set(key, value)?;
        }
        Ok(f())
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
///
/// ```
/// use plt::pyplot as plt;
/// use ::plt::style::XkcdSketch;
/// let png = plt::xkcd(XkcdSketch::default(), || -> Result<Vec<u8>, Box<dyn std::error::Error>> {
///     let fig = plt::figure();
///     plt::plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.5])?;
///     Ok(fig.to_png()?)
/// })??;
/// assert!(!png.is_empty());
/// assert_eq!(::plt::rc::params().path_sketch, None, "the style ends with the scope");
/// plt::close_all();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// [`XkcdError::Usetex`] when `text.usetex` is on.
pub fn xkcd<R>(sketch: XkcdSketch, f: impl FnOnce() -> R) -> Result<R, XkcdError> {
    crate::rc::context(|| {
        let mut rc = (*crate::rc::params()).clone();
        crate::style::apply_xkcd(&mut rc, sketch)?;
        crate::rc::replace(rc);
        Ok(f())
    })
}

#[cfg(test)]
mod tests {
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
            crate::rc::params().lines_linewidth
        });
        assert_eq!(width, Ok(7.0));
        assert_eq!(crate::rc::params().lines_linewidth, 1.5);
        assert!(rc_context(&[("lines.linewidth", "wide")], || ()).is_err());
    }

    #[test]
    fn xkcd_styles_what_is_made_in_its_scope() {
        close_all();
        let sketched = xkcd(XkcdSketch::default(), || {
            plot(vec![0.0, 1.0], vec![0.0, 1.0])
        })
        .expect("usetex is off")
        .expect("valid data");
        let plain = plot(vec![0.0, 1.0], vec![1.0, 0.0]).expect("valid data");
        let sketch = |line: &Line2D| line.raw().with_base(|base| base.sketch);
        let effects = |line: &Line2D| line.raw().with_base(|base| base.path_effects.len());
        assert_eq!(sketch(&sketched).map(|s| s.length), Some(100.0));
        assert_eq!(effects(&sketched), 1, "the white outline");
        assert_eq!((sketch(&plain), effects(&plain)), (None, 0));
        close_all();
    }
}
