//! Contours, scatter, text, annotations, fills and clipped lines drawn
//! through the projections of `figures/cases_geo.py`.

use std::f64::consts::{FRAC_PI_2, PI};

use plt::artist::ArtistHandle;
use plt::axes::{FillBetweenOptions, GridOptions, ScatterColors, ScatterOptions};
use plt::collections::{CollectionProps, EdgeColors};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{Color, ColorizerOptions, colormaps};
use plt::contour::{ContourOptions, Levels, QuadCoords, QuadField};
use plt::figure::Figure;
use plt::geo::GeoProjection;
use plt::lines::LineProps;
use plt::numeric::{Grid, linspace, meshgrid};
use plt::orientation::Orientation;
use plt::patches::{ArrowStyle, PatchProps};
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps, HAlign};
use plt::transforms::Point;

use super::track;
use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("geo/hammer_contourf_colorbar", contourf_colorbar).sized(6.4, 4.0, 100.0),
    FigureCase::new("geo/aitoff_scatter_text_annotate", scatter_text_annotate),
    FigureCase::new("geo/mollweide_lines_beyond_edge", beyond_edge),
    FigureCase::new("geo/hammer_facecolor_no_labels", facecolor_no_labels),
];

fn contourf_colorbar(fig: &Figure) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Hammer);
    let (lon, lat) = meshgrid(&linspace(-PI, PI, 73), &linspace(-FRAC_PI_2, FRAC_PI_2, 37));
    let z = Grid::from_fn(37, 73, |i, j| {
        lat[(i, j)].cos().powi(2) * (3.0 * lon[(i, j)]).cos()
    });
    let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x: lon, y: lat });
    let options = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("viridis")?),
            ..ColorizerOptions::default()
        },
        levels: Levels::Count(8),
        ..ContourOptions::default()
    };
    let contours = ax.contourf(&field, options)?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax.axes().clone()),
        orientation: Some(Orientation::Horizontal),
        shrink: 0.6,
        pad: Some(0.08),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&contours, options)?;
    ax.grid(true);
    Ok(())
}

fn scatter_text_annotate(fig: &Figure) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Aitoff);
    let radians = |degrees: [f64; 6]| degrees.map(f64::to_radians);
    let lon = radians([-120.0, -60.0, 0.0, 45.0, 100.0, 150.0]);
    let lat = radians([40.0, -20.0, 51.0, 10.0, -35.0, 60.0]);
    let options = ScatterOptions {
        sizes: Some(vec![30.0, 60.0, 90.0, 120.0, 150.0, 180.0]),
        colors: ScatterColors::Single(Color::from("tab:orange")),
        edgecolors: Some(EdgeColors::Colors(vec![Color::from("k")])),
        ..ScatterOptions::default()
    };
    ax.scatter_with(lon, lat, options)?.set_zorder(3.0);
    ax.text(lon[2], lat[2] + 0.12, "Greenwich")
        .set_halign(HAlign::Center)
        .set_fontsize(8.0);
    let arrow = FancyArrowProps {
        patch: PatchProps::default().color("0.3"),
        ..FancyArrowProps::new("->".parse::<ArrowStyle>()?)
    };
    ax.annotate(
        "far east",
        (lon[5], lat[5]),
        AnnotateOptions {
            xytext: Some(Point::new(1.5, 1.2).into()),
            arrowprops: Some(ArrowProps::Fancy(arrow)),
            ..AnnotateOptions::default()
        },
    )?
    .set_fontsize(8.0);
    ax.grid(true);
    Ok(())
}

fn beyond_edge(fig: &Figure) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Mollweide);
    let lon = linspace(-4.0, 4.0, 100);
    let lat: Vec<f64> = lon.iter().map(|lon| 0.5 * lon.cos()).collect();
    ax.plot_with(lon, lat, LineProps::default().linewidth(2.0))?;
    ax.plot_with(
        [2.8, 3.6],
        [-1.0, -0.6],
        LineProps::fmt("o-")?.color("tab:red"),
    )?;
    let band = FillBetweenOptions {
        props: CollectionProps::default().color("tab:green").alpha(0.3),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(linspace(-1.0, 1.0, 20), [-0.4], [0.4], band)?;
    ax.grid(true);
    Ok(())
}

fn facecolor_no_labels(fig: &Figure) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Hammer);
    ax.set_facecolor("#dceefc");
    ax.set_xticklabels(Vec::<String>::new())?;
    ax.set_yticklabels(Vec::<String>::new())?;
    ax.grid_with(
        true,
        GridOptions {
            color: Some(Color::from("w")),
            linewidth: Some(1.5),
            ..GridOptions::default()
        },
    );
    let (lon, lat) = track();
    ax.plot_with(lon, lat, LineProps::default().color("tab:blue"))?;
    Ok(())
}
