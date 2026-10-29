//! Mosaic layouts: grids of labelled cells, written as rows of cells or
//! as matplotlib's text mosaics, and the rectangle each label covers.

use std::str::FromStr;

use super::MosaicError;

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
    /// use pltrs::figure::Mosaic;
    /// let mosaic: Mosaic = "AAB;C.B".parse()?;
    /// assert_eq!(mosaic.shape(), (2, 3));
    /// # Ok::<(), pltrs::figure::MosaicError>(())
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

impl TryFrom<&str> for Mosaic {
    type Error = MosaicError;

    /// The mosaic `layout` describes (see [`FromStr`](#impl-FromStr-for-Mosaic)).
    fn try_from(layout: &str) -> Result<Self, MosaicError> {
        layout.parse()
    }
}

impl From<&Mosaic> for Mosaic {
    /// A copy of the mosaic, so that methods taking a mosaic take a
    /// reference to one too.
    fn from(mosaic: &Mosaic) -> Self {
        mosaic.clone()
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

    /// Whether a cell of the mosaic or of a nested one has `label`.
    pub fn contains(&self, label: &str) -> bool {
        self.rows.iter().flatten().any(|cell| match cell {
            MosaicCell::Label(name) => name == label,
            MosaicCell::Empty => false,
            MosaicCell::Nested(inner) => inner.contains(label),
        })
    }

    /// Labels in order of first appearance, with the rectangle each
    /// covers, and the nested cells.
    pub(super) fn regions(&self) -> Result<Vec<Region<'_>>, MosaicError> {
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
pub(super) enum Region<'a> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
