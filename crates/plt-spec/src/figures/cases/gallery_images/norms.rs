//! Colormap normalisations: `colormap_normalizations` and
//! `colormap_normalizations_symlognorm`.

use plt::axes::{Axes, Mesh, PcolorOptions, Shading};
use plt::colorbar::ColorbarOptions;
use plt::colors::{
    AsinhNorm, BoundaryNorm, Colorize, ColorizerOptions, Extend, LogNorm, Norm, NormError,
    Normalized, PowerNorm, SymLogNorm, colormaps,
};
use plt::figure::{Figure, Share};
use plt::layout::LayoutEngine;
use plt::numeric::{Grid, interp, linspace};
use plt::orientation::Orientation;

use super::data::{extremes, mgrid, on_mesh, spike};
use crate::figures::cases::common::{mgrid_100, mgrid_count};
use crate::figures::{CaseResult, FigureCase};

/// A mesh of the values `z` at the points `x`, `y`.
fn points_mesh(x: &Grid<f64>, y: &Grid<f64>, z: &Grid<f64>) -> Mesh {
    Mesh::from_coords(x.clone(), y.clone(), z.clone())
}

/// `pcolor`/`pcolormesh` options: a colormap, a norm or limits, a shading.
fn shaded(
    cmap: &str,
    colorizer: ColorizerOptions,
    shading: Shading,
) -> Result<PcolorOptions, plt::colors::ColormapError> {
    Ok(PcolorOptions {
        colorizer: colorizer.cmap(colormaps::get(cmap)?),
        shading: Some(shading),
        ..PcolorOptions::default()
    })
}

/// A colorbar beside `ax`, extended as `extend` says and labelled.
fn labelled_colorbar(
    fig: &Figure,
    mesh: &plt::collections::Collection,
    ax: &Axes,
    extend: Extend,
    label: &str,
) -> CaseResult {
    fig.colorbar(
        mesh,
        ColorbarOptions {
            extend: Some(extend),
            label: label.into(),
            ..ColorbarOptions::beside(ax)
        },
    )?;
    Ok(())
}

/// How one row of `colormap_normalizations` draws its mesh.
type Plot<'a> = &'a dyn Fn(
    &Axes,
    ColorizerOptions,
) -> Result<plt::collections::Collection, Box<dyn std::error::Error>>;

/// `colormap_normalizations`' two rows: linear scaling (with `linear`
/// colours), then `norm`, each with a colorbar.
fn norm_pair(
    fig: &Figure,
    plot: Plot<'_>,
    linear: ColorizerOptions,
    norm: ColorizerOptions,
    extend: Extend,
    label: &str,
) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    let mesh = plot(&axs[(0, 0)], linear)?;
    labelled_colorbar(fig, &mesh, &axs[(0, 0)], extend, "linear scaling")?;
    let mesh = plot(&axs[(1, 0)], norm)?;
    labelled_colorbar(fig, &mesh, &axs[(1, 0)], extend, label)?;
    Ok(())
}

fn colormap_normalizations_1(fig: &Figure) -> CaseResult {
    let (x, y) = mgrid_100(-3.0, 3.0, -2.0, 2.0);
    let z = on_mesh(&x, &y, spike);
    let (lo, hi) = extremes(&z);
    let plot = |ax: &Axes, colors| -> Result<_, Box<dyn std::error::Error>> {
        Ok(ax.pcolor(
            points_mesh(&x, &y, &z),
            shaded("PuBu_r", colors, Shading::Nearest)?,
        )?)
    };
    norm_pair(
        fig,
        &plot,
        ColorizerOptions::default(),
        ColorizerOptions::default().norm(LogNorm::new(lo, hi)),
        Extend::Max,
        "LogNorm",
    )
}

fn colormap_normalizations_2(fig: &Figure) -> CaseResult {
    let (x, y) = mgrid_100(0.0, 3.0, 0.0, 2.0);
    let z = on_mesh(&x, &y, |x, y| (1.0 + (y * 10.0).sin()) * (x * x));
    let plot = |ax: &Axes, colors| -> Result<_, Box<dyn std::error::Error>> {
        Ok(ax.pcolormesh(
            points_mesh(&x, &y, &z),
            shaded("PuBu_r", colors, Shading::Nearest)?,
        )?)
    };
    norm_pair(
        fig,
        &plot,
        ColorizerOptions::default(),
        ColorizerOptions::default().norm(PowerNorm::new(0.5)),
        Extend::Max,
        "PowerNorm",
    )
}

/// `colormap_normalizations`' field of a large positive and a small
/// negative bump, on its grid.
fn bipolar() -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let (x, y) = mgrid_100(-3.0, 3.0, -2.0, 2.0);
    let z = on_mesh(&x, &y, |x, y| {
        let z1 = (-(x * x) - y * y).exp();
        let z2 = (-((x - 1.0) * (x - 1.0)) - (y - 1.0) * (y - 1.0)).exp();
        (5.0 * z1 - z2) * 2.0
    });
    (x, y, z)
}

/// `pcolormesh(X, Y, Z, cmap='RdBu_r', shading='nearest', ...)` of the
/// bipolar field; its linear scaling starts at `-max(Z)`.
fn bipolar_pair(fig: &Figure, norm: ColorizerOptions, label: &str) -> CaseResult {
    let (x, y, z) = bipolar();
    let top = extremes(&z).1;
    let plot = |ax: &Axes, colors| -> Result<_, Box<dyn std::error::Error>> {
        Ok(ax.pcolormesh(
            points_mesh(&x, &y, &z),
            shaded("RdBu_r", colors, Shading::Nearest)?,
        )?)
    };
    norm_pair(
        fig,
        &plot,
        ColorizerOptions::default().limits(Some(-top), None),
        norm,
        Extend::Both,
        label,
    )
}

fn colormap_normalizations_3(fig: &Figure) -> CaseResult {
    let symlog = SymLogNorm::new(0.015, 1.0, 10.0)?
        .with_vmin(-10.0)
        .with_vmax(10.0);
    bipolar_pair(fig, ColorizerOptions::default().norm(symlog), "SymLogNorm")
}

/// `colormap_normalizations`' `MidpointNormalize`: linear on each side of
/// `midpoint`, which maps to 0.5; its inverse is `Normalize`'s.
#[derive(Clone, Debug)]
struct MidpointNormalize {
    vmin: Option<f64>,
    vmax: Option<f64>,
    midpoint: f64,
    clip: bool,
}

impl Norm for MidpointNormalize {
    fn vmin(&self) -> Option<f64> {
        self.vmin
    }

    fn vmax(&self) -> Option<f64> {
        self.vmax
    }

    fn set_vmin(&mut self, vmin: Option<f64>) {
        self.vmin = vmin;
    }

    fn set_vmax(&mut self, vmax: Option<f64>) {
        self.vmax = vmax;
    }

    fn clip(&self) -> bool {
        self.clip
    }

    fn set_clip(&mut self, clip: bool) {
        self.clip = clip;
    }

    fn autoscale_none(&mut self, data: &[Option<f64>]) {
        let values = data.iter().flatten().copied();
        if let Some((lo, hi)) = values.fold(None, |range: Option<(f64, f64)>, v| {
            Some(range.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))))
        }) {
            self.vmin.get_or_insert(lo);
            self.vmax.get_or_insert(hi);
        }
    }

    /// `np.interp(value, [vmin, midpoint, vmax], [0, 0.5, 1])`, masks
    /// dropped.
    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        _clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        let (lo, hi) = self.vmin.zip(self.vmax).ok_or(NormError::Unscaled)?;
        let xp = [lo, self.midpoint, hi];
        Ok(data
            .iter()
            .map(|v| {
                let v = v.unwrap_or(f64::NAN);
                Normalized::Fraction(interp(v, &xp, &[0.0, 0.5, 1.0], None))
            })
            .collect())
    }

    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        let (lo, hi) = self.vmin.zip(self.vmax).ok_or(NormError::Unscaled)?;
        Ok(values.iter().map(|&v| Some(lo + v * (hi - lo))).collect())
    }
}

fn colormap_normalizations_4(fig: &Figure) -> CaseResult {
    let midpoint = MidpointNormalize {
        vmin: None,
        vmax: None,
        midpoint: 0.0,
        clip: false,
    };
    bipolar_pair(
        fig,
        ColorizerOptions::default().norm(midpoint),
        "Custom norm",
    )
}

fn colormap_normalizations_5(fig: &Figure) -> CaseResult {
    let (x, y, z) = bipolar();
    let top = extremes(&z).1;
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let axs = fig.subplots(3, 1)?;
    let vertical = |ax: &Axes, label: &str| ColorbarOptions {
        extend: Some(Extend::Both),
        orientation: Some(Orientation::Vertical),
        label: label.into(),
        ..ColorbarOptions::beside(ax)
    };
    let linear = ColorizerOptions::default().limits(Some(-top), None);
    let mesh = axs[(0, 0)].pcolormesh(
        points_mesh(&x, &y, &z),
        shaded("RdBu_r", linear, Shading::Nearest)?,
    )?;
    fig.colorbar(&mesh, vertical(&axs[(0, 0)], "linear scaling"))?;
    for (ax, bounds, label) in [
        (
            &axs[(1, 0)],
            linspace(-2.0, 2.0, 11),
            "BoundaryNorm\nlinspace(-2, 2, 11)",
        ),
        (
            &axs[(2, 0)],
            vec![-1.0, -0.5, 0.0, 2.5, 5.0],
            "BoundaryNorm\n[-1, -0.5, 0, 2.5, 5]",
        ),
    ] {
        let norm = BoundaryNorm::new(bounds, 256, Extend::Neither)?;
        let colors = ColorizerOptions::default().norm(norm);
        let mesh = ax.pcolormesh(
            points_mesh(&x, &y, &z),
            shaded("RdBu_r", colors, Shading::Nearest)?,
        )?;
        fig.colorbar(&mesh, vertical(ax, label))?;
    }
    Ok(())
}

/// `colormap_normalizations_symlognorm`'s field: a large and a small
/// radial basis function of opposite sign, on 40 × 40 points (`_rbf_field`;
/// the example's 200 × 200 make SVGs of half a gigabyte).
fn rbf_field() -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let rbf = |x: f64, y: f64| 1.0 / (1.0 + 5.0 * ((x * x) + (y * y)));
    let (x, y) = mgrid(&mgrid_count(-3.0, 3.0, 40), &mgrid_count(-2.0, 2.0, 40));
    let z = on_mesh(&x, &y, |x, y| {
        8.0 * rbf(x + 0.5, y + 0.5) - rbf(x - 0.5, y - 0.5)
    });
    (x, y, z)
}

/// The symlog panel above a panel coloured as `colors` says, labelled
/// `label`, both with colorbars.
fn symlog_against(fig: &Figure, colorizer: ColorizerOptions, label: &str) -> CaseResult {
    let (x, y, z) = rbf_field();
    let axs = fig.subplots_shared(2, 1, Share::All, Share::All)?;
    let symlog = SymLogNorm::new(0.5, 1.0, 10.0)?
        .with_vmin(-8.0)
        .with_vmax(8.0);
    for (ax, colors, text) in [
        (
            &axs[(0, 0)],
            ColorizerOptions::default().norm(symlog),
            "symlog",
        ),
        (&axs[(1, 0)], colorizer, label),
    ] {
        let mesh = ax.pcolormesh(
            points_mesh(&x, &y, &z),
            shaded("PRGn", colors, Shading::Gouraud)?,
        )?;
        fig.colorbar(
            &mesh,
            ColorbarOptions {
                extend: Some(Extend::Both),
                ..ColorbarOptions::beside(ax)
            },
        )?;
        ax.text(-2.5, 1.5, text);
    }
    Ok(())
}

fn colormap_normalizations_symlognorm_1(fig: &Figure) -> CaseResult {
    symlog_against(
        fig,
        ColorizerOptions::default().limits(Some(-8.0), Some(8.0)),
        "linear",
    )
}

fn colormap_normalizations_symlognorm_2(fig: &Figure) -> CaseResult {
    let asinh = AsinhNorm::new(0.5)?.with_vmin(-8.0).with_vmax(8.0);
    symlog_against(fig, ColorizerOptions::default().norm(asinh), "asinh")
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_images/colormap_normalizations_1",
        colormap_normalizations_1,
    ),
    FigureCase::new(
        "gallery_images/colormap_normalizations_2",
        colormap_normalizations_2,
    ),
    FigureCase::new(
        "gallery_images/colormap_normalizations_3",
        colormap_normalizations_3,
    ),
    FigureCase::new(
        "gallery_images/colormap_normalizations_4",
        colormap_normalizations_4,
    ),
    FigureCase::new(
        "gallery_images/colormap_normalizations_5",
        colormap_normalizations_5,
    ),
    FigureCase::new(
        "gallery_images/colormap_normalizations_symlognorm_1",
        colormap_normalizations_symlognorm_1,
    ),
    FigureCase::new(
        "gallery_images/colormap_normalizations_symlognorm_2",
        colormap_normalizations_symlognorm_2,
    ),
];
