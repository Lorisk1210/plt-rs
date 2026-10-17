//! Discrete normalisation into colour bins (`BoundaryNorm`).

use super::{Limits, Norm, NormError, NormKind};
use crate::colors::npmath::{np_clip, np_less};
use crate::colors::{Extend, Normalized};

/// Maps data to colour *indices* by the bin it falls in (matplotlib's
/// `BoundaryNorm`): bin `i` is `boundaries[i] <= x < boundaries[i + 1]`.
///
/// When there are fewer bins (plus extensions) than colours, bin indices
/// spread over the whole colormap. Values below the first boundary give
/// `-1` (*under*) and from the last boundary on `ncolors` (*over*), unless
/// clipping maps them to the end colours.
///
/// ```
/// use plt::colors::{BoundaryNorm, Extend, Norm, Normalized::Index};
/// let mut norm = BoundaryNorm::new(vec![0.0, 1.0, 2.0, 5.0, 10.0], 4, Extend::Neither)?;
/// assert_eq!(norm.normalize(&[-1.0, 0.5, 7.0, 10.0])?, [Index(-1), Index(0), Index(3), Index(4)]);
/// # Ok::<(), plt::colors::NormError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryNorm {
    boundaries: Vec<f64>,
    ncolors: usize,
    extend: Extend,
    limits: Limits,
}

impl BoundaryNorm {
    /// A norm with the given bin edges (at least two, non-decreasing) for a
    /// colormap of `ncolors` colours; `extend` adds bins below and/or above.
    pub fn new(
        boundaries: impl Into<Vec<f64>>,
        ncolors: usize,
        extend: Extend,
    ) -> Result<Self, NormError> {
        let boundaries = boundaries.into();
        let (Some(&first), Some(&last), 2..) =
            (boundaries.first(), boundaries.last(), boundaries.len())
        else {
            return Err(NormError::TooFewBoundaries(boundaries.len()));
        };
        if boundaries.windows(2).any(|w| w[1] < w[0]) {
            return Err(NormError::UnsortedBoundaries);
        }
        let norm = Self {
            boundaries,
            ncolors,
            extend,
            limits: Limits::new(Some(first), Some(last)),
        };
        let bins = norm.regions();
        if bins > ncolors {
            return Err(NormError::TooManyBins { bins, ncolors });
        }
        Ok(norm)
    }

    /// The same norm, clipping out-of-range values to the end colours;
    /// incompatible with extensions.
    pub fn with_clip(mut self, clip: bool) -> Result<Self, NormError> {
        if clip && self.extend != Extend::Neither {
            return Err(NormError::ClipWithExtend);
        }
        self.limits.clip = clip;
        Ok(self)
    }

    /// The bin edges.
    pub fn boundaries(&self) -> &[f64] {
        &self.boundaries
    }

    /// The number of colours mapped onto.
    pub fn ncolors(&self) -> usize {
        self.ncolors
    }

    /// The extensions beyond the boundaries.
    pub fn extend(&self) -> Extend {
        self.extend
    }

    /// Bins including extensions.
    fn regions(&self) -> usize {
        self.boundaries.len() - 1
            + usize::from(self.extend.extends_min())
            + usize::from(self.extend.extends_max())
    }

    /// The colour index of one value.
    fn index(&self, x: f64, clip: bool) -> i64 {
        let (lo, hi) = (self.limits.vmin, self.limits.vmax);
        let x = match (clip, lo, hi) {
            (true, Some(lo), Some(hi)) => np_clip(x, lo, hi),
            _ => x,
        };
        let max_col = if clip { self.ncolors - 1 } else { self.ncolors } as i64;
        if lo.is_some_and(|lo| x < lo) {
            return -1;
        }
        if hi.is_some_and(|hi| x >= hi) {
            return max_col;
        }
        // np.digitize(x, boundaries) - 1 + offset, NaN sorting last.
        let bin = self.boundaries.partition_point(|&b| !np_less(x, b)) as i64 - 1
            + i64::from(self.extend.extends_min());
        let (regions, ncolors) = (self.regions() as i64, self.ncolors as i64);
        if ncolors <= regions {
            bin
        } else if regions == 1 {
            if bin == 0 { (ncolors - 1) / 2 } else { bin }
        } else {
            ((ncolors - 1) as f64 / (regions - 1) as f64 * bin as f64) as i64
        }
    }
}

impl Norm for BoundaryNorm {
    fn vmin(&self) -> Option<f64> {
        self.limits.vmin
    }

    fn vmax(&self) -> Option<f64> {
        self.limits.vmax
    }

    fn set_vmin(&mut self, vmin: Option<f64>) {
        self.limits.vmin = vmin;
    }

    fn set_vmax(&mut self, vmax: Option<f64>) {
        self.limits.vmax = vmax;
    }

    fn clip(&self) -> bool {
        self.limits.clip
    }

    fn autoscale_none(&mut self, data: &[Option<f64>]) {
        self.limits.autoscale_none(data);
    }

    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        let clip = clip.unwrap_or(self.limits.clip);
        Ok(data
            .iter()
            .map(|v| {
                v.map_or(Normalized::Masked, |x| {
                    Normalized::Index(self.index(x, clip))
                })
            })
            .collect())
    }

    /// Always fails: binning loses the position within a bin.
    fn inverse(&self, _values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        Err(NormError::NotInvertible)
    }

    fn kind(&self) -> NormKind<'_> {
        NormKind::Boundary(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Normalized::{Index, Masked};

    const EDGES: [f64; 5] = [0.0, 1.0, 2.0, 5.0, 10.0];

    #[test]
    fn spreads_bins_over_colours() {
        let mut norm = BoundaryNorm::new(EDGES, 256, Extend::Neither).unwrap();
        assert_eq!(
            norm.normalize(&[-1.0, 0.5, 1.0, 2.0, 5.0, 10.0, f64::NAN]),
            Ok(vec![
                Index(-1),
                Index(0),
                Index(85),
                Index(170),
                Index(255),
                Index(256),
                Index(340)
            ])
        );
        let mut both = BoundaryNorm::new(EDGES, 6, Extend::Both).unwrap();
        assert_eq!(both.normalize(&[0.5, 9.0]), Ok(vec![Index(1), Index(4)]));
    }

    #[test]
    fn clipping_masks_and_single_region() {
        let mut norm = BoundaryNorm::new(EDGES, 4, Extend::Neither)
            .unwrap()
            .with_clip(true)
            .unwrap();
        assert_eq!(norm.normalize(&[-1.0, 11.0]), Ok(vec![Index(0), Index(3)]));
        assert_eq!(
            norm.normalize_masked(&[None, Some(0.5)], Some(false)),
            Ok(vec![Masked, Index(0)])
        );
        let mut one = BoundaryNorm::new([0.0, 1.0], 5, Extend::Neither).unwrap();
        assert_eq!(one.normalize(&[0.5]), Ok(vec![Index(2)]));
    }

    #[test]
    fn validation() {
        assert_eq!(
            BoundaryNorm::new([0.0], 4, Extend::Neither),
            Err(NormError::TooFewBoundaries(1))
        );
        assert_eq!(
            BoundaryNorm::new(EDGES, 3, Extend::Neither),
            Err(NormError::TooManyBins {
                bins: 4,
                ncolors: 3
            })
        );
        assert_eq!(
            BoundaryNorm::new(EDGES, 8, Extend::Min)
                .unwrap()
                .with_clip(true),
            Err(NormError::ClipWithExtend)
        );
        assert_eq!(
            BoundaryNorm::new(EDGES, 4, Extend::Neither)
                .unwrap()
                .inverse(&[0.5]),
            Err(NormError::NotInvertible)
        );
    }
}
