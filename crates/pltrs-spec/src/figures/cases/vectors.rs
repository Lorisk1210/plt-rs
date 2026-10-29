//! `figures/cases_vectors.py`: rasterized artists, a colorbar with enough
//! colours to be rasterized, and text to embed fonts for.

use pltrs::artist::{AggFilter, ArtistHandle, FilterImage, Filtered};
use pltrs::axes::{ImshowOptions, MappedColors, Mesh, PcolorOptions, ScatterOptions};
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions};
use pltrs::figure::Figure;
use pltrs::lines::LineProps;
use pltrs::numeric::{Grid, linspace};

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

/// `_half_opacity`: the image at half opacity, `dpi / 25` to the right.
fn half_opacity(image: &FilterImage, dpi: f64) -> Filtered {
    let mut out = image.clone();
    out.pixels_mut()
        .iter_mut()
        .for_each(|pixel| pixel[3] *= 0.5);
    Filtered {
        image: out,
        offset: (dpi / 25.0, 0.0),
    }
}

fn agg_filter(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 11.0, 30);
    let filter = AggFilter::new(half_opacity);
    let wide = || LineProps::default().linewidth(4.0);
    let rising: Vec<f64> = x.iter().map(|v| 0.6 * v).collect();
    ax.plot_with(x.clone(), rising, wide())?
        .set_rasterized(true)
        .set_agg_filter(Some(filter.clone()));
    let falling: Vec<f64> = x.iter().map(|v| 6.0 - 0.5 * v).collect();
    ax.plot_with(x, falling, wide())?
        .set_agg_filter(Some(filter));
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
    FigureCase::new("vectors/agg_filter", agg_filter).sized(2.4, 1.6, 50.0),
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
