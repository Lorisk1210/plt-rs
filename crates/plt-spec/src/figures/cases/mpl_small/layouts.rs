//! The cases from `test_subplots.py` and `test_transforms.py`.

use plt::axes::HistPlotOptions;
use plt::figure::{Figure, Mosaic, MosaicOptions, Share};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::numeric::{arange, linspace};
use plt::stats::HistOptions;

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `test_subplots_offsettext`: offset texts on axes sharing columns and
/// every y axis.
fn offset_text(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 1e10, 1e9);
    let y: Vec<f64> = arange(0.0, 100.0, 10.0).iter().map(|v| v + 1e5).collect();
    let axs = fig.subplots_shared(2, 2, Share::Col, Share::All)?;
    axs[(0, 0)].plot(&x, &x)?;
    axs[(1, 0)].plot(&x, &x)?;
    axs[(0, 1)].plot(&y, &x)?;
    axs[(1, 1)].plot(&y, &x)?;
    Ok(())
}

/// The column widths of the ratio cases.
const WIDTHS: [f64; 3] = [1.0, 3.0, 2.0];
/// The row heights of the ratio cases.
const HEIGHTS: [f64; 2] = [1.0, 2.0];

/// `test_scale_swapping`: a log histogram drawn, then made linear.
fn scale_swapping(fig: &Figure) -> CaseResult {
    let samples = SpecRng::new(19_680_801).normal(10, 0.0, 1.0);
    let x = linspace(-5.0, 5.0, 10);
    let ax = fig.subplot();
    let options = HistPlotOptions {
        layout: HistOptions {
            density: true,
            ..HistOptions::default()
        },
        log: true,
        ..HistPlotOptions::default()
    };
    ax.hist(&[&samples], &options)?;
    let density: Vec<f64> = x
        .iter()
        .map(|x| (-(x * x) / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt())
        .collect();
    ax.plot(&x, density)?;
    fig.layout();
    ax.set_yscale("linear")?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_small/subplots_offset_text", offset_text),
    FigureCase::new("mpl_small/width_and_height_ratios", |fig| {
        let grid = GridSpec::with_ratios(
            2,
            3,
            Some(WIDTHS.to_vec()),
            Some(HEIGHTS.to_vec()),
            SubplotOverrides::default(),
        )?;
        fig.subplots_on(&grid, Share::None, Share::None)?;
        Ok(())
    }),
    FigureCase::new("mpl_small/width_and_height_ratios_mosaic", |fig| {
        let options = MosaicOptions {
            width_ratios: Some(WIDTHS.to_vec()),
            height_ratios: Some(HEIGHTS.to_vec()),
            ..MosaicOptions::default()
        };
        fig.subplot_mosaic(&Mosaic::parse("ABB;ACD")?, &options)?;
        Ok(())
    }),
    FigureCase::new("mpl_small/scale_swapping", scale_swapping),
];
