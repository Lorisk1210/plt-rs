//! `figures/cases_mpl_tests_marker.py`: matplotlib's non-image tests of
//! `test_marker.py` that only check that a figure draws, in the test
//! suite's default style.

use plt::axes::ScatterOptions;
use plt::markers::{FillStyle, Marker, MarkerStyle, PolygonStyle};

use super::mpl_tests::classic;
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        // `test_star_marker`: five-pointed stars given without and with
        // their angle.
        classic("mpl_tests_marker/star_marker", |fig| {
            let ax = fig.subplot();
            let star = Marker::polygon(5, PolygonStyle::Star, 0.0);
            for at in [0.0, 1.0] {
                let options = ScatterOptions {
                    sizes: Some(vec![20.0 * 20.0]),
                    marker: Some(MarkerStyle::new(star.clone(), FillStyle::Full)?),
                    ..ScatterOptions::default()
                };
                ax.scatter_with(vec![at], vec![at], options)?;
            }
            ax.set_xlim(-0.5, 0.5)?.set_ylim(-0.5, 1.5)?;
            Ok(())
        }),
    ]
}
