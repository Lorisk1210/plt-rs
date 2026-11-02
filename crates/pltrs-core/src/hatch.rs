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
//!
//! Each piece is one of matplotlib's hatch pattern classes: a family of
//! lines ([`HatchLines`]: `HorizontalHatch`, `VerticalHatch`,
//! `NorthEastHatch`, `SouthEastHatch`) or rows of a shape
//! ([`HatchShapes`]: `SmallCircles`, `LargeCircles`, `SmallFilledCircles`,
//! `Stars`, and any shape of the caller's, as a subclass of `Shapes`).

use std::fmt;

use crate::Point;
use crate::numeric::linspace_with;
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

/// How many times `pattern` holds any of `chars`, times `density`.
fn count(pattern: &str, chars: &[char], density: usize) -> usize {
    pattern
        .chars()
        .filter(|c| chars.contains(c))
        .count()
        .saturating_mul(density)
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

    fn path(self) -> Path {
        if self.vertices.is_empty() {
            return Path::default();
        }
        Path::new(self.vertices, self.codes).expect("hatch codes are well formed")
    }
}

/// A family of straight hatch lines (matplotlib's `HorizontalHatch`,
/// `VerticalHatch`, `NorthEastHatch` and `SouthEastHatch`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HatchLines {
    /// Horizontal lines, for `-` and `+`.
    Horizontal,
    /// Vertical lines, for `|` and `+`.
    Vertical,
    /// Rising diagonals, for `/`, `x` and `X`.
    NorthEast,
    /// Falling diagonals, for `\`, `x` and `X`.
    SouthEast,
}

impl HatchLines {
    /// The families in the order a pattern draws them.
    pub const ALL: [Self; 4] = [
        Self::Horizontal,
        Self::Vertical,
        Self::NorthEast,
        Self::SouthEast,
    ];

    /// The pattern characters that ask for these lines.
    pub const fn characters(self) -> &'static [char] {
        match self {
            Self::Horizontal => &['-', '+'],
            Self::Vertical => &['|', '+'],
            Self::NorthEast => &['/', 'x', 'X'],
            Self::SouthEast => &['\\', 'x', 'X'],
        }
    }

    /// How many lines `pattern` asks for at `density` (matplotlib's
    /// `num_lines`).
    pub fn count(self, pattern: &str, density: usize) -> usize {
        count(pattern, self.characters(), density)
    }

    /// The number of vertices of `lines` lines (matplotlib's
    /// `num_vertices`): two per line, and one line more for the
    /// diagonals, which run through both corners.
    pub fn vertex_count(self, lines: usize) -> usize {
        match self {
            Self::Horizontal | Self::Vertical => lines.saturating_mul(2),
            Self::NorthEast | Self::SouthEast if lines == 0 => 0,
            Self::NorthEast | Self::SouthEast => lines.saturating_add(1).saturating_mul(2),
        }
    }

    /// `lines` lines across the unit tile, each a move and a line
    /// (matplotlib's `set_vertices_and_codes`).
    ///
    /// ```
    /// use pltrs_core::hatch::HatchLines;
    /// use pltrs_core::Point;
    /// let path = HatchLines::Vertical.path(2);
    /// assert_eq!(path.vertices()[..2], [Point::new(0.25, 0.0), Point::new(0.25, 1.0)]);
    /// assert_eq!(path.len(), HatchLines::Vertical.vertex_count(2));
    /// ```
    pub fn path(self, lines: usize) -> Path {
        let mut b = Builder::default();
        self.draw(lines, &mut b);
        b.path()
    }

    fn draw(self, lines: usize, b: &mut Builder) {
        if lines == 0 {
            return;
        }
        match self {
            Self::Horizontal | Self::Vertical => {
                let step = 1.0 / lines as f64;
                let horizontal = self == Self::Horizontal;
                for s in linspace_with(0.0, 1.0, lines, false)
                    .into_iter()
                    .map(|s| s + step / 2.0)
                {
                    if horizontal {
                        b.segment(Point::new(0.0, s), Point::new(1.0, s));
                    } else {
                        b.segment(Point::new(s, 0.0), Point::new(s, 1.0));
                    }
                }
            }
            Self::NorthEast | Self::SouthEast => {
                let rising = self == Self::NorthEast;
                for s in linspace_with(-0.5, 0.5, lines + 1, true) {
                    if rising {
                        b.segment(Point::new(0.0 + s, 0.0 - s), Point::new(1.0 + s, 1.0 - s));
                    } else {
                        b.segment(Point::new(0.0 + s, 1.0 + s), Point::new(1.0 + s, 0.0 + s));
                    }
                }
            }
        }
    }
}

/// Rows of a shape across the unit tile (matplotlib's `Shapes`): copies
/// of `shape`, scaled by `size` over the row spacing, offset by half a
/// column on odd rows; an unfilled shape is drawn forward, then backward
/// at 90% of its size.
#[derive(Clone, Debug, PartialEq)]
pub struct HatchShapes {
    /// The shape, around the origin in a unit box.
    pub shape: Path,
    /// The shape's size as a fraction of the row spacing.
    pub size: f64,
    /// Whether the shape is filled.
    pub filled: bool,
    /// The pattern character that asks for a row (`None` for a shape of
    /// the caller's).
    pub character: Option<char>,
}

impl HatchShapes {
    /// Small circles, for `o` (matplotlib's `SmallCircles`).
    pub fn small_circles() -> Self {
        Self::circles(0.2, false, 'o')
    }

    /// Large circles, for `O` (matplotlib's `LargeCircles`).
    pub fn large_circles() -> Self {
        Self::circles(0.35, false, 'O')
    }

    /// Small filled circles, for `.` (matplotlib's `SmallFilledCircles`).
    pub fn small_filled_circles() -> Self {
        Self::circles(0.1, true, '.')
    }

    /// Filled five-pointed stars, for `*` (matplotlib's `Stars`).
    pub fn stars() -> Self {
        Self {
            shape: star(),
            size: 1.0 / 3.0,
            filled: true,
            character: Some('*'),
        }
    }

    fn circles(size: f64, filled: bool, character: char) -> Self {
        Self {
            shape: Path::unit_circle(),
            size,
            filled,
            character: Some(character),
        }
    }

    /// The built-in shapes in the order a pattern draws them.
    pub fn all() -> [Self; 4] {
        [
            Self::small_circles(),
            Self::large_circles(),
            Self::small_filled_circles(),
            Self::stars(),
        ]
    }

    /// How many rows `pattern` asks for at `density` (matplotlib's
    /// `num_rows`; none for a shape of the caller's).
    pub fn count(&self, pattern: &str, density: usize) -> usize {
        self.character.map_or(0, |c| count(pattern, &[c], density))
    }

    /// The number of shapes in `rows` rows (matplotlib's `num_shapes`),
    /// counting even rows of `rows + 1` and odd rows of `rows`.
    pub fn shape_count(rows: usize) -> usize {
        if rows == 0 {
            return 0;
        }
        (rows / 2 + 1)
            .saturating_mul(rows + 1)
            .saturating_add((rows / 2).saturating_mul(rows))
    }

    /// The number of vertices of `rows` rows (matplotlib's
    /// `num_vertices`).
    pub fn vertex_count(&self, rows: usize) -> usize {
        let per_shape = self.shape.len() * if self.filled { 1 } else { 2 };
        Self::shape_count(rows).saturating_mul(per_shape)
    }

    /// The shapes of `rows` rows (matplotlib's `set_vertices_and_codes`).
    /// An odd number of rows holds one shape more than
    /// [`HatchShapes::vertex_count`] counts, as in matplotlib.
    ///
    /// ```
    /// use pltrs_core::hatch::HatchShapes;
    /// let dots = HatchShapes::small_filled_circles();
    /// assert_eq!(dots.path(2).len(), dots.vertex_count(2));
    /// ```
    pub fn path(&self, rows: usize) -> Path {
        let mut b = Builder::default();
        self.draw(rows, &mut b);
        b.path()
    }

    fn draw(&self, rows: usize, b: &mut Builder) {
        if rows == 0 {
            return;
        }
        let offset = 1.0 / rows as f64;
        let mut vertices: Vec<Point> = self
            .shape
            .vertices()
            .iter()
            .map(|v| (*v * offset) * self.size)
            .collect();
        let mut codes = self.shape.resolved_codes();
        if !self.filled {
            let backward: Vec<Point> = vertices.iter().rev().map(|v| *v * 0.9).collect();
            vertices.extend(backward);
            codes.extend_from_within(..);
        }
        for row in 0..=rows {
            let row_pos = row as f64 * offset;
            let cols: Vec<f64> = if row % 2 == 0 {
                linspace_with(0.0, 1.0, rows + 1, true)
            } else {
                linspace_with(offset / 2.0, 1.0 - offset / 2.0, rows, true)
            };
            for col in cols {
                let shift = Point::new(col, row_pos);
                b.vertices.extend(vertices.iter().map(|v| *v + shift));
                b.codes.extend_from_slice(&codes);
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
    let lines = HatchLines::ALL.map(|family| (family, family.count(pattern, density)));
    let shapes = HatchShapes::all().map(|shapes| {
        let rows = shapes.count(pattern, density);
        (shapes, rows)
    });
    if shapes.iter().any(|(_, rows)| rows % 2 == 1) {
        return Err(HatchError::OddShapeRows);
    }
    // An upper bound on the vertex count: two per line (diagonals have one
    // extra line), and up to `(rows + 1)²` copies of each shape, drawn twice
    // when unfilled.
    let estimate = lines
        .iter()
        .map(|&(_, n)| n.saturating_add(1).saturating_mul(2))
        .chain(shapes.iter().map(|(shapes, rows)| {
            let side = rows.saturating_add(1);
            let copies = if shapes.filled { 1 } else { 2 };
            side.saturating_mul(side)
                .saturating_mul(shapes.shape.len() * copies)
        }))
        .fold(0, usize::saturating_add);
    if estimate > MAX_VERTICES {
        return Err(HatchError::TooDense);
    }
    let mut b = Builder::default();
    for (family, n) in lines {
        family.draw(n, &mut b);
    }
    for (shapes, rows) in &shapes {
        shapes.draw(*rows, &mut b);
    }
    Ok(b.path())
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

    #[test]
    fn pieces_count_as_matplotlibs_classes() {
        assert_eq!(HatchLines::Horizontal.count("-+|", 3), 6);
        assert_eq!(HatchLines::SouthEast.count("\\x", 2), 4);
        assert_eq!(HatchLines::NorthEast.vertex_count(0), 0);
        assert_eq!(HatchLines::NorthEast.vertex_count(4), 10);
        assert_eq!(HatchShapes::shape_count(2), 8);
        assert_eq!(HatchShapes::shape_count(0), 0);
        let stars = HatchShapes::stars();
        assert_eq!(stars.count("**", 3), 6);
        assert_eq!(stars.vertex_count(2), 8 * 11);
        assert_eq!(HatchShapes::small_circles().vertex_count(2), 8 * 26 * 2);
        // An odd row count makes one shape more than counted, as in
        // matplotlib, whose arrays then refuse it.
        assert_eq!(stars.path(1).len(), 3 * 11);
    }

    #[test]
    fn the_pattern_is_its_pieces_in_order() {
        let pattern = "-|/\\oO.*";
        let whole = pattern_path(pattern, 2).expect("valid");
        let mut vertices = Vec::new();
        for family in HatchLines::ALL {
            vertices.extend_from_slice(family.path(family.count(pattern, 2)).vertices());
        }
        for shapes in HatchShapes::all() {
            vertices.extend_from_slice(shapes.path(shapes.count(pattern, 2)).vertices());
        }
        assert_eq!(whole.vertices(), vertices);
    }

    #[test]
    fn shapes_of_the_callers_own() {
        let square = Path::closed(
            [
                (-0.5, -0.5),
                (0.5, -0.5),
                (0.5, 0.5),
                (-0.5, 0.5),
                (0.0, 0.0),
            ]
            .map(Point::from)
            .to_vec(),
        );
        let shapes = HatchShapes {
            shape: square,
            size: 0.5,
            filled: false,
            character: None,
        };
        assert_eq!(shapes.count("oooo", 6), 0);
        let path = shapes.path(2);
        assert_eq!(path.len(), shapes.vertex_count(2));
        assert_eq!(path.vertices()[0], Point::new(-0.125, -0.125));
    }
}
