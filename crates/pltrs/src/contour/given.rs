//! Contour sets made from given segments instead of a field (matplotlib's
//! `ContourSet(ax, levels, allsegs, allkinds)`).

use pltrs_core::{Path, Point};

use super::ContourError;
use super::set::{ContourSource, Generator};

/// Paths given per level or band, handed out in the order the contour set
/// asks for them (levels, or bands between consecutive levels, ascending).
#[derive(Debug)]
pub(crate) struct GivenPaths(std::vec::IntoIter<Path>);

impl GivenPaths {
    fn next_path(&mut self) -> Path {
        self.0.next().unwrap_or_else(|| Path::polyline(Vec::new()))
    }
}

impl Generator for GivenPaths {
    fn lines(&mut self, _level: f64) -> Path {
        self.next_path()
    }

    fn filled(&mut self, _lower: f64, _upper: f64) -> Result<Path, ContourError> {
        Ok(self.next_path())
    }
}

/// The source of a contour set drawing `segments`: for each level (lines)
/// or band (`filled`), the segments joined into one compound path. z
/// spans the levels, and the data limits the segments' points.
///
/// # Errors
///
/// When there is not one list of segments per level (lines) or per band
/// (fills), or the segments have no points.
pub(crate) fn given_source(
    levels: &[f64],
    segments: &[Vec<Path>],
    filled: bool,
) -> Result<ContourSource<GivenPaths>, ContourError> {
    let expected = if filled {
        levels.len().saturating_sub(1)
    } else {
        levels.len()
    };
    if segments.len() != expected {
        return Err(ContourError::SegmentCount {
            expected,
            actual: segments.len(),
        });
    }
    let points = segments
        .iter()
        .flatten()
        .flat_map(|path| path.vertices().iter().copied());
    let data_lim = points
        .fold(None, |bounds: Option<(Point, Point)>, p| {
            Some(bounds.map_or((p, p), |(min, max)| {
                (
                    Point::new(min.x.min(p.x), min.y.min(p.y)),
                    Point::new(max.x.max(p.x), max.y.max(p.y)),
                )
            }))
        })
        .ok_or(ContourError::NoSegmentPoints)?;
    let paths: Vec<Path> = segments.iter().map(Path::compound).collect();
    Ok(ContourSource {
        generator: GivenPaths(paths.into_iter()),
        zmin: levels.iter().copied().reduce(f64::min),
        zmax: levels.iter().copied().reduce(f64::max),
        data_lim,
        boolean: false,
        warnings: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(points: &[(f64, f64)]) -> Path {
        Path::polyline(points.iter().map(|&(x, y)| Point::new(x, y)).collect())
    }

    #[test]
    fn segments_become_one_path_per_level_bounded_by_their_points() {
        let segments = vec![
            vec![line(&[(2.0, 0.0), (1.0, 2.0)])],
            vec![
                line(&[(3.0, 0.0), (3.0, 2.0)]),
                line(&[(3.0, 3.0), (3.0, 4.0)]),
            ],
        ];
        let mut source = given_source(&[1.0, 2.0], &segments, false).unwrap();
        assert_eq!((source.zmin, source.zmax), (Some(1.0), Some(2.0)));
        assert_eq!(
            source.data_lim,
            (Point::new(1.0, 0.0), Point::new(3.0, 4.0))
        );
        assert_eq!(source.generator.lines(1.0).len(), 2);
        assert_eq!(source.generator.lines(2.0).subpaths().len(), 2);
    }

    #[test]
    fn segment_lists_must_match_the_levels() {
        let one = || vec![vec![line(&[(0.0, 0.0), (1.0, 1.0)])]];
        assert_eq!(
            given_source(&[0.0, 1.0], &one(), false).unwrap_err(),
            ContourError::SegmentCount {
                expected: 2,
                actual: 1
            }
        );
        assert!(given_source(&[0.0, 1.0], &one(), true).is_ok());
        assert_eq!(
            given_source(&[0.0, 1.0], &[Vec::new()], true).unwrap_err(),
            ContourError::NoSegmentPoints
        );
    }
}
