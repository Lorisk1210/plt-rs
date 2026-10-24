//! The basic plot types: lines, scatter, bars, stems, fills, stack plots
//! and stairs.

use plt::axes::{FillBetweenOptions, StackplotOptions, StairsOptions, StemOptions};
use plt::collections::CollectionProps;
use plt::prelude::*;

use super::{gridded, unit_square};
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The bar heights and stair values of several thumbnails.
const HEIGHTS: [f64; 8] = [4.8, 5.5, 3.5, 4.6, 6.5, 6.6, 2.6, 3.0];

/// The centres of eight unit-wide bars from 0 to 8.
fn centres() -> Vec<f64> {
    arange(0.5, 8.0, 1.0)
}

fn bar(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = BarOptions {
        thickness: 1.0.into(),
        edgecolor: vec!["white".into()],
        linewidth: vec![0.7],
        ..BarOptions::default()
    };
    ax.bar(centres(), HEIGHTS, options)?;
    unit_square(&ax)?;
    Ok(())
}

fn fill_between(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(1);
    let x = linspace(0.0, 8.0, 16);
    let noise1 = rng.uniform(x.len(), 0.0, 0.5);
    let noise2 = rng.uniform(x.len(), 0.0, 0.5);
    let y1: Vec<f64> = x
        .iter()
        .zip(noise1)
        .map(|(x, e)| 3.0 + 4.0 * x / 8.0 + e)
        .collect();
    let y2: Vec<f64> = x
        .iter()
        .zip(noise2)
        .map(|(x, e)| 1.0 + 2.0 * x / 8.0 + e)
        .collect();
    let middle: Vec<f64> = y1.iter().zip(&y2).map(|(a, b)| (a + b) / 2.0).collect();
    let ax = fig.subplot();
    let band = FillBetweenOptions {
        props: CollectionProps::default().alpha(0.5).linewidth(0.0),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(&x, y1, y2, band)?;
    ax.plot_with(&x, middle, LineProps::default().linewidth(2.0))?;
    unit_square(&ax)?;
    Ok(())
}

fn plot(fig: &Figure) -> CaseResult {
    let wave = |x: f64| 4.0 + (2.0 * x).sin();
    let x = linspace(0.0, 10.0, 100);
    let x2 = linspace(0.0, 10.0, 25);
    let ax = fig.subplot();
    ax.plot_with(
        &x2,
        map(&x2, |x| wave(x) + 2.5),
        LineProps::fmt("x")?.markeredgewidth(2.0),
    )?;
    ax.plot_with(&x, map(&x, wave), LineProps::default().linewidth(2.0))?;
    ax.plot_with(
        &x2,
        map(&x2, |x| wave(x) - 2.5),
        LineProps::fmt("o-")?.linewidth(2.0),
    )?;
    unit_square(&ax)?;
    Ok(())
}

fn scatter_plot(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(3);
    let x = map(&rng.normal(24, 0.0, 2.0), |v| 4.0 + v);
    let y = map(&rng.normal(x.len(), 0.0, 2.0), |v| 4.0 + v);
    let sizes = rng.uniform(x.len(), 15.0, 80.0);
    let colors = rng.uniform(x.len(), 15.0, 80.0);
    let ax = fig.subplot();
    let options = ScatterOptions {
        sizes: Some(sizes),
        colors: MappedColors::new(colors)
            .limits(Some(0.0), Some(100.0))
            .into(),
        ..ScatterOptions::default()
    };
    ax.scatter_with(&x, &y, options)?;
    unit_square(&ax)?;
    Ok(())
}

fn stackplot(fig: &Figure) -> CaseResult {
    let layers = [
        vec![1.0, 1.25, 2.0, 2.75, 3.0],
        vec![1.0; 5],
        vec![2.0, 1.0, 2.0, 1.0, 2.0],
    ];
    let ax = fig.subplot();
    ax.stackplot(arange(0.0, 10.0, 2.0), &layers, StackplotOptions::default())?;
    unit_square(&ax)?;
    Ok(())
}

fn stairs(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.stairs(HEIGHTS, StairsOptions::default())?
        .set_linewidth(2.5);
    unit_square(&ax)?;
    Ok(())
}

fn stem(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.stem(centres(), HEIGHTS, StemOptions::default())?;
    unit_square(&ax)?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    gridded("gallery_plot_types/bar", bar),
    gridded("gallery_plot_types/fill_between", fill_between),
    gridded("gallery_plot_types/plot", plot),
    gridded("gallery_plot_types/scatter_plot", scatter_plot),
    gridded("gallery_plot_types/stackplot", stackplot),
    gridded("gallery_plot_types/stairs", stairs),
    gridded("gallery_plot_types/stem", stem),
];
