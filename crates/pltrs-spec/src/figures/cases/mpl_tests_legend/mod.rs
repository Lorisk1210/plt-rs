//! `figures/cases_mpl_tests_legend.py`: matplotlib's non-image tests from
//! `test_legend.py` that only draw, in the test suite's default style: the
//! legend of a fill whose edges take the face colour, and the legends of
//! a scatter of one NaN point and of three scatters of random points.

use pltrs::axes::{FillBetweenOptions, ScatterColors, ScatterOptions};
use pltrs::collections::EdgeColors;
use pltrs::colors::{Alpha, Color};
use pltrs::legend::LegendOptions;
use pltrs::prelude::*;

use super::mpl_tests::classic;
use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        classic("mpl_tests_legend/legend_face_edgecolor", |fig| {
            let ax = fig.subplot();
            let mut options = FillBetweenOptions::default();
            options.props = options.props.facecolor("r");
            options.props.edgecolors = Some(EdgeColors::Face);
            options.props.base.label = Some("Fill".to_owned());
            ax.fill_between([0.0, 1.0, 2.0], [1.0, 2.0, 3.0], [2.0, 3.0, 4.0], options)?;
            ax.legend(LegendOptions::default())?;
            Ok(())
        }),
        classic("mpl_tests_legend/nanscatter", |fig| {
            let ax = fig.subplot();
            let options = ScatterOptions {
                sizes: Some(vec![3.0]),
                colors: ScatterColors::Single(Color::from("r")),
                marker: Some("o".parse()?),
                edgecolors: Some(EdgeColors::Colors(vec![Color::from("r")])),
                ..ScatterOptions::default()
            };
            let dot = ax.scatter_with(vec![f64::NAN], vec![f64::NAN], options)?;
            ax.legend(LegendOptions::default().handles([&dot]).labels(["scatter"]))?;
            Ok(())
        }),
        classic("mpl_tests_legend/nanscatter_colors", |fig| {
            let ax = fig.subplot();
            let mut rng = SpecRng::new(0);
            let n = 750;
            for color in ["red", "green", "blue"] {
                let xy = rng.uniform(2 * n, 0.0, 1.0);
                let sizes = rng.uniform(n, 0.0, 1.0).iter().map(|v| 200.0 * v).collect();
                let options = ScatterOptions {
                    sizes: Some(sizes),
                    colors: ScatterColors::Single(Color::from(color)),
                    alpha: Alpha::Uniform(0.3),
                    edgecolors: Some(EdgeColors::Colors(vec![Color::none()])),
                    ..ScatterOptions::default()
                };
                ax.scatter_with(xy[..n].to_vec(), xy[n..].to_vec(), options)?
                    .set_label(color);
            }
            ax.legend(LegendOptions::default())?;
            ax.grid(true);
            Ok(())
        }),
    ]
}
