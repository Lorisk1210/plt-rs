//! Hatched contours, colorbar lines and two-dimensional histograms.

use pltrs::axes::Hist2dOptions;
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions};
use pltrs::colors::{Color, ColorizerOptions, Extend, colormaps};
use pltrs::contour::{ContourOptions, Levels, QuadCoords, QuadField};
use pltrs::figure::Figure;
use pltrs::lines::LineStyle;
use pltrs::numeric::{Grid, arange, linspace};
use pltrs::prelude::ArtistHandle;
use pltrs::stats::{AxisBins, Histogram2dOptions};

use super::remove_ticks_and_titles;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `contour_dat()`: `cos(x) + sin(y)` on a 150 × 120 grid over `[-3, 5]²`.
fn contour_data() -> QuadField {
    let x = linspace(-3.0, 5.0, 150);
    let y = linspace(-3.0, 5.0, 120);
    let z = Grid::from_fn(y.len(), x.len(), |r, c| x[c].cos() + y[r].sin());
    QuadField::new(z).with_coords(QuadCoords::Axes { x, y })
}

/// `test_contour_colorbar`: filled contours with extensions, two sets of
/// line contours and their lines on the colorbar.
fn contour_colorbar(fig: &Figure) -> CaseResult {
    let field = contour_data();
    let ax = fig.subplot();
    let fills = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("RdBu")?),
            vmin: Some(-0.6),
            vmax: Some(0.6),
            ..ColorizerOptions::default()
        },
        levels: Levels::Values(arange(-1.8, 1.801, 0.2)),
        extend: Extend::Both,
        ..ContourOptions::default()
    };
    let cs = ax.contourf(&field, fills)?;
    let lines = |levels: Vec<f64>, color: &str| ContourOptions {
        levels: Levels::Values(levels),
        colors: Some(vec![Color::from(color)]),
        linewidths: Some(vec![2.0]),
        ..ContourOptions::default()
    };
    let cs1 = ax.contour(
        &field,
        ContourOptions {
            linestyles: Some(vec![LineStyle::Solid]),
            ..lines(arange(-2.2, -0.599, 0.2), "y")
        },
    )?;
    let cs2 = ax.contour(&field, lines(arange(0.6, 2.2, 0.2), "c"))?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax.clone()),
        ..ColorbarOptions::default()
    };
    let cbar = fig.colorbar(&cs, options)?;
    cbar.add_lines(&cs1, true)?;
    cbar.add_lines(&cs2, false)?;
    Ok(())
}

/// `ax.hist2d(x, y, bins=10, rasterized=True)` with the text removed.
fn hist2d(fig: &Figure, x: &[f64], y: &[f64]) -> CaseResult {
    let options = Hist2dOptions {
        histogram: Histogram2dOptions {
            bins: [AxisBins::Count(10), AxisBins::Count(10)],
            ..Histogram2dOptions::default()
        },
        ..Hist2dOptions::default()
    };
    fig.subplot()
        .hist2d(x, y, options)?
        .mesh
        .set_rasterized(true);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The rcParams of the hist2d tests, whose baselines predate mesh snapping.
const NO_MESH_SNAP: &[(&str, &str)] = &[("pcolormesh.snap", "False")];

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_axes_b/contour_hatching", |fig| {
        let options = ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("gray")?.with_alpha(0.5)?),
                ..ColorizerOptions::default()
            },
            levels: Levels::Count(7),
            hatches: ["/", "\\", "//", "-"]
                .map(|hatch| Some(hatch.to_owned()))
                .to_vec(),
            extend: Extend::Both,
            ..ContourOptions::default()
        };
        fig.subplot().contourf(&contour_data(), options)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_axes_b/contour_colorbar", contour_colorbar),
    FigureCase::new("mpl_axes_b/hist2d", |fig| {
        let mut stream = SpecRng::new(0);
        let x = stream.normal(100, 5.0, 2.0);
        let y = stream.normal(100, -2.0, 1.0);
        hist2d(fig, &x, &y)
    })
    .with_rc(NO_MESH_SNAP),
    FigureCase::new("mpl_axes_b/hist2d_transpose", |fig| {
        let y = SpecRng::new(0).normal(100, -2.0, 1.0);
        hist2d(fig, &[5.0; 100], &y)
    })
    .with_rc(NO_MESH_SNAP),
];
