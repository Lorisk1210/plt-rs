//! Two-dimensional histograms (matplotlib's `hist2d`).

use super::{Axes, Mesh, PcolorOptions};
use crate::collections::Collection;
use crate::image::{ImageError, ScalarField};
use crate::numeric::Grid;
use crate::stats::{Histogram2d, Histogram2dOptions, histogram2d};

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
    /// [`histogram2d`]) or the colormap options
    /// are.
    pub fn hist2d(
        &self,
        x: &[f64],
        y: &[f64],
        options: Hist2dOptions<'_>,
    ) -> Result<Hist2d, ImageError> {
        let Histogram2d {
            mut counts,
            xedges,
            yedges,
        } = histogram2d(x, y, &options.histogram)?;
        for count in counts.as_mut_slice() {
            let below = options.cmin.is_some_and(|cmin| *count < cmin);
            let above = options.cmax.is_some_and(|cmax| *count > cmax);
            if below || above {
                *count = f64::NAN;
            }
        }
        let values = ScalarField::new(counts.transposed());
        let mesh = self.pcolormesh(
            Mesh::with_coords(xedges.clone(), yedges.clone(), values),
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
    use crate::axes::Hist2dOptions;
    use crate::figure::Figure;
    use crate::stats::{AxisBins, Histogram2dOptions};

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
    fn mismatched_samples_are_an_error() {
        let ax = Figure::new().subplot();
        assert!(
            ax.hist2d(&[0.0], &[0.0, 1.0], Hist2dOptions::default())
                .is_err()
        );
    }
}
