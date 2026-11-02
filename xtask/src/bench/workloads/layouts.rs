//! Figures whose cost lies in their arrangement and their text: a legend
//! of many entries, annotations with arrows, a mosaic of panels, tight
//! layout, and a large grid of axes.

use pltrs::axes::{BarOptions, HistPlotOptions};
use pltrs::figure::MosaicOptions;
use pltrs::layout::{LayoutEngine, TightLayout};
use pltrs::legend::LegendLoc;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;
use pltrs::stats::{Bins, HistOrientation};
use pltrs::text::{AnnotateOptions, ArrowProps, FancyArrowProps};
use pltrs_spec::rng::SpecRng;

use super::calls::waves;
use super::{Build, Tier, Workload};

/// The workloads of this module.
pub(super) const WORKLOADS: &[Workload] = &[
    Workload::new("text/legend_40_entries", "legend_40", legend)
        .sized(9.6, 7.2)
        .in_tier(Tier::Medium),
    Workload::new("text/annotate_arrows_100", "annotate_arrows_100", arrows)
        .in_tier(Tier::Medium)
        .calls(101),
    Workload::new("layout/subplot_mosaic", "mosaic", mosaic)
        .sized(8.0, 5.0)
        .in_tier(Tier::Medium),
    Workload::new("layout/tight_layout_4x4", "tight_4x4", tight)
        .sized(9.6, 7.2)
        .in_tier(Tier::Medium),
    Workload::new("layout/grid_16x16", "grid_16x16", grid).sized(16.0, 16.0),
];

/// 40 labelled lines under a legend of 40 entries in two columns.
fn legend() -> Build {
    let (x, lines) = waves(40, 100);
    Box::new(move |fig| {
        let ax = fig.subplot();
        for (k, y) in lines.iter().enumerate() {
            ax.plot_with(&x, y, LineProps::default().label(format!("series {k}")))?;
        }
        let options = LegendOptions {
            loc: Some(LegendLoc::Anchored(AnchorLoc::UpperLeft)),
            fontsize: Some(7.0.into()),
            ..LegendOptions::default().ncols(2)
        };
        ax.legend(options)?;
        Ok(())
    })
}

/// 100 annotations, each pointing at its point with an arrow.
fn arrows() -> Build {
    let mut rng = SpecRng::new(50);
    let x = rng.uniform(100, 0.0, 10.0);
    let y = rng.uniform(100, 0.0, 10.0);
    Box::new(move |fig| {
        let ax = fig.subplot();
        ax.plot_with(&x, &y, LineProps::fmt("o")?.markersize(3.0))?;
        for (i, (&px, &py)) in x.iter().zip(&y).enumerate() {
            let options = AnnotateOptions {
                xytext: Some((12.0, 8.0).into()),
                textcoords: Some("offset points".parse()?),
                arrowprops: Some(ArrowProps::Fancy(FancyArrowProps::new("->".parse()?))),
                ..AnnotateOptions::default()
            };
            ax.annotate(format!("p{i}"), (px, py), options)?
                .set_fontsize(6.0);
        }
        Ok(())
    })
}

/// A mosaic of four panels of different sizes in constrained layout: a
/// wide line plot, a tall histogram, bars and a scatter.
fn mosaic() -> Build {
    let mut rng = SpecRng::new(51);
    let t = linspace(0.0, 10.0, 500);
    let signal: Vec<f64> = t.iter().map(|t| t.sin() + 0.1 * t).collect();
    let sample = rng.normal(5000, 0.0, 1.0);
    let heights = rng.uniform(8, 1.0, 5.0);
    let px = rng.uniform(300, 0.0, 1.0);
    let py = rng.uniform(300, 0.0, 1.0);
    Box::new(move |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        let axd = fig.subplot_mosaic("AAB;CDB", MosaicOptions::default())?;
        axd["A"].plot(&t, &signal)?;
        axd["A"].set_title("signal");
        let options = HistPlotOptions {
            bins: Some(Bins::Count(40)),
            orientation: HistOrientation::Horizontal,
            ..HistPlotOptions::default()
        };
        axd["B"].hist(&[sample.as_slice()], options)?;
        axd["B"].set_title("distribution");
        let positions: Vec<f64> = (0..8).map(f64::from).collect();
        axd["C"].bar(positions, &heights, BarOptions::default())?;
        axd["C"].set_title("bars");
        let options = ScatterOptions {
            sizes: Some(vec![6.0]),
            ..ScatterOptions::default()
        };
        axd["D"].scatter_with(&px, &py, options)?;
        axd["D"].set_title("points");
        fig.suptitle("A mosaic of panels");
        Ok(())
    })
}

/// 16 titled and labelled panels, arranged by `tight_layout`.
fn tight() -> Build {
    let x = linspace(0.0, 10.0, 100);
    let curves: Vec<Vec<f64>> = (0..16)
        .map(|k| {
            let k = f64::from(k);
            x.iter()
                .map(|x| (x + k / 4.0).sin() * (-x / (5.0 + k)).exp())
                .collect()
        })
        .collect();
    Box::new(move |fig| {
        let grid = fig.subplots(4, 4)?;
        for (k, (ax, y)) in grid.as_slice().iter().zip(&curves).enumerate() {
            ax.plot(&x, y)?;
            ax.set_title(format!("damping {k}"));
            ax.set_xlabel("time").set_ylabel("level");
        }
        fig.tight_layout(TightLayout::default())?;
        Ok(())
    })
}

/// A 16 × 16 grid of 256 small line plots without labels.
fn grid() -> Build {
    let x = linspace(0.0, 2.0 * std::f64::consts::PI, 30);
    let curves: Vec<Vec<f64>> = (0..256)
        .map(|k| {
            let k = f64::from(k);
            x.iter().map(|x| (x * (1.0 + k / 64.0)).sin()).collect()
        })
        .collect();
    Box::new(move |fig| {
        let grid = fig.subplots(16, 16)?;
        for (ax, y) in grid.as_slice().iter().zip(&curves) {
            ax.plot(&x, y)?;
        }
        Ok(())
    })
}
