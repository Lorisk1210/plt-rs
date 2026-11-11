//! Axes sharing an axis with axes of other figures (matplotlib's one
//! `Grouper` for the axes of every figure): pltrs (Rust) links them, and
//! Python wraps the handles of the other figures' axes in the objects of
//! their own figures.

use pltrs::axes::Axes;

use super::axis_name;
use crate::binding::prelude::*;

/// The axes of other figures sharing axis `name` with `axes`
/// (`get_shared_x_axes().get_siblings(ax)` lists them with those of its
/// own figure).
fn shared_across(axes: Axes, name: String) -> PyResult<Vec<Axes>> {
    Ok(axes.shared_across(axis_name(&name)?))
}

/// The axes of another figure whose axis `name` `axes` took (`_sharex`,
/// `_sharey`).
fn shared_from_across(axes: Axes, name: String) -> PyResult<Option<Axes>> {
    Ok(axes.shared_from_across(axis_name(&name)?))
}

/// Joins the groups sharing axis `name` of `axes` and `other` as they
/// are, `other` being the axes whose axis `axes` took with `follows`: for
/// the axes of the figures of a pickle, which come back with all they
/// shared.
fn join_across(axes: Axes, name: String, other: Axes, follows: bool) -> PyResult<()> {
    axes.join_across(axis_name(&name)?, &other, follows);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("shared_across", shared_across)?;
    module.add("shared_from_across", shared_from_across)?;
    module.add("join_across", join_across)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn axes_of_two_figures_share_by_axis_name() {
        let (first, second) = (Figure::new().subplot(), Figure::new().subplot());
        second.share_x(&first).unwrap();
        assert_eq!(
            shared_across(first.clone(), "x".into()).unwrap(),
            std::slice::from_ref(&second)
        );
        assert_eq!(
            shared_across(first.clone(), "y".into()).unwrap(),
            Vec::<Axes>::new()
        );
        assert_eq!(
            shared_from_across(second.clone(), "x".into()).unwrap(),
            Some(first.clone())
        );
        join_across(first.clone(), "y".into(), second.clone(), false).unwrap();
        second.set_ylim(2.0, 3.0).unwrap();
        assert_eq!(first.ylim(), (2.0, 3.0));
        assert!(join_across(first, "z".into(), second, false).is_err());
    }
}
