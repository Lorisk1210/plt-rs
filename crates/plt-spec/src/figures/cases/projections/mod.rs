//! `figures/cases_projections.py`: projections the program registers,
//! wherever axes are made — subplot grids, mosaics, insets and twins —
//! with the options that reach the projection when the axes is made.

mod range_frame;

use plt::axes::{AxisName, ProjectionKind, ScatterColors, ScatterOptions, SecondaryLocation};
use plt::figure::{Mosaic, MosaicOptions, Share};
use plt::prelude::*;
use plt::scale::FuncTransform;
use plt::spines::SpineSide;

pub(crate) use self::range_frame::{
    RangeFrame, RangeFrameOptions, register as register_range_frame,
};
use crate::figures::{CaseResult, FigureCase};

/// `projection="range_frame", pad=pad`.
fn range_frame(pad: f64) -> Result<plt::projections::ProjectionSpec, Box<dyn std::error::Error>> {
    Ok(register_range_frame()?.with_options(RangeFrameOptions { pad }))
}

fn grid(fig: &Figure) -> CaseResult {
    let axs = fig.subplots_with_projection(2, 2, Share::All, Share::All, range_frame(0.08)?)?;
    let x = linspace(0.0, 2.0 * std::f64::consts::PI, 25);
    for (i, ax) in (0_u8..).zip(axs.as_slice()) {
        let y: Vec<f64> = x.iter().map(|x| (x + f64::from(i)).sin()).collect();
        ax.plot_with(&x, &y, LineProps::fmt("o-")?.markersize(3.0))?;
        ax.set_title(format!("phase {i}"));
        ax.grid(true);
    }
    Ok(())
}

fn geo_grid(fig: &Figure) -> CaseResult {
    let axs =
        fig.subplots_with_projection(1, 2, Share::None, Share::None, ProjectionKind::Hammer)?;
    let lon = linspace(-std::f64::consts::PI, std::f64::consts::PI, 30);
    for (i, ax) in (0_u8..).zip(axs.as_slice()) {
        let lat: Vec<f64> = lon.iter().map(|x| 0.5 * (x + f64::from(i)).sin()).collect();
        ax.plot_with(&lon, &lat, LineProps::default().color(format!("C{i}")))?;
        ax.grid(true);
    }
    Ok(())
}

fn mosaic(fig: &Figure) -> CaseResult {
    let options = MosaicOptions {
        projection: register_range_frame()?.into(),
        per_subplot: vec![
            ("B".into(), range_frame(0.2)?),
            ("C".into(), ProjectionKind::Hammer.into()),
        ],
        ..MosaicOptions::default()
    };
    let axd = fig.subplot_mosaic(&"AB;CC".parse::<Mosaic>()?, options)?;
    let panel = |label: &str| axd.get(label).ok_or(format!("no axes labelled {label:?}"));
    let x = linspace(-std::f64::consts::PI, std::f64::consts::PI, 30);
    let wave = |f: fn(f64) -> f64, scale: f64| x.iter().map(|&x| scale * f(x)).collect::<Vec<_>>();
    let a = panel("A")?;
    a.plot(&x, wave(f64::cos, 1.0))?;
    a.set_title("pad 0.05");
    let b = panel("B")?;
    b.scatter_with(
        &x,
        wave(f64::sin, 1.0),
        ScatterOptions {
            sizes: Some(vec![8.0]),
            colors: ScatterColors::Single("C1".into()),
            ..ScatterOptions::default()
        },
    )?;
    b.set_title("pad 0.2");
    let c = panel("C")?;
    let doubled: Vec<f64> = x.iter().map(|x| 0.5 * (2.0 * x).sin()).collect();
    c.plot_with(&x, &doubled, LineProps::default().color("C2"))?;
    c.grid(true);
    Ok(())
}

fn insets(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_with_projection(range_frame(0.1)?)?;
    let x = linspace(0.0, 10.0, 60);
    ax.plot(&x, x.iter().map(|x| x.sin()).collect::<Vec<_>>())?;
    let inset = ax.inset_axes_with_projection([0.55, 0.6, 0.4, 0.35], range_frame(0.15)?)?;
    let cosine: Vec<f64> = x.iter().map(|x| x.cos()).collect();
    inset.plot_with(&x, &cosine, LineProps::default().color("C1"))?;
    inset.set_title("pad 0.15").set_fontsize(8.0);
    let polar = ax.inset_axes_with_projection([0.05, 0.05, 0.3, 0.3], ProjectionKind::Polar)?;
    let radii: Vec<f64> = x.iter().map(|x| x / 10.0).collect();
    polar.plot_with(&x, &radii, LineProps::default().color("C2"))?;
    polar
        .as_polar()
        .ok_or("a polar inset")?
        .set_rticks(&[0.5, 1.0]);
    Ok(())
}

fn twins(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_with_projection(range_frame(0.1)?)?;
    let x = linspace(0.0, 10.0, 40);
    let squares: Vec<f64> = x.iter().map(|x| x * x).collect();
    ax.plot_with(&x, &squares, LineProps::default().color("C0"))?;
    ax.set_ylabel("x squared");
    let twin = ax.twin_with_projection(AxisName::X, 0.0, range_frame(0.1)?)?;
    let roots: Vec<f64> = x.iter().map(|x| x.sqrt()).collect();
    twin.plot_with(&x, &roots, LineProps::default().color("C1"))?;
    twin.set_ylabel("square root");
    let doubled = FuncTransform::new(|v| 2.0 * v, |v| v / 2.0);
    let top = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(doubled))?;
    top.set_xlabel("twice x");
    Ok(())
}

/// Every case of the group.
pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("projections/grid", grid),
    FigureCase::new("projections/geo_grid", geo_grid),
    FigureCase::new("projections/mosaic", mosaic),
    FigureCase::new("projections/insets", insets),
    FigureCase::new("projections/twins", twins),
];
