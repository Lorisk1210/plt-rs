//! Tables of text cells drawn in or next to an axes (matplotlib.table).
//!
//! [`Axes::table`](crate::axes::Axes::table) builds a [`Table`] from rows of
//! texts with optional row and column headers. Cells are sized in axes
//! fractions; at draw time the table fits its columns and font size as
//! requested and moves the grid to its [`TableLoc`].
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::table::{TableLoc, TableOptions};
//! let ax = Figure::new().subplot();
//! let table = ax.table(TableOptions {
//!     col_labels: Some(vec!["A".into(), "B".into()]),
//!     loc: TableLoc::Bottom,
//!     ..TableOptions::with_text(vec![vec!["1".into(), "2".into()]])
//! })?;
//! table.scale(1.0, 1.5);
//! # Ok::<(), pltrs::table::TableError>(())
//! ```

mod cell;
mod draw;
mod handle;
mod layout;
mod look;
mod measure;

use std::fmt;
use std::rc::Rc;
use std::str::FromStr;

use pltrs_core::Bbox;
use pltrs_text::HAlign;

pub use cell::CellEdges;
pub use handle::{Table, TableCell};

use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext, LayoutContext};
use crate::colors::Color;
use crate::fonts::FontSize;
use crate::rc::RcParams;
pub(crate) use cell::CellShape;
use cell::{Cell, PlacedCell};
use layout::AxesMaps;

/// Where a table goes relative to its axes (matplotlib's `Table.codes`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableLoc {
    /// Like `UpperRight`.
    Best,
    /// Inside, top right.
    UpperRight,
    /// Inside, top left.
    UpperLeft,
    /// Inside, bottom left.
    LowerLeft,
    /// Inside, bottom right.
    LowerRight,
    /// Inside, middle left.
    CenterLeft,
    /// Inside, middle right.
    CenterRight,
    /// Inside, bottom middle.
    LowerCenter,
    /// Inside, top middle.
    UpperCenter,
    /// Inside, centred.
    Center,
    /// Outside, above the right end.
    TopRight,
    /// Outside, above the left end.
    TopLeft,
    /// Outside, below the left end.
    BottomLeft,
    /// Outside, below the right end.
    BottomRight,
    /// Outside, right of the axes.
    Right,
    /// Outside, left of the axes.
    Left,
    /// Outside, above the axes.
    Top,
    /// Outside, below the axes.
    #[default]
    Bottom,
}

pltrs_core::archive_enum!(TableLoc {
    Best,
    UpperRight,
    UpperLeft,
    LowerLeft,
    LowerRight,
    CenterLeft,
    CenterRight,
    LowerCenter,
    UpperCenter,
    Center,
    TopRight,
    TopLeft,
    BottomLeft,
    BottomRight,
    Right,
    Left,
    Top,
    Bottom,
});

impl FromStr for TableLoc {
    type Err = TableError;

    /// Parses matplotlib's names (`"upper right"`, `"bottom"`, ...).
    fn from_str(text: &str) -> Result<Self, TableError> {
        Ok(match text {
            "best" => Self::Best,
            "upper right" => Self::UpperRight,
            "upper left" => Self::UpperLeft,
            "lower left" => Self::LowerLeft,
            "lower right" => Self::LowerRight,
            "center left" => Self::CenterLeft,
            "center right" => Self::CenterRight,
            "lower center" => Self::LowerCenter,
            "upper center" => Self::UpperCenter,
            "center" => Self::Center,
            "top right" => Self::TopRight,
            "top left" => Self::TopLeft,
            "bottom left" => Self::BottomLeft,
            "bottom right" => Self::BottomRight,
            "right" => Self::Right,
            "left" => Self::Left,
            "top" => Self::Top,
            "bottom" => Self::Bottom,
            _ => return Err(TableError::Location(text.to_owned())),
        })
    }
}

/// Why a table cannot be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableError {
    /// Neither texts nor colours were given.
    NoCells,
    /// A row of texts or colours has the wrong length, or a header list
    /// does not match the table.
    Shape(&'static str),
    /// An unknown location name.
    Location(String),
    /// An invalid edge description.
    Edges(String),
}

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCells => f.write_str("at least one of cell texts or cell colours is needed"),
            Self::Shape(what) => write!(f, "{what} does not match the table's rows and columns"),
            Self::Location(loc) => write!(f, "unrecognized table location {loc:?}"),
            Self::Edges(edges) => write!(
                f,
                "invalid edges {edges:?}: use open, closed, horizontal, vertical or letters of BRTL"
            ),
        }
    }
}

impl std::error::Error for TableError {}

/// The contents and placement of a new table (matplotlib's `table()`
/// arguments).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableOptions {
    /// The texts, row by row; empty with `cell_colours` only.
    pub cell_text: Vec<Vec<String>>,
    /// The cell backgrounds, row by row (`pltrs.table.facecolor`, white,
    /// when unset).
    pub cell_colours: Option<Vec<Vec<Color>>>,
    /// Text alignment in the cells (`Right` by default).
    pub cell_loc: Option<HAlign>,
    /// Column widths in axes fractions (`1 / columns` each when unset).
    pub col_widths: Option<Vec<f64>>,
    /// Row header texts.
    pub row_labels: Option<Vec<String>>,
    /// Row header backgrounds.
    pub row_colours: Option<Vec<Color>>,
    /// Text alignment in the row headers (`Left` by default).
    pub row_loc: Option<HAlign>,
    /// Column header texts.
    pub col_labels: Option<Vec<String>>,
    /// Column header backgrounds.
    pub col_colours: Option<Vec<Color>>,
    /// Text alignment in the column headers (`Center` by default).
    pub col_loc: Option<HAlign>,
    /// Where the table goes.
    pub loc: TableLoc,
    /// A box in axes fractions the table is stretched into, overriding
    /// `loc`.
    pub bbox: Option<Bbox>,
    /// The drawn edges of every cell.
    pub edges: CellEdges,
    /// The text size in points (still shrunk to fit unless automatic font
    /// sizing is turned off).
    pub fontsize: Option<f64>,
}

impl TableOptions {
    /// A table of `cell_text` with matplotlib's defaults.
    pub fn with_text(cell_text: Vec<Vec<String>>) -> Self {
        Self {
            cell_text,
            ..Self::default()
        }
    }
}

/// State of a table (matplotlib's `Table`).
#[derive(Debug)]
pub(crate) struct TableArtist {
    base: ArtistBase,
    /// The axes the table is laid out in; `None` for the holder of a cell
    /// made outside any table, which is never drawn.
    axes: Option<ArtistId>,
    loc: TableLoc,
    bbox: Option<Bbox>,
    /// The cells keyed by `(row, column)`, in insertion order.
    cells: Vec<((i32, i32), Cell)>,
    auto_columns: Vec<i32>,
    auto_fontsize: bool,
    rc: Rc<RcParams>,
}

pltrs_core::archive_struct!(TableArtist {
    base,
    axes,
    loc,
    bbox,
    cells,
    auto_columns,
    auto_fontsize,
    rc
});

/// Matplotlib's `Table.FONTSIZE`, used to size the rows.
const FONTSIZE: f64 = 10.0;

impl TableArtist {
    /// The table `options` describe in `axes`, whose display height is
    /// `axes_height` pixels at `dpi` (`matplotlib.table.table`).
    pub(crate) fn new(
        axes: ArtistId,
        options: &TableOptions,
        (axes_height, dpi): (f64, f64),
        rc: Rc<RcParams>,
    ) -> Result<Self, TableError> {
        let builder = TableBuilder::new(options)?;
        let height = FONTSIZE / 72.0 * dpi / axes_height * 1.2;
        let mut table = Self::empty(Some(axes), options.loc, options.bbox, rc);
        builder.fill(&mut table, options, height);
        if let Some(size) = options.fontsize {
            table.set_fontsize(size);
        }
        Ok(table)
    }

    /// A table without cells in `axes` (in its axes fractions, unclipped),
    /// placed at `loc` or stretched into `bbox` (matplotlib's `Table(ax,
    /// loc, bbox)`).
    pub(crate) fn empty(
        axes: Option<ArtistId>,
        loc: TableLoc,
        bbox: Option<Bbox>,
        rc: Rc<RcParams>,
    ) -> Self {
        let mut base = ArtistBase::with_zorder(0.0);
        base.clip_on = false;
        base.transform = axes.map(crate::transforms::Transform::Axes);
        Self {
            base,
            axes,
            loc,
            bbox,
            cells: Vec::new(),
            auto_columns: Vec::new(),
            auto_fontsize: true,
            rc,
        }
    }

    /// Puts `cell` at `(row, col)`; a cell replacing another keeps its
    /// place in the insertion order, as in matplotlib's dictionary.
    fn add_cell(&mut self, row: i32, col: i32, cell: Cell) {
        match self.cells.iter_mut().find(|(key, _)| *key == (row, col)) {
            Some((_, old)) => *old = cell,
            None => self.cells.push(((row, col), cell)),
        }
    }

    fn set_fontsize(&mut self, size: f64) {
        let base = self.rc.font_size;
        for (_, cell) in &mut self.cells {
            cell.font.set_size(FontSize::Points(size), base);
        }
    }

    /// The axes box in display pixels.
    fn axes_box(&self, layout: LayoutContext<'_>) -> Option<Bbox> {
        layout.axes_extent(self.axes?)
    }
}

/// The checked shape and defaults of a table being built.
struct TableBuilder {
    rows: usize,
    cols: usize,
    col_widths: Vec<f64>,
}

impl TableBuilder {
    fn new(options: &TableOptions) -> Result<Self, TableError> {
        let (rows, cols) = if options.cell_text.is_empty() {
            let colours = options.cell_colours.as_ref().ok_or(TableError::NoCells)?;
            (colours.len(), colours.first().map_or(0, Vec::len))
        } else {
            (options.cell_text.len(), options.cell_text[0].len())
        };
        if !options.cell_text.is_empty() && options.cell_text.iter().any(|r| r.len() != cols) {
            return Err(TableError::Shape("a row of cell texts"));
        }
        if let Some(colours) = &options.cell_colours
            && (colours.len() != rows || colours.iter().any(|r| r.len() != cols))
        {
            return Err(TableError::Shape("the cell colours"));
        }
        let checks = [
            (
                options.row_labels.as_ref().map(Vec::len),
                rows,
                "the row labels",
            ),
            (
                options.row_colours.as_ref().map(Vec::len),
                rows,
                "the row colours",
            ),
            (
                options.col_labels.as_ref().map(Vec::len),
                cols,
                "the column labels",
            ),
            (
                options.col_colours.as_ref().map(Vec::len),
                cols,
                "the column colours",
            ),
            (
                options.col_widths.as_ref().map(Vec::len),
                cols,
                "the column widths",
            ),
        ];
        if let Some((_, _, what)) = checks
            .iter()
            .find(|(len, expected, _)| len.is_some_and(|len| len < *expected))
        {
            return Err(TableError::Shape(what));
        }
        Ok(Self {
            rows,
            cols,
            col_widths: options
                .col_widths
                .clone()
                .unwrap_or_else(|| vec![1.0 / cols as f64; cols]),
        })
    }

    /// Adds the cells, the column headers (row 0) and the row headers
    /// (column -1).
    fn fill(&self, table: &mut TableArtist, options: &TableOptions, height: f64) {
        let rc = Rc::clone(&table.rc);
        let white = || rc.pltrs_table_facecolor.clone();
        let make = |width: f64, text: &str, facecolor: Color, loc: HAlign| {
            let mut cell = Cell {
                text: text.to_owned(),
                loc,
                edges: options.edges,
                ..Cell::new(width, height, &rc)
            };
            cell.style.facecolor = Some(facecolor);
            cell
        };
        let has_col_header = options.col_labels.is_some() || options.col_colours.is_some();
        let offset = i32::from(has_col_header);
        let row_index = |row: usize| {
            i32::try_from(row)
                .unwrap_or(i32::MAX)
                .saturating_add(offset)
        };
        let col_index = |col: usize| i32::try_from(col).unwrap_or(i32::MAX);
        for row in 0..self.rows {
            for col in 0..self.cols {
                let text = options
                    .cell_text
                    .get(row)
                    .and_then(|r| r.get(col))
                    .map_or("", String::as_str);
                let face = options
                    .cell_colours
                    .as_ref()
                    .map_or_else(white, |c| c[row][col].clone());
                let cell = make(
                    self.col_widths[col],
                    text,
                    face,
                    options.cell_loc.unwrap_or(HAlign::Right),
                );
                table.add_cell(row_index(row), col_index(col), cell);
            }
        }
        if has_col_header {
            for col in 0..self.cols {
                let text = options
                    .col_labels
                    .as_ref()
                    .map_or("", |labels| labels[col].as_str());
                let face = options
                    .col_colours
                    .as_ref()
                    .map_or_else(white, |c| c[col].clone());
                let cell = make(
                    self.col_widths[col],
                    text,
                    face,
                    options.col_loc.unwrap_or(HAlign::Center),
                );
                table.add_cell(0, col_index(col), cell);
            }
        }
        let has_row_header = options.row_labels.is_some() || options.row_colours.is_some();
        if has_row_header {
            // Without labels the header column takes the first column's
            // width; with labels it fits them.
            let width = if options.row_labels.is_none() {
                self.col_widths[0]
            } else {
                1e-15
            };
            for row in 0..self.rows {
                let text = options
                    .row_labels
                    .as_ref()
                    .map_or("", |labels| labels[row].as_str());
                let face = options
                    .row_colours
                    .as_ref()
                    .map_or_else(white, |c| c[row].clone());
                let cell = make(width, text, face, options.row_loc.unwrap_or(HAlign::Left));
                table.add_cell(row_index(row), -1, cell);
            }
            if options.row_labels.is_some() {
                table.auto_columns.push(-1);
            }
        }
    }
}

impl Artist for TableArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let Some((maps, placed)) = self.placed_for_drawing(ctx) else {
            return;
        };
        let mut order: Vec<usize> = (0..self.cells.len()).collect();
        order.sort_by_key(|&i| self.cells[i].0);
        ctx.renderer().open_group("table", self.base.gid.as_deref());
        for i in order {
            self.draw_cell(ctx, i, &placed[i], &maps);
        }
        ctx.renderer().close_group("table");
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let maps = AxesMaps::of(&self.axes_box(*ctx)?)?;
        self.place(*ctx, &maps)
            .ok()?
            .iter()
            .map(|cell| PlacedCell::display_box(&cell.bbox, &maps.to_display))
            .reduce(|a, b| a.union(&b))
    }
}
