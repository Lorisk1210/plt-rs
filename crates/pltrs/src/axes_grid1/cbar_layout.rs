//! The columns, rows and cells of an [`ImageGrid`]'s divider: axes,
//! paddings and colorbars (matplotlib's `ImageGrid._init_locators`).

use super::image_grid::{CbarMode, ImageGrid, ImageGridOptions, Layout, relative_extent};
use super::size::{Size, SizeSpec};
use crate::artist::{ArtistHandle, ArtistId};
use crate::axes::Axes;
use crate::spines::SpineSide;

/// A block of divider cells: `(start, end)` columns or rows, Python-style
/// indices.
type Span = (isize, isize);

/// What the divider of an image grid holds and where each axes goes.
pub(super) struct Plan {
    /// Column widths, left to right.
    pub(super) horizontal: Vec<Size>,
    /// Row heights, bottom to top.
    pub(super) vertical: Vec<Size>,
    /// The cells of the axes and of the colorbar axes in use.
    pub(super) cells: Vec<(ArtistId, Span, Span)>,
    /// Whether each colorbar axes is shown.
    pub(super) cbar_visible: Vec<bool>,
}

/// The single cell starting at `index`.
fn single(index: usize) -> Span {
    let index = isize::try_from(index).unwrap_or(isize::MAX);
    (index, index + 1)
}

/// The divider indices of the axes columns (rows) and of the colorbar
/// columns (rows).
#[derive(Default)]
struct Positions {
    axes: Vec<usize>,
    cbars: Vec<usize>,
}

/// The grid and options a plan is made from.
struct Planner<'a> {
    axes: &'a [Axes],
    cbars: Vec<ArtistId>,
    layout: &'a Layout,
    options: &'a ImageGridOptions,
    /// The colorbar padding.
    cbar_pad: SizeSpec,
    /// The lower-left axes (matplotlib's `axes_llc`).
    llc: Axes,
}

/// The sizes, cells and colorbar visibility of `grid`, whose axes are
/// `rows` (top to bottom).
pub(super) fn plan(
    grid: &ImageGrid,
    layout: &Layout,
    rows: &[Vec<Axes>],
    options: &ImageGridOptions,
) -> Plan {
    let axes = grid.axes();
    let (h_pad, v_pad) = options.axes_pad;
    let beside = matches!(options.cbar_location, SpineSide::Left | SpineSide::Right);
    let first_row = rows.first().cloned().unwrap_or_default();
    let first_column: Vec<Axes> = rows.iter().filter_map(|row| row.first().cloned()).collect();
    let planner = Planner {
        axes,
        cbars: grid.cbar_axes().iter().map(|c| c.ax().id()).collect(),
        layout,
        options,
        cbar_pad: options
            .cbar_pad
            .unwrap_or(SizeSpec::Inches(if beside { h_pad } else { v_pad })),
        llc: grid.axes_llc().clone(),
    };
    let mut plan = Plan {
        horizontal: Vec::new(),
        vertical: Vec::new(),
        cells: Vec::new(),
        cbar_visible: vec![false; planner.cbars.len()],
    };
    let single_mode = options.cbar_mode == Some(CbarMode::Single);
    let mut single_cell = single_mode
        .then(|| planner.single_before(&mut plan))
        .flatten();
    let columns = planner.lines(&mut plan.horizontal, &first_row, true);
    let rows = planner.lines(&mut plan.vertical, &first_column, false);
    planner.place_axes(&mut plan, &columns, &rows);
    if single_mode {
        single_cell = single_cell.or_else(|| planner.single_after(&mut plan));
    }
    planner.show_cbars(&mut plan, single_cell);
    plan
}

impl Planner<'_> {
    /// `count` times the data width (or height) of the lower-left axes.
    fn span_size(&self, count: usize, horizontal: bool) -> Size {
        let extent = if horizontal {
            Size::axes_x(self.llc.id())
        } else {
            Size::axes_y(self.llc.id())
        };
        extent.times(count as f64)
    }

    /// Whether the line (column or row) `index` of `last + 1` gets a
    /// colorbar on `side`.
    fn wants_bar(&self, index: usize, last: usize, side: SpineSide) -> bool {
        let at = if matches!(side, SpineSide::Left | SpineSide::Bottom) {
            0
        } else {
            last
        };
        self.options.cbar_location == side
            && match self.options.cbar_mode {
                Some(CbarMode::Each) => true,
                Some(CbarMode::Edge) => index == at,
                _ => false,
            }
    }

    /// The colorbar of a single-mode grid on the left or at the bottom,
    /// before the axes.
    fn single_before(&self, plan: &mut Plan) -> Option<(Span, Span)> {
        let (sizes, horizontal, count, cell) = match self.options.cbar_location {
            SpineSide::Left => (
                &mut plan.horizontal,
                true,
                self.layout.nrows,
                ((0, 1), (0, -1)),
            ),
            SpineSide::Bottom => (
                &mut plan.vertical,
                false,
                self.layout.ncols,
                ((0, -1), (0, 1)),
            ),
            SpineSide::Right | SpineSide::Top => return None,
        };
        let sz = self.span_size(count, horizontal);
        sizes.push(self.options.cbar_size.resolve(&sz));
        sizes.push(self.cbar_pad.resolve(&sz));
        Some(cell)
    }

    /// The colorbar of a single-mode grid on the right or at the top,
    /// after the axes.
    fn single_after(&self, plan: &mut Plan) -> Option<(Span, Span)> {
        let (sizes, horizontal, count, cell) = match self.options.cbar_location {
            SpineSide::Right => (
                &mut plan.horizontal,
                true,
                self.layout.nrows,
                ((-2, -1), (0, -1)),
            ),
            SpineSide::Top => (
                &mut plan.vertical,
                false,
                self.layout.ncols,
                ((0, -1), (-2, -1)),
            ),
            SpineSide::Left | SpineSide::Bottom => return None,
        };
        let sz = self.span_size(count, horizontal);
        sizes.push(self.cbar_pad.resolve(&sz));
        sizes.push(self.options.cbar_size.resolve(&sz));
        Some(cell)
    }

    /// The columns (`horizontal`) or rows of the axes in `line` (left to
    /// right, or top to bottom), with paddings and colorbars between them.
    fn lines(&self, sizes: &mut Vec<Size>, line: &[Axes], horizontal: bool) -> Positions {
        let (pad, before, after, count) = if horizontal {
            let (pad, _) = self.options.axes_pad;
            (pad, SpineSide::Left, SpineSide::Right, self.layout.ncols)
        } else {
            let (_, pad) = self.options.axes_pad;
            (pad, SpineSide::Bottom, SpineSide::Top, self.layout.nrows)
        };
        let ordered: Vec<&Axes> = if horizontal {
            line.iter().collect()
        } else {
            line.iter().rev().collect()
        };
        let mut positions = Positions::default();
        for (index, ax) in ordered.into_iter().enumerate() {
            if index != 0 {
                sizes.push(Size::Fixed(pad));
            }
            // A plain grid, without colorbar axes, gives every column and
            // row the same share (matplotlib's `Grid._init_locators`).
            let sz = if self.cbars.is_empty() {
                Size::Scaled(1.0)
            } else {
                relative_extent(ax, &self.axes[0], horizontal)
            };
            if self.wants_bar(index, count - 1, before) {
                positions.cbars.push(sizes.len());
                sizes.push(self.options.cbar_size.resolve(&sz));
                sizes.push(self.cbar_pad.resolve(&sz));
            }
            positions.axes.push(sizes.len());
            sizes.push(sz.clone());
            if self.wants_bar(index, count - 1, after) {
                sizes.push(self.cbar_pad.resolve(&sz));
                positions.cbars.push(sizes.len());
                sizes.push(self.options.cbar_size.resolve(&sz));
            }
        }
        positions
    }

    /// The cells of the axes, and of their colorbars in the `each` and
    /// `edge` modes.
    fn place_axes(&self, plan: &mut Plan, columns: &Positions, rows: &Positions) {
        let beside = matches!(
            self.options.cbar_location,
            SpineSide::Left | SpineSide::Right
        );
        for (i, ax) in self.axes.iter().enumerate() {
            let (col, row) = self.layout.col_row(i);
            let below = self.layout.nrows - 1 - row;
            let (Some(&h_ax), Some(&v_ax)) = (columns.axes.get(col), rows.axes.get(below)) else {
                continue;
            };
            plan.cells.push((ax.id(), single(h_ax), single(v_ax)));
            let cbar_cell = match self.options.cbar_mode {
                Some(CbarMode::Each) if beside => columns
                    .cbars
                    .get(col)
                    .map(|&h| (i, (single(h), single(v_ax)))),
                Some(CbarMode::Each) => rows
                    .cbars
                    .get(below)
                    .map(|&v| (i, (single(h_ax), single(v)))),
                Some(CbarMode::Edge) => self.edge_cell((col, row), (h_ax, v_ax), columns, rows),
                _ => None,
            };
            if let Some((index, (x, y))) = cbar_cell
                && let Some(&cbar) = self.cbars.get(index)
            {
                plan.cells.push((cbar, x, y));
            }
        }
    }

    /// The colorbar axes (by index) and cell of an edge-mode grid for the
    /// axes at `(col, row)` in cells `(h_ax, v_ax)`, if it is on the edge.
    fn edge_cell(
        &self,
        (col, row): (usize, usize),
        (h_ax, v_ax): (usize, usize),
        columns: &Positions,
        rows: &Positions,
    ) -> Option<(usize, (Span, Span))> {
        let layout = self.layout;
        match self.options.cbar_location {
            SpineSide::Left if col == 0 => {
                Some((row, (single(*columns.cbars.first()?), single(v_ax))))
            }
            SpineSide::Right if col == layout.ncols - 1 => {
                Some((row, (single(*columns.cbars.first()?), single(v_ax))))
            }
            SpineSide::Bottom if row == layout.nrows - 1 => {
                Some((col, (single(h_ax), single(*rows.cbars.first()?))))
            }
            SpineSide::Top if row == 0 => Some((col, (single(h_ax), single(*rows.cbars.first()?)))),
            _ => None,
        }
    }

    /// Which colorbar axes are shown, and the cell of a single colorbar.
    fn show_cbars(&self, plan: &mut Plan, single_cell: Option<(Span, Span)>) {
        match self.options.cbar_mode {
            Some(CbarMode::Single) => {
                if let (Some((x, y)), Some(&first)) = (single_cell, self.cbars.first()) {
                    plan.cells.push((first, x, y));
                    plan.cbar_visible[0] = true;
                }
            }
            Some(CbarMode::Each) => plan.cbar_visible.fill(true),
            Some(CbarMode::Edge) => {
                let beside = matches!(
                    self.options.cbar_location,
                    SpineSide::Left | SpineSide::Right
                );
                let count = if beside {
                    self.layout.nrows
                } else {
                    self.layout.ncols
                };
                for visible in plan.cbar_visible.iter_mut().take(count) {
                    *visible = true;
                }
            }
            None => {}
        }
    }
}
