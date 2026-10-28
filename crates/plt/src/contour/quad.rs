//! Contour generators for fields on quadrilateral grids: contourpy's
//! algorithms behind one type (matplotlib's `algorithm` keyword and
//! `contour.algorithm`).

use std::fmt;
use std::str::FromStr;

use plt_core::Path;

use super::ContourError;
use super::chunks::Chunks;
use super::mpl2005::Mpl2005;
use super::mpl2014::Mpl2014;
use super::serial::Serial;
use crate::numeric::Grid;

/// The contouring algorithm for quadrilateral grids (contourpy's
/// generators, as matplotlib names them).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContourAlgorithm {
    /// matplotlib's original algorithm, from 2005. It cannot mask corners.
    Mpl2005,
    /// The 2014 rewrite, matplotlib's default.
    #[default]
    Mpl2014,
    /// contourpy's own algorithm.
    Serial,
    /// contourpy's own algorithm contouring chunks in parallel threads.
    /// The contours are those of [`Serial`](Self::Serial): plt-rs traces
    /// the chunks in order.
    Threaded,
}

impl ContourAlgorithm {
    /// matplotlib's name for the algorithm.
    pub fn name(self) -> &'static str {
        match self {
            Self::Mpl2005 => "mpl2005",
            Self::Mpl2014 => "mpl2014",
            Self::Serial => "serial",
            Self::Threaded => "threaded",
        }
    }

    /// Whether the algorithm can keep the unmasked triangles of quads with
    /// one masked point.
    pub fn supports_corner_mask(self) -> bool {
        self != Self::Mpl2005
    }
}

impl fmt::Display for ContourAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for ContourAlgorithm {
    type Err = ContourError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        [Self::Mpl2005, Self::Mpl2014, Self::Serial, Self::Threaded]
            .into_iter()
            .find(|algorithm| algorithm.name() == name)
            .ok_or_else(|| ContourError::UnknownAlgorithm(name.to_owned()))
    }
}

/// The index of a grid entry from a signed index.
pub(super) fn slot(index: isize) -> usize {
    usize::try_from(index).expect("contours stay on the grid")
}

/// A grid size as a signed index.
pub(super) fn signed(size: usize) -> isize {
    isize::try_from(size).expect("grids fit in memory")
}

/// A validated grid: coordinates and values of `nx × ny` points, row by
/// row, with the point mask if anything is masked.
#[derive(Clone, Copy, Debug)]
pub(super) struct QuadGrid<'a> {
    pub(super) x: &'a [f64],
    pub(super) y: &'a [f64],
    pub(super) z: &'a [f64],
    pub(super) mask: Option<&'a [bool]>,
    pub(super) nx: usize,
    pub(super) ny: usize,
}

/// The generator of one algorithm.
#[derive(Clone, Debug)]
enum Engine {
    Mpl2005(Mpl2005),
    Mpl2014(Mpl2014),
    Serial(Serial),
}

/// Contour lines and filled contours of a field on a quadrilateral grid
/// (contourpy's contour generators with matplotlib's line and fill
/// types).
///
/// ```
/// use plt::contour::{ContourAlgorithm, QuadContourGenerator};
/// use plt::numeric::Grid;
///
/// let x = Grid::from_rows([[0.0, 1.0], [0.0, 1.0]])?;
/// let y = Grid::from_rows([[0.0, 0.0], [1.0, 1.0]])?;
/// let z = Grid::from_rows([[0.0, 1.0], [1.0, 2.0]])?;
/// let mut generator = QuadContourGenerator::new(&x, &y, &z, None, true, (0, 0))?;
/// assert_eq!(generator.algorithm(), ContourAlgorithm::Mpl2014);
/// let line = generator.lines(1.5);
/// assert_eq!(line.vertices().len(), 2);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct QuadContourGenerator {
    algorithm: ContourAlgorithm,
    corner_mask: bool,
    engine: Engine,
}

impl QuadContourGenerator {
    /// A generator using matplotlib's default algorithm,
    /// [`ContourAlgorithm::Mpl2014`]; see
    /// [`with_algorithm`](Self::with_algorithm).
    ///
    /// # Errors
    ///
    /// When the shapes disagree or the grid is smaller than 2 × 2.
    pub fn new(
        x: &Grid<f64>,
        y: &Grid<f64>,
        z: &Grid<f64>,
        mask: Option<&Grid<bool>>,
        corner_mask: bool,
        chunk_size: (usize, usize),
    ) -> Result<Self, ContourError> {
        Self::with_algorithm(
            ContourAlgorithm::Mpl2014,
            x,
            y,
            z,
            mask,
            corner_mask,
            chunk_size,
        )
    }

    /// A generator using `algorithm` for the field `z` at points `(x, y)`
    /// (all of the same shape, at least 2 × 2), with an optional point
    /// mask (`true` = masked).
    ///
    /// With `corner_mask`, a quad with exactly one masked point keeps the
    /// triangle opposite that point; otherwise any quad touching a masked
    /// point is left out. `chunk_size` is `(y, x)` in quads; 0 means no
    /// chunking in that direction.
    ///
    /// # Errors
    ///
    /// When the shapes disagree, the grid is smaller than 2 × 2, or
    /// corners are to be masked by an algorithm that cannot.
    pub fn with_algorithm(
        algorithm: ContourAlgorithm,
        x: &Grid<f64>,
        y: &Grid<f64>,
        z: &Grid<f64>,
        mask: Option<&Grid<bool>>,
        corner_mask: bool,
        chunk_size: (usize, usize),
    ) -> Result<Self, ContourError> {
        let (ny, nx) = z.shape();
        if ny < 2 || nx < 2 {
            return Err(ContourError::GridTooSmall { rows: ny, cols: nx });
        }
        let shapes = [x.shape(), y.shape()];
        if let Some(&shape) = shapes
            .iter()
            .chain(mask.map(Grid::shape).as_ref())
            .find(|&&s| s != (ny, nx))
        {
            return Err(ContourError::ShapeMismatch {
                expected: (ny, nx),
                actual: shape,
            });
        }
        if corner_mask && !algorithm.supports_corner_mask() {
            return Err(ContourError::CornerMaskUnsupported(algorithm));
        }
        let grid = QuadGrid {
            x: x.as_slice(),
            y: y.as_slice(),
            z: z.as_slice(),
            mask: mask.map(Grid::as_slice).filter(|m| m.contains(&true)),
            nx,
            ny,
        };
        let chunks = Chunks::new(nx, ny, chunk_size);
        let engine = match algorithm {
            ContourAlgorithm::Mpl2005 => Engine::Mpl2005(Mpl2005::new(&grid, chunks)),
            ContourAlgorithm::Mpl2014 => Engine::Mpl2014(Mpl2014::new(&grid, corner_mask, chunks)),
            ContourAlgorithm::Serial | ContourAlgorithm::Threaded => {
                Engine::Serial(Serial::new(&grid, corner_mask, chunks))
            }
        };
        Ok(Self {
            algorithm,
            corner_mask,
            engine,
        })
    }

    /// The algorithm the generator uses.
    pub fn algorithm(&self) -> ContourAlgorithm {
        self.algorithm
    }

    /// Whether quads with one masked point keep their opposite triangle.
    pub fn corner_mask(&self) -> bool {
        self.corner_mask
    }

    /// The contour lines at `level` as one compound path: each line starts
    /// with a MOVETO and loops end with a CLOSEPOLY when their last point
    /// is exactly their first (matplotlib's `LineType.SeparateCode`
    /// concatenated). No lines give an empty path without codes.
    pub fn lines(&mut self, level: f64) -> Path {
        match &mut self.engine {
            Engine::Mpl2005(engine) => engine.lines(level),
            Engine::Mpl2014(engine) => engine.lines(level),
            Engine::Serial(engine) => engine.lines(level),
        }
    }

    /// The filled contour `lower < z <= upper` as one compound path of
    /// closed rings: every outer polygon (anticlockwise) followed by its
    /// holes (clockwise), polygon by polygon and chunk by chunk
    /// (matplotlib's `FillType.OuterCode` concatenated). No polygons give an
    /// empty path without codes.
    ///
    /// # Errors
    ///
    /// When a level is NaN or `lower >= upper`.
    pub fn filled(&mut self, lower: f64, upper: f64) -> Result<Path, ContourError> {
        if lower.is_nan() || upper.is_nan() {
            return Err(ContourError::NanLevel);
        }
        if lower >= upper {
            return Err(ContourError::LevelOrder { lower, upper });
        }
        Ok(match &mut self.engine {
            Engine::Mpl2005(engine) => engine.filled(lower, upper),
            Engine::Mpl2014(engine) => engine.filled(lower, upper),
            Engine::Serial(engine) => engine.filled(lower, upper),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rc::RcParams;

    const ALGORITHMS: [ContourAlgorithm; 4] = [
        ContourAlgorithm::Mpl2005,
        ContourAlgorithm::Mpl2014,
        ContourAlgorithm::Serial,
        ContourAlgorithm::Threaded,
    ];

    #[test]
    fn algorithms_parse_from_their_names() {
        for algorithm in ALGORITHMS {
            assert_eq!(algorithm.name().parse::<ContourAlgorithm>(), Ok(algorithm));
            assert_eq!(algorithm.to_string(), algorithm.name());
        }
        assert_eq!(
            "fast".parse::<ContourAlgorithm>(),
            Err(ContourError::UnknownAlgorithm("fast".to_owned()))
        );
        let mut rc = RcParams::matplotlib();
        assert_eq!(rc.contour_algorithm, ContourAlgorithm::Mpl2014);
        rc.set("contour.algorithm", "serial").unwrap();
        assert_eq!(rc.contour_algorithm, ContourAlgorithm::Serial);
    }

    #[test]
    fn every_algorithm_finds_the_same_crossing() {
        let x = Grid::from_rows([[0.0, 1.0], [0.0, 1.0]]).unwrap();
        let y = Grid::from_rows([[0.0, 0.0], [1.0, 1.0]]).unwrap();
        let z = Grid::from_rows([[0.0, 1.0], [0.0, 1.0]]).unwrap();
        for algorithm in ALGORITHMS {
            let corner_mask = algorithm.supports_corner_mask();
            let mut generator = QuadContourGenerator::with_algorithm(
                algorithm,
                &x,
                &y,
                &z,
                None,
                corner_mask,
                (0, 0),
            )
            .unwrap();
            let mut xs: Vec<f64> = generator
                .lines(0.5)
                .vertices()
                .iter()
                .map(|p| p.x)
                .collect();
            xs.dedup();
            assert_eq!(xs, [0.5], "{algorithm}");
            assert_eq!(generator.filled(0.5, 2.0).unwrap().subpaths().len(), 1);
        }
    }

    #[test]
    fn invalid_inputs() {
        let square = Grid::filled(2, 2, 0.0);
        let mut generator =
            QuadContourGenerator::new(&square, &square, &square, None, true, (0, 0)).unwrap();
        assert_eq!(generator.filled(f64::NAN, 1.0), Err(ContourError::NanLevel));
        assert!(matches!(
            generator.filled(1.0, 1.0),
            Err(ContourError::LevelOrder { .. })
        ));
        let row = Grid::from_rows([[0.0, 1.0]]).unwrap();
        let small = QuadContourGenerator::new(&row, &row, &row, None, true, (0, 0));
        assert!(matches!(
            small,
            Err(ContourError::GridTooSmall { rows: 1, cols: 2 })
        ));
        let mask = Grid::filled(3, 2, false);
        let mismatch =
            QuadContourGenerator::new(&square, &square, &square, Some(&mask), true, (0, 0));
        assert!(matches!(mismatch, Err(ContourError::ShapeMismatch { .. })));
        let corners = QuadContourGenerator::with_algorithm(
            ContourAlgorithm::Mpl2005,
            &square,
            &square,
            &square,
            None,
            true,
            (0, 0),
        );
        assert_eq!(
            corners.err(),
            Some(ContourError::CornerMaskUnsupported(
                ContourAlgorithm::Mpl2005
            ))
        );
    }
}
