//! The plots of arrays and vector fields: images, meshes, contours,
//! barbs, arrows and streamlines.

use std::f64::consts::PI;

use plt::axes::{ImshowOptions, Mesh, PcolorOptions};
use plt::collections::CollectionProps;
use plt::contour::{QuadCoords, QuadField};
use plt::image::{ColorizerOptions, Origin};
use plt::numeric::{Grid, linspace, meshgrid};
use plt::prelude::*;
use plt::quiver::{Angles, BarbsOptions, QuiverOptions, QuiverUnits, VectorField};
use plt::streamplot::{StreamField, StreamplotOptions};

use super::{extrema_levels, func3_grid, plain};
use crate::figures::cases::common::func3;
use crate::figures::{CaseResult, FigureCase};

/// `func3` sampled on a 256 × 256 grid, ready to contour.
fn func3_field() -> QuadField {
    let (t, z) = func3_grid(256);
    QuadField::new(z).with_coords(QuadCoords::Axes { x: t.clone(), y: t })
}

fn barbs(fig: &Figure) -> CaseResult {
    let positions = [1.0, 2.0, 3.0, 4.0];
    let (x, y) = meshgrid(&positions, &positions);
    let degrees = [
        15.0, 30.0, 35.0, 45.0, 25.0, 40.0, 55.0, 60.0, 35.0, 50.0, 65.0, 75.0, 45.0, 60.0, 75.0,
        90.0,
    ];
    let amplitude = [
        5.0, 10.0, 25.0, 50.0, 10.0, 15.0, 30.0, 60.0, 15.0, 26.0, 50.0, 70.0, 20.0, 45.0, 80.0,
        100.0,
    ];
    let angle = degrees.map(|d| PI / 180.0 * d);
    let u: Vec<f64> = amplitude
        .iter()
        .zip(angle)
        .map(|(a, t)| a * t.sin())
        .collect();
    let v: Vec<f64> = amplitude
        .iter()
        .zip(angle)
        .map(|(a, t)| a * t.cos())
        .collect();
    let ax = fig.subplot();
    let options = BarbsOptions {
        barbcolor: Some("C0".into()),
        flagcolor: Some("C0".into()),
        length: 7.0,
        props: CollectionProps::default().linewidth(1.5),
        ..BarbsOptions::default()
    };
    ax.barbs(VectorField::new(x.as_slice(), y.as_slice(), u, v), options)?;
    ax.set_xlim(0.0, 4.5).set_ylim(0.0, 4.5);
    Ok(())
}

fn contour(fig: &Figure) -> CaseResult {
    let field = func3_field();
    let levels = extrema_levels(field.z.as_slice());
    fig.subplot().contour(&field, levels)?;
    Ok(())
}

fn contourf(fig: &Figure) -> CaseResult {
    let field = func3_field();
    let levels = extrema_levels(field.z.as_slice());
    fig.subplot().contourf(&field, levels)?;
    Ok(())
}

fn imshow(fig: &Figure) -> CaseResult {
    let (_, z) = func3_grid(16);
    let options = ImshowOptions {
        origin: Some(Origin::Lower),
        ..ImshowOptions::default()
    };
    fig.subplot().imshow(z, options)?;
    Ok(())
}

fn pcolormesh(fig: &Figure) -> CaseResult {
    let x = vec![
        -3.0, -2.0, -1.6, -1.2, -0.8, -0.5, -0.2, 0.1, 0.3, 0.5, 0.8, 1.1, 1.5, 1.9, 2.3, 3.0,
    ];
    let y = linspace(-3.0, 3.0, 128);
    let z = Grid::from_fn(y.len(), x.len(), |i, j| func3(x[j], y[i]));
    let options = PcolorOptions {
        colors: ColorizerOptions {
            vmin: Some(-0.5),
            vmax: Some(1.0),
            ..ColorizerOptions::default()
        },
        ..PcolorOptions::default()
    };
    fig.subplot()
        .pcolormesh(Mesh::with_coords(x, y, z), options)?;
    Ok(())
}

fn quiver(fig: &Figure) -> CaseResult {
    let t = linspace(-4.0, 4.0, 6);
    let (x, y) = meshgrid(&t, &t);
    let u: Vec<f64> = x
        .as_slice()
        .iter()
        .zip(y.as_slice())
        .map(|(x, y)| x + y)
        .collect();
    let v: Vec<f64> = x
        .as_slice()
        .iter()
        .zip(y.as_slice())
        .map(|(x, y)| y - x)
        .collect();
    let ax = fig.subplot();
    let options = QuiverOptions {
        color: "C0".into(),
        angles: Angles::Xy,
        scale_units: Some(QuiverUnits::Xy),
        scale: Some(5.0),
        width: Some(0.015),
        ..QuiverOptions::default()
    };
    ax.quiver(VectorField::new(x.as_slice(), y.as_slice(), u, v), options)?;
    ax.set_xlim(-5.0, 5.0).set_ylim(-5.0, 5.0);
    Ok(())
}

fn streamplot(fig: &Figure) -> CaseResult {
    let (t, z) = func3_grid(256);
    let n = t.len() - 1;
    // The flow along the contours of `func3`: `np.diff` of the stream
    // function across the columns and down the rows.
    let v = Grid::from_fn(n, n, |i, j| z[(i + 1, j + 1)] - z[(i + 1, j)]);
    let u = Grid::from_fn(n, n, |i, j| z[(i, j + 1)] - z[(i + 1, j + 1)]);
    let field = StreamField::new(t[1..].to_vec(), t[1..].to_vec(), u, v);
    fig.subplot()
        .streamplot(&field, StreamplotOptions::default())?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    plain("gallery_plot_types/barbs", barbs),
    plain("gallery_plot_types/contour", contour),
    plain("gallery_plot_types/contourf", contourf),
    plain("gallery_plot_types/imshow", imshow),
    plain("gallery_plot_types/pcolormesh", pcolormesh),
    plain("gallery_plot_types/quiver", quiver),
    plain("gallery_plot_types/streamplot", streamplot),
];
