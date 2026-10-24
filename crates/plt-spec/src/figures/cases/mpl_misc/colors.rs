//! The cases from `test_colors.py`.

use plt::axes::{ImshowOptions, Mesh, PcolorOptions, TickParams};
use plt::colorbar::{ColorbarAxes, ColorbarOptions, Spacing};
use plt::colors::{
    BlendMode, BoundaryNorm, Color, ColorizerOptions, Colormap, Extend, LightSource, Norm,
    NormError, Normalize, Normalized, Relief, ScalarMappable, ShadeOptions, colormaps,
    from_levels_and_colors,
};
use plt::contour::{ContourOptions, QuadField};
use plt::image::ScalarField;
use plt::numeric::{Grid, linspace};
use plt::orientation::Orientation;
use plt::prelude::*;

use super::{classic, remove_text};
use crate::figures::{CaseResult, FigureCase};

/// `test_cmap_and_norm_from_levels_and_colors`: a mesh coloured by
/// levels, with both extensions, and its colorbar.
fn cmap_and_norm_from_levels_and_colors(fig: &Figure) -> CaseResult {
    let data = Grid::from_vec(7, 7, linspace(-2.0, 4.0, 49))?;
    let colors = ["red", "green", "blue", "yellow", "black"].map(Color::from);
    let (cmap, norm) = from_levels_and_colors(&[-1.0, 2.0, 2.5, 3.0], &colors, Extend::Both)?;
    let ax = fig.add_subplot(1, 1, 1)?;
    let mesh = ax.pcolormesh(
        Mesh::new(data),
        PcolorOptions {
            colorizer: ColorizerOptions::default().cmap(cmap).norm(norm),
            ..PcolorOptions::default()
        },
    )?;
    fig.colorbar(&mesh, ColorbarOptions::default())?;
    ax.tick_params(&TickParams {
        labelleft: Some(false),
        labelbottom: Some(false),
        ..TickParams::default()
    });
    Ok(())
}

/// `test_boundarynorm_and_colorbarbase`: horizontal colorbars of boundary
/// norms, extended by the colorbar, by the norm, and not at all.
fn boundarynorm_and_colorbarbase(fig: &Figure) -> CaseResult {
    let bounds = [-1.0, 2.0, 5.0, 7.0, 12.0, 15.0];
    let cmap = colormaps::get("viridis")?;
    let bars = [
        (0.80, Extend::Neither, Some(Extend::Both), Spacing::Uniform),
        (0.475, Extend::Both, None, Spacing::default()),
        (
            0.15,
            Extend::Both,
            Some(Extend::Neither),
            Spacing::default(),
        ),
    ];
    for (bottom, norm_extend, bar_extend, spacing) in bars {
        let ax = fig.add_axes([0.05, bottom, 0.9, 0.15])?;
        let norm = BoundaryNorm::new(bounds, cmap.n(), norm_extend)?;
        let mappable = ScalarMappable::with_norm(cmap.clone(), Box::new(norm));
        fig.colorbar(
            &mappable,
            ColorbarOptions {
                axes: ColorbarAxes::Cax(ax),
                orientation: Some(Orientation::Horizontal),
                extend: bar_extend,
                spacing,
                ..ColorbarOptions::default()
            },
        )?;
    }
    Ok(())
}

/// The stand-in for the Jacksboro fault elevation model: a ridge with
/// ripples, cut by a fault, on a 120 × 100 grid.
fn elevation() -> Grid<f64> {
    let (ys, xs) = (linspace(0.0, 1.0, 120), linspace(0.0, 1.0, 100));
    Grid::from_fn(120, 100, |row, col| {
        let (x, y) = (xs[col], ys[row]);
        let ridge = 120.0 * (-(x - 0.6).powi(2) / 0.05).exp() * (1.0 + 0.3 * (9.0 * y).sin());
        let ripples = 40.0 * (13.0 * x).sin() * (11.0 * y).cos();
        let fault = 30.0 * ((y - 0.4 - 0.2 * x) / 0.02).tanh();
        350.0 + ridge + ripples + fault
    })
}

/// `test_light_source_topo_surface`: the elevation model shaded with
/// three vertical exaggerations in three blend modes.
fn light_source_topo_surface(fig: &Figure) -> CaseResult {
    let elev = elevation();
    let light = LightSource::new(315.0, 45.0);
    let cmap = colormaps::get("gist_earth")?;
    let axes = fig.subplots(3, 3)?;
    let modes = [BlendMode::Hsv, BlendMode::Overlay, BlendMode::SoftLight];
    for (row, mode) in modes.into_iter().enumerate() {
        for (col, vert_exag) in [0.1, 1.0, 10.0].into_iter().enumerate() {
            let options = ShadeOptions {
                blend: mode.clone(),
                relief: Relief {
                    vert_exag,
                    dx: 25.0,
                    dy: 30.0,
                    ..Relief::default()
                },
                ..ShadeOptions::default()
            }
            .cmap(cmap.clone());
            let rgba = light.shade(&elev, &options)?.map(|c| [c.r, c.g, c.b, c.a]);
            let ax = &axes[(row, col)];
            ax.imshow(rgba, ImshowOptions::default())?;
            ax.set_xticks(Vec::<f64>::new())
                .set_yticks(Vec::<f64>::new());
        }
    }
    Ok(())
}

/// `test_norm_update_figs`: limits of an image's norm changed after a
/// draw.
fn norm_update_figs(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    let z = Grid::from_fn(10, 10, |row, col| (10 * row + col) as i64);
    let image = ax.imshow(
        ScalarField::integers(&z),
        ImshowOptions {
            colorizer: ColorizerOptions::default().norm(Normalize::new(0.0, 1.0)),
            ..ImshowOptions::default()
        },
    )?;
    fig.layout();
    image.set_clim(Some(10.0), Some(90.0));
    Ok(())
}

/// `test_norm_abc`'s `CustomHalfNorm`: halves the data, its limits fixed
/// at 0 and 1.
#[derive(Clone, Debug)]
struct HalfNorm;

impl Norm for HalfNorm {
    fn vmin(&self) -> Option<f64> {
        Some(0.0)
    }

    fn vmax(&self) -> Option<f64> {
        Some(1.0)
    }

    fn set_vmin(&mut self, _vmin: Option<f64>) {}

    fn set_vmax(&mut self, _vmax: Option<f64>) {}

    fn clip(&self) -> bool {
        false
    }

    fn set_clip(&mut self, _clip: bool) {}

    fn scaled(&self) -> bool {
        true
    }

    fn autoscale(&mut self, _data: &[Option<f64>]) {}

    fn autoscale_none(&mut self, _data: &[Option<f64>]) {}

    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        _clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        Ok(data
            .iter()
            .map(|value| value.map_or(Normalized::Masked, |v| Normalized::Fraction(v / 2.0)))
            .collect())
    }

    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        Ok(values.iter().map(|v| Some(2.0 * v)).collect())
    }
}

/// `test_norm_abc`: an image, a mesh and line and filled contours sharing
/// a custom norm.
fn norm_abc(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(2, 2)?;
    let r = Grid::from_vec(16, 16, linspace(-1.0, 3.0, 256))?;
    let viridis = colormaps::get("viridis")?;
    let colorizer = || {
        ColorizerOptions::default()
            .cmap(viridis.clone())
            .norm(HalfNorm)
    };
    axes[(0, 0)].imshow(
        r.clone(),
        ImshowOptions {
            colorizer: colorizer(),
            ..ImshowOptions::default()
        },
    )?;
    axes[(0, 1)].pcolor(
        Mesh::new(r.clone()),
        PcolorOptions {
            colorizer: colorizer(),
            ..PcolorOptions::default()
        },
    )?;
    let contours = |cmap: Colormap| ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(cmap),
            norm: Some(Box::new(HalfNorm)),
            ..ColorizerOptions::default()
        },
        ..ContourOptions::default()
    };
    let field = QuadField::new(r);
    axes[(1, 0)].contour(&field, contours(viridis.clone()))?;
    axes[(1, 1)].contourf(&field, contours(viridis))?;
    remove_text(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "mpl_misc/cmap_and_norm_from_levels_and_colors",
        cmap_and_norm_from_levels_and_colors,
    ),
    classic(
        "mpl_misc/boundarynorm_and_colorbarbase",
        boundarynorm_and_colorbarbase,
    ),
    classic(
        "mpl_misc/light_source_topo_surface",
        light_source_topo_surface,
    ),
    classic("mpl_misc/norm_update_figs", norm_update_figs),
    classic("mpl_misc/norm_abc", norm_abc),
];
