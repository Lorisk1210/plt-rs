//! Box plot tests: `bxp` widths, positions and cap widths on lognormal
//! statistics, and `boxplot` notches, outlier symbols, whisker ranges,
//! rcParams, user medians and confidence intervals.

use plt::axes::{BoxArtist, BoxplotOptions, BxpOptions, GridOptions, TickLevel};
use plt::figure::Figure;
use plt::numeric::linspace;
use plt::stats::{BoxplotOptions as StatsOptions, boxplot_stats};

use super::{at_40_dpi, classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `_bxp_test_helper` sets the box line width to `lines.linewidth`.
const BXP_RC: &[(&str, &str)] = &[("boxplot.boxprops.linewidth", "1.5")];

pub(crate) const CASES: &[FigureCase] = &[
    at_40_dpi("mpl_axes_c/bxp_customwidths", |fig| {
        bxp(
            fig,
            &BxpOptions {
                widths: vec![0.10, 0.25, 0.65, 0.85],
                ..BxpOptions::default()
            },
        )?;
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .with_rc(BXP_RC),
    at_40_dpi("mpl_axes_c/bxp_custompositions", |fig| {
        bxp(
            fig,
            &BxpOptions {
                positions: Some(vec![1.0, 5.0, 6.0, 7.0]),
                ..BxpOptions::default()
            },
        )?;
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .with_rc(BXP_RC),
    at_40_dpi("mpl_axes_c/bxp_custom_capwidths", |fig| {
        bxp(
            fig,
            &BxpOptions {
                capwidths: vec![0.0, 0.1, 0.5, 1.0],
                ..BxpOptions::default()
            },
        )
    })
    .with_rc(BXP_RC),
    at_40_dpi("mpl_axes_c/bxp_custom_capwidth", |fig| {
        bxp(
            fig,
            &BxpOptions {
                capwidths: vec![0.6],
                ..BxpOptions::default()
            },
        )
    })
    .with_rc(BXP_RC),
    FigureCase::new("mpl_axes_c/boxplot", |fig| {
        let ax = fig.subplot();
        ax.boxplot(&[boxplot_x(), boxplot_x()], &notched())?;
        ax.set_ylim(-30.0, 30.0);
        Ok(())
    }),
    classic("mpl_axes_c/boxplot_masked", |fig| {
        let x: Vec<f64> = linspace(-1.0, 1.0, 200)
            .into_iter()
            .filter(|&x| x >= 0.0)
            .collect();
        fig.subplot().boxplot(&[x], &BoxplotOptions::default())?;
        Ok(())
    }),
    at_40_dpi("mpl_axes_c/boxplot_custom_capwidths", |fig| {
        let mut options = notched();
        options.draw.capwidths = vec![0.01, 0.2];
        fig.subplot()
            .boxplot(&[boxplot_x(), boxplot_x()], &options)?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/boxplot_sym2", |fig| {
        let axs = fig.subplots(1, 2)?;
        for (ax, sym) in axs.as_slice().iter().zip(["^", "g"]) {
            ax.boxplot(&[boxplot_x(), boxplot_x()], &with_sym(sym))?;
            ax.set_ylim(-30.0, 30.0);
        }
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    at_40_dpi("mpl_axes_c/boxplot_sym", |fig| {
        let ax = fig.subplot();
        ax.boxplot(&[boxplot_x(), boxplot_x()], &with_sym("gs"))?;
        ax.set_ylim(-30.0, 30.0);
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/boxplot_autorange_whiskers_1", |fig| {
        autorange_whiskers(fig, false)
    }),
    FigureCase::new("mpl_axes_c/boxplot_autorange_whiskers_2", |fig| {
        autorange_whiskers(fig, true)
    }),
    FigureCase::new("mpl_axes_c/boxplot_rc_parameters", rc_parameters),
    at_40_dpi("mpl_axes_c/boxplot_with_CIarray", |fig| {
        let ax = fig.subplot();
        let options = BoxplotOptions {
            usermedians: Some(vec![None, Some(1.0)]),
            conf_intervals: Some(vec![(Some(-1.5), Some(3.0)), (Some(-1.0), Some(3.5))]),
            ..notched()
        };
        ax.boxplot(&[boxplot_x(), boxplot_x()], &options)?;
        ax.set_ylim(-30.0, 30.0);
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    at_40_dpi("mpl_axes_c/boxplot_no_weird_whisker", |fig| {
        let x = [3.0, 9000.0, 150.0, 88.0, 350.0, 200_000.0, 1400.0, 960.0];
        let ax = fig.subplot();
        ax.boxplot(&[x], &BoxplotOptions::default())?;
        ax.set_yscale("log")?;
        let minor = GridOptions {
            which: TickLevel::Minor,
            ..GridOptions::default()
        };
        ax.yaxis().grid(false, &minor);
        ax.xaxis().grid(false, &GridOptions::default());
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    at_40_dpi("mpl_axes_c/boxplot_mod_artist_after_plotting", |fig| {
        let x = [0.15, 0.11, 0.06, 0.06, 0.12, 0.56, -0.56];
        let parts = fig.subplot().boxplot(&[x], &with_sym("o"))?;
        let lines = parts
            .whiskers
            .iter()
            .chain(&parts.caps)
            .chain(&parts.medians)
            .chain(&parts.fliers)
            .chain(&parts.means);
        for line in lines {
            line.set_color("green");
        }
        for part in &parts.boxes {
            match part {
                BoxArtist::Line(line) => {
                    line.set_color("green");
                }
                BoxArtist::Patch(patch) => {
                    patch.set_color("green");
                }
            }
        }
        remove_ticks_and_titles(fig);
        Ok(())
    }),
];

/// `_bxp_test_helper`: the statistics of four columns of 37 lognormal
/// samples, drawn with `options` on a log y axis.
fn bxp(fig: &Figure, options: &BxpOptions) -> CaseResult {
    let samples: Vec<f64> = SpecRng::new(937)
        .normal(37 * 4, 1.25, 1.0)
        .into_iter()
        .map(f64::exp)
        .collect();
    let columns: Vec<Vec<f64>> = (0..4)
        .map(|column| samples.iter().skip(column).step_by(4).copied().collect())
        .collect();
    let stats = boxplot_stats(&columns, &StatsOptions::default())?;
    let ax = fig.subplot();
    ax.set_yscale("log")?;
    ax.bxp(&stats, options)?;
    Ok(())
}

/// `np.hstack([-25, np.linspace(-7, 7, 140), 25])`.
fn boxplot_x() -> Vec<f64> {
    let mut x = vec![-25.0];
    x.extend(linspace(-7.0, 7.0, 140));
    x.push(25.0);
    x
}

/// `notch=1`.
fn notched() -> BoxplotOptions {
    let mut options = BoxplotOptions::default();
    options.draw.shownotches = Some(true);
    options
}

/// `sym=sym`.
fn with_sym(sym: &str) -> BoxplotOptions {
    BoxplotOptions {
        sym: Some(sym.into()),
        ..BoxplotOptions::default()
    }
}

/// `test_boxplot_autorange_whiskers`: data whose quartiles coincide.
fn autorange_whiskers(fig: &Figure, autorange: bool) -> CaseResult {
    let mut x = vec![0.0];
    x.extend([1.0; 140]);
    x.push(2.0);
    let ax = fig.subplot();
    let options = BoxplotOptions {
        autorange,
        ..notched()
    };
    ax.boxplot(&[x.clone(), x], &options)?;
    ax.set_ylim(-5.0, 5.0);
    Ok(())
}

/// The rcParams of each axes of `test_boxplot_rc_parameters`.
const RC_AXES: [&[(&str, &str)]; 3] = [
    &[
        ("boxplot.notch", "True"),
        ("boxplot.whiskers", "5, 95"),
        ("boxplot.flierprops.color", "b"),
        ("boxplot.flierprops.marker", "o"),
        ("boxplot.flierprops.markerfacecolor", "g"),
        ("boxplot.flierprops.markeredgecolor", "b"),
        ("boxplot.flierprops.markersize", "5"),
        ("boxplot.flierprops.linestyle", "--"),
        ("boxplot.flierprops.linewidth", "2.0"),
        ("boxplot.boxprops.color", "r"),
        ("boxplot.boxprops.linewidth", "2.0"),
        ("boxplot.boxprops.linestyle", "--"),
        ("boxplot.capprops.color", "c"),
        ("boxplot.capprops.linewidth", "2.0"),
        ("boxplot.capprops.linestyle", "--"),
        ("boxplot.medianprops.color", "k"),
        ("boxplot.medianprops.linewidth", "2.0"),
        ("boxplot.medianprops.linestyle", "--"),
    ],
    &[
        ("boxplot.whiskers", "0, 100"),
        ("boxplot.patchartist", "True"),
    ],
    &[
        ("boxplot.whiskers", "2.0"),
        ("boxplot.showcaps", "False"),
        ("boxplot.showbox", "False"),
        ("boxplot.showfliers", "False"),
        ("boxplot.showmeans", "True"),
        ("boxplot.meanline", "True"),
        ("boxplot.meanprops.color", "c"),
        ("boxplot.meanprops.linewidth", "2.0"),
        ("boxplot.meanprops.linestyle", "--"),
        ("boxplot.whiskerprops.color", "r"),
        ("boxplot.whiskerprops.linewidth", "2.0"),
        ("boxplot.whiskerprops.linestyle", "-."),
    ],
];

/// `test_boxplot_rc_parameters`: each axes draws its box plots inside an
/// `rc_context` of its own parameters.
fn rc_parameters(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(3, 1)?;
    for (ax, params) in axs.as_slice().iter().zip(RC_AXES) {
        plt::rc::context(|| -> CaseResult {
            for (key, value) in params {
                plt::rc::set(key, value)?;
            }
            ax.boxplot(&[boxplot_x(), boxplot_x()], &BoxplotOptions::default())?;
            Ok(())
        })?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}
