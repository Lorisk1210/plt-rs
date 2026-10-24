//! `figures/cases_mpl_tests_lines.py`: matplotlib's non-image tests of
//! `test_lines.py` that only check that a figure draws, in the test
//! suite's default style.

use plt::lines::LineStyle;

use super::mpl_tests::classic;
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        // `test_linestyle_variants`: every named style, then offset dash
        // patterns without dashes, which draw solid: no dash list
        // (matplotlib's `(0, None)`) and an empty one, spelled `(0, ())`
        // and `(0, [])` in matplotlib.
        classic("mpl_tests_lines/linestyle_variants", |fig| {
            let ax = fig.subplot();
            let y: Vec<f64> = (0..10).map(f64::from).collect();
            let named = ["-", "solid", "--", "dashed", "-.", "dashdot", ":", "dotted"];
            let mut styles = named
                .iter()
                .map(|name| name.parse::<LineStyle>())
                .collect::<Result<Vec<_>, _>>()?;
            styles.push(LineStyle::Dashes {
                offset: 0.0,
                pattern: None,
            });
            styles.extend(["(0, ())".parse()?, "(0, [])".parse()?]);
            for style in styles {
                ax.plot_y(y.clone()).set_linestyle(style);
            }
            Ok(())
        }),
    ]
}
