//! Two-dimensional histograms (matplotlib's `hist2d`).

use super::{Axes, Mesh, PcolorOptions};
use crate::collections::Collection;
use crate::image::{ImageError, ScalarField};
use crate::numeric::Grid;
use crate::stats::{AxisBins, Histogram2d, Histogram2dOptions, histogram2d};

/// The most cells [`Axes::hist2d`] draws, its x bins times its y bins
/// (2048 × 2048): every cell is a quadrilateral of the mesh, which vector
/// formats write out one by one, so that a mesh of the
/// [`MAX_EQUAL_BINS`](crate::stats::MAX_EQUAL_BINS) bins a histogram may
/// count would take tens of gigabytes to save. matplotlib draws whatever
/// bins are asked for, until memory runs out.
pub const MAX_HIST2D_CELLS: usize = 1 << 22;

/// The number of bins of one dimension, as far as it is known before
/// counting.
fn bin_count(bins: &AxisBins) -> usize {
    match bins {
        AxisBins::Count(count) => *count,
        AxisBins::Edges(edges) => edges.len().saturating_sub(1),
    }
}

/// Options of [`Axes::hist2d`].
#[derive(Debug, Default)]
pub struct Hist2dOptions<'a> {
    /// Bins, ranges, density and weights.
    pub histogram: Histogram2dOptions<'a>,
    /// Cells with fewer counts are left blank.
    pub cmin: Option<f64>,
    /// Cells with more counts are left blank.
    pub cmax: Option<f64>,
    /// Colormap, norm and mesh properties of the drawn cells.
    pub mesh: PcolorOptions,
}

/// What [`Axes::hist2d`] computed and drew.
#[derive(Clone, Debug)]
pub struct Hist2d {
    /// Counts indexed `(x bin, y bin)`; blanked cells are NaN.
    pub counts: Grid<f64>,
    /// The x bin edges.
    pub xedges: Vec<f64>,
    /// The y bin edges.
    pub yedges: Vec<f64>,
    /// The mesh drawing the counts.
    pub mesh: Collection,
}

impl Axes {
    /// The histogram of the points `(x[i], y[i])` drawn as a pseudocolour
    /// mesh (matplotlib's `hist2d`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::Hist2dOptions;
    /// let ax = Figure::new().subplot();
    /// let h = ax.hist2d(&[0.0, 1.0, 1.0], &[0.0, 0.0, 1.0], Hist2dOptions::default())?;
    /// assert_eq!(h.counts.shape(), (10, 10));
    /// assert_eq!(ax.xlim(), (0.0, 1.0));
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the binning is invalid (see
    /// [`histogram2d`]), makes more than [`MAX_HIST2D_CELLS`] cells, or
    /// the colormap options are invalid.
    pub fn hist2d(
        &self,
        x: &[f64],
        y: &[f64],
        options: Hist2dOptions<'_>,
    ) -> Result<Hist2d, ImageError> {
        let [xbins, ybins] = &options.histogram.bins;
        let cells = bin_count(xbins).saturating_mul(bin_count(ybins));
        if cells > MAX_HIST2D_CELLS {
            return Err(ImageError::TooManyCells(cells));
        }
        let Histogram2d {
            mut counts,
            xedges,
            yedges,
        } = histogram2d(x, y, options.histogram)?;
        for count in counts.as_mut_slice() {
            let below = options.cmin.is_some_and(|cmin| *count < cmin);
            let above = options.cmax.is_some_and(|cmax| *count > cmax);
            if below || above {
                *count = f64::NAN;
            }
        }
        let values = ScalarField::new(counts.transposed());
        let mesh = self.pcolormesh(
            Mesh::from_coords(xedges.clone(), yedges.clone(), values),
            options.mesh,
        )?;
        Ok(Hist2d {
            counts,
            xedges,
            yedges,
            mesh,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn counts_outside_the_limits_are_blanked() {
        let ax = Figure::new().subplot();
        let options = Hist2dOptions {
            histogram: Histogram2dOptions {
                bins: [AxisBins::Count(2), AxisBins::Count(1)],
                ..Histogram2dOptions::default()
            },
            cmin: Some(2.0),
            ..Hist2dOptions::default()
        };
        let h = ax
            .hist2d(&[0.0, 0.0, 0.0, 2.0], &[0.0; 4], options)
            .unwrap();
        assert_eq!(h.counts[(0, 0)], 3.0);
        assert!(h.counts[(1, 0)].is_nan());
        assert_eq!(ax.xlim(), (0.0, 2.0));
        assert_eq!(ax.ylim(), (-0.5, 0.5));
    }

    #[test]
    fn meshes_of_too_many_cells_are_refused() {
        // Found by fuzzing: 2^20 × 12 cells took more than 16 GB to save
        // as PGF.
        let ax = Figure::new().subplot();
        let options = |x: usize, y: usize| Hist2dOptions {
            histogram: Histogram2dOptions {
                bins: [AxisBins::Count(x), AxisBins::Edges(vec![0.0; y + 1])],
                ..Histogram2dOptions::default()
            },
            ..Hist2dOptions::default()
        };
        let err = ax
            .hist2d(&[0.0, 1.0], &[0.0, 1.0], options(1 << 20, 12))
            .unwrap_err();
        assert_eq!(err, ImageError::TooManyCells(12 << 20));
        assert!(err.to_string().contains("12582912 cells"), "{err}");
        let err = ax
            .hist2d(&[0.0], &[0.0], options(usize::MAX, 2))
            .unwrap_err();
        assert_eq!(err, ImageError::TooManyCells(usize::MAX));
    }

    #[test]
    fn mismatched_samples_are_an_error() {
        let ax = Figure::new().subplot();
        assert!(
            ax.hist2d(&[0.0], &[0.0, 1.0], Hist2dOptions::default())
                .is_err()
        );
    }
}
