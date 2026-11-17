//! Placing the cells of a table (matplotlib's `Table._update_positions`).
//!
//! matplotlib measures the grid of cells in display pixels and maps the
//! box back to axes fractions through the inverse of `transAxes` before
//! it moves the cells; the layout follows the same round trip, so that
//! the cells land on matplotlib's bits.

use std::collections::BTreeMap;

use pltrs_core::{Affine2D, Bbox, Point};

use super::cell::PlacedCell;
use super::{TableArtist, TableLoc};
use crate::artist::LayoutContext;
use crate::text::TextDrawError;
use crate::transforms::{ResolvedTransform, Transform, unit_to_bbox};

/// The gap between the axes and a table placed inside them, in axes
/// fractions.
const AXES_PAD: f64 = 0.02;

/// The largest font size the automatic sizing starts from. matplotlib
/// steps down a point at a time from the cells' size: without end from
/// sizes so large that a point less is the same size, or infinite ones.
const MAX_AUTO_FONTSIZE: f64 = 1000.0;

/// The maps between the axes fractions of a table's axes and display
/// pixels (matplotlib's `transAxes` and its inverse).
#[derive(Clone)]
pub(crate) struct AxesMaps {
    /// Axes fractions to display pixels.
    pub(crate) to_display: Affine2D,
    /// Display pixels to axes fractions, as numpy inverts `transAxes`.
    pub(crate) to_axes: Affine2D,
}

impl AxesMaps {
    /// The maps of an axes at `axes` in display pixels; `None` for an axes
    /// of no area, whose fractions cannot be found again.
    pub(crate) fn of(axes: &Bbox) -> Option<Self> {
        let to_display = unit_to_bbox(axes);
        Some(Self {
            to_display,
            to_axes: to_display.inverted()?,
        })
    }

    /// The maps of the coordinates of `transform` in the layout `layout`;
    /// `None` for a transform that cannot be resolved, is not affine or
    /// not invertible.
    pub(crate) fn through(layout: LayoutContext<'_>, transform: &Transform) -> Option<Self> {
        let resolved = layout
            .resolve(transform)
            .filter(ResolvedTransform::is_affine)?;
        let to_display = resolved.affine_part();
        Some(Self {
            to_display,
            to_axes: to_display.inverted()?,
        })
    }
}

/// The size of every cell while the table is laid out.
struct Sizes {
    width: Vec<f64>,
    height: Vec<f64>,
    fontsize: Vec<f64>,
}

impl TableArtist {
    /// Every cell where it is now: where the table places it
    /// ([`place`](Self::place)) once the table placed its cells, and
    /// until then where it was made, at the origin with its own size and
    /// font size, as a cell of matplotlib's measured or drawn alone before
    /// its table was (`Table._update_positions` runs when the table is
    /// drawn or measured).
    pub(crate) fn place_as_is(
        &self,
        layout: LayoutContext<'_>,
        maps: &AxesMaps,
    ) -> Result<Vec<PlacedCell>, TextDrawError> {
        if self.positioned.get() {
            return self.place(layout, maps);
        }
        Ok((self.cells.iter())
            .map(|(_, cell)| PlacedCell {
                bbox: Bbox::from_bounds(0.0, 0.0, cell.width.get(), cell.height.get()),
                size: (cell.width.get(), cell.height.get()),
                fontsize: cell.fontsize(),
            })
            .collect())
    }

    /// Every cell with its place and font size for a table in the axes
    /// `maps` give.
    pub(crate) fn place(
        &self,
        layout: LayoutContext<'_>,
        maps: &AxesMaps,
    ) -> Result<Vec<PlacedCell>, TextDrawError> {
        let cells = &self.cells;
        let mut sizes = Sizes {
            width: cells.iter().map(|(_, c)| c.width.get()).collect(),
            height: cells.iter().map(|(_, c)| c.height.get()).collect(),
            fontsize: cells.iter().map(|(_, c)| c.fontsize()).collect(),
        };
        let cell_maps = self.cell_maps(layout, maps);
        let required = |i: usize, fontsize: f64| {
            cells[i]
                .1
                .required_width(fontsize, layout, &self.rc, &cell_maps[i].to_axes)
        };
        for &column in &self.auto_columns {
            let members: Vec<usize> = (0..cells.len())
                .filter(|&i| cells[i].0.1 == column)
                .collect();
            let mut widest: f64 = 0.0;
            for &i in &members {
                widest = widest.max(required(i, sizes.fontsize[i])?);
            }
            for &i in &members {
                sizes.width[i] = widest;
            }
        }
        if self.auto_fontsize && !cells.is_empty() {
            let mut fontsize = sizes.fontsize[0];
            for (i, (key, _)) in cells.iter().enumerate() {
                if self.auto_columns.contains(&key.1) {
                    continue;
                }
                let mut size = sizes.fontsize[i];
                if size > MAX_AUTO_FONTSIZE {
                    size = MAX_AUTO_FONTSIZE;
                }
                let mut needed = required(i, size)?;
                while size > 1.0 && needed > sizes.width[i] {
                    size -= 1.0;
                    needed = required(i, size.max(1.0))?;
                }
                sizes.fontsize[i] = size.max(1.0);
                fontsize = fontsize.min(size);
            }
            sizes.fontsize.fill(fontsize.max(1.0));
        }
        let mut origins = self.align(&sizes);
        let grid = self.grid_box(&sizes, &origins, &cell_maps, &self.own_maps(layout, maps));
        let (l, b, w, h) = grid.bounds();
        let (ox, oy) = match self.bbox {
            Some(target) => {
                let (rl, rb, rw, rh) = target.bounds();
                for width in &mut sizes.width {
                    *width *= rw / w;
                }
                for height in &mut sizes.height {
                    *height *= rh / h;
                }
                origins = self.align(&sizes);
                (rl - l, rb - b)
            }
            None => self.loc.offset(l, b, w, h),
        };
        Ok(origins
            .iter()
            .zip(sizes.width.iter().zip(&sizes.height))
            .zip(&sizes.fontsize)
            .map(|((origin, (&width, &height)), &fontsize)| PlacedCell {
                bbox: Bbox::from_bounds(origin.x + ox, origin.y + oy, width, height),
                size: (width, height),
                fontsize,
            })
            .collect())
    }

    /// Writes what placing the cells as `placed` set into them, as
    /// matplotlib's `_update_positions` does when the table is drawn or
    /// measured: their sizes (fitted columns, a table stretched over its
    /// box), and the font size the texts were fitted to, from which they
    /// are fitted next (never growing back).
    pub(super) fn remember_placed(&self, placed: &[PlacedCell]) {
        for (((_, column), cell), placed) in self.cells.iter().zip(placed) {
            if self.bbox.is_some() || self.auto_columns.contains(column) {
                cell.width_fitted.set(true);
            }
            if self.bbox.is_some() {
                cell.height_fitted.set(true);
            }
            cell.width.set(placed.size.0);
            cell.height.set(placed.size.1);
            if self.auto_fontsize {
                cell.fitted(placed.fontsize);
            }
        }
    }

    /// The lower-left corner of every cell: columns side by side from 0,
    /// rows stacked upwards from the last (`Table._do_cell_alignment`).
    fn align(&self, sizes: &Sizes) -> Vec<Point> {
        let mut widths: BTreeMap<i32, f64> = BTreeMap::new();
        let mut heights: BTreeMap<i32, f64> = BTreeMap::new();
        for (i, ((row, col), _)) in self.cells.iter().enumerate() {
            let height = heights.entry(*row).or_insert(0.0);
            *height = height.max(sizes.height[i]);
            let width = widths.entry(*col).or_insert(0.0);
            *width = width.max(sizes.width[i]);
        }
        let mut lefts = BTreeMap::new();
        let mut x = 0.0;
        for (col, width) in &widths {
            lefts.insert(*col, x);
            x += width;
        }
        let mut bottoms = BTreeMap::new();
        let mut y = 0.0;
        for (row, height) in heights.iter().rev() {
            bottoms.insert(*row, y);
            y += height;
        }
        self.cells
            .iter()
            .map(|((row, col), _)| Point::new(lefts[col], bottoms[row]))
            .collect()
    }

    /// The maps of each cell's coordinates in the layout `layout`, the
    /// table's axes being at `maps`: those of its own transform, else of
    /// the fractions of the table's axes (a cell whose transform cannot be
    /// mapped back is measured in those too; drawing it fails).
    pub(crate) fn cell_maps(&self, layout: LayoutContext<'_>, maps: &AxesMaps) -> Vec<AxesMaps> {
        self.cells
            .iter()
            .map(|(_, cell)| cell.maps(layout, maps).unwrap_or_else(|| maps.clone()))
            .collect()
    }

    /// The maps of the table's own coordinates, in which its grid is
    /// measured and moved: the fractions of its axes (`maps`), unless the
    /// program gave it another transform (`Table.set_transform`), which
    /// matplotlib's cells, keeping theirs, are then measured back through.
    fn own_maps(&self, layout: LayoutContext<'_>, maps: &AxesMaps) -> AxesMaps {
        self.own_transform()
            .and_then(|transform| AxesMaps::through(layout, &transform))
            .unwrap_or_else(|| maps.clone())
    }

    /// The box around the cells of non-negative rows and columns, in the
    /// table's own coordinates (`own`): the union of their display boxes,
    /// each cell through its own maps, mapped back
    /// (`Table._get_grid_bbox`).
    fn grid_box(
        &self,
        sizes: &Sizes,
        origins: &[Point],
        cell_maps: &[AxesMaps],
        own: &AxesMaps,
    ) -> Bbox {
        self.cells
            .iter()
            .enumerate()
            .filter(|(_, ((row, col), _))| *row >= 0 && *col >= 0)
            .map(|(i, (_, cell))| {
                let bbox =
                    Bbox::from_bounds(origins[i].x, origins[i].y, sizes.width[i], sizes.height[i]);
                cell.display_box(&bbox, &cell_maps[i].to_display)
            })
            .reduce(|a, b| a.union(&b))
            .map_or(Bbox::NULL, |display| display.transformed(&own.to_axes))
    }
}

impl TableLoc {
    /// How far a grid at `(l, b)` of `w × h` moves to sit at this location.
    fn offset(self, l: f64, b: f64, w: f64, h: f64) -> (f64, f64) {
        use TableLoc::{
            Best, Bottom, BottomLeft, BottomRight, Center, CenterLeft, CenterRight, Left,
            LowerCenter, LowerLeft, LowerRight, Right, Top, TopLeft, TopRight, UpperCenter,
            UpperLeft, UpperRight,
        };
        let mut ox = (0.5 - w / 2.0) - l;
        let mut oy = (0.5 - h / 2.0) - b;
        if matches!(self, UpperLeft | LowerLeft | CenterLeft) {
            ox = AXES_PAD - l;
        }
        if matches!(self, Best | UpperRight | LowerRight | Right | CenterRight) {
            ox = 1.0 - (l + w + AXES_PAD);
        }
        if matches!(self, Best | UpperRight | UpperLeft | UpperCenter) {
            oy = 1.0 - (b + h + AXES_PAD);
        }
        if matches!(self, LowerLeft | LowerRight | LowerCenter) {
            oy = AXES_PAD - b;
        }
        if matches!(self, LowerCenter | UpperCenter | Center) {
            ox = (0.5 - w / 2.0) - l;
        }
        if matches!(self, CenterLeft | CenterRight | Center) {
            oy = (0.5 - h / 2.0) - b;
        }
        if matches!(self, TopLeft | BottomLeft | Left) {
            ox = -(l + w);
        }
        if matches!(self, TopRight | BottomRight | Right) {
            ox = 1.0 - l;
        }
        if matches!(self, TopRight | TopLeft | Top) {
            oy = 1.0 - b;
        }
        if matches!(self, BottomLeft | BottomRight | Bottom) {
            oy = -(b + h);
        }
        (ox, oy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::table::TableOptions;

    /// The table cells of `options` placed in a 640 × 480 px axes.
    fn placed(options: &TableOptions, tweak: impl FnOnce(&crate::table::Table)) -> Vec<PlacedCell> {
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.125, 0.11, 0.775, 0.77])
            .expect("a valid rectangle");
        let table = ax.table(options.clone()).expect("valid table");
        tweak(&table);
        let state = table.raw().figure_state().borrow();
        let layout = LayoutContext::new(&state);
        let artist = state
            .store
            .get_as::<TableArtist>(table.id())
            .expect("a table");
        let axes = Bbox::from_bounds(80.0, 52.8, 496.0, 369.6);
        let maps = AxesMaps::of(&axes).expect("an axes of some area");
        artist.place(layout, &maps).expect("placeable")
    }

    fn texts(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|row| row.iter().map(|&s| s.to_owned()).collect())
            .collect()
    }

    #[test]
    fn cells_stack_below_the_axes_by_default() {
        let cells = placed(
            &TableOptions::with_text(texts(&[&["a", "b"], &["c", "d"]])),
            |_| {},
        );
        let height = 10.0 / 72.0 * 100.0 / 369.6 * 1.2;
        assert!((cells[0].bbox.height() - height).abs() < 1e-12);
        // Row 0 sits above row 1, both below the axes.
        assert!(cells[0].bbox.y0() > cells[2].bbox.y0());
        assert!((cells[0].bbox.y1() - 0.0).abs() < 1e-12);
        assert_eq!(cells[1].bbox.x0(), 0.5);
    }

    #[test]
    fn long_texts_shrink_the_font_unless_turned_off() {
        let options = TableOptions {
            col_widths: Some(vec![0.05]),
            ..TableOptions::with_text(texts(&[&["a rather long cell text"]]))
        };
        let shrunk = placed(&options, |_| {});
        assert!(shrunk[0].fontsize < 10.0);
        let fixed = placed(&options, |table| {
            table.auto_set_font_size(false);
        });
        assert_eq!(fixed[0].fontsize, 10.0);
    }

    #[test]
    fn fonts_fitted_when_drawn_stay_until_given_again() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let options = TableOptions {
            col_widths: Some(vec![0.05]),
            ..TableOptions::with_text(texts(&[&["a rather long cell text"]]))
        };
        let table = ax.table(options).expect("valid table");
        let cell = table.cell(0, 0).expect("a cell");
        assert_eq!(cell.fontsize(), 10.0, "fitted only when drawn");
        fig.render_raster().expect("drawn");
        let fitted = cell.fontsize();
        assert!(fitted < 10.0);
        // A wider cell keeps the size its text shrank to, as matplotlib's
        // fitting never grows a font back.
        cell.set_width(10.0);
        fig.render_raster().expect("drawn");
        assert_eq!(cell.fontsize(), fitted);
        assert_eq!(cell.font().size, fitted);
        cell.set_fontsize(12.0);
        fig.render_raster().expect("drawn");
        assert_eq!(cell.fontsize(), 12.0);
    }

    #[test]
    fn huge_fonts_shrink_in_bounded_steps() {
        // Found by fuzzing: stepping down a point at a time from an
        // infinite size never ended.
        let options = TableOptions {
            col_widths: Some(vec![0.05]),
            fontsize: Some(f64::INFINITY),
            ..TableOptions::with_text(texts(&[&["a rather long cell text"]]))
        };
        let shrunk = placed(&options, |_| {});
        assert!(shrunk[0].fontsize < 10.0);
    }

    #[test]
    fn row_labels_get_a_fitted_column() {
        let options = TableOptions {
            row_labels: Some(vec!["a label".to_owned()]),
            ..TableOptions::with_text(texts(&[&["x"]]))
        };
        let cells = placed(&options, |_| {});
        assert!(cells[1].bbox.width() > 0.01, "fitted to its text");
        assert!(cells[1].bbox.x1() <= cells[0].bbox.x0() + 1e-12);
    }

    #[test]
    fn locations_place_the_grid() {
        let (l, b, w, h) = (0.0, 0.0, 0.5, 0.2);
        let place = |loc: TableLoc| loc.offset(l, b, w, h);
        assert_eq!(place(TableLoc::Bottom), (0.25, -0.2));
        assert_eq!(place(TableLoc::UpperLeft), (0.02, 0.78));
        assert_eq!(place(TableLoc::Best), place(TableLoc::UpperRight));
        assert_eq!(place(TableLoc::Center), (0.25, 0.4));
        let (ox, oy) = place(TableLoc::LowerRight);
        assert!((ox - 0.48).abs() < 1e-12 && (oy - 0.02).abs() < 1e-12);
        assert_eq!(place(TableLoc::TopLeft), (-0.5, 1.0));
    }
}
