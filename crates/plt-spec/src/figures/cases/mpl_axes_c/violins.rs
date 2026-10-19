//! Violin plot tests: vertical and horizontal violins with their means,
//! extrema, medians, quantiles and evaluation points, half violins and
//! colours.

use plt::artist::ArtistHandle;
use plt::axes::{ViolinDrawOptions, ViolinSide, ViolinplotOptions};
use plt::colors::Color;
use plt::figure::Figure;
use plt::orientation::Orientation;
use plt::stats::ViolinOptions;

use super::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_axes_c/vert_violinplot_baseline", |fig| {
        four_violins(fig, 414_213_562, Orientation::Vertical, Variant::Baseline)
    }),
    FigureCase::new("mpl_axes_c/vert_violinplot_showmeans", |fig| {
        four_violins(fig, 732_050_807, Orientation::Vertical, Variant::Means)
    }),
    FigureCase::new("mpl_axes_c/vert_violinplot_showextrema", |fig| {
        four_violins(fig, 236_067_977, Orientation::Vertical, Variant::Extrema)
    }),
    FigureCase::new("mpl_axes_c/vert_violinplot_showmedians", |fig| {
        four_violins(fig, 645_751_311, Orientation::Vertical, Variant::Medians)
    }),
    FigureCase::new("mpl_axes_c/vert_violinplot_showall", |fig| {
        four_violins(fig, 316_624_790, Orientation::Vertical, Variant::All)
    }),
    FigureCase::new("mpl_axes_c/vert_violinplot_custompoints_10", |fig| {
        four_violins(fig, 605_551_275, Orientation::Vertical, Variant::Points(10))
    }),
    FigureCase::new("mpl_axes_c/vert_violinplot_custompoints_200", |fig| {
        four_violins(
            fig,
            123_105_625,
            Orientation::Vertical,
            Variant::Points(200),
        )
    }),
    FigureCase::new("mpl_axes_c/horiz_violinplot_baseline", |fig| {
        four_violins(fig, 358_898_943, Orientation::Horizontal, Variant::Baseline)
    }),
    FigureCase::new("mpl_axes_c/horiz_violinplot_showmedians", |fig| {
        four_violins(fig, 795_831_523, Orientation::Horizontal, Variant::Medians)
    }),
    FigureCase::new("mpl_axes_c/horiz_violinplot_showmeans", |fig| {
        four_violins(fig, 385_164_807, Orientation::Horizontal, Variant::Means)
    }),
    FigureCase::new("mpl_axes_c/horiz_violinplot_showextrema", |fig| {
        four_violins(fig, 567_764_362, Orientation::Horizontal, Variant::Extrema)
    }),
    FigureCase::new("mpl_axes_c/horiz_violinplot_showall", |fig| {
        four_violins(fig, 82_762_530, Orientation::Horizontal, Variant::All)
    }),
    FigureCase::new("mpl_axes_c/horiz_violinplot_custompoints_10", |fig| {
        four_violins(
            fig,
            403_124_237,
            Orientation::Horizontal,
            Variant::Points(10),
        )
    }),
    FigureCase::new("mpl_axes_c/horiz_violinplot_custompoints_200", |fig| {
        four_violins(
            fig,
            557_438_524,
            Orientation::Horizontal,
            Variant::Points(200),
        )
    }),
    FigureCase::new("mpl_axes_c/violinplot_sides", sides),
    classic(
        "mpl_axes_c/violinplot_color_specification",
        color_specification,
    ),
    classic(
        "mpl_axes_c/violinplot_color_specification_ref",
        color_specification_ref,
    ),
    classic("mpl_axes_c/violinplot_single_list_quantiles", |fig| {
        let data = [SpecRng::new(110_433_579).normal(100, 0.0, 1.0)];
        let options = ViolinplotOptions {
            stats: ViolinOptions {
                quantiles: Some(vec![vec![0.1, 0.3, 0.9]]),
                ..ViolinOptions::default()
            },
            ..ViolinplotOptions::default()
        };
        fig.subplot().violinplot(&data, &options)?;
        Ok(())
    }),
    classic("mpl_axes_c/violinplot_pandas_series_ref", |fig| {
        let mut rng = SpecRng::new(110_433_579);
        let data = [7, 9, 11].map(|n| rng.normal(n, 0.0, 1.0));
        fig.subplot()
            .violinplot(&data, &ViolinplotOptions::default())?;
        Ok(())
    }),
];

/// What a four-violin test draws besides the bodies.
#[derive(Clone, Copy)]
enum Variant {
    /// Nothing (`showmeans=False, showextrema=False, showmedians=False`).
    Baseline,
    /// Lines at the means.
    Means,
    /// Lines at the extrema and bars between them.
    Extrema,
    /// Lines at the medians.
    Medians,
    /// Every line, and the quantiles of [`QUANTILES`].
    All,
    /// Nothing, with the densities evaluated at this many points.
    Points(usize),
}

/// The quantiles of the `showall` tests, one pair per violin.
const QUANTILES: [[f64; 2]; 4] = [[0.1, 0.9], [0.2, 0.8], [0.3, 0.7], [0.4, 0.6]];

/// Four violins of 100 normal samples at positions 0 to 3.
fn four_violins(fig: &Figure, seed: u64, orientation: Orientation, variant: Variant) -> CaseResult {
    let mut rng = SpecRng::new(seed);
    let data: Vec<Vec<f64>> = (0..4).map(|_| rng.normal(100, 0.0, 1.0)).collect();
    let all = matches!(variant, Variant::All);
    let options = ViolinplotOptions {
        stats: ViolinOptions {
            points: match variant {
                Variant::Points(points) => points,
                _ => 100,
            },
            quantiles: all.then(|| QUANTILES.iter().map(|pair| pair.to_vec()).collect()),
            ..ViolinOptions::default()
        },
        draw: ViolinDrawOptions {
            positions: Some(vec![0.0, 1.0, 2.0, 3.0]),
            orientation,
            showmeans: all || matches!(variant, Variant::Means),
            showextrema: all || matches!(variant, Variant::Extrema),
            showmedians: all || matches!(variant, Variant::Medians),
            ..ViolinDrawOptions::default()
        },
    };
    fig.subplot().violinplot(&data, &options)?;
    Ok(())
}

/// `test_violinplot_sides`: whole, low and high halves of one violin,
/// horizontal then vertical.
fn sides(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let data = [SpecRng::new(19_680_801).normal(100, 0.0, 1.0)];
    let rows = [
        (Orientation::Horizontal, [0.0, -0.5, 0.5]),
        (Orientation::Vertical, [4.0, 3.5, 4.5]),
    ];
    for (orientation, positions) in rows {
        let halves = [ViolinSide::Both, ViolinSide::Low, ViolinSide::High];
        for (position, side) in positions.into_iter().zip(halves) {
            let options = ViolinplotOptions {
                draw: ViolinDrawOptions {
                    positions: Some(vec![position]),
                    orientation,
                    showmedians: true,
                    side,
                    ..ViolinDrawOptions::default()
                },
                ..ViolinplotOptions::default()
            };
            ax.violinplot(&data, &options)?;
        }
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// Three sorted normal samples with standard deviations 1, 2 and 3.
fn sorted_samples() -> Vec<Vec<f64>> {
    let mut rng = SpecRng::new(19_680_801);
    [1.0, 2.0, 3.0]
        .into_iter()
        .map(|std| {
            let mut samples = rng.normal(100, 0.0, std);
            samples.sort_by(f64::total_cmp);
            samples
        })
        .collect()
}

/// The face and line colours of each axes of
/// `test_violinplot_color_specification` (empty: the defaults).
fn colorings() -> [(Vec<Color>, Vec<Color>); 3] {
    let red = |alpha| Color::from((1.0, 0.0, 0.0, alpha));
    [
        (vec![red(0.5)], vec![red(0.2)]),
        (vec![Color::from("r")], Vec::new()),
        (Vec::new(), vec![Color::from("r")]),
    ]
}

/// Every statistic line (means, extrema, medians).
fn all_lines() -> ViolinDrawOptions {
    ViolinDrawOptions {
        showmeans: true,
        showmedians: true,
        ..ViolinDrawOptions::default()
    }
}

/// `test_violinplot_color_specification`: face and line colours, with
/// alpha, face only and line only.
fn color_specification(fig: &Figure) -> CaseResult {
    let data = sorted_samples();
    let axs = fig.subplots(1, 3)?;
    for (ax, (facecolors, linecolors)) in axs.as_slice().iter().zip(colorings()) {
        let options = ViolinplotOptions {
            draw: ViolinDrawOptions {
                facecolors,
                linecolors,
                ..all_lines()
            },
            ..ViolinplotOptions::default()
        };
        ax.violinplot(&data, &options)?;
    }
    Ok(())
}

/// The `fig_ref` side of `test_violinplot_color_specification`: default
/// violins recoloured part by part, the bodies losing their alpha.
fn color_specification_ref(fig: &Figure) -> CaseResult {
    let data = sorted_samples();
    let axs = fig.subplots(1, 3)?;
    for (ax, (facecolors, linecolors)) in axs.as_slice().iter().zip(colorings()) {
        let options = ViolinplotOptions {
            draw: all_lines(),
            ..ViolinplotOptions::default()
        };
        let parts = ax.violinplot(&data, &options)?;
        if let Some(face) = facecolors.first() {
            for body in &parts.bodies {
                body.set_facecolor(face.clone()).set_alpha(None);
            }
        }
        if let Some(line) = linecolors.first() {
            let lines = [
                &parts.cbars,
                &parts.cmins,
                &parts.cmaxes,
                &parts.cmeans,
                &parts.cmedians,
            ];
            for part in lines.into_iter().flatten() {
                part.set_edgecolor(line.clone());
            }
        }
    }
    Ok(())
}
