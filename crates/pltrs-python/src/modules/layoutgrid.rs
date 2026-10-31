//! The layout grids constrained layout solved, as `_layoutgrid` mirrors
//! them (matplotlib's `layoutgrids`, which `ConstrainedLayoutEngine.execute`
//! returns): `_native.figure.execute_layout` hands each grid over as a
//! dict of plain values.

use pltrs::layout::{GridOwner, GridParent, Margin, MarginValues, SolvedGrid};

use crate::binding::new_dict;
use crate::binding::prelude::*;

impl IntoPy for SolvedGrid {
    /// `{"owner": ..., "parent": ..., "width_ratios": [...], ...,
    /// "margins": {"left": [...], ...}, "margin_vals": {...}}`: the owner
    /// is `("frame", artist id)`, `("gridspec", grid)` or `("nested",
    /// grid)`, the parent `("rect", (left, bottom, width, height))` or
    /// `("cells", grid index, (row0, row1), (col0, col1), inner)`.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let grid = new_dict(py)?;
        match self.owner {
            GridOwner::Frame(id) => grid.set_item("owner", ("frame", id.index()))?,
            GridOwner::GridSpec(spec) => grid.set_item("owner", ("gridspec", spec))?,
            GridOwner::Nested(spec) => grid.set_item("owner", ("nested", spec))?,
        }
        match self.parent {
            GridParent::Rect([left, bottom, width, height]) => {
                grid.set_item("parent", ("rect", (left, bottom, width, height)))?;
            }
            GridParent::Cells {
                grid: index,
                rows,
                cols,
                inner,
            } => grid.set_item(
                "parent",
                (
                    "cells",
                    index,
                    (rows.start, rows.end),
                    (cols.start, cols.end),
                    inner,
                ),
            )?,
        }
        grid.set_item("width_ratios", self.width_ratios)?;
        grid.set_item("height_ratios", self.height_ratios)?;
        grid.set_item("lefts", self.lefts)?;
        grid.set_item("rights", self.rights)?;
        grid.set_item("bottoms", self.bottoms)?;
        grid.set_item("tops", self.tops)?;
        grid.set_item("margins", margins(py, &self.margins)?)?;
        grid.set_item("margin_vals", margins(py, &self.margin_vals)?)?;
        Ok(grid)
    }
}

/// `{"left": [...], "right": [...], ...}`, matplotlib's keys.
fn margins<'py>(py: Python<'py>, values: &MarginValues) -> PyResult<Bound<'py>> {
    let dict = new_dict(py)?;
    for kind in Margin::ALL {
        dict.set_item(kind.name(), values.get(kind).to_vec())?;
    }
    Ok(dict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::dict_value;
    use crate::binding::testing::{extension, with_python};
    use pltrs::artist::ArtistHandle;
    use pltrs::figure::Figure;
    use pltrs::layout::LayoutEngine;

    #[test]
    fn grids_cross_as_dicts_of_plain_values() {
        with_python(|py| {
            extension(py);
            let fig = Figure::new();
            fig.subplots(1, 2).unwrap();
            let grids = fig.execute_layout(LayoutEngine::constrained()).unwrap();
            let figure = grids[0].clone().into_py(py).unwrap();
            let owner: (String, u32) = dict_value(&figure, "owner").unwrap();
            assert_eq!(owner, ("frame".to_owned(), fig.raw().id().index()));
            let parent: (String, (f64, f64, f64, f64)) = dict_value(&figure, "parent").unwrap();
            assert_eq!(parent, ("rect".to_owned(), (0.0, 0.0, 1.0, 1.0)));
            let spec = grids[1].clone().into_py(py).unwrap();
            let lefts: Vec<f64> = dict_value(&spec, "lefts").unwrap();
            assert_eq!(lefts.len(), 2);
            let parent: (String, usize, (usize, usize), (usize, usize), bool) =
                dict_value(&spec, "parent").unwrap();
            assert_eq!(parent, ("cells".to_owned(), 0, (0, 1), (0, 1), true));
            let margins: Bound<'_> = dict_value(&spec, "margins").unwrap();
            let left: Vec<f64> = dict_value(&margins, "left").unwrap();
            assert_eq!(left.len(), 2);
        });
    }
}
