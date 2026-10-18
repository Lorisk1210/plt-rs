//! `figures/cases_vectors.py`: rasterized artists, a colorbar with enough
//! colours to be rasterized, and text to embed fonts for.

use plt::artist::ArtistHandle;
use plt::axes::{ImshowOptions, MappedColors, Mesh, PcolorOptions, ScatterOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::figure::Figure;
use plt::lines::LineProps;
use plt::numeric::{Grid, linspace};

use crate::figures::{CaseResult, FigureCase};

/// `_cells()`: an 8 x 12 field of smooth values.
fn cells() -> Grid<f64> {
    Grid::from_fn(8, 12, |i, j| {
        (0.4 * i as f64).sin() + (0.3 * j as f64).cos()
    })
}

fn rasterized_artists(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.pcolormesh(Mesh::new(cells()), PcolorOptions::default())?
        .set_rasterized(true);
    let x = linspace(0.0, 11.0, 30);
    let line: Vec<f64> = x.iter().map(|v| 0.6 * v).collect();
    ax.plot_with(
        x.clone(),
        line,
        LineProps::default().color("k").linewidth(2.0),
    )?;
    let falling: Vec<f64> = x.iter().map(|v| 7.0 - 0.5 * v).collect();
    let options = ScatterOptions {
        colors: MappedColors::new(x.clone()).into(),
        ..ScatterOptions::default()
    };
    ax.scatter_with(x.clone(), falling, options)?
        .set_rasterized(true)
        .set_zorder(3.0);
    let wave: Vec<f64> = x.iter().map(|v| 4.0 + 2.0 * v.sin()).collect();
    ax.plot(x, wave)?.set_rasterized(true).set_zorder(4.0);
    ax.set_title("rasterized");
    Ok(())
}

fn rasterization_zorder(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_rasterization_zorder(Some(1.5));
    ax.pcolormesh(Mesh::new(cells()), PcolorOptions::default())?;
    let x = linspace(0.0, 11.0, 30);
    let line: Vec<f64> = x.iter().map(|v| 0.6 * v).collect();
    ax.plot_with(x.clone(), line, LineProps::default().color("k"))?
        .set_zorder(1.5);
    let wave: Vec<f64> = x.iter().map(|v| 4.0 + 2.0 * v.sin()).collect();
    ax.plot(x, wave)?;
    Ok(())
}

fn colorbar_many_colors(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let image = ax.imshow(cells(), ImshowOptions::default())?;
    fig.colorbar(
        &image,
        ColorbarOptions {
            axes: ColorbarAxes::Parent(ax.clone()),
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("vectors/rasterized_artists", rasterized_artists).sized(2.4, 1.6, 50.0),
    FigureCase::new("vectors/rasterization_zorder", rasterization_zorder).sized(2.4, 1.6, 50.0),
    FigureCase::new("vectors/colorbar_many_colors", colorbar_many_colors).sized(2.4, 1.6, 50.0),
    FigureCase::new("vectors/text_fonts", |fig| {
        for (x, y, text) in [
            (0.04, 0.65, "office fit caf\u{e9}"),
            (0.04, 0.2, r"$\alpha^2+\frac{1}{2}$"),
            (0.6, 0.2, "\u{2212}1.5"),
        ] {
            fig.text(x, y, text).set_fontsize(12.0);
        }
        Ok(())
    })
    .sized(3.0, 1.2, 100.0),
];
