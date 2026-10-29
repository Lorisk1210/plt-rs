//! Colormap normalisations: logarithmic, centred, symmetric logarithmic,
//! power-law, discrete, two-slope, function and custom norms, and the
//! colorbar of a two-slope norm on a linear scale (`colormapnorms`).

use std::error::Error;

use pltrs::axes::{Mesh, PcolorOptions, Shading};
use pltrs::collections::Collection;
use pltrs::colorbar::{Colorbar, ColorbarOptions};
use pltrs::colors::{
    BoundaryNorm, CenteredNorm, ColorizerOptions, Colormap, Extend, FuncNorm, LogNorm, Norm,
    NormError, Normalized, PowerNorm, SymLogNorm, TwoSlopeNorm, colormaps,
};
use pltrs::layout::LayoutEngine;
use pltrs::numeric::{Grid, interp};
use pltrs::orientation::Orientation;
use pltrs::prelude::*;
use pltrs::rc::Aspect;

use crate::figures::cases::common::autoscale_unset;
use crate::figures::cases::common::{
    GALLERY_SEED, extremes, mesh_field, mgrid_100, on_mesh, spike,
};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// What drawing a mesh gives back.
type Drawn = Result<Collection, Box<dyn Error>>;

/// `pcolor`/`pcolormesh` options: a named colormap with `colors`' norm or
/// limits, and a shading.
fn shaded(
    cmap: &str,
    colorizer: ColorizerOptions,
    shading: Shading,
) -> Result<PcolorOptions, Box<dyn Error>> {
    Ok(PcolorOptions {
        colorizer: colorizer.cmap(colormaps::get(cmap)?),
        shading: Some(shading),
        ..PcolorOptions::default()
    })
}

/// One row per colour mapping, each drawn by `draw` with a colorbar
/// extended as `extend` says (`_rows`); returns the axes.
fn rows(
    fig: &Figure,
    draw: impl Fn(&Axes, ColorizerOptions) -> Drawn,
    mappings: Vec<ColorizerOptions>,
    extend: Extend,
) -> Result<Vec<Axes>, Box<dyn Error>> {
    let axs = fig.subplots(mappings.len(), 1)?.as_slice().to_vec();
    for (ax, mapping) in axs.iter().zip(mappings) {
        let mesh = draw(ax, mapping)?;
        fig.colorbar(
            &mesh,
            ColorbarOptions {
                extend: Some(extend),
                ..ColorbarOptions::beside(ax)
            },
        )?;
    }
    Ok(axs)
}

fn colormapnorms_1(fig: &Figure) -> CaseResult {
    let (x, y) = mgrid_100(-3.0, 3.0, -2.0, 2.0);
    let z = on_mesh(&x, &y, spike);
    let (lo, hi) = extremes(z.as_slice());
    let draw = |ax: &Axes, colors| -> Drawn {
        let mesh = Mesh::from_coords(x.clone(), y.clone(), z.clone());
        Ok(ax.pcolor(mesh, shaded("PuBu_r", colors, Shading::Auto)?)?)
    };
    let log = ColorizerOptions::default().norm(LogNorm::new(lo, hi));
    rows(
        fig,
        draw,
        vec![log, ColorizerOptions::default()],
        Extend::Max,
    )?;
    Ok(())
}

fn colormapnorms_2(fig: &Figure) -> CaseResult {
    let delta = 0.1;
    let x = arange(-3.0, 4.001, delta);
    let y = arange(-4.0, 3.001, delta);
    let (_, _, z) = mesh_field(&x, &y, |x, y| {
        let z1 = (-(x * x) - y * y).exp();
        let z2 = (-((x - 1.0) * (x - 1.0)) - (y - 1.0) * (y - 1.0)).exp();
        (0.9 * z1 - 0.5 * z2) * 2.0
    });
    let axs = fig.subplots(1, 2)?;
    let centered = ColorizerOptions::default().norm(CenteredNorm::new(0.0, None));
    for (ax, colors, title) in [
        (&axs[(0, 0)], ColorizerOptions::default(), "Normalize()"),
        (&axs[(0, 1)], centered, "CenteredNorm()"),
    ] {
        let pc = ax.pcolormesh(
            Mesh::new(z.clone()),
            PcolorOptions {
                colorizer: colors.cmap(colormaps::get("coolwarm")?),
                ..PcolorOptions::default()
            },
        )?;
        fig.colorbar(&pc, ColorbarOptions::beside(ax))?;
        ax.set_title(title);
    }
    Ok(())
}

fn colormapnorms_3(fig: &Figure) -> CaseResult {
    let (x, y) = mgrid_100(-3.0, 3.0, -2.0, 2.0);
    let z = on_mesh(&x, &y, |x, y| {
        let z1 = (-(x * x) - y * y).exp();
        let z2 = (-((x - 1.0) * (x - 1.0)) - (y - 1.0) * (y - 1.0)).exp();
        (z1 - z2) * 2.0
    });
    let top = extremes(z.as_slice()).1;
    let draw = |ax: &Axes, colors| -> Drawn {
        let mesh = Mesh::from_coords(x.clone(), y.clone(), z.clone());
        Ok(ax.pcolormesh(mesh, shaded("RdBu_r", colors, Shading::Auto)?)?)
    };
    let symlog = SymLogNorm::new(0.03, 0.03, 10.0)?
        .with_vmin(-1.0)
        .with_vmax(1.0);
    let mappings = vec![
        ColorizerOptions::default().norm(symlog),
        ColorizerOptions::default().limits(Some(-top), None),
    ];
    rows(fig, draw, mappings, Extend::Both)?;
    Ok(())
}

/// The power-law sections' field, with its grid: stripes growing with
/// `x` (`_power_field`).
fn power_field() -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let (x, y) = mgrid_100(0.0, 3.0, 0.0, 2.0);
    let z = on_mesh(&x, &y, |x, y| (1.0 + (y * 10.0).sin()) * x.powi(2));
    (x, y, z)
}

fn colormapnorms_4(fig: &Figure) -> CaseResult {
    let (x, y, z) = power_field();
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let draw = |ax: &Axes, colors| -> Drawn {
        let mesh = Mesh::from_coords(x.clone(), y.clone(), z.clone());
        Ok(ax.pcolormesh(mesh, shaded("PuBu_r", colors, Shading::Auto)?)?)
    };
    let power = ColorizerOptions::default().norm(PowerNorm::new(0.5));
    let axs = rows(
        fig,
        draw,
        vec![power, ColorizerOptions::default()],
        Extend::Max,
    )?;
    axs[0].set_title("PowerNorm()");
    axs[1].set_title("Normalize()");
    Ok(())
}

fn colormapnorms_5(fig: &Figure) -> CaseResult {
    let (x, y, z) = mesh_field(
        &linspace(-3.0, 3.0, 100),
        &linspace(-2.0, 2.0, 100),
        |x, y| {
            let z1 = (-(x * x) - y * y).exp();
            let z2 = (-((x - 1.0) * (x - 1.0)) - (y - 1.0) * (y - 1.0)).exp();
            (z1 - z2) * 2.0
        },
    );
    let z = Grid::from_fn(99, 99, |r, c| z[(r, c)]);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 2)?;
    let ax: Vec<&Axes> = axs.as_slice().iter().collect();
    let mesh = |ax: &Axes, colorizer: ColorizerOptions| -> Drawn {
        let mesh = Mesh::from_coords(x.clone(), y.clone(), z.clone());
        Ok(ax.pcolormesh(mesh, shaded("RdBu_r", colorizer, Shading::Auto)?)?)
    };
    let vertical = |ax: &Axes, extend| ColorbarOptions {
        extend,
        orientation: Some(Orientation::Vertical),
        ..ColorbarOptions::beside(ax)
    };
    let pcm = mesh(ax[0], ColorizerOptions::default())?;
    fig.colorbar(&pcm, vertical(ax[0], None))?;
    ax[0].set_title("Default norm");
    let even = linspace(-1.5, 1.5, 7);
    for (axis, norm, extend, title) in [
        (
            ax[1],
            BoundaryNorm::new(even.clone(), 256, Extend::Neither)?,
            Some(Extend::Both),
            "BoundaryNorm: 7 boundaries",
        ),
        (
            ax[2],
            BoundaryNorm::new([-0.2, -0.1, 0.0, 0.5, 1.0], 256, Extend::Neither)?,
            Some(Extend::Both),
            "BoundaryNorm: nonuniform",
        ),
        (
            ax[3],
            BoundaryNorm::new(even, 256, Extend::Both)?,
            None,
            "BoundaryNorm: extend=\"both\"",
        ),
    ] {
        let pcm = mesh(axis, ColorizerOptions::default().norm(norm))?;
        fig.colorbar(&pcm, vertical(axis, extend))?;
        axis.set_title(title);
    }
    Ok(())
}

/// A synthetic stand-in for `topobathy.npz` (`_topobathy`): 120
/// longitudes, 91 latitudes and the elevations on their grid, sea floor
/// sloping down west of a wavy coast, hills east of it.
fn topobathy() -> (Vec<f64>, Vec<f64>, Grid<f64>) {
    let longitude: Vec<f64> = (0..120)
        .map(|i| 234.0 + (f64::from(i) + 0.5) / 30.0)
        .collect();
    let latitude = linspace(48.0164, 49.9842, 91);
    let noise = SpecRng::new(GALLERY_SEED).normal(91 * 120, 0.0, 40.0);
    let topo = Grid::from_fn(91, 120, |r, c| {
        let (lon, lat) = (longitude[c], latitude[r]);
        let shore = lon - (235.7 + 0.3 * (4.0 * lat).sin());
        let ridges = 250.0 * (5.0 * lon).sin() * (7.0 * lat).cos();
        let slope = if shore < 0.0 { 400.0 } else { 900.0 };
        slope * shore + ridges + noise[r * 120 + c]
    });
    (longitude, latitude, topo)
}

/// The tutorial's land and sea colormap: 256 colours of the blue start of
/// `terrain` for the sea, 256 of its rest for the land (`_terrain_map`).
fn terrain_map() -> Result<Colormap, Box<dyn Error>> {
    let terrain = colormaps::get("terrain")?;
    let colors: Vec<_> = linspace(0.0, 0.17, 256)
        .into_iter()
        .chain(linspace(0.25, 1.0, 256))
        .map(|x| terrain.sample(x))
        .collect();
    Ok(Colormap::from_list("terrain_map", colors, 256, 1.0)?)
}

/// The tutorial's map of the coast under `norm`, its aspect set for
/// latitude 49° (`_topo_map`); returns the mesh.
fn topo_map(ax: &Axes, norm: impl Norm + 'static, title: &str) -> Drawn {
    let (longitude, latitude, topo) = topobathy();
    let pcm = ax.pcolormesh(
        Mesh::from_coords(longitude, latitude, topo),
        PcolorOptions {
            colorizer: ColorizerOptions::default().norm(norm).cmap(terrain_map()?),
            shading: Some(Shading::Auto),
            ..PcolorOptions::default()
        },
    )?;
    pcm.set_rasterized(true);
    ax.set_aspect(Aspect::Ratio(1.0 / 49.0_f64.to_radians().cos()))?;
    ax.set_title(title);
    Ok(pcm)
}

/// The ticks of the tutorial's elevation colorbars.
const TOPO_TICKS: [f64; 6] = [-500.0, 0.0, 1000.0, 2000.0, 3000.0, 4000.0];

/// The tutorial's two-slope norm: sea from -500 m and land up to 4000 m on
/// the two halves of the colormap.
fn divnorm() -> Result<TwoSlopeNorm, NormError> {
    TwoSlopeNorm::new(0.0, Some(-500.0), Some(4000.0))
}

/// A colorbar shrunk to 0.6 stealing space from the axes of `pcm`.
fn shrunk(
    fig: &Figure,
    pcm: &Collection,
    options: ColorbarOptions,
) -> Result<Colorbar, Box<dyn Error>> {
    Ok(fig.colorbar(
        pcm,
        ColorbarOptions {
            shrink: 0.6,
            ..options
        },
    )?)
}

fn colormapnorms_6(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let pcm = topo_map(&ax, divnorm()?, "TwoSlopeNorm(x)")?;
    let cb = shrunk(fig, &pcm, ColorbarOptions::default())?;
    cb.set_ticks(TOPO_TICKS.to_vec());
    Ok(())
}

fn colormapnorms_7(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let norm = divnorm()?;
    let mut last = None;
    for (ax, title) in axs
        .as_slice()
        .iter()
        .zip(["Default: Scaled colorbar", "Linear colorbar spacing"])
    {
        let pcm = topo_map(ax, norm, title)?;
        let cb = shrunk(fig, &pcm, ColorbarOptions::beside(ax))?;
        cb.set_ticks(arange(-500.0, 4001.0, 500.0));
        last = Some(cb);
    }
    last.ok_or("two colorbars")?.ax().set_yscale("linear")?;
    Ok(())
}

fn colormapnorms_8(fig: &Figure) -> CaseResult {
    let (x, y, z) = power_field();
    let ax = fig.subplot();
    let norm = FuncNorm::new(f64::sqrt, |x| x.powi(2))
        .with_vmin(0.0)
        .with_vmax(20.0);
    let pcm = ax.pcolormesh(
        Mesh::from_coords(x, y, z),
        shaded(
            "PuBu_r",
            ColorizerOptions::default().norm(norm),
            Shading::Auto,
        )?,
    )?;
    ax.set_title("FuncNorm(x)");
    shrunk(fig, &pcm, ColorbarOptions::default())?;
    Ok(())
}

/// The tutorial's `MidpointNormalize`: linear from `vmin` to `vcenter`
/// (mapped to 0.5) and from there to `vmax`, extrapolated to infinities,
/// with the matching inverse.
#[derive(Clone, Debug)]
struct MidpointNormalize {
    vmin: Option<f64>,
    vmax: Option<f64>,
    vcenter: f64,
    clip: bool,
}

impl MidpointNormalize {
    /// The three anchors `[vmin, vcenter, vmax]`.
    fn anchors(&self) -> Result<[f64; 3], NormError> {
        let (vmin, vmax) = self.vmin.zip(self.vmax).ok_or(NormError::Unscaled)?;
        Ok([vmin, self.vcenter, vmax])
    }
}

/// `np.interp(x, xp, fp, left=-np.inf, right=np.inf)`.
fn extrapolated(x: f64, xp: &[f64], fp: &[f64]) -> f64 {
    interp(x, xp, fp, Some((f64::NEG_INFINITY, f64::INFINITY)))
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
        autoscale_unset(&mut self.vmin, &mut self.vmax, data);
    }

    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        _clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        let xp = self.anchors()?;
        Ok(data
            .iter()
            .map(|v| {
                let v = v.unwrap_or(f64::NAN);
                Normalized::Fraction(extrapolated(v, &xp, &[0.0, 0.5, 1.0]))
            })
            .collect())
    }

    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        let fp = self.anchors()?;
        Ok(values
            .iter()
            .map(|&v| Some(extrapolated(v, &[0.0, 0.5, 1.0], &fp)))
            .collect())
    }
}

fn colormapnorms_9(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let midnorm = MidpointNormalize {
        vmin: Some(-500.0),
        vmax: Some(4000.0),
        vcenter: 0.0,
        clip: false,
    };
    let pcm = topo_map(&ax, midnorm, "Custom norm")?;
    let cb = shrunk(
        fig,
        &pcm,
        ColorbarOptions {
            extend: Some(Extend::Both),
            ..ColorbarOptions::default()
        },
    )?;
    cb.set_ticks(TOPO_TICKS.to_vec());
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_guides_colors_text/colormapnorms_1",
        colormapnorms_1,
    ),
    FigureCase::new(
        "gallery_guides_colors_text/colormapnorms_2",
        colormapnorms_2,
    ),
    FigureCase::new(
        "gallery_guides_colors_text/colormapnorms_3",
        colormapnorms_3,
    ),
    FigureCase::new(
        "gallery_guides_colors_text/colormapnorms_4",
        colormapnorms_4,
    ),
    FigureCase::new(
        "gallery_guides_colors_text/colormapnorms_5",
        colormapnorms_5,
    )
    .sized(8.0, 6.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormapnorms_6",
        colormapnorms_6,
    ),
    FigureCase::new(
        "gallery_guides_colors_text/colormapnorms_7",
        colormapnorms_7,
    )
    .sized(10.0, 4.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormapnorms_8",
        colormapnorms_8,
    ),
    FigureCase::new(
        "gallery_guides_colors_text/colormapnorms_9",
        colormapnorms_9,
    ),
];
