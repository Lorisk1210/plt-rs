//! `figures/cases_mpl_tests_contour.py`: matplotlib's non-image tests of
//! `test_contour.py` that only check that a figure draws, in the test
//! suite's default style.

use pltrs::contour::{ClabelOptions, ContourOptions, QuadCoords, QuadField};
use pltrs::figure::SubfigureOptions;
use pltrs::fonts::FontSize;
use pltrs::numeric::{Grid, arange};

use super::mpl_tests::classic;
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        // `test_subfigure_clabel`: labelled contours of two Gaussians in
        // each of two subfigures.
        classic("mpl_tests_contour/subfigure_clabel", |fig| {
            let (x, y) = (arange(-3.0, 3.0, 0.025), arange(-2.0, 2.0, 0.025));
            let z = Grid::from_fn(y.len(), x.len(), |r, c| {
                let (x, y) = (x[c], y[r]);
                let z1 = (-(x * x) - y * y).exp();
                let z2 = (-((x - 1.0).powi(2)) - (y - 1.0).powi(2)).exp();
                (z1 - z2) * 2.0
            });
            let field = QuadField::new(z).with_coords(QuadCoords::Axes { x, y });
            for part in fig
                .subfigures(1, 2, SubfigureOptions::default())?
                .as_slice()
            {
                let ax = part.subplot();
                let cs = ax.contour(&field, ContourOptions::default())?;
                cs.clabel(ClabelOptions {
                    fontsize: Some(FontSize::Points(10.0)),
                    ..ClabelOptions::default()
                })?;
                ax.set_title("Simplest default with labels");
            }
            Ok(())
        }),
    ]
}
