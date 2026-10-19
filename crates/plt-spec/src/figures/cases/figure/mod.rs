//! `figures/cases_figure.py`: the gallery's subfigures (nested, with
//! ratios, spacing, colours, colorbars and super labels), super labels
//! under tight layout, figure legends outside and anchored, the compressed
//! layout, nested grid specs under constrained layout, subfigure legends
//! and `savefig.bbox = "tight"` with padding.

mod legends;
mod subfigures;

use plt::axes::PcolorOptions;
use plt::image::ColorizerOptions;
use plt::numeric::{Grid, linspace};
use plt::prelude::*;

use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// `T`: the abscissa of the line plots.
fn t() -> Vec<f64> {
    linspace(0.0, 10.0, 40)
}

/// `values` mapped through `f`.
fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}

/// `_field(seed)`: a smooth 30 × 30 field in about `[-2.5, 2.5]` whose
/// phase comes from the seed.
fn field(seed: u64) -> Grid<f64> {
    let phase = SpecRng::new(seed).next_f64();
    let x = linspace(0.0, 3.0, 30);
    Grid::from_fn(30, 30, |r, c| 2.5 * (x[r] + 1.3 * x[c] + 6.0 * phase).sin())
}

/// `pcolormesh(_field(seed), vmin=-2.5, vmax=2.5)` with `cmap`.
fn field_mesh(
    ax: &Axes,
    seed: u64,
    cmap: Option<&str>,
) -> Result<plt::collections::Collection, Box<dyn std::error::Error>> {
    let mut colors = ColorizerOptions::default().limits(Some(-2.5), Some(2.5));
    if let Some(name) = cmap {
        colors = colors.cmap(plt::colors::colormaps::get(name)?);
    }
    Ok(ax.pcolormesh(
        plt::axes::Mesh::new(field(seed)),
        PcolorOptions {
            colors,
            ..PcolorOptions::default()
        },
    )?)
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("figure/gallery_subfigures", subfigures::gallery_subfigures)
        .sized(9.0, 4.0, 100.0),
    FigureCase::new(
        "figure/subfigures_nested_ratios",
        subfigures::subfigures_nested_ratios,
    )
    .sized(8.0, 6.5, 100.0),
    FigureCase::new(
        "figure/tight_suptitle_suplabels",
        subfigures::tight_suptitle_suplabels,
    )
    .sized(6.0, 4.5, 100.0),
    FigureCase::new(
        "figure/legends_outside_constrained",
        legends::legends_outside_constrained,
    )
    .sized(7.0, 4.0, 100.0),
    FigureCase::new(
        "figure/legend_bbox_figure_coords",
        legends::legend_bbox_figure_coords,
    )
    .sized(6.0, 4.0, 100.0),
    FigureCase::new(
        "figure/compressed_layout_images",
        subfigures::compressed_layout_images,
    )
    .sized(5.0, 5.0, 100.0),
    FigureCase::new(
        "figure/constrained_subgridspec_colorbar",
        subfigures::constrained_subgridspec_colorbar,
    )
    .sized(7.0, 4.0, 100.0),
    FigureCase::new(
        "figure/subfigure_legends_suplabels",
        legends::subfigure_legends_suplabels,
    )
    .sized(7.0, 3.5, 100.0),
    FigureCase::new(
        "figure/savefig_tight_pad_extra_text",
        legends::savefig_tight_pad_extra_text,
    )
    .sized(5.0, 3.5, 100.0)
    .with_rc(&[("savefig.bbox", "tight"), ("savefig.pad_inches", "0.3")]),
];
