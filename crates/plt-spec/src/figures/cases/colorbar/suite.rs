//! `figures/cases_colorbar_ext.py`: colorbars from matplotlib's test
//! suite — extension lengths and shapes for both spacings, edges with
//! extensions, explicit boundaries and values, log, power, symlog and
//! centred norms, anchors and tick locations.

use plt::axes::ImshowOptions;
use plt::axes::{TickLevel, TickParams};
use plt::colorbar::{
    ColorbarAxes, ColorbarFormat, ColorbarOptions, ColorbarTicks, ExtendFrac, ParentAnchor, Spacing,
};
use plt::colors::{
    BoundaryNorm, CenteredNorm, ColorizerOptions, Colormap, Extend, LogNorm, Norm, PowerNorm,
    ScalarMappable, SymLogNorm, colormaps,
};
use plt::figure::Figure;
use plt::gridspec::SubplotOverrides;
use plt::numeric::{Grid, linspace};
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::spines::SpineSide;
use plt_core::Anchor;

use super::data::field;
use crate::figures::CaseResult;
use plt::ticker::LabelFormat;

/// The extension kinds in the order of the test suite.
const EXTENSIONS: [Extend; 4] = [Extend::Neither, Extend::Min, Extend::Max, Extend::Both];

/// `_cmap_norms()`: `RdBu` resampled to five colours and the boundary norm
/// of each extension kind.
fn cmap_norm(extend: Extend) -> Result<(Colormap, BoundaryNorm), Box<dyn std::error::Error>> {
    let cmap = colormaps::get("RdBu")?.resampled(5)?;
    let clevs = [-5.0, -2.5, -0.5, 0.5, 1.5, 3.5];
    let mut levels = clevs.to_vec();
    if matches!(extend, Extend::Min | Extend::Both) {
        levels[0] = -10.0;
    }
    if matches!(extend, Extend::Max | Extend::Both) {
        levels[5] = 10.0;
    }
    let norm = BoundaryNorm::new(levels, clevs.len() - 1, Extend::Neither)?;
    Ok((cmap, norm))
}

/// `Colorbar(cax, cmap=cmap, norm=norm, boundaries=..., values=...,
/// orientation="horizontal", ...)` of the extension cases, with the ticks
/// and labels turned off.
fn bare_colorbar(fig: &Figure, cax: &Axes, extend: Extend, options: ColorbarOptions) -> CaseResult {
    let (cmap, norm) = cmap_norm(extend)?;
    let boundaries = norm.boundaries().to_vec();
    let values = boundaries[..boundaries.len() - 1].to_vec();
    let mappable = ScalarMappable::with_norm(cmap, Box::new(norm));
    fig.colorbar(
        &mappable,
        ColorbarOptions {
            axes: ColorbarAxes::Cax(cax.clone()),
            boundaries: Some(boundaries),
            values: Some(values),
            extend: Some(extend),
            orientation: Some(Orientation::Horizontal),
            ..options
        },
    )?;
    cax.tick_params(&TickParams {
        left: Some(false),
        labelleft: Some(false),
        bottom: Some(false),
        labelbottom: Some(false),
        ..TickParams::default()
    });
    Ok(())
}

/// `_extension_length(fig, spacing)`: every extension kind with the
/// default, automatic and 0.1 extension lengths.
fn extension_length(fig: &Figure, spacing: Spacing) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        hspace: Some(0.6),
        ..SubplotOverrides::default()
    })?;
    for (i, extend) in EXTENSIONS.into_iter().enumerate() {
        for (j, extendfrac) in [ExtendFrac::Default, ExtendFrac::Auto, ExtendFrac::Both(0.1)]
            .into_iter()
            .enumerate()
        {
            let cax = fig.add_subplot(12, 1, i * 3 + j + 1)?;
            let options = ColorbarOptions {
                extendfrac,
                spacing,
                ..ColorbarOptions::default()
            };
            bare_colorbar(fig, &cax, extend, options)?;
        }
    }
    Ok(())
}

pub(super) fn extension_length_uniform(fig: &Figure) -> CaseResult {
    extension_length(fig, Spacing::Uniform)
}

pub(super) fn extension_length_proportional(fig: &Figure) -> CaseResult {
    extension_length(fig, Spacing::Proportional)
}

pub(super) fn extension_shape_rect(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        hspace: Some(4.0),
        wspace: Some(0.3),
        ..SubplotOverrides::default()
    })?;
    for (col, spacing) in [Spacing::Uniform, Spacing::Proportional]
        .into_iter()
        .enumerate()
    {
        for (i, extend) in EXTENSIONS.into_iter().enumerate() {
            let cax = fig.add_subplot(4, 2, 2 * i + col + 1)?;
            let options = ColorbarOptions {
                extendrect: true,
                spacing,
                ..ColorbarOptions::default()
            };
            bare_colorbar(fig, &cax, extend, options)?;
        }
    }
    Ok(())
}

pub(super) fn drawedges_with_extensions(fig: &Figure) -> CaseResult {
    let cmap = Colormap::listed(
        "from_list",
        vec![
            "tab:blue",
            "tab:orange",
            "tab:green",
            "tab:red",
            "tab:purple",
            "tab:brown",
        ],
    )?;
    fig.subplots_adjust(&SubplotOverrides {
        hspace: Some(1.5),
        ..SubplotOverrides::default()
    })?;
    let variants = [
        (Extend::Neither, ExtendFrac::Default, "neither"),
        (Extend::Both, ExtendFrac::Default, "both"),
        (Extend::Min, ExtendFrac::Both(0.3), "min"),
        (Extend::Max, ExtendFrac::Auto, "max"),
    ];
    let edges = |norm: BoundaryNorm, options: ColorbarOptions, cax: Axes| -> CaseResult {
        let mappable = ScalarMappable::with_norm(cmap.clone(), Box::new(norm));
        fig.colorbar(
            &mappable,
            ColorbarOptions {
                axes: ColorbarAxes::Cax(cax),
                orientation: Some(Orientation::Horizontal),
                drawedges: true,
                ..options
            },
        )?;
        Ok(())
    };
    for (i, (extend, extendfrac, name)) in variants.into_iter().enumerate() {
        let cax = fig.add_subplot(5, 1, i + 1)?;
        let lo = usize::from(matches!(extend, Extend::Min | Extend::Both));
        let hi = 6 - usize::from(matches!(extend, Extend::Max | Extend::Both));
        let levels: Vec<f64> = (lo..=hi).map(|v| v as f64).collect();
        let norm = BoundaryNorm::new(levels, cmap.n(), extend)?;
        edges(
            norm,
            ColorbarOptions {
                extendfrac,
                ..ColorbarOptions::default()
            },
            cax.clone(),
        )?;
        cax.set_title(format!("extend={name}")).set_fontsize(8.0);
    }
    let cax = fig.add_subplot(5, 1, 5)?;
    let norm = BoundaryNorm::new([0.0, 1.0, 2.0, 4.0, 6.0], cmap.n(), Extend::Both)?;
    edges(
        norm,
        ColorbarOptions {
            extendrect: true,
            spacing: Spacing::Proportional,
            ..ColorbarOptions::default()
        },
        cax,
    )
}

/// `imshow` options with a colormap and a norm.
fn normed(
    cmap: &str,
    norm: impl Norm + 'static,
) -> Result<plt::axes::ImshowOptions, Box<dyn std::error::Error>> {
    Ok(ImshowOptions::default().colorizer(
        ColorizerOptions::default()
            .cmap(colormaps::get(cmap)?)
            .norm(norm),
    ))
}

pub(super) fn boundaries_values_irregular(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let im = ax1.imshow(
        field(40),
        ImshowOptions::default().cmap(colormaps::get("viridis")?),
    )?;
    fig.colorbar(
        &im,
        ColorbarOptions {
            boundaries: Some(vec![-0.8, -0.4, -0.1, 0.0, 0.2, 0.8]),
            values: Some(vec![-0.6, -0.25, -0.05, 0.1, 0.5]),
            ticks: Some(ColorbarTicks::Values(vec![-0.8, -0.1, 0.2, 0.8])),
            format: Some(ColorbarFormat::Label(LabelFormat::Template("%.2f".into()))),
            ..ColorbarOptions::beside(ax1)
        },
    )?;
    let norm = BoundaryNorm::new([-1.0, -0.3, -0.1, 0.0, 0.1, 0.3, 1.0], 256, Extend::Neither)?;
    let im = ax2.imshow(field(40), normed("coolwarm", norm)?)?;
    fig.colorbar(
        &im,
        ColorbarOptions {
            orientation: Some(Orientation::Horizontal),
            spacing: Spacing::Proportional,
            ticks: Some(ColorbarTicks::Values(vec![-1.0, -0.1, 0.1, 1.0])),
            label: "proportional".into(),
            ..ColorbarOptions::beside(ax2)
        },
    )?;
    Ok(())
}

/// `10 ** np.add.outer(x, x / 2)` for 40 points over `[0, 4]`.
fn decades() -> Grid<f64> {
    let x = linspace(0.0, 4.0, 40);
    Grid::from_fn(40, 40, |r, c| 10f64.powf(x[r] + x[c] / 2.0))
}

pub(super) fn lognorm_extend_minor_custom_ticks(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let im = ax1.imshow(
        decades(),
        ImshowOptions::default()
            .colorizer(ColorizerOptions::default().norm(LogNorm::new(10.0, 1e4))),
    )?;
    let cb = fig.colorbar(
        &im,
        ColorbarOptions {
            extend: Some(Extend::Both),
            ..ColorbarOptions::beside(ax1)
        },
    )?;
    cb.minorticks_on();
    cb.ax().tick_params(&TickParams {
        which: TickLevel::Minor,
        length: Some(4.0),
        color: Some(Color::from("tab:red")),
        ..TickParams::default()
    });
    let im = ax2.imshow(decades(), normed("magma", LogNorm::default())?)?;
    let cb = fig.colorbar(
        &im,
        ColorbarOptions {
            orientation: Some(Orientation::Horizontal),
            ..ColorbarOptions::beside(ax2)
        },
    )?;
    cb.set_ticks(vec![1.0, 100.0, 1e4, 1e6]);
    cb.long_axis()
        .set_ticklabels(["one", "hundred", "10k", "1M"])?;
    cb.minorticks_off();
    Ok(())
}

pub(super) fn power_symlog_centered_norms(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    let data = field(40).map(|v| v * 3.0);
    let im = axs[(0, 0)].imshow(
        data.map(|v| v.abs()),
        normed("cividis", PowerNorm::new(0.5))?,
    )?;
    fig.colorbar(&im, ColorbarOptions::beside(&axs[(0, 0)]))?;
    let symlog = SymLogNorm::new(0.3, 1.0, 10.0)?
        .with_vmin(-30.0)
        .with_vmax(30.0);
    let im = axs[(0, 1)].imshow(data.map(|v| v * 10.0), normed("RdBu_r", symlog)?)?;
    fig.colorbar(&im, ColorbarOptions::beside(&axs[(0, 1)]))?;
    let centered = CenteredNorm::new(0.5, Some(2.0));
    let im = axs[(0, 2)].imshow(data.map(|v| v + 0.5), normed("PiYG", centered)?)?;
    fig.colorbar(
        &im,
        ColorbarOptions {
            extend: Some(Extend::Both),
            ..ColorbarOptions::beside(&axs[(0, 2)])
        },
    )?;
    Ok(())
}

pub(super) fn anchor_panchor_ticklocation(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let im = ax1.imshow(
        field(40),
        ImshowOptions::default().cmap(colormaps::get("plasma")?),
    )?;
    fig.colorbar(
        &im,
        ColorbarOptions {
            shrink: 0.5,
            anchor: Some(Anchor::Fraction(0.0, 1.0)),
            panchor: ParentAnchor::At(Anchor::Fraction(1.0, 0.0)),
            ticklocation: Some(SpineSide::Left),
            label: "left ticks".into(),
            ..ColorbarOptions::beside(ax1)
        },
    )?;
    let im = ax2.imshow(
        field(40),
        ImshowOptions::default().cmap(colormaps::get("plasma")?),
    )?;
    fig.colorbar(
        &im,
        ColorbarOptions {
            location: Some(SpineSide::Top),
            shrink: 0.7,
            panchor: ParentAnchor::Unchanged,
            aspect: 10.0,
            label: "on top".into(),
            ..ColorbarOptions::beside(ax2)
        },
    )?;
    Ok(())
}
