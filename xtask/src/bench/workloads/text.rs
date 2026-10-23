//! Text and layout, and one mid-size figure in every output format.

use plt::axes::{BarOptions, HistPlotOptions, ImshowOptions};
use plt::colorbar::ColorbarOptions;
use plt::figure::SaveFormat;
use plt::layout::LayoutEngine;
use plt::legend::LegendOptions;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::stats::{Bins, HistOptions};
use plt_spec::figures::CaseResult;
use plt_spec::rng::SpecRng;

use super::{Build, Workload};

/// The workloads of this module.
pub(super) const WORKLOADS: &[Workload] = &[
    Workload::new("text/subplots_10x10", subplots).sized(12.8, 9.6),
    Workload::new("text/text_2000", texts),
    Workload::new("text/mathtext_50", mathtext),
    Workload::new("text/constrained_6x6_colorbars", constrained).sized(14.0, 12.0),
    Workload::new("output/mid_png", mid_size),
    Workload::new("output/mid_svg", mid_size).saved_as(SaveFormat::Svg),
    Workload::new("output/mid_pdf", mid_size).saved_as(SaveFormat::Pdf),
    Workload::new("output/mid_ps", mid_size).saved_as(SaveFormat::Ps),
    Workload::new("output/mid_pgf", mid_size).saved_as(SaveFormat::Pgf),
    Workload::new("output/mid_eps", mid_size).saved_as(SaveFormat::Eps),
];

/// 100 subplots, each a titled and labelled sine wave.
fn subplots() -> Build {
    let x = linspace(0.0, 10.0, 100);
    Box::new(move |fig| {
        let grid = fig.subplots(10, 10)?;
        for (k, ax) in grid.as_slice().iter().enumerate() {
            let shift = k as f64 / 10.0;
            ax.plot(&x, x.iter().map(|x| (x + shift).sin()).collect::<Vec<_>>())?;
            ax.set_title(format!("panel {k}"));
            ax.set_xlabel("x").set_ylabel("y");
        }
        Ok(())
    })
}

/// 2000 small labels on a 50 × 40 lattice.
fn texts() -> Build {
    Box::new(|fig| {
        let ax = fig.subplot();
        for i in 0..2000_u32 {
            let (column, row) = (f64::from(i % 50), f64::from(i / 50));
            ax.text(column / 50.0, row / 40.0, format!("t{i}"))
                .set_fontsize(6.0);
        }
        Ok(())
    })
}

/// 50 distinct formulas: sums, fractions, roots, sub- and superscripts.
fn mathtext() -> Build {
    Box::new(|fig| {
        let ax = fig.subplot();
        for k in 0..50_u32 {
            let formula = format!(
                r"$\sum_{{n=1}}^{{{k}}} \frac{{x^{{n}}}}{{n!}} + \sqrt{{\alpha_{{{k}}}}} - \int_0^{{{k}}} \beta\,dt$"
            );
            let (column, row) = (f64::from(k % 5), f64::from(k / 5));
            ax.text(column / 5.0, row / 10.0, formula);
        }
        Ok(())
    })
}

/// A 6 × 6 grid of images, each with a colorbar, in constrained layout.
fn constrained() -> Build {
    let image = Grid::from_fn(20, 20, |i, j| ((i * j) as f64 / 40.0).sin());
    Box::new(move |fig| {
        fig.set_layout_engine(LayoutEngine::constrained());
        let grid = fig.subplots(6, 6)?;
        for (k, ax) in grid.as_slice().iter().enumerate() {
            let shown = ax.imshow(image.clone(), ImshowOptions::default())?;
            fig.colorbar(&shown, ColorbarOptions::default())?;
            ax.set_title(format!("image {k}"));
        }
        Ok(())
    })
}

/// Four panels: labelled lines with a legend, a colour-mapped scatter,
/// bars, and a histogram under a mathtext title (nothing rasterized, so
/// that PGF can be streamed).
fn mid_size() -> Build {
    let mut rng = SpecRng::new(13);
    let x = linspace(0.0, 10.0, 2000);
    let scatter_x = rng.uniform(2000, 0.0, 1.0);
    let scatter_y = rng.uniform(2000, 0.0, 1.0);
    let heights = rng.uniform(30, 1.0, 10.0);
    let sample = rng.normal(10_000, 0.0, 1.0);
    Box::new(move |fig| -> CaseResult {
        let grid = fig.subplots(2, 2)?;
        let lines = &grid[(0, 0)];
        for k in 0..3 {
            let k = f64::from(k);
            let y: Vec<f64> = x
                .iter()
                .map(|x| (x + k).sin() * (-x / 10.0).exp())
                .collect();
            lines.plot_with(&x, y, LineProps::default().label(format!("phase {k}")))?;
        }
        lines.set_title("damped waves");
        lines.set_xlabel("time").set_ylabel("amplitude");
        lines.legend(LegendOptions::default())?;
        let options = ScatterOptions {
            colors: MappedColors::new(scatter_y.clone()).into(),
            ..ScatterOptions::default()
        };
        grid[(0, 1)].scatter_with(&scatter_x, &scatter_y, options)?;
        let positions: Vec<f64> = (0..heights.len()).map(|i| i as f64).collect();
        grid[(1, 0)].bar(positions, &heights, BarOptions::default())?;
        let options = HistPlotOptions {
            layout: HistOptions {
                bins: Some(Bins::Count(50)),
                ..HistOptions::default()
            },
            ..HistPlotOptions::default()
        };
        grid[(1, 1)].hist(&[sample.as_slice()], &options)?;
        grid[(1, 1)].set_title(r"$\mathcal{N}(0, 1)$: $\frac{1}{\sqrt{2\pi}} e^{-x^2/2}$");
        Ok(())
    })
}
