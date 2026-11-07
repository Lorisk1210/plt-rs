//! `figures/cases_polar.py`: the basic cases here, sectors and radial
//! settings in submodules.

mod radial;
mod sectors;

pub(crate) use radial::CASES as RADIAL;
pub(crate) use sectors::CASES as SECTORS;

use std::f64::consts::{PI, TAU};

use pltrs::axes::{AxisName, FillBetweenOptions};
use pltrs::axes::{
    BarOptions, GridOptions, MappedColors, Mesh, PcolorOptions, ScatterOptions, TickParams,
};
use pltrs::collections::{CollectionProps, EdgeColors};
use pltrs::colors::{Color, Colorize, ColorizerOptions, colormaps};
use pltrs::contour::{ContourOptions, Levels, QuadCoords, QuadField};
use pltrs::figure::Figure;
use pltrs::fonts::FontSize;
use pltrs::legend::LegendOptions;
use pltrs::lines::{LineProps, LineStyle};
use pltrs::numeric::{Grid, linspace, meshgrid, pow};
use pltrs::offsetbox::AnchorLoc;
use pltrs::patches::PatchProps;
use pltrs::polar::{Compass, PolarAxes, ThetaDirection};

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Two turns of an Archimedean spiral (`_spiral`).
fn spiral() -> (Vec<f64>, Vec<f64>) {
    let r = linspace(0.0, 2.0, 200);
    (r.iter().map(|r| 2.0 * PI * r).collect(), r)
}

/// Polar axes with the spiral plotted.
fn spiral_axes(fig: &Figure) -> Result<PolarAxes, Box<dyn std::error::Error>> {
    let ax = fig.polar_subplot();
    let (theta, r) = spiral();
    ax.plot(theta, r)?;
    Ok(ax)
}

const fn square(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build).sized(4.0, 4.0, 100.0)
}

/// `meshgrid(linspace(0, 2π, n_theta), linspace(0, 2, n_r))`.
fn polar_grid(n_theta: usize, n_r: usize) -> (Grid<f64>, Grid<f64>) {
    meshgrid(&linspace(0.0, TAU, n_theta), &linspace(0.0, 2.0, n_r))
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("polar/plot_default", |fig| {
        spiral_axes(fig)?;
        Ok(())
    }),
    square("polar/plot_markers_multiple", |fig| {
        let ax = fig.polar_subplot();
        let theta = linspace(0.0, TAU, 13);
        let trefoil: Vec<f64> = theta.iter().map(|t| 1.0 + 0.5 * (3.0 * t).cos()).collect();
        ax.plot_with(&theta, trefoil, LineProps::fmt("o-")?.label("trefoil"))?;
        ax.plot_with(
            &theta,
            vec![0.8; 13],
            LineProps::fmt("s--")?.label("circle"),
        )?;
        ax.legend(LegendOptions::default().loc(AnchorLoc::LowerLeft))?;
        Ok(())
    }),
    square("polar/scatter", |fig| {
        let ax = fig.polar_subplot();
        let mut rng = SpecRng::new(31);
        let theta = rng.uniform(80, 0.0, TAU);
        let r = rng.uniform(80, 0.0, 2.0);
        let options = ScatterOptions {
            sizes: Some(r.iter().map(|r| 200.0 * r * r).collect()),
            colors: MappedColors::new(theta.clone())
                .cmap(colormaps::get("hsv")?)
                .into(),
            alpha: 0.75.into(),
            ..ScatterOptions::default()
        };
        ax.scatter_with(theta, r, options)?;
        Ok(())
    }),
    square("polar/bars", |fig| {
        let ax = fig.polar_subplot();
        let theta: Vec<f64> = (0..12).map(|i| f64::from(i) * (TAU / 12.0)).collect();
        let radii: Vec<f64> = (0..12).map(|i| f64::from(1 + i % 5)).collect();
        let options = BarOptions {
            thickness: 0.45.into(),
            base: vec![1.0].into(),
            color: ["C0", "C1", "C2"]
                .repeat(4)
                .into_iter()
                .map(Color::from)
                .collect(),
            edgecolor: vec![Color::from("k")],
            props: PatchProps::default().alpha(0.6),
            ..BarOptions::default()
        };
        ax.bar(theta, radii, options)?;
        Ok(())
    }),
    square("polar/fill_and_fill_between", |fig| {
        let ax = fig.polar_subplot();
        let theta = linspace(0.0, TAU, 7);
        ax.fill(
            theta,
            [3.0, 2.0, 3.5, 1.5, 2.5, 3.0, 3.0],
            PatchProps::default().alpha(0.4).color("tab:green"),
        )?;
        let t = linspace(0.0, PI / 2.0, 30);
        let upper: Vec<f64> = t.iter().map(|t| 1.0 + (4.0 * t).sin()).collect();
        let band = FillBetweenOptions {
            props: CollectionProps::default().color("tab:purple").alpha(0.5),
            ..FillBetweenOptions::default()
        };
        ax.fill_between(&t, [0.5], upper, band)?;
        Ok(())
    }),
    square("polar/theta_zero_north_clockwise", |fig| {
        spiral_axes(fig)?
            .set_theta_zero_location(Compass::N, 0.0)
            .set_theta_direction(ThetaDirection::Clockwise);
        Ok(())
    }),
    square("polar/theta_zero_offset", |fig| {
        spiral_axes(fig)?.set_theta_zero_location(Compass::W, -20.0);
        Ok(())
    }),
    square("polar/thetagrids_labels", |fig| {
        spiral_axes(fig)?
            .set_thetagrids_labeled([0.0, 90.0, 180.0, 270.0], ["E", "N", "W", "S"])?;
        Ok(())
    }),
    square("polar/sector", |fig| {
        spiral_axes(fig)?.set_thetamin(45.0).set_thetamax(135.0);
        Ok(())
    }),
    FigureCase::new("polar/half_sector_rorigin", |fig| {
        let ax = fig.polar_subplot();
        let theta = linspace(-PI / 2.0, PI / 2.0, 50);
        let r: Vec<f64> = theta.iter().map(|t| 2.0 + (3.0 * t).cos()).collect();
        ax.plot(theta, r)?;
        ax.set_thetamin(-90.0)
            .set_thetamax(90.0)
            .set_rorigin(Some(-1.5));
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    square("polar/rorigin_hole", |fig| {
        spiral_axes(fig)?.set_rorigin(Some(-1.0));
        Ok(())
    }),
    square("polar/rmin_rmax", |fig| {
        spiral_axes(fig)?.set_rmin(0.5).set_rmax(1.5);
        Ok(())
    }),
    square("polar/rticks_rlabel_position", |fig| {
        spiral_axes(fig)?
            .set_rticks([0.5, 1.0, 1.5, 2.0])
            .set_rlabel_position(-22.5);
        Ok(())
    }),
    square("polar/rscale_log", |fig| {
        let ax = fig.polar_subplot();
        let theta = linspace(0.0, TAU, 200);
        let r: Vec<f64> = theta
            .iter()
            .map(|t| pow(10.0, 1.0 + 2.0 * t / TAU))
            .collect();
        ax.plot(theta, r)?;
        ax.set_rscale("log")?;
        Ok(())
    }),
    square("polar/grid_styling", |fig| {
        let ax = fig.polar_subplot();
        let (theta, r) = spiral();
        ax.plot_fmt(theta, r, "k")?;
        ax.grid_with(
            true,
            GridOptions {
                color: Some(Color::from("tab:orange")),
                linestyle: Some(LineStyle::Dotted),
                linewidth: Some(1.5),
                ..GridOptions::default()
            },
        );
        ax.set_facecolor("#f0f0ff");
        ax.tick_params(TickParams {
            axis: Some(AxisName::X),
            colors: Some(Color::from("tab:blue")),
            labelsize: Some(FontSize::Points(8.0)),
            ..TickParams::default()
        });
        ax.set_title("styled grid");
        Ok(())
    }),
    square("polar/contourf", |fig| {
        let (theta, r) = polar_grid(60, 30);
        let z = Grid::from_fn(30, 60, |i, j| r[(i, j)] * (3.0 * theta[(i, j)]).cos());
        let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x: theta, y: r });
        let options = ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("RdBu")?),
                ..ColorizerOptions::default()
            },
            levels: Levels::Count(10),
            ..ContourOptions::default()
        };
        fig.polar_subplot().contourf(&field, options)?;
        Ok(())
    }),
    square("polar/pcolormesh", |fig| {
        let (theta, r) = polar_grid(25, 9);
        let values = Grid::from_fn(8, 24, |i, j| theta[(i, j)].sin() * r[(i, j)]);
        let options = PcolorOptions {
            props: CollectionProps {
                edgecolors: Some(EdgeColors::Colors(vec!["k".into()])),
                linewidths: Some(vec![0.3]),
                ..CollectionProps::default()
            },
            ..PcolorOptions::default()
        };
        fig.polar_subplot()
            .pcolormesh(Mesh::from_coords(theta, r, values), options)?;
        Ok(())
    }),
];
