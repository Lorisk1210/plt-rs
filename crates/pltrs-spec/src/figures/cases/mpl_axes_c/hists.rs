//! Histogram tests: stacked, offset, weighted and normalised histograms,
//! steps (horizontal, with a per-bin bottom), stacked bars with a legend,
//! and style sequences cycled over the datasets.

use pltrs::axes::HistPlotOptions;
use pltrs::colors::Color;
use pltrs::figure::Figure;
use pltrs::legend::{LegendLoc, LegendOptions};
use pltrs::lines::LineStyle;
use pltrs::numeric::{arange, linspace};
use pltrs::offsetbox::{AnchorLoc, BboxToAnchor};
use pltrs::patches::PatchProps;
use pltrs::stats::{Bins, HistAlign, HistBottom, HistOrientation, HistType};

use super::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_axes_c/hist_stacked_stepfilled", |fig| {
        let options = HistPlotOptions {
            histtype: HistType::StepFilled,
            stacked: true,
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[d_1_3(), d_0_10()], options)?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/hist_offset", |fig| {
        let ax = fig.subplot();
        for (data, bottom) in [(d_0_10(), 5.0), (d_1_3(), 15.0)] {
            let options = HistPlotOptions {
                bottom: HistBottom::Uniform(bottom),
                ..HistPlotOptions::default()
            };
            ax.hist(&[data], options)?;
        }
        Ok(())
    }),
    classic("mpl_axes_c/hist_step", |fig| {
        let ax = fig.subplot();
        let options = HistPlotOptions {
            histtype: HistType::Step,
            ..HistPlotOptions::default()
        };
        ax.hist(&[d_1_3()], options)?;
        ax.set_ylim(0.0, 10.0)?;
        ax.set_xlim(-1.0, 5.0)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/hist_step_horiz", |fig| {
        let options = HistPlotOptions {
            histtype: HistType::Step,
            orientation: HistOrientation::Horizontal,
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[d_0_10(), d_1_3()], options)?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/hist_stacked_weighted", |fig| {
        let weights = [linspace(0.01, 3.5, 50), linspace(0.05, 2.0, 20)];
        let options = HistPlotOptions {
            weights: Some(&weights),
            histtype: HistType::StepFilled,
            stacked: true,
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[d_0_10(), d_1_3()], options)?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/hist_stacked_stepfilled_alpha", |fig| {
        let options = HistPlotOptions {
            props: PatchProps::default().alpha(0.5),
            histtype: HistType::StepFilled,
            stacked: true,
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[d_1_3(), d_0_10()], options)?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/hist_stacked_step", |fig| {
        let options = HistPlotOptions {
            histtype: HistType::Step,
            stacked: true,
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[d_1_3(), d_0_10()], options)?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/hist_stacked_density", |fig| {
        let options = HistPlotOptions {
            stacked: true,
            density: true,
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[d_1_3(), d_0_10()], options)?;
        Ok(())
    }),
    classic("mpl_axes_c/hist_step_bottom", |fig| {
        let options = HistPlotOptions {
            histtype: HistType::StepFilled,
            bottom: HistBottom::PerBin(arange(0.0, 10.0, 1.0)),
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[d_1_3()], options)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/hist_stacked_bar", stacked_bar),
    classic("mpl_axes_c/hist_vectorized_params_facecolor", |fig| {
        vectorized(
            fig,
            &HistPlotOptions {
                facecolors: blue_green_red(),
                ..HistPlotOptions::default()
            },
        )
    }),
    classic("mpl_axes_c/hist_vectorized_params_edgecolor", |fig| {
        vectorized(
            fig,
            &HistPlotOptions {
                edgecolors: blue_green_red(),
                ..HistPlotOptions::default()
            },
        )
    }),
    classic("mpl_axes_c/hist_vectorized_params_hatch", |fig| {
        vectorized(
            fig,
            &HistPlotOptions {
                hatches: ["/", "\\", "."].map(String::from).to_vec(),
                ..HistPlotOptions::default()
            },
        )
    }),
    classic("mpl_axes_c/hist_vectorized_params_linestyle", |fig| {
        vectorized(
            fig,
            &HistPlotOptions {
                linestyles: vec![LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted],
                ..HistPlotOptions::default()
            },
        )
    }),
    classic("mpl_axes_c/hist_vectorized_params_linewidth", |fig| {
        vectorized(
            fig,
            &HistPlotOptions {
                linewidths: vec![1.0, 1.5, 2.0],
                ..HistPlotOptions::default()
            },
        )
    }),
    classic("mpl_axes_c/hist_vectorized_params_color", |fig| {
        vectorized(
            fig,
            &HistPlotOptions {
                color: blue_green_red(),
                ..HistPlotOptions::default()
            },
        )
    }),
];

/// `np.linspace(1, 3, 20)`.
fn d_1_3() -> Vec<f64> {
    linspace(1.0, 3.0, 20)
}

/// `np.linspace(0, 10, 50)`.
fn d_0_10() -> Vec<f64> {
    linspace(0.0, 10.0, 50)
}

/// `test_hist_stacked_bar`: six stacked datasets, five of them labelled.
fn stacked_bar(fig: &Figure) -> CaseResult {
    let data: [&[f64]; 6] = [
        &[
            100.0, 100.0, 100.0, 100.0, 200.0, 320.0, 450.0, 80.0, 20.0, 600.0, 310.0, 800.0,
        ],
        &[20.0, 23.0, 50.0, 11.0, 100.0, 420.0],
        &[120.0, 120.0, 120.0, 140.0, 140.0, 150.0, 180.0],
        &[
            60.0, 60.0, 60.0, 60.0, 300.0, 300.0, 5.0, 5.0, 5.0, 5.0, 10.0, 300.0,
        ],
        &[
            555.0, 555.0, 555.0, 30.0, 30.0, 30.0, 30.0, 30.0, 100.0, 100.0, 100.0, 100.0, 30.0,
            30.0,
        ],
        &[
            30.0, 30.0, 30.0, 30.0, 400.0, 400.0, 400.0, 400.0, 400.0, 400.0, 400.0, 400.0,
        ],
    ];
    let color = [
        (0.575_984_969_675_896_1, 1.0, 0.0),
        (0.0, 1.0, 0.350_624_650_815_206),
        (0.0, 1.0, 0.654_983_415_600_599_8),
        (0.0, 0.656_906_462_527_662_2, 1.0),
        (0.283_026_996_078_235_45, 0.0, 1.0),
        (0.684_912_346_229_982_2, 0.0, 1.0),
    ]
    .map(Color::from)
    .to_vec();
    let options = HistPlotOptions {
        color,
        label: ["first", "second", "third", "fourth", "fifth"].into(),
        bins: Some(Bins::Count(10)),
        histtype: HistType::BarStacked,
        align: HistAlign::Mid,
        ..HistPlotOptions::default()
    };
    let ax = fig.subplot();
    ax.hist(&data, options)?;
    ax.legend(LegendOptions {
        loc: Some(LegendLoc::Anchored(AnchorLoc::UpperRight)),
        bbox_to_anchor: Some(BboxToAnchor::Point(1.0, 1.0)),
        ncols: 1,
        ..LegendOptions::default()
    })?;
    Ok(())
}

/// `["b", "g", "r"]`.
fn blue_green_red() -> Vec<Color> {
    ["b", "g", "r"].map(Color::from).to_vec()
}

/// `test_hist_vectorized_params`: three normal samples as filled and as
/// outlined steps, styled by the sequences of `styling`.
fn vectorized(fig: &Figure, styling: &HistPlotOptions<'static>) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let xs = [20, 50, 100].map(|n| rng.normal(n, 0.0, 1.0));
    let axs = fig.subplots(2, 1)?;
    for (ax, histtype) in axs
        .as_slice()
        .iter()
        .zip([HistType::StepFilled, HistType::Step])
    {
        let options = HistPlotOptions {
            bins: Some(Bins::Count(10)),
            histtype,
            ..styling.clone()
        };
        ax.hist(&xs, options)?;
    }
    Ok(())
}
