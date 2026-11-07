//! `hexbin`: a fixed extent, empty data with every kind of binning, `mincnt`
//! with and without values, and the marginal bars of a correlated sample
//! on linear and log axes.

use pltrs::axes::{HexBins, HexGridSize, HexReduce, HexScale, HexbinOptions};
use pltrs::colorbar::ColorbarOptions;
use pltrs::image::Extent;
use pltrs::numeric::arange;
use pltrs::numeric::pow;
use pltrs::prelude::*;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;
use crate::rng::SpecRng;

pub(super) fn hexbin_extent(fig: &Figure) -> CaseResult {
    let data: Vec<f64> = (0..2000).map(|i| f64::from(i) / 2000.0).collect();
    let (x, y) = data.split_at(1000);
    let options = HexbinOptions {
        extent: Some(Extent::new(0.1, 0.3, 0.6, 0.7)),
        ..HexbinOptions::default()
    };
    fig.subplot().hexbin(x, y, options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn hexbin_empty(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let binned = |bins| HexbinOptions {
        bins: Some(bins),
        ..HexbinOptions::default()
    };
    let empty: &[f64] = &[];
    ax.hexbin(empty, empty, HexbinOptions::default())?;
    ax.hexbin(empty, empty, binned(HexBins::Log))?;
    let reduced = HexbinOptions {
        values: Some(Vec::new()),
        reduce: HexReduce::Max,
        ..HexbinOptions::default()
    };
    ax.hexbin(empty, empty, reduced)?;
    ax.hexbin(empty, empty, binned(HexBins::Edges(arange(0.0, 10.0, 1.0))))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `_hexbin_sample`: the correlated normal sample of the hexbin tests.
fn hexbin_sample() -> (Vec<f64>, Vec<f64>) {
    let mut rng = SpecRng::new(19_680_801);
    let n = 100_000;
    let x = rng.normal(n, 0.0, 1.0);
    let noise = rng.normal(n, 0.0, 1.0);
    let y = x
        .iter()
        .zip(noise)
        .map(|(x, e)| 2.0 + 3.0 * x + 4.0 * e)
        .collect();
    (x, y)
}

pub(super) fn hexbin_log(fig: &Figure) -> CaseResult {
    let (x, y) = hexbin_sample();
    let y: Vec<f64> = y.into_iter().map(|y| pow(2.0, y * 0.5)).collect();
    let options = HexbinOptions {
        yscale: HexScale::Log,
        bins: Some(HexBins::Log),
        marginals: true,
        reduce: HexReduce::Sum,
        ..HexbinOptions::default()
    };
    let hexagons = fig.subplot().hexbin(&x, &y, options)?;
    fig.colorbar(&hexagons, ColorbarOptions::default())?;
    Ok(())
}

pub(super) fn hexbin_linear(fig: &Figure) -> CaseResult {
    let (x, y) = hexbin_sample();
    let options = HexbinOptions {
        gridsize: HexGridSize::Shape(10, 5),
        marginals: true,
        reduce: HexReduce::Sum,
        ..HexbinOptions::default()
    };
    fig.subplot().hexbin(&x, &y, options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `lambda v: sum(v)`.
fn builtin_sum(values: &[f64]) -> f64 {
    values.iter().sum()
}

/// `_hexbin_mincnt`: four points (two coinciding) binned with `mincnt=1`
/// on green, with or without values.
fn hexbin_mincnt(fig: &Figure, values: Option<Vec<f64>>) -> CaseResult {
    let ax = fig.subplot();
    let options = HexbinOptions {
        reduce: HexReduce::custom(builtin_sum),
        values,
        extent: Some(Extent::new(-10.0, 10.0, -10.0, 10.0)),
        gridsize: HexGridSize::Shape(7, 7),
        mincnt: Some(1),
        ..HexbinOptions::default()
    };
    ax.hexbin(&[0.0, 0.0, 6.0, 0.0], &[0.0, 0.0, 0.0, 6.0], options)?;
    ax.set_facecolor("green");
    Ok(())
}

pub(super) fn hexbin_mincnt_behavior_upon_c_parameter(fig: &Figure) -> CaseResult {
    hexbin_mincnt(fig, Some(vec![1.0; 4]))
}

pub(super) fn hexbin_mincnt_behavior_upon_c_parameter_ref(fig: &Figure) -> CaseResult {
    hexbin_mincnt(fig, None)
}
