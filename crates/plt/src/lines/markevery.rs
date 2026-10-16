//! Marker subsampling (matplotlib's `Line2D.set_markevery`).

use std::fmt;

use plt_core::Point;

use crate::numeric::remainder;

/// Which data points of a line get a marker.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkEvery {
    /// Every `step`-th point from `start` (Python slice `[start::step]`;
    /// a plain integer `n` means `start = 0`).
    Every {
        /// First index (negative counts from the end).
        start: i64,
        /// Stride; must not be zero (negative walks backwards).
        step: i64,
    },
    /// A Python slice `[start:stop:step]`.
    Slice {
        /// Start index.
        start: Option<i64>,
        /// End index (exclusive).
        stop: Option<i64>,
        /// Stride.
        step: Option<i64>,
    },
    /// Explicit indices (negative count from the end).
    Indices(Vec<i64>),
    /// One flag per data point.
    Mask(Vec<bool>),
    /// Points spaced evenly *along the line on screen*: every `step` times
    /// the axes diagonal, starting `start` diagonals in.
    Distance {
        /// Offset of the first marker, in axes diagonals.
        start: f64,
        /// Spacing between markers, in axes diagonals.
        step: f64,
    },
}

/// An unusable `markevery` value for the line it is applied to.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkEveryError {
    /// A slice or stride step of zero.
    ZeroStep,
    /// A non-positive distance step.
    NonPositiveDistance(f64),
    /// An index outside the data.
    IndexOutOfRange(i64),
    /// A distance-based selection without the axes geometry it is
    /// measured against.
    NeedsGeometry,
    /// A distance-based selection on a line without a finite vertex.
    NoFiniteVertices,
    /// A mask whose length differs from the data length.
    MaskLength {
        /// Mask length.
        mask: usize,
        /// Number of points.
        points: usize,
    },
}

impl fmt::Display for MarkEveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroStep => f.write_str("markevery step cannot be zero"),
            Self::NonPositiveDistance(step) => {
                write!(f, "markevery distance step {step} must be positive")
            }
            Self::IndexOutOfRange(i) => write!(f, "markevery index {i} is out of range"),
            Self::NeedsGeometry => f.write_str("a distance-based selection needs the line's axes"),
            Self::NoFiniteVertices => {
                f.write_str("a distance-based selection needs a finite vertex")
            }
            Self::MaskLength { mask, points } => {
                write!(
                    f,
                    "markevery mask has {mask} entries but the line has {points} points"
                )
            }
        }
    }
}

impl std::error::Error for MarkEveryError {}

impl MarkEvery {
    /// The indices (into `points`) that receive markers, in drawing order.
    ///
    /// `points` are the line's vertices in display coordinates and
    /// `axes_diagonal` the length of the diagonal of the line's axes in
    /// pixels (`None` outside an axes); both are only used by
    /// [`MarkEvery::Distance`].
    pub fn select(
        &self,
        points: &[Point],
        axes_diagonal: Option<f64>,
    ) -> Result<Vec<usize>, MarkEveryError> {
        match self {
            Self::Distance { start, step } => {
                let diagonal = axes_diagonal.ok_or(MarkEveryError::NeedsGeometry)?;
                by_distance(points, *start, *step, diagonal)
            }
            _ => self.indices(points.len()),
        }
    }

    /// The selected indices among `n` items, for the variants that do not
    /// depend on screen geometry (also matplotlib's `errorevery`).
    ///
    /// # Errors
    ///
    /// For invalid selections and for [`MarkEvery::Distance`].
    pub fn indices(&self, n: usize) -> Result<Vec<usize>, MarkEveryError> {
        match self {
            Self::Every { start, step } => slice_indices(n, Some(*start), None, Some(*step)),
            Self::Slice { start, stop, step } => slice_indices(n, *start, *stop, *step),
            Self::Indices(indices) => indices
                .iter()
                .map(|&i| {
                    let resolved = if i < 0 { i + n as i64 } else { i };
                    usize::try_from(resolved)
                        .ok()
                        .filter(|&r| r < n)
                        .ok_or(MarkEveryError::IndexOutOfRange(i))
                })
                .collect(),
            Self::Mask(mask) if mask.len() == n => Ok(mask
                .iter()
                .enumerate()
                .filter_map(|(i, &on)| on.then_some(i))
                .collect()),
            Self::Mask(mask) => Err(MarkEveryError::MaskLength {
                mask: mask.len(),
                points: n,
            }),
            Self::Distance { .. } => Err(MarkEveryError::NeedsGeometry),
        }
    }
}

/// Python's `slice(start, stop, step).indices(n)` expanded to indices.
fn slice_indices(
    n: usize,
    start: Option<i64>,
    stop: Option<i64>,
    step: Option<i64>,
) -> Result<Vec<usize>, MarkEveryError> {
    let step = step.unwrap_or(1);
    if step == 0 {
        return Err(MarkEveryError::ZeroStep);
    }
    let len = n as i64;
    let (lower, upper) = if step > 0 { (0, len) } else { (-1, len - 1) };
    let clamp = |i: i64| {
        if i < 0 {
            (i + len).max(lower)
        } else {
            i.min(upper)
        }
    };
    let start = start.map_or(if step > 0 { 0 } else { upper }, clamp);
    let stop = stop.map_or(if step > 0 { len } else { lower }, clamp);
    let mut indices = Vec::new();
    let mut i = start;
    while (step > 0 && i < stop) || (step < 0 && i > stop) {
        indices.push(i as usize);
        i += step;
    }
    Ok(indices)
}

/// matplotlib's float `markevery` (`_mark_every_path`): markers sit every
/// `step` axes diagonals along the cumulative on-screen length of the
/// finite vertices, from `start` diagonals in, and each picks its nearest
/// vertex (the earlier one on ties, the first of coincident ones).
///
/// Only the markers immediately before and after each vertex can select
/// it, so those are the only candidates evaluated, as in matplotlib.
fn by_distance(
    points: &[Point],
    start: f64,
    step: f64,
    diagonal: f64,
) -> Result<Vec<usize>, MarkEveryError> {
    let finite: Vec<usize> = (0..points.len())
        .filter(|&i| points[i].is_finite())
        .collect();
    let mut total = 0.0;
    let along: Vec<f64> = finite
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            if k > 0 {
                let d = points[i] - points[finite[k - 1]];
                total += d.x.hypot(d.y);
            }
            total
        })
        .collect();
    let &end = along.last().ok_or(MarkEveryError::NoFiniteVertices)?;
    let (marker_start, marker_step) = (start * diagonal, step * diagonal);
    if marker_step <= 0.0 {
        return Err(MarkEveryError::NonPositiveDistance(step));
    }
    let first_reaching = |target: f64| along.partition_point(|&d| d < target);
    let last = along.len() - 1;
    let mut chosen: Vec<usize> = along
        .iter()
        .map(|&d| d - remainder(d - marker_start, marker_step))
        .flat_map(|marker| [marker, marker + marker_step])
        .filter(|&marker| marker >= marker_start && marker < end)
        .map(|marker| {
            let after = first_reaching(marker);
            let before = after.saturating_sub(1);
            let after = after.min(last);
            let nearest = if (along[after] - marker).abs() < (marker - along[before]).abs() {
                after
            } else {
                before
            };
            first_reaching(along[nearest])
        })
        .collect();
    chosen.sort_unstable();
    chosen.dedup();
    Ok(chosen.into_iter().map(|k| finite[k]).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: usize) -> Vec<Point> {
        (0..n).map(|i| Point::new(i as f64 * 10.0, 0.0)).collect()
    }

    #[test]
    fn strides_and_slices() {
        let pts = line(7);
        assert_eq!(
            MarkEvery::Every { start: 0, step: 3 }.select(&pts, None),
            Ok(vec![0, 3, 6])
        );
        assert_eq!(
            MarkEvery::Every { start: -3, step: 1 }.select(&pts, None),
            Ok(vec![4, 5, 6])
        );
        assert_eq!(
            MarkEvery::Every { start: 1, step: -1 }.select(&pts, None),
            Ok(vec![1, 0])
        );
        assert_eq!(
            MarkEvery::Every { start: 0, step: 0 }.select(&pts, None),
            Err(MarkEveryError::ZeroStep)
        );
        let reversed = MarkEvery::Slice {
            start: None,
            stop: None,
            step: Some(-3),
        };
        assert_eq!(reversed.select(&pts, None), Ok(vec![6, 3, 0]));
        let window = MarkEvery::Slice {
            start: Some(2),
            stop: Some(6),
            step: Some(2),
        };
        assert_eq!(window.select(&pts, None), Ok(vec![2, 4]));
    }

    #[test]
    fn indices_and_masks() {
        let pts = line(4);
        assert_eq!(
            MarkEvery::Indices(vec![-1, 0, 0]).select(&pts, None),
            Ok(vec![3, 0, 0])
        );
        assert_eq!(
            MarkEvery::Indices(vec![9]).select(&pts, None),
            Err(MarkEveryError::IndexOutOfRange(9))
        );
        assert_eq!(
            MarkEvery::Mask(vec![true, false, false, true]).select(&pts, None),
            Ok(vec![0, 3])
        );
        assert!(MarkEvery::Mask(vec![true]).select(&pts, None).is_err());
    }

    #[test]
    fn distance_along_screen() {
        let pts = line(11); // 100 px long
        // Targets 0, 25, 50, 75 px; ties (25 between 20 and 30) pick the
        // first vertex, like numpy's argmin.
        let every_quarter = MarkEvery::Distance {
            start: 0.0,
            step: 0.25,
        };
        assert_eq!(
            every_quarter.select(&pts, Some(100.0)),
            Ok(vec![0, 2, 5, 7])
        );
        assert_eq!(
            every_quarter.select(&pts, None),
            Err(MarkEveryError::NeedsGeometry)
        );
        assert_eq!(
            MarkEvery::Distance {
                start: 0.0,
                step: 0.0
            }
            .select(&pts, Some(100.0)),
            Err(MarkEveryError::NonPositiveDistance(0.0))
        );
        // The last vertex is only picked by a marker strictly before it.
        let single = MarkEvery::Distance {
            start: 0.0,
            step: 0.5,
        };
        assert_eq!(single.select(&pts[..1], Some(100.0)), Ok(vec![]));
        assert_eq!(
            single.select(&[Point::new(f64::NAN, 0.0)], Some(100.0)),
            Err(MarkEveryError::NoFiniteVertices)
        );
        let gappy = vec![
            Point::new(0.0, 0.0),
            Point::new(f64::NAN, 0.0),
            Point::new(10.0, 0.0),
        ];
        assert_eq!(
            MarkEvery::Distance {
                start: 0.0,
                step: 0.08
            }
            .select(&gappy, Some(100.0)),
            Ok(vec![0, 2])
        );
    }
}
