//! `figures/cases_geo.py`: the basic cases here, grids and artists in
//! submodules.

mod artists;
mod grids;

pub(crate) use artists::CASES as ARTISTS;
pub(crate) use grids::CASES as GRIDS;

use std::f64::consts::{FRAC_PI_2, PI};

use pltrs::artist::ArtistHandle;
use pltrs::axes::{GridOptions, MappedColors, Mesh, PcolorOptions, ScatterOptions};
use pltrs::colors::{Color, Colorize, ColorizerOptions, colormaps};
use pltrs::figure::Figure;
use pltrs::geo::{GeoAxes, GeoProjection};
use pltrs::lines::{LineProps, LineStyle};
use pltrs::numeric::{Grid, linspace};
use pltrs::patches::PatchProps;

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// A wavy track across every longitude (`_track`).
fn track() -> (Vec<f64>, Vec<f64>) {
    let lon = linspace(-PI, PI, 60);
    let lat = lon.iter().map(|lon| 0.6 * (2.0 * lon).sin()).collect();
    (lon, lat)
}

/// Geographic axes with their grid shown.
fn gridded(fig: &Figure, projection: GeoProjection) -> GeoAxes {
    let ax = fig.geo_subplot(projection);
    ax.grid(true);
    ax
}

const fn square(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build).sized(5.0, 5.0, 100.0)
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("geo/aitoff_grid", |fig| {
        gridded(fig, GeoProjection::Aitoff);
        Ok(())
    }),
    FigureCase::new("geo/hammer_grid", |fig| {
        gridded(fig, GeoProjection::Hammer);
        Ok(())
    }),
    square("geo/lambert_grid", |fig| {
        gridded(fig, GeoProjection::LAMBERT);
        Ok(())
    }),
    FigureCase::new("geo/mollweide_grid_title", |fig| {
        gridded(fig, GeoProjection::Mollweide).set_title("Mollweide");
        Ok(())
    }),
    FigureCase::new("geo/plot_scatter", |fig| {
        let ax = fig.geo_subplot(GeoProjection::Mollweide);
        let (lon, lat) = track();
        ax.plot_with(
            lon,
            lat,
            LineProps::default().color("tab:red").linewidth(2.0),
        )?;
        let mut rng = SpecRng::new(7);
        let lon = rng.uniform(40, -3.0, 3.0);
        let lat = rng.uniform(40, -1.4, 1.4);
        let options = ScatterOptions {
            sizes: Some(vec![30.0]),
            colors: MappedColors::new(lat.clone())
                .cmap(colormaps::get("viridis")?)
                .into(),
            ..ScatterOptions::default()
        };
        ax.scatter_with(lon, lat, options)?.set_zorder(3.0);
        ax.grid(true);
        Ok(())
    }),
    FigureCase::new("geo/fill_markers", |fig| {
        let ax = fig.geo_subplot(GeoProjection::Hammer);
        ax.fill(
            [-2.5, -0.5, 0.5, 2.0, 1.0, -1.5],
            [-0.4, -1.0, -0.2, 0.3, 1.2, 0.9],
            PatchProps::default().color("tab:green").alpha(0.5),
        )?;
        ax.plot_with(
            linspace(-2.8, 2.8, 8),
            vec![-1.2; 8],
            LineProps::fmt("o--")?.color("tab:purple"),
        )?;
        ax.grid(true);
        Ok(())
    }),
    FigureCase::new("geo/pcolormesh", |fig| {
        let ax = fig.geo_subplot(GeoProjection::Mollweide);
        let lon = linspace(-PI, PI, 37);
        let lat = linspace(-FRAC_PI_2, FRAC_PI_2, 19);
        let values = Grid::from_fn(18, 36, |i, j| {
            let (lon, lat) = (lon[j] + PI / 36.0, lat[i] + PI / 36.0);
            lat.cos() * (2.0 * lon).sin()
        });
        let options = PcolorOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("RdBu_r")?),
            ..PcolorOptions::default()
        };
        ax.pcolormesh(Mesh::from_coords(lon, lat, values), options)?;
        ax.grid(true);
        Ok(())
    }),
    FigureCase::new("geo/custom_grid", |fig| {
        let ax = fig.geo_subplot(GeoProjection::Aitoff);
        ax.set_longitude_grid(45.0)?
            .set_latitude_grid(30.0)?
            .set_longitude_grid_ends(60.0);
        ax.grid_with(
            true,
            GridOptions {
                color: Some(Color::from("tab:blue")),
                linestyle: Some(LineStyle::Dashed),
                linewidth: Some(0.8),
                ..GridOptions::default()
            },
        );
        Ok(())
    }),
    square("geo/lambert_center", |fig| {
        let ax = gridded(
            fig,
            GeoProjection::Lambert {
                center_longitude: PI / 4.0,
                center_latitude: PI / 6.0,
            },
        );
        let (lon, lat) = track();
        ax.plot_with(lon, lat, LineProps::default().color("k"))?;
        ax.set_title("centred at 45°E, 30°N");
        Ok(())
    }),
    FigureCase::new("geo/subplots_grid_on_off", |fig| {
        let (lon, lat) = track();
        let left = fig.add_geo_subplot(GeoProjection::Aitoff, 1, 2, 1)?;
        left.plot(&lon, &lat)?;
        left.set_title("grid off");
        let right = fig.add_geo_subplot(GeoProjection::Hammer, 1, 2, 2)?;
        right.plot(&lon, &lat)?;
        right.grid(true);
        right.set_title("grid on");
        Ok(())
    })
    .sized(8.0, 3.0, 100.0),
];
