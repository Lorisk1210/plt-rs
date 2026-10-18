//! Hatch patterns (matplotlib's `hatch.py`): the path drawn in each unit
//! hatch tile.
//!
//! A pattern string combines `-` `|` `+` (horizontal/vertical lines), `/`
//! `\` `x` `X` (diagonals), `o` `O` (small/large circles), `.` (small
//! filled circles) and `*` (stars); repeating a character increases the
//! density. Other characters are ignored. The pieces are always emitted in
//! matplotlib's fixed order (horizontal, vertical, north-east, south-east,
//! small circles, large circles, filled circles, stars), with matplotlib's
//! exact vertex arithmetic.

use std::fmt;

use crate::Point;
use crate::path::{Path, PathCode};

/// matplotlib's default hatch density (lines per unit tile).
pub const DEFAULT_DENSITY: i32 = 6;

/// The most vertices a hatch path may have; see [`HatchError::TooDense`].
pub const MAX_VERTICES: usize = 1 << 22;

/// Why a hatch pattern could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HatchError {
    /// The density is negative.
    NegativeDensity,
    /// A shape pattern (`o`, `O`, `.`, `*`) needs an even number of rows
    /// (character count times density); matplotlib fails on odd counts.
    OddShapeRows,
    /// The pattern and density would need more than [`MAX_VERTICES`]
    /// vertices (matplotlib builds such paths regardless, exhausting memory).
    TooDense,
}

impl fmt::Display for HatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NegativeDensity => "hatch density must not be negative",
            Self::OddShapeRows => "shape hatches need an even number of rows",
            Self::TooDense => "the hatch pattern is too dense",
        })
    }
}

impl std::error::Error for HatchError {}

/// numpy's `linspace(start, stop, num, endpoint)`: `i * step + start`, with
/// the exact end point when `endpoint` is set.
fn linspace(start: f64, stop: f64, num: usize, endpoint: bool) -> impl Iterator<Item = f64> {
    let div = if endpoint { num.saturating_sub(1) } else { num };
    let step = (stop - start) / div as f64;
    (0..num).map(move |i| {
        if endpoint && num > 1 && i == num - 1 {
            stop
        } else {
            i as f64 * step + start
        }
    })
}

/// Accumulates the pattern's vertices and codes.
#[derive(Default)]
struct Builder {
    vertices: Vec<Point>,
    codes: Vec<PathCode>,
}

impl Builder {
    fn segment(&mut self, from: Point, to: Point) {
        self.vertices.extend([from, to]);
        self.codes.extend([PathCode::MoveTo, PathCode::LineTo]);
    }

    /// Evenly spaced lines through the tile centre bands.
    fn straight_lines(&mut self, count: usize, horizontal: bool) {
        let step = 1.0 / count as f64;
        for s in linspace(0.0, 1.0, count, false).map(|s| s + step / 2.0) {
            if horizontal {
                self.segment(Point::new(0.0, s), Point::new(1.0, s));
            } else {
                self.segment(Point::new(s, 0.0), Point::new(s, 1.0));
            }
        }
    }

    /// `count + 1` diagonals, rising (`/`) or falling (`\`).
    fn diagonals(&mut self, count: usize, rising: bool) {
        if count == 0 {
            return;
        }
        for s in linspace(-0.5, 0.5, count + 1, true) {
            if rising {
                self.segment(Point::new(0.0 + s, 0.0 - s), Point::new(1.0 + s, 1.0 - s));
            } else {
                self.segment(Point::new(0.0 + s, 1.0 + s), Point::new(1.0 + s, 0.0 + s));
            }
        }
    }

    /// matplotlib's `Shapes`: rows of `shape` copies, offset by half a
    /// column on odd rows; unfilled shapes are drawn forward, then backward
    /// at 90% size.
    fn shapes(&mut self, rows: usize, shape: &Path, size: f64, filled: bool) {
        if rows == 0 {
            return;
        }
        let offset = 1.0 / rows as f64;
        let mut vertices: Vec<Point> = shape
            .vertices()
            .iter()
            .map(|v| (*v * offset) * size)
            .collect();
        let mut codes = shape.resolved_codes();
        if !filled {
            let backward: Vec<Point> = vertices.iter().rev().map(|v| *v * 0.9).collect();
            vertices.extend(backward);
            codes.extend_from_within(..);
        }
        for row in 0..=rows {
            let row_pos = row as f64 * offset;
            let cols: Vec<f64> = if row % 2 == 0 {
                linspace(0.0, 1.0, rows + 1, true).collect()
            } else {
                linspace(offset / 2.0, 1.0 - offset / 2.0, rows, true).collect()
            };
            for col in cols {
                let shift = Point::new(col, row_pos);
                self.vertices.extend(vertices.iter().map(|v| *v + shift));
                self.codes.extend_from_slice(&codes);
            }
        }
    }
}

/// The star used by the `*` hatch: `unit_regular_star(5)` with explicit
/// move/line/close codes.
fn star() -> Path {
    let vertices = Path::unit_regular_star(5, 0.5).vertices().to_vec();
    let n = vertices.len();
    let codes = (0..n)
        .map(|i| match i {
            0 => PathCode::MoveTo,
            _ if i == n - 1 => PathCode::ClosePoly,
            _ => PathCode::LineTo,
        })
        .collect();
    Path::new(vertices, codes).expect("star codes are well formed")
}

/// matplotlib's `hatch.get_path(pattern, density)`: the hatch path in the
/// unit square. Patterns without any hatch character give an empty path.
pub fn pattern_path(pattern: &str, density: i32) -> Result<Path, HatchError> {
    let density = usize::try_from(density).map_err(|_| HatchError::NegativeDensity)?;
    let count = |chars: &[char]| {
        pattern
            .chars()
            .filter(|c| chars.contains(c))
            .count()
            .saturating_mul(density)
    };
    let shape_rows = |c: char| {
        let rows = count(&[c]);
        if rows % 2 == 0 {
            Ok(rows)
        } else {
            Err(HatchError::OddShapeRows)
        }
    };
    let (small, large, dots, stars) = (
        shape_rows('o')?,
        shape_rows('O')?,
        shape_rows('.')?,
        shape_rows('*')?,
    );
    let lines = [
        count(&['-', '+']),
        count(&['|', '+']),
        count(&['/', 'x', 'X']),
        count(&['\\', 'x', 'X']),
    ];
    let circle = Path::unit_circle();
    let star = star();
    // An upper bound on the vertex count: two per line (diagonals have one
    // extra line), and up to `(rows + 1)²` copies of each shape, drawn twice
    // when unfilled.
    let shape_vertices = |rows: usize, shape: &Path, copies: usize| {
        let side = rows.saturating_add(1);
        side.saturating_mul(side)
            .saturating_mul(shape.len() * copies)
    };
    let estimate = lines
        .iter()
        .map(|n| n.saturating_add(1).saturating_mul(2))
        .chain([
            shape_vertices(small, &circle, 2),
            shape_vertices(large, &circle, 2),
            shape_vertices(dots, &circle, 1),
            shape_vertices(stars, &star, 1),
        ])
        .fold(0, usize::saturating_add);
    if estimate > MAX_VERTICES {
        return Err(HatchError::TooDense);
    }
    let mut b = Builder::default();
    b.straight_lines(lines[0], true);
    b.straight_lines(lines[1], false);
    b.diagonals(lines[2], true);
    b.diagonals(lines[3], false);
    b.shapes(small, &circle, 0.2, false);
    b.shapes(large, &circle, 0.35, false);
    b.shapes(dots, &circle, 0.1, true);
    b.shapes(stars, &star, 1.0 / 3.0, true);
    if b.vertices.is_empty() {
        return Ok(Path::default());
    }
    Ok(Path::new(b.vertices, b.codes).expect("hatch codes are well formed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_lines() {
        let path = pattern_path("-", 2).expect("valid");
        assert_eq!(
            path.vertices(),
            &[
                Point::new(0.0, 0.25),
                Point::new(1.0, 0.25),
                Point::new(0.0, 0.75),
                Point::new(1.0, 0.75)
            ]
        );
        assert_eq!(path.code_at(2), PathCode::MoveTo);
    }

    #[test]
    fn diagonals_include_both_corners() {
        let path = pattern_path("/", 2).expect("valid");
        assert_eq!(path.len(), 6);
        assert_eq!(path.vertices()[0], Point::new(-0.5, 0.5));
        assert_eq!(path.vertices()[5], Point::new(1.5, 0.5));
        assert_eq!(pattern_path("x", 1).expect("valid").len(), 8);
    }

    #[test]
    fn shapes_and_errors() {
        let circles = pattern_path("o", 2).expect("valid");
        // Rows 0 and 2 hold three circles, row 1 two; each is drawn twice.
        assert_eq!(circles.len(), 8 * 26 * 2);
        assert_eq!(pattern_path("o", 1), Err(HatchError::OddShapeRows));
        assert_eq!(pattern_path("-", -1), Err(HatchError::NegativeDensity));
        assert!(pattern_path("", 6).expect("valid").is_empty());
        assert!(pattern_path("/", 0).expect("valid").is_empty());
    }

    #[test]
    fn too_dense_patterns_are_rejected() {
        assert_eq!(pattern_path("/", i32::MAX), Err(HatchError::TooDense));
        assert_eq!(pattern_path("oo", 4096), Err(HatchError::TooDense));
        assert_eq!(
            pattern_path(&"x".repeat(1 << 20), 6),
            Err(HatchError::TooDense)
        );
        assert_eq!(pattern_path("/", 1000).expect("valid").len(), 2002);
    }
}
