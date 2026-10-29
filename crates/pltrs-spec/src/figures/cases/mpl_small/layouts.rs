//! The cases from `test_subplots.py` and `test_transforms.py`.

use pltrs::axes::{HistPlotOptions, Mesh, PcolorOptions, ScatterOptions};
use pltrs::collections::CollectionProps;
use pltrs::contour::{ContourOptions, QuadField};
use pltrs::figure::{Figure, Mosaic, MosaicOptions, Share};
use pltrs::gridspec::{GridSpec, SubplotOverrides};
use pltrs::numeric::{Grid, arange, linspace, meshgrid};
use pltrs::quiver::{BarbsOptions, QuiverOptions, VectorField};
use pltrs::streamplot::{StreamField, StreamWidths, StreamplotOptions};
use pltrs::transforms::{Affine2D, Transform};

use super::remove_text;

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
        density: true,
        log: true,
        ..HistPlotOptions::default()
    };
    ax.hist(&[&samples], options)?;
    let density: Vec<f64> = x
        .iter()
        .map(|x| (-(x * x) / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt())
        .collect();
    ax.plot(&x, density)?;
    fig.layout();
    ax.set_yscale("linear")?;
    Ok(())
}

/// `test_pre_transform_plotting`: every kind of plot placed through an
/// affine map in front of the data transform, which the data limits
/// follow.
fn pre_transform(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let times10 = Transform::Affine(Affine2D::identity().scale(10.0, 10.0)).then(ax.trans_data());
    let counting = |rows, cols| Grid::from_fn(rows, cols, |r, c| (r * cols + c) as f64);
    ax.contourf(
        &QuadField::new(counting(6, 8)),
        ContourOptions {
            transform: Some(times10.clone()),
            ..ContourOptions::default()
        },
    )?;
    ax.pcolormesh(
        Mesh::from_coords(linspace(0.0, 4.0, 7), linspace(5.5, 8.0, 9), counting(8, 6)),
        PcolorOptions {
            props: CollectionProps::default().transform(times10.clone()),
            ..PcolorOptions::default()
        },
    )?;
    ax.scatter_with(
        linspace(0.0, 10.0, 50),
        linspace(10.0, 0.0, 50),
        ScatterOptions {
            offset_transform: Some(times10.clone()),
            ..ScatterOptions::default()
        },
    )?;
    let (x, y) = (linspace(8.0, 10.0, 20), linspace(1.0, 5.0, 20));
    let u = Grid::from_fn(20, 20, |r, c| 2.0 * x[c].sin() + y[r].cos());
    let v = Grid::from_fn(20, 20, |r, c| x[c].sin() - y[r].cos());
    let speed = Grid::from_fn(20, 20, |r, c| u[(r, c)].hypot(v[(r, c)]));
    ax.streamplot(
        &StreamField::new(x.clone(), y.clone(), u.clone(), v.clone()),
        StreamplotOptions {
            linewidth: Some(StreamWidths::Varying(speed)),
            transform: Some(times10.clone()),
            ..StreamplotOptions::default()
        },
    )?;
    let every_third = |values: &[f64]| values.iter().step_by(3).copied().collect::<Vec<_>>();
    let thinned = |grid: &Grid<f64>| Grid::from_fn(7, 7, |r, c| grid[(3 * r, 3 * c)]);
    let (x, y) = (every_third(&x), every_third(&y));
    let (u, v) = (thinned(&u), thinned(&v));
    let shifted = |values: &[f64], by: f64| values.iter().map(|v| v + by).collect::<Vec<_>>();
    let (qx, qy) = meshgrid(&x, &shifted(&y, 5.0));
    ax.quiver(
        VectorField::on_mesh(&qx, &qy, &u, &v),
        QuiverOptions {
            transform: Some(times10.clone()),
            ..QuiverOptions::default()
        },
    )?;
    let (bx, by) = meshgrid(&shifted(&x, -3.0), &shifted(&y, 5.0));
    let square = |grid: &Grid<f64>| Grid::from_fn(7, 7, |r, c| grid[(r, c)].powi(2));
    ax.barbs(
        VectorField::on_mesh(&bx, &by, &square(&u), &square(&v)),
        BarbsOptions {
            transform: Some(times10),
            ..BarbsOptions::default()
        },
    )?;
    remove_text(fig);
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
        fig.subplot_mosaic(&"ABB;ACD".parse::<Mosaic>()?, options)?;
        Ok(())
    }),
    FigureCase::new("mpl_small/pre_transform_data", pre_transform),
    FigureCase::new("mpl_small/scale_swapping", scale_swapping),
];
