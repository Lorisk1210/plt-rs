//! `_native.gridspec`: grids of subplots (matplotlib's `gridspec`), by
//! pltrs (Rust)'s `gridspec` module.
//!
//! A Python `GridSpec` holds one native grid for its whole life (pltrs
//! (Rust) and the layout engines tell grids apart by identity), and a
//! `SubplotSpec` the native block of cells it selects. Margins and
//! spacings cross as six optional floats `(left, bottom, right, top,
//! wspace, hspace)`, positions as `(x0, y0, x1, y1)`.

use pltrs::axes::Axes;
use pltrs::figure::Figure;
use pltrs::gridspec::{GridSpec, SubplotOverrides, SubplotParams, SubplotSpec};

use crate::binding::prelude::*;

native_value!(GridSpec, SubplotSpec);

/// `(left, bottom, right, top, wspace, hspace)`, each optional.
pub(super) type Overrides = (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

/// `(left, bottom, right, top, wspace, hspace)`.
type Params = (f64, f64, f64, f64, f64, f64);

fn overrides_of((left, bottom, right, top, wspace, hspace): Overrides) -> SubplotOverrides {
    SubplotOverrides {
        left,
        bottom,
        right,
        top,
        wspace,
        hspace,
    }
}

fn params_tuple(p: SubplotParams) -> Params {
    (p.left, p.bottom, p.right, p.top, p.wspace, p.hspace)
}

fn params_of((left, bottom, right, top, wspace, hspace): Params) -> SubplotParams {
    SubplotParams {
        left,
        bottom,
        right,
        top,
        wspace,
        hspace,
    }
}

/// A figure-level grid (`GridSpec(nrows, ncols, ...)`).
fn new(
    nrows: usize,
    ncols: usize,
    width_ratios: Option<Vec<f64>>,
    height_ratios: Option<Vec<f64>>,
    overrides: Overrides,
) -> PyResult<GridSpec> {
    Ok(GridSpec::with_ratios(
        nrows,
        ncols,
        width_ratios,
        height_ratios,
        overrides_of(overrides),
    )?)
}

/// A grid subdividing the block `parent` (`GridSpecFromSubplotSpec`).
fn nested(
    parent: SubplotSpec,
    (nrows, ncols): (usize, usize),
    width_ratios: Option<Vec<f64>>,
    height_ratios: Option<Vec<f64>>,
    spacing: (Option<f64>, Option<f64>),
) -> PyResult<GridSpec> {
    Ok(GridSpec::nested(
        &parent,
        nrows,
        ncols,
        width_ratios,
        height_ratios,
        spacing,
    )?)
}

/// `(nrows, ncols)`.
fn shape(grid: GridSpec) -> PyResult<(usize, usize)> {
    Ok(grid.shape())
}

/// `(width_ratios, height_ratios)`.
fn ratios(grid: GridSpec) -> PyResult<(Vec<f64>, Vec<f64>)> {
    Ok((grid.width_ratios().to_vec(), grid.height_ratios().to_vec()))
}

/// The margins and spacings the grid sets itself.
fn overrides(grid: GridSpec) -> PyResult<Overrides> {
    let o = grid.overrides();
    Ok((o.left, o.bottom, o.right, o.top, o.wspace, o.hspace))
}

/// Replaces the margins and spacings the grid sets itself.
fn set_overrides(grid: GridSpec, overrides: Overrides) -> PyResult<()> {
    grid.set_overrides(overrides_of(overrides));
    Ok(())
}

/// Whether the grid sets non-zero margins or spacings of its own.
fn locally_modified(grid: GridSpec) -> PyResult<bool> {
    Ok(grid.locally_modified())
}

/// Sets whether constrained layout places the axes on the grid (not for
/// a grid made without a figure).
fn set_in_layout(grid: GridSpec, in_layout: bool) -> PyResult<()> {
    grid.set_in_layout(in_layout);
    Ok(())
}

/// `repr` of the grid: `GridSpec(2, 2, height_ratios=(3, 1))`.
fn repr(grid: GridSpec) -> PyResult<String> {
    Ok(grid.to_string())
}

/// Whether two native grids are the same grid.
fn same(grid: GridSpec, other: GridSpec) -> PyResult<bool> {
    Ok(grid.is_same(&other))
}

/// The block a nested grid subdivides, `None` for a figure-level grid.
fn parent(grid: GridSpec) -> PyResult<Option<SubplotSpec>> {
    Ok(grid.parent().cloned())
}

/// The margins and spacings in effect for the grid in a figure whose
/// parameters are `figure` (`get_subplot_params`).
fn subplot_params(grid: GridSpec, figure: Params) -> PyResult<Params> {
    Ok(params_tuple(grid.subplot_params(&params_of(figure))?))
}

/// The cell edges `(bottoms, tops, lefts, rights)` in a figure whose
/// parameters are `figure` (`get_grid_positions`).
fn grid_positions(
    grid: GridSpec,
    figure: Params,
) -> PyResult<(NdArray, NdArray, NdArray, NdArray)> {
    let p = grid.grid_positions(&params_of(figure))?;
    Ok((
        NdArray::from_vec(p.bottoms),
        NdArray::from_vec(p.tops),
        NdArray::from_vec(p.lefts),
        NdArray::from_vec(p.rights),
    ))
}

/// The block covering rows `rows.0..rows.1` and columns `cols.0..cols.1`.
fn span(grid: GridSpec, rows: (usize, usize), cols: (usize, usize)) -> PyResult<SubplotSpec> {
    Ok(grid.span(rows.0..rows.1, cols.0..cols.1)?)
}

/// The grid of a block.
fn spec_gridspec(spec: SubplotSpec) -> PyResult<GridSpec> {
    Ok(spec.gridspec().clone())
}

/// `((row_start, row_stop), (col_start, col_stop))` of a block.
fn spec_cells(spec: SubplotSpec) -> PyResult<((usize, usize), (usize, usize))> {
    let (rows, cols) = (spec.rows(), spec.cols());
    Ok(((rows.start, rows.end), (cols.start, cols.end)))
}

/// The block of a figure-level grid a block lies in.
fn spec_topmost(spec: SubplotSpec) -> PyResult<SubplotSpec> {
    Ok(spec.topmost_subplotspec())
}

/// The position `(x0, y0, x1, y1)` of a block in a figure whose
/// parameters are `figure`.
fn spec_position(spec: SubplotSpec, figure: Params) -> PyResult<(f64, f64, f64, f64)> {
    Ok(spec.position(&params_of(figure))?.extents())
}

/// The subplot parameters of a figure (or subfigure).
fn figure_params(figure: Figure) -> PyResult<Params> {
    Ok(params_tuple(figure.subplot_params()))
}

/// Moves the axes of `figure` placed on `grid` after its margins changed
/// (`GridSpec.update` for the figures pyplot manages).
fn update(grid: GridSpec, figure: Figure, changes: Overrides) -> PyResult<()> {
    grid.update(&figure, overrides_of(changes))?;
    Ok(())
}

/// Axes on the block `spec` of `figure` (`fig.add_subplot(gs[...])`).
fn add_subplot(figure: Figure, spec: SubplotSpec) -> PyResult<Axes> {
    Ok(figure.add_subplot_spec(&spec)?)
}

/// The block an axes is placed on, if any.
fn axes_spec(axes: Axes) -> PyResult<Option<SubplotSpec>> {
    Ok(axes.subplot_spec())
}

/// Places an axes on a block.
fn set_axes_spec(axes: Axes, spec: SubplotSpec) -> PyResult<()> {
    axes.set_subplot_spec(spec)?;
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new", new)?;
    module.add("nested", nested)?;
    module.add("shape", shape)?;
    module.add("ratios", ratios)?;
    module.add("overrides", overrides)?;
    module.add("set_overrides", set_overrides)?;
    module.add("locally_modified", locally_modified)?;
    module.add("set_in_layout", set_in_layout)?;
    module.add("repr", repr)?;
    module.add("same", same)?;
    module.add("parent", parent)?;
    module.add("subplot_params", subplot_params)?;
    module.add("grid_positions", grid_positions)?;
    module.add("span", span)?;
    module.add("spec_gridspec", spec_gridspec)?;
    module.add("spec_cells", spec_cells)?;
    module.add("spec_topmost", spec_topmost)?;
    module.add("spec_position", spec_position)?;
    module.add("figure_params", figure_params)?;
    module.add("update", update)?;
    module.add("add_subplot", add_subplot)?;
    module.add("axes_spec", axes_spec)?;
    module.add("set_axes_spec", set_axes_spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_cross_as_cell_ranges_and_extents() {
        let grid = new(3, 3, None, None, (None, None, None, None, None, None)).unwrap();
        let block = span(grid.clone(), (2, 3), (1, 3)).unwrap();
        assert_eq!(spec_cells(block.clone()).unwrap(), ((2, 3), (1, 3)));
        assert!(same(spec_gridspec(block.clone()).unwrap(), grid.clone()).unwrap());
        assert_eq!(block.to_string(), "GridSpec(3, 3)[2:3, 1:3]");
        let params = (0.125, 0.11, 0.9, 0.88, 0.2, 0.2);
        let (x0, _, x1, y1) = spec_position(block, params).unwrap();
        assert!(x0 > 0.125 && (x1 - 0.9).abs() < 1e-12 && y1 < 0.88);
        set_overrides(grid.clone(), (Some(0.2), None, None, None, None, None)).unwrap();
        assert_eq!(overrides(grid).unwrap().0, Some(0.2));
    }
}
