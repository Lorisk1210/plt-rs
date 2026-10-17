//! Placing the cells of a table (matplotlib's `Table._update_positions`).

use std::collections::BTreeMap;

use plt_core::{Bbox, Point};

use super::cell::{Cell, PlacedCell};
use super::{TableArtist, TableLoc};
use crate::artist::LayoutContext;
use crate::text::TextDrawError;

/// The gap between the axes and a table placed inside them, in axes
/// fractions.
const AXES_PAD: f64 = 0.02;

/// The size of every cell while the table is laid out.
struct Sizes {
    width: Vec<f64>,
    height: Vec<f64>,
    fontsize: Vec<f64>,
}

impl TableArtist {
    /// Every cell with its place and font size for a table in an axes
    /// `axes_width` pixels wide.
    pub(crate) fn place(
        &self,
        layout: LayoutContext<'_>,
        axes_width: f64,
    ) -> Result<Vec<PlacedCell>, TextDrawError> {
        let cells = &self.cells;
        let mut sizes = Sizes {
            width: cells.iter().map(|(_, c)| c.width).collect(),
            height: cells.iter().map(|(_, c)| c.height).collect(),
            fontsize: cells.iter().map(|(_, c)| c.font.size).collect(),
        };
        let required = |cell: &Cell, fontsize: f64| {
            cell.required_width(fontsize, layout, &self.rc, axes_width)
        };
        for &column in &self.auto_columns {
            let members: Vec<usize> = (0..cells.len())
                .filter(|&i| cells[i].0.1 == column)
                .collect();
            let mut widest: f64 = 0.0;
            for &i in &members {
                widest = widest.max(required(&cells[i].1, sizes.fontsize[i])?);
            }
            for &i in &members {
                sizes.width[i] = widest;
            }
        }
        if self.auto_fontsize && !cells.is_empty() {
            let mut fontsize = sizes.fontsize[0];
            for (i, (key, cell)) in cells.iter().enumerate() {
                if self.auto_columns.contains(&key.1) {
                    continue;
                }
                let mut size = sizes.fontsize[i];
                let mut needed = required(cell, size)?;
                while size > 1.0 && needed > sizes.width[i] {
                    size -= 1.0;
                    needed = required(cell, size.max(1.0))?;
                }
                sizes.fontsize[i] = size.max(1.0);
                fontsize = fontsize.min(size);
            }
            sizes.fontsize.fill(fontsize.max(1.0));
        }
        let mut origins = self.align(&sizes);
        let grid = self.grid_box(&sizes, &origins);
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
                fontsize,
            })
            .collect())
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

    /// The box around the cells of non-negative rows and columns
    /// (`Table._get_grid_bbox`).
    fn grid_box(&self, sizes: &Sizes, origins: &[Point]) -> Bbox {
        self.cells
            .iter()
            .enumerate()
            .filter(|(_, ((row, col), _))| *row >= 0 && *col >= 0)
            .map(|(i, _)| {
                Bbox::from_bounds(origins[i].x, origins[i].y, sizes.width[i], sizes.height[i])
            })
            .reduce(|a, b| a.union(&b))
            .unwrap_or(Bbox::NULL)
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
        let ax = fig.add_axes([0.125, 0.11, 0.775, 0.77]);
        let table = ax.table(options).expect("valid table");
        tweak(&table);
        let state = table.raw().figure_state().borrow();
        let layout = LayoutContext::new(&state);
        let artist = state
            .store
            .get_as::<TableArtist>(table.id())
            .expect("a table");
        artist.place(layout, 496.0).expect("placeable")
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
