//! Text and layout, and one mid-size figure in every output format.
//!
//! The PGF document of the mid-size figure is timed in pltrs only: to
//! measure text, matplotlib's PGF backend runs LaTeX, which pltrs does
//! not, so the two do different work.

use pltrs::axes::{BarOptions, HistPlotOptions, ImshowOptions};
use pltrs::colorbar::ColorbarOptions;
use pltrs::figure::SaveFormat;
use pltrs::layout::LayoutEngine;
use pltrs::legend::LegendOptions;
use pltrs::numeric::Grid;
use pltrs::prelude::*;
use pltrs::stats::Bins;
use pltrs::text::{HAlign, VAlign};
use pltrs_spec::figures::CaseResult;
use pltrs_spec::rng::SpecRng;

use super::{Build, Tier, Workload};

/// The workloads of this module.
pub(super) const WORKLOADS: &[Workload] = &[
    Workload::new("text/subplots_10x10", "subplots_10x10", subplots).sized(12.8, 9.6),
    Workload::new("text/text_2000", "text_2000", texts).calls(2000),
    Workload::new("text/mathtext_50", "mathtext_50", mathtext),
    Workload::new(
        "text/mixed_text_math_300",
        "mixed_text_math_300",
        mixed_text,
    ),
    Workload::new(
        "text/constrained_6x6_colorbars",
        "constrained_6x6_colorbars",
        constrained,
    )
    .sized(14.0, 12.0),
    Workload::new(
        "layout/constrained_8x8_no_draw",
        "constrained_8x8",
        constrained_layout,
    )
    .sized(16.0, 12.0)
    .layout_only(),
    mid("output/mid_png"),
    mid("output/mid_svg").saved_as(SaveFormat::Svg),
    mid("output/mid_pdf").saved_as(SaveFormat::Pdf),
    mid("output/mid_ps").saved_as(SaveFormat::Ps),
    mid("output/mid_pgf")
        .saved_as(SaveFormat::Pgf)
        .not_comparable(
            "matplotlib's PGF backend runs LaTeX to measure every text, while pltrs lays PGF \
             out with its own font metrics",
        ),
    mid("output/mid_eps").saved_as(SaveFormat::Eps),
    mid("output/mid_png_tight").tight(),
    Workload::new("output/lissajous_1e5_svg", "lissajous_1e5", lissajous)
        .saved_as(SaveFormat::Svg)
        .with_rc(UNSIMPLIFIED),
    Workload::new("output/lissajous_1e5_pdf", "lissajous_1e5", lissajous)
        .saved_as(SaveFormat::Pdf)
        .with_rc(UNSIMPLIFIED),
    Workload::new("output/lissajous_1e5_ps", "lissajous_1e5", lissajous)
        .saved_as(SaveFormat::Ps)
        .with_rc(UNSIMPLIFIED),
];

/// The mid-size figure as PNG, a medium workload.
const fn mid(name: &'static str) -> Workload {
    Workload::new(name, "mid_size", mid_size).in_tier(Tier::Medium)
}

/// Every segment of a line reaches the document.
const UNSIMPLIFIED: &[(&str, &str)] = &[("path.simplify", "False")];

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

/// 300 labels: rotated two-line texts, formulas, and texts with inline
/// math, under a title and axis labels that hold math too.
fn mixed_text() -> Build {
    Box::new(|fig| {
        let ax = fig.subplot();
        ax.set_title(r"Mixed text: $\alpha_i + \beta^2 = \gamma$");
        ax.set_xlabel(r"time $t$ (s)")
            .set_ylabel(r"response $\sigma(t)$");
        for k in 0..300_u32 {
            let (x, y) = (f64::from(k % 15) / 15.0, f64::from(k / 15) / 20.0);
            match k % 3 {
                0 => {
                    ax.text(x, y, format!("label {k}\nsecond line"))
                        .set_fontsize(7.0)
                        .set_rotation(f64::from(k % 90));
                }
                1 => {
                    let formula = format!(r"$x_{{{k}}}^{{2}} + \frac{{\pi}}{{{}}}$", k + 1);
                    ax.text(x, y, formula).set_fontsize(8.0);
                }
                _ => {
                    ax.text(x, y, format!(r"value {k}: $\sqrt{{{k}}} \leq \Omega$"))
                        .set_fontsize(7.0)
                        .set_halign(HAlign::Center)
                        .set_valign(VAlign::Center);
                }
            }
        }
        Ok(())
    })
}

/// A 6 × 6 grid of images, each with a colorbar, in constrained layout.
fn constrained() -> Build {
    let image = Grid::from_fn(20, 20, |i, j| ((i * j) as f64 / 40.0).sin());
    Box::new(move |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        let grid = fig.subplots(6, 6)?;
        for (k, ax) in grid.as_slice().iter().enumerate() {
            let shown = ax.imshow(image.clone(), ImshowOptions::default())?;
            fig.colorbar(&shown, ColorbarOptions::default())?;
            ax.set_title(format!("image {k}"));
        }
        Ok(())
    })
}

/// 64 titled and labelled cosine panels under a suptitle, in
/// constrained layout.
fn constrained_layout() -> Build {
    let x = linspace(0.0, 10.0, 50);
    Box::new(move |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        let grid = fig.subplots(8, 8)?;
        for (k, ax) in grid.as_slice().iter().enumerate() {
            let shift = k as f64 / 10.0;
            ax.plot(&x, x.iter().map(|x| (x + shift).cos()).collect::<Vec<_>>())?;
            ax.set_title(format!("panel {k}"));
            ax.set_xlabel("x").set_ylabel("y");
        }
        fig.suptitle("64 panels in constrained layout");
        Ok(())
    })
}

/// A thin Lissajous curve of 10⁵ points (frequencies 37 and 41).
fn lissajous() -> Build {
    let t = linspace(0.0, 2.0 * std::f64::consts::PI, 100_000);
    let x: Vec<f64> = t.iter().map(|t| (37.0 * t).sin()).collect();
    let y: Vec<f64> = t.iter().map(|t| (41.0 * t + 0.5).sin()).collect();
    Box::new(move |fig| {
        fig.subplot()
            .plot_with(&x, &y, LineProps::default().linewidth(0.5))?;
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
            bins: Some(Bins::Count(50)),
            ..HistPlotOptions::default()
        };
        grid[(1, 1)].hist(&[sample.as_slice()], options)?;
        grid[(1, 1)].set_title(r"$\mathcal{N}(0, 1)$: $\frac{1}{\sqrt{2\pi}} e^{-x^2/2}$");
        Ok(())
    })
}
