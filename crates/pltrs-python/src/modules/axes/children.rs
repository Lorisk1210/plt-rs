//! What an axes holds, and clearing it (matplotlib's `ax._children`,
//! `get_children` and `clear`).

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes::Axes;

use crate::binding::prelude::*;

/// The artists added to the axes, in the order they were added
/// (matplotlib's `ax._children`).
fn children(axes: Axes) -> PyResult<Vec<RawHandle>> {
    Ok(axes.added_children())
}

/// The artists of the axes that are no line, patch, collection, text,
/// image or table, in the order they were added (matplotlib's
/// `ax.artists`).
fn artists(axes: Axes) -> PyResult<Vec<RawHandle>> {
    Ok(axes.artists())
}

/// Removes everything drawn in the axes and resets it to a fresh axes at
/// the same place (matplotlib's `Axes.clear`); returns the artists it
/// removed: the added ones, the legend and the child axes.
fn clear(axes: Axes) -> PyResult<Vec<RawHandle>> {
    let removed: Vec<RawHandle> = axes
        .added_children()
        .into_iter()
        .chain(axes.current_legend().map(|legend| legend.raw().clone()))
        .chain(
            axes.child_axes()
                .into_iter()
                .map(|child| child.raw().clone()),
        )
        .collect();
    axes.clear();
    Ok(removed)
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("children", children)?;
    module.add("artists", artists)?;
    module.add("clear", clear)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn clearing_returns_what_it_removed() {
        let ax = Figure::new().subplot();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("equal lengths");
        assert_eq!(
            children(ax.clone()).expect("children"),
            [line.raw().clone()]
        );
        assert_eq!(clear(ax.clone()).expect("cleared"), [line.raw().clone()]);
        assert_eq!(children(ax).expect("children"), Vec::<RawHandle>::new());
    }
}
