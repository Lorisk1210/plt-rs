//! Named subplot layouts (matplotlib's `subplot_mosaic`).

use std::fmt;
use std::str::FromStr;

use super::Figure;
use crate::axes::{Axes, AxisName};
use crate::gridspec::{GridSpec, GridSpecError, SubplotOverrides};

/// One cell of a [`Mosaic`].
#[derive(Clone, Debug, PartialEq)]
pub enum MosaicCell {
    /// Part of the axes with this label.
    Label(String),
    /// Left empty.
    Empty,
    /// Subdivided by a nested mosaic.
    Nested(Mosaic),
}

/// A grid of labelled cells; every label must cover a rectangle of cells.
#[derive(Clone, Debug, PartialEq)]
pub struct Mosaic {
    rows: Vec<Vec<MosaicCell>>,
}

impl FromStr for Mosaic {
    type Err = MosaicError;

    /// matplotlib's string mosaics: one character per cell, rows separated
    /// by `;` (or by line breaks, ignoring blank lines and common
    /// indentation), and `.` for empty cells; fails as [`Mosaic::new`].
    ///
    /// ```
    /// use plt::figure::Mosaic;
    /// let mosaic: Mosaic = "AAB;C.B".parse()?;
    /// assert_eq!(mosaic.shape(), (2, 3));
    /// # Ok::<(), plt::figure::MosaicError>(())
    /// ```
    fn from_str(layout: &str) -> Result<Self, MosaicError> {
        let lines: Vec<String> = if layout.contains('\n') {
            cleandoc(layout)
        } else {
            layout.split(';').map(str::to_owned).collect()
        };
        Self::new(
            lines
                .iter()
                .map(|line| {
                    line.chars()
                        .map(|c| match c {
                            '.' => MosaicCell::Empty,
                            c => MosaicCell::Label(c.to_string()),
                        })
                        .collect()
                })
                .collect(),
        )
    }
}

/// Options of [`Figure::subplot_mosaic`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MosaicOptions {
    /// Relative column widths of the outer grid.
    pub width_ratios: Option<Vec<f64>>,
    /// Relative row heights of the outer grid.
    pub height_ratios: Option<Vec<f64>>,
    /// Share the x axis of every axes with the first one.
    pub sharex: bool,
    /// Share the y axis of every axes with the first one.
    pub sharey: bool,
    /// Make every axes polar (matplotlib's
    /// `subplot_kw={"projection": "polar"}`).
    pub polar: bool,
    /// Labels of axes made polar, the others staying rectilinear
    /// (matplotlib's `per_subplot_kw={label: {"projection": "polar"}}`).
    pub polar_labels: Vec<String>,
    /// The margins and spacing of the outer grid (matplotlib's
    /// `gridspec_kw` `left`, `right`, `bottom`, `top`, `wspace` and
    /// `hspace`).
    pub gridspec: SubplotOverrides,
}

/// The axes of a mosaic, by label, in creation order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MosaicAxes {
    entries: Vec<(String, Axes)>,
}

impl MosaicAxes {
    /// The axes labelled `label`.
    pub fn get(&self, label: &str) -> Option<&Axes> {
        self.entries
            .iter()
            .find(|(name, _)| name == label)
            .map(|(_, ax)| ax)
    }

    /// Labels and axes in creation order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Axes)> {
        self.entries.iter().map(|(name, ax)| (name.as_str(), ax))
    }

    /// The number of axes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the mosaic has no axes.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Why a mosaic cannot be laid out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MosaicError {
    /// The mosaic has no rows or no columns.
    Empty,
    /// Rows of different lengths.
    Ragged {
        /// Length of the first row.
        expected: usize,
        /// Length of the offending row.
        actual: usize,
    },
    /// A label covers cells that do not form a rectangle.
    NonRectangular(String),
    /// A label appears in two separate places (for example inside and
    /// outside a nested mosaic).
    Duplicate(String),
    /// The grid itself is invalid (for example mismatched ratios).
    Grid(GridSpecError),
}

impl fmt::Display for MosaicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("a mosaic needs at least one row and one column"),
            Self::Ragged { expected, actual } => write!(
                f,
                "all mosaic rows must be the same length: the first has {expected} cells, \
                 another {actual}"
            ),
            Self::NonRectangular(label) => {
                write!(f, "the label {label:?} specifies a non-rectangular area")
            }
            Self::Duplicate(label) => write!(f, "the label {label:?} appears more than once"),
            Self::Grid(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for MosaicError {}

impl From<GridSpecError> for MosaicError {
    fn from(err: GridSpecError) -> Self {
        Self::Grid(err)
    }
}

/// Python's `str.expandtabs()`: each tab advances to the next multiple of
/// eight columns.
fn expand_tabs(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut column = 0;
    for c in line.chars() {
        if c == '\t' {
            let spaces = 8 - column % 8;
            out.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else {
            out.push(c);
            column += 1;
        }
    }
    out
}

/// The lines of `text` as Python's `inspect.cleandoc` leaves them: tabs
/// expanded, the first line's leading spaces and the common indentation
/// of the others removed, and blank lines dropped at either end (blank
/// lines between rows stay, and make the mosaic ragged).
fn cleandoc(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.split('\n').map(expand_tabs).collect();
    let indent = |line: &str| line.len() - line.trim_start_matches(' ').len();
    let margin = lines
        .iter()
        .skip(1)
        .filter(|line| !line.trim_start_matches(' ').is_empty())
        .map(|line| indent(line))
        .min();
    if let Some(first) = lines.first_mut() {
        *first = first.trim_start_matches(' ').to_owned();
    }
    if let Some(margin) = margin {
        for line in lines.iter_mut().skip(1) {
            *line = line.get(margin..).unwrap_or_default().to_owned();
        }
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    let leading = lines.iter().take_while(|line| line.is_empty()).count();
    lines.drain(..leading);
    lines
}

impl Mosaic {
    /// A mosaic from rows of cells.
    ///
    /// # Errors
    ///
    /// When there are no cells or the rows differ in length.
    pub fn new(rows: Vec<Vec<MosaicCell>>) -> Result<Self, MosaicError> {
        let width = rows.first().map_or(0, Vec::len);
        if width == 0 {
            return Err(MosaicError::Empty);
        }
        if let Some(row) = rows.iter().find(|row| row.len() != width) {
            return Err(MosaicError::Ragged {
                expected: width,
                actual: row.len(),
            });
        }
        Ok(Self { rows })
    }

    /// `(rows, columns)`.
    pub fn shape(&self) -> (usize, usize) {
        (self.rows.len(), self.rows[0].len())
    }

    /// Labels in order of first appearance, with the rectangle each
    /// covers, and the nested cells.
    fn regions(&self) -> Result<Vec<Region<'_>>, MosaicError> {
        let mut labels: Vec<&str> = Vec::new();
        let mut regions = Vec::new();
        for (row, cells) in self.rows.iter().enumerate() {
            for (col, cell) in cells.iter().enumerate() {
                match cell {
                    MosaicCell::Label(label) if !labels.contains(&label.as_str()) => {
                        labels.push(label);
                    }
                    MosaicCell::Nested(inner) => regions.push(Region::Nested { row, col, inner }),
                    _ => {}
                }
            }
        }
        for label in labels {
            let cells: Vec<(usize, usize)> = self
                .rows
                .iter()
                .enumerate()
                .flat_map(|(r, row)| row.iter().enumerate().map(move |(c, cell)| (r, c, cell)))
                .filter(|(_, _, cell)| matches!(cell, MosaicCell::Label(l) if l == label))
                .map(|(r, c, _)| (r, c))
                .collect();
            let rows = cells.iter().map(|&(r, _)| r).min().unwrap_or(0)
                ..cells.iter().map(|&(r, _)| r).max().unwrap_or(0) + 1;
            let cols = cells.iter().map(|&(_, c)| c).min().unwrap_or(0)
                ..cells.iter().map(|&(_, c)| c).max().unwrap_or(0) + 1;
            if cells.len() != rows.len() * cols.len() {
                return Err(MosaicError::NonRectangular(label.to_owned()));
            }
            regions.push(Region::Axes { label, rows, cols });
        }
        regions.sort_by_key(Region::corner);
        Ok(regions)
    }
}

/// A labelled rectangle or a nested cell of one mosaic level.
enum Region<'a> {
    Axes {
        label: &'a str,
        rows: std::ops::Range<usize>,
        cols: std::ops::Range<usize>,
    },
    Nested {
        row: usize,
        col: usize,
        inner: &'a Mosaic,
    },
}

impl Region<'_> {
    /// The top-left cell, which orders creation as in matplotlib.
    fn corner(&self) -> (usize, usize) {
        match self {
            Self::Axes { rows, cols, .. } => (rows.start, cols.start),
            Self::Nested { row, col, .. } => (*row, *col),
        }
    }
}

impl Figure {
    /// Adds one axes per label of `mosaic` (matplotlib's
    /// `subplot_mosaic`), created level by level in order of their
    /// top-left cells.
    ///
    /// ```
    /// use plt::figure::{Figure, Mosaic, MosaicOptions};
    /// let fig = Figure::new();
    /// let axes = fig.subplot_mosaic(&"AAB;C.B".parse::<Mosaic>()?, MosaicOptions::default())?;
    /// let labels: Vec<&str> = axes.iter().map(|(label, _)| label).collect();
    /// assert_eq!(labels, ["A", "B", "C"]);
    /// # Ok::<(), plt::figure::MosaicError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a label covers a non-rectangular area or appears twice, or the
    /// ratios do not match the grid.
    pub fn subplot_mosaic(
        &self,
        mosaic: &Mosaic,
        options: MosaicOptions,
    ) -> Result<MosaicAxes, MosaicError> {
        let (rows, cols) = mosaic.shape();
        let grid = GridSpec::with_ratios(
            rows,
            cols,
            options.width_ratios.clone(),
            options.height_ratios.clone(),
            options.gridspec,
        )?;
        let mut entries = Vec::new();
        self.lay_out_mosaic(&grid, mosaic, &options, &mut entries)?;
        if let Some((_, first)) = entries.first().cloned() {
            for (share, name) in [(options.sharex, AxisName::X), (options.sharey, AxisName::Y)] {
                if !share {
                    continue;
                }
                for (_, ax) in &entries {
                    ax.share(name, &first)
                        .expect("a new mosaic axes shares nothing yet");
                    ax.label_outer_shared(name);
                }
            }
        }
        Ok(MosaicAxes { entries })
    }

    fn lay_out_mosaic(
        &self,
        grid: &GridSpec,
        mosaic: &Mosaic,
        options: &MosaicOptions,
        entries: &mut Vec<(String, Axes)>,
    ) -> Result<(), MosaicError> {
        for region in mosaic.regions()? {
            match region {
                Region::Axes { label, rows, cols } => {
                    if entries.iter().any(|(name, _)| name == label) {
                        return Err(MosaicError::Duplicate(label.to_owned()));
                    }
                    let spec = grid.span(rows, cols)?;
                    let polar =
                        options.polar || options.polar_labels.iter().any(|polar| polar == label);
                    let ax = if polar {
                        let polar_axes = self.add_polar_subplot_spec(&spec)?;
                        Axes::clone(&polar_axes)
                    } else {
                        self.add_subplot_spec(&spec)?
                    };
                    entries.push((label.to_owned(), ax));
                }
                Region::Nested { row, col, inner } => {
                    let (rows, cols) = inner.shape();
                    let sub = GridSpec::nested(
                        &grid.cell(row, col)?,
                        rows,
                        cols,
                        None,
                        None,
                        (None, None),
                    )?;
                    self.lay_out_mosaic(&sub, inner, options, entries)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ProjectionKind;

    #[test]
    fn multi_line_layouts_are_dedented_like_docstrings() {
        let shape = |layout: &str| layout.parse::<Mosaic>().map(|m| m.shape());
        assert_eq!(shape("\n    AB\n    CD\n"), Ok((2, 2)));
        assert_eq!(
            shape("  AB\n   CD"),
            Ok((2, 2)),
            "the first line is stripped"
        );
        assert_eq!(shape("\tAB\n\tCD"), Ok((2, 2)));
        assert_eq!(
            shape("AB\n\nCD"),
            Err(MosaicError::Ragged {
                expected: 2,
                actual: 0
            }),
            "a blank row between rows stays"
        );
        assert!(shape("  AB\n  C D").is_err(), "inner spaces are cells");
    }

    #[test]
    fn strings_parse_into_rows() {
        let mosaic = "AAB;C.B".parse::<Mosaic>().unwrap();
        assert_eq!(mosaic.shape(), (2, 3));
        let multiline = "\n    AB\n    CD\n".parse::<Mosaic>().unwrap();
        assert_eq!(multiline.shape(), (2, 2));
        assert_eq!(
            "AB;C".parse::<Mosaic>().err(),
            Some(MosaicError::Ragged {
                expected: 2,
                actual: 1
            })
        );
    }

    #[test]
    fn labels_must_be_rectangles() {
        let fig = Figure::new();
        let options = MosaicOptions::default();
        let bent = "AA;A.".parse::<Mosaic>().unwrap();
        assert_eq!(
            fig.subplot_mosaic(&bent, options).err(),
            Some(MosaicError::NonRectangular("A".into()))
        );
    }

    #[test]
    fn nested_mosaics_and_sharing() {
        let fig = Figure::new();
        let inner = "ab;cd".parse::<Mosaic>().unwrap();
        let mosaic = Mosaic::new(vec![
            vec![MosaicCell::Label("main".into()), MosaicCell::Nested(inner)],
            vec![MosaicCell::Label("bottom".into()); 2],
        ])
        .unwrap();
        let options = MosaicOptions {
            sharex: true,
            ..MosaicOptions::default()
        };
        let axes = fig.subplot_mosaic(&mosaic, options).unwrap();
        let labels: Vec<&str> = axes.iter().map(|(label, _)| label).collect();
        assert_eq!(labels, ["main", "a", "b", "c", "d", "bottom"]);
        let main = axes.get("main").unwrap();
        main.set_xlim(0.0, 5.0);
        assert_eq!(axes.get("d").unwrap().xlim(), (0.0, 5.0));
    }

    #[test]
    fn labelled_axes_turn_polar_and_the_grid_takes_its_margins() {
        let fig = Figure::new();
        let options = MosaicOptions {
            polar_labels: vec!["B".into()],
            gridspec: SubplotOverrides {
                left: Some(0.6),
                right: Some(0.95),
                ..SubplotOverrides::default()
            },
            ..MosaicOptions::default()
        };
        let axes = fig
            .subplot_mosaic(&"AB".parse::<Mosaic>().unwrap(), options)
            .unwrap();
        let kind = |label: &str| axes.get(label).unwrap().projection();
        assert_eq!(
            (kind("A"), kind("B")),
            (ProjectionKind::Rectilinear, ProjectionKind::Polar)
        );
        let (left, _, _, _) = axes.get("A").unwrap().position().bounds();
        assert!((left - 0.6).abs() < 1e-12, "{left}");
    }
}
