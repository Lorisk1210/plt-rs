//! Standalone colorbars: continuous, beside an axes, discrete with
//! extensions, of listed colours spaced proportionally and with extensions
//! as long as the interior colours (`colorbar_only`).

use plt::colorbar::{ColorbarAxes, ColorbarOptions, ExtendFrac, Spacing};
use plt::colors::{BoundaryNorm, Colormap, Extend, Norm, Normalize, ScalarMappable, colormaps};
use plt::layout::LayoutEngine;
use plt::orientation::Orientation;
use plt::prelude::*;

use crate::figures::{CaseResult, FigureCase};

/// The tutorial's usual figure: a horizontal colorbar of `cmap` under
/// `norm`, without data, drawn on the only axes of a constrained figure.
fn strip(
    fig: &Figure,
    norm: impl Norm + 'static,
    cmap: Colormap,
    options: ColorbarOptions,
) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    fig.colorbar(
        &ScalarMappable::with_norm(cmap, Box::new(norm)),
        ColorbarOptions {
            axes: ColorbarAxes::Cax(ax),
            orientation: Some(Orientation::Horizontal),
            ..options
        },
    )?;
    Ok(())
}

/// Default colorbar options with a label.
fn labelled(label: &str) -> ColorbarOptions {
    ColorbarOptions {
        label: label.into(),
        ..ColorbarOptions::default()
    }
}

fn colorbar_only_1(fig: &Figure) -> CaseResult {
    strip(
        fig,
        Normalize::new(5.0, 10.0),
        colormaps::get("cool")?,
        labelled("Some Units"),
    )
}

fn colorbar_only_2(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    let magma =
        ScalarMappable::with_norm(colormaps::get("magma")?, Box::new(Normalize::new(0.0, 1.0)));
    fig.colorbar(
        &magma,
        ColorbarOptions {
            orientation: Some(Orientation::Vertical),
            label: "a colorbar label".into(),
            ..ColorbarOptions::beside(&ax)
        },
    )?;
    Ok(())
}

fn colorbar_only_3(fig: &Figure) -> CaseResult {
    let cmap = colormaps::get("viridis")?;
    let bounds = [-1.0, 2.0, 5.0, 7.0, 12.0, 15.0];
    let norm = BoundaryNorm::new(bounds, cmap.n(), Extend::Both)?;
    strip(
        fig,
        norm,
        cmap,
        labelled("Discrete intervals with extend='both' keyword"),
    )
}

/// A listed colormap with under and over colours.
fn listed(
    colors: [&str; 4],
    under: &str,
    over: &str,
) -> Result<Colormap, Box<dyn std::error::Error>> {
    Ok(Colormap::listed("from_list", colors.to_vec())?
        .with_under(under)?
        .with_over(over)?)
}

fn colorbar_only_4(fig: &Figure) -> CaseResult {
    let cmap = listed(["red", "green", "blue", "cyan"], "yellow", "magenta")?;
    let norm = BoundaryNorm::new([1.0, 2.0, 4.0, 7.0, 8.0], cmap.n(), Extend::Neither)?;
    strip(
        fig,
        norm,
        cmap,
        ColorbarOptions {
            extend: Some(Extend::Both),
            spacing: Spacing::Proportional,
            ..labelled("Discrete intervals, some other units")
        },
    )
}

fn colorbar_only_5(fig: &Figure) -> CaseResult {
    let cmap = listed(["royalblue", "cyan", "yellow", "orange"], "blue", "red")?;
    let norm = BoundaryNorm::new([-1.0, -0.5, 0.0, 0.5, 1.0], cmap.n(), Extend::Neither)?;
    strip(
        fig,
        norm,
        cmap,
        ColorbarOptions {
            extend: Some(Extend::Both),
            extendfrac: ExtendFrac::Auto,
            spacing: Spacing::Uniform,
            ..labelled("Custom extension lengths, some other units")
        },
    )
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_guides_colors_text/colorbar_only_1",
        colorbar_only_1,
    )
    .sized(6.0, 1.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colorbar_only_2",
        colorbar_only_2,
    ),
    FigureCase::new(
        "gallery_guides_colors_text/colorbar_only_3",
        colorbar_only_3,
    )
    .sized(6.0, 1.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colorbar_only_4",
        colorbar_only_4,
    )
    .sized(6.0, 1.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colorbar_only_5",
        colorbar_only_5,
    )
    .sized(6.0, 1.0, 100.0),
];
