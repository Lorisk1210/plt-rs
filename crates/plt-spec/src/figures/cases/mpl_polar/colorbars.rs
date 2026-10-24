//! `test_colorbar.py` apart from the positioning test: contour and
//! scatter colorbars, closed outlines under wide lines, labels on both
//! axes, changed limits and scales, two-slope, proportional, edged,
//! hatched, `NoNorm`, bounded and translucent colorbars, and the
//! `location` of a given colorbar axes.

use plt::axes::{ImshowOptions, Mesh, PcolorOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions, ColorbarTicks, ExtendFrac, Spacing};
use plt::colors::{
    BoundaryNorm, ColorizerOptions, Colormap, Extend, NoNorm, ScalarMappable, TwoSlopeNorm,
    colormaps, from_levels_and_colors,
};
use plt::contour::{ContourOptions, Levels, QuadCoords, QuadField};
use plt::figure::SubfigureOptions;
use plt::gridspec::SubplotOverrides;
use plt::image::{ImageAlpha, Interpolation, ScalarField};
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::spines::SpineSide;

use super::{RAMP_LEVELS, ramp, ramp_levels, remove_ticks_and_titles};
use crate::figures::CaseResult;
use crate::rng::SpecRng;

/// Colorbar options drawing into `cax`.
fn inside(cax: &Axes) -> ColorbarOptions {
    ColorbarOptions {
        axes: ColorbarAxes::Cax(cax.clone()),
        ..ColorbarOptions::default()
    }
}

/// `[[0, 1], [2, 3]]` as an integer image.
fn two_by_two() -> ScalarField {
    ScalarField::integers(&Grid::from_fn(2, 2, |r, c| (2 * r + c) as i64))
}

/// A pcolormesh of `values` coloured by `colors`.
fn mesh(
    ax: &Axes,
    values: Grid<f64>,
    colorizer: ColorizerOptions,
) -> Result<plt::collections::Collection, plt::image::ImageError> {
    ax.pcolormesh(
        Mesh::new(values),
        PcolorOptions {
            colorizer,
            ..PcolorOptions::default()
        },
    )
}

/// `np.arange(100).reshape(10, 10) + offset`.
fn hundred(offset: f64) -> Grid<f64> {
    Grid::from_fn(10, 10, |r, c| (r * 10 + c) as f64 + offset)
}

pub(super) fn contour_colorbar(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let data = QuadField::new(ramp().z.map(|v| v - 500.0));
    let cs = ax.contour(
        &data,
        ContourOptions {
            levels: Levels::Values(RAMP_LEVELS.map(|v| v - 500.0).to_vec()),
            extend: Extend::Both,
            ..ContourOptions::default()
        },
    )?;
    fig.colorbar(
        &cs,
        ColorbarOptions {
            orientation: Some(Orientation::Horizontal),
            extend: Some(Extend::Both),
            ..ColorbarOptions::default()
        },
    )?;
    fig.colorbar(
        &cs,
        ColorbarOptions {
            orientation: Some(Orientation::Vertical),
            ..ColorbarOptions::default()
        },
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn gridspec_make_colorbar(fig: &Figure) -> CaseResult {
    for (index, orientation) in [(1, Orientation::Vertical), (2, Orientation::Horizontal)] {
        let cs = fig
            .add_subplot(1, 2, index)?
            .contourf(&ramp(), ramp_levels())?;
        fig.colorbar(
            &cs,
            ColorbarOptions {
                use_gridspec: true,
                orientation: Some(orientation),
                ..ColorbarOptions::default()
            },
        )?;
    }
    fig.subplots_adjust(&SubplotOverrides {
        top: Some(0.95),
        right: Some(0.95),
        bottom: Some(0.2),
        hspace: Some(0.25),
        ..SubplotOverrides::default()
    })?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn colorbar_single_scatter(fig: &Figure) -> CaseResult {
    let cmap = colormaps::get("jet")?.resampled(16)?;
    let options = ScatterOptions {
        sizes: Some(vec![50.0]),
        colors: MappedColors::new(vec![50.0]).cmap(cmap).into(),
        ..ScatterOptions::default()
    };
    let cs = fig.subplot().scatter_with([0.0], [0.0], options)?;
    fig.colorbar(&cs, ColorbarOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn colorbar_closed_patch(fig: &Figure) -> CaseResult {
    let ax1 = fig.add_axes([0.05, 0.85, 0.9, 0.1])?;
    let caxes = [
        fig.add_axes([0.1, 0.65, 0.75, 0.1])?,
        fig.add_axes([0.05, 0.45, 0.9, 0.1])?,
        fig.add_axes([0.05, 0.25, 0.9, 0.1])?,
        fig.add_axes([0.05, 0.05, 0.9, 0.1])?,
    ];
    let cmap = colormaps::get("RdBu")?.resampled(5)?;
    let values = linspace(0.0, 10.0, 16);
    let im = mesh(
        &ax1,
        Grid::from_vec(4, 4, values)?,
        ColorizerOptions::default().cmap(cmap),
    )?;
    let variants = [
        (Extend::Both, ExtendFrac::Both(0.5), false),
        (Extend::Both, ExtendFrac::Default, false),
        (Extend::Both, ExtendFrac::Default, true),
        (Extend::Neither, ExtendFrac::Default, false),
    ];
    // The wide outline shows that the closed path is handled correctly.
    plt::rc::context(|| -> CaseResult {
        plt::rc::set("axes.linewidth", "16")?;
        for (cax, (extend, extendfrac, extendrect)) in caxes.iter().zip(variants) {
            fig.colorbar(
                &im,
                ColorbarOptions {
                    orientation: Some(Orientation::Horizontal),
                    values: Some(linspace(0.0, 10.0, 5)),
                    ticks: Some(ColorbarTicks::Values(Vec::new())),
                    extend: Some(extend),
                    extendfrac,
                    extendrect,
                    ..inside(cax)
                },
            )?;
        }
        Ok(())
    })?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn keeping_xlabel(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let arr = ScalarField::integers(&Grid::from_fn(5, 5, |r, c| (r * 5 + c) as i64));
    let im = ax.imshow(arr, ImshowOptions::default())?;
    let cbar = fig.colorbar(&im, ColorbarOptions::default())?;
    cbar.ax().set_xlabel("Visible Xlabel");
    cbar.set_label("YLabel", None);
    Ok(())
}

pub(super) fn colorbar_change_lim_scale(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplots(1, 2)?;
    let extended = |ax: &Axes| ColorbarOptions {
        extend: Some(Extend::Both),
        ..ColorbarOptions::beside(ax)
    };
    let pc = mesh(&ax[(0, 0)], hundred(1.0), ColorizerOptions::default())?;
    let cb = fig.colorbar(&pc, extended(&ax[(0, 0)]))?;
    cb.ax().set_yscale("log")?;
    let pc = mesh(&ax[(0, 1)], hundred(1.0), ColorizerOptions::default())?;
    let cb = fig.colorbar(&pc, extended(&ax[(0, 1)]))?;
    cb.ax().set_ylim(20.0, 90.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn axes_handles_same_functions(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let pc = mesh(&ax, Grid::filled(10, 30, 1.0), ColorizerOptions::default())?;
    let cax = fig.add_axes([0.9, 0.1, 0.03, 0.8])?;
    let cb = fig.colorbar(&pc, inside(&cax))?;
    cb.ax().set_yticks(arange(0.0, 20.0, 1.0));
    cb.ax().set_yscale("log")?;
    cb.ax().set_position([0.92, 0.1, 0.02, 0.7]);
    Ok(())
}

pub(super) fn twoslope_colorbar(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let norm = TwoSlopeNorm::new(20.0, Some(5.0), Some(95.0))?;
    let x = arange(1.0, 11.0, 1.0);
    let pc = ax.pcolormesh(
        Mesh::from_coords(x.clone(), x, hundred(0.0)),
        PcolorOptions {
            colorizer: ColorizerOptions {
                norm: Some(norm.into()),
                ..ColorizerOptions::default().cmap(colormaps::get("RdBu_r")?)
            },
            ..PcolorOptions::default()
        },
    )?;
    fig.colorbar(&pc, ColorbarOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn proportional_colorbars(fig: &Figure) -> CaseResult {
    let x = arange(-3.0, 3.01, 0.025);
    let (xx, yy) = meshgrid(&x, &x);
    let z = Grid::from_fn(x.len(), x.len(), |r, c| {
        let (x, y) = (xx[(r, c)], yy[(r, c)]);
        ((-x * x - y * y).exp() - (-(x - 1.0).powi(2) - (y - 1.0).powi(2)).exp()) * 2.0
    });
    let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x: xx, y: yy });
    let levels = vec![-1.25, -0.5, -0.125, 0.125, 0.5, 1.25];
    let cmap = Colormap::listed(
        "from_list",
        vec!["0.3", "0.5", "white", "lightblue", "steelblue"],
    )?
    .with_under("darkred")?
    .with_over("crimson")?;
    let axs = fig.subplots(2, 2)?;
    for (i, extend) in [Extend::Neither, Extend::Both].into_iter().enumerate() {
        for (j, spacing) in [Spacing::Uniform, Spacing::Proportional]
            .into_iter()
            .enumerate()
        {
            let norm = BoundaryNorm::new(levels.clone(), cmap.n(), Extend::Neither)?;
            let options = ContourOptions {
                colorizer: ColorizerOptions {
                    cmap: Some(cmap.clone()),
                    norm: Some(norm.into()),
                    ..ColorizerOptions::default()
                },
                levels: Levels::Values(levels.clone()),
                extend,
                ..ContourOptions::default()
            };
            let cs = axs[(i, j)].contourf(&field, options)?;
            fig.colorbar(
                &cs,
                ColorbarOptions {
                    spacing,
                    ..ColorbarOptions::beside(&axs[(i, j)])
                },
            )?;
        }
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The two halves of the `extend` tests: horizontal colorbars on the
/// left, vertical ones on the right, the subplots spread out.
fn halves(fig: &Figure) -> Result<[(Orientation, Figure); 2], Box<dyn std::error::Error>> {
    let subfigs = fig.subfigures(1, 2, SubfigureOptions::default())?;
    Ok([
        (Orientation::Horizontal, subfigs[(0, 0)].clone()),
        (Orientation::Vertical, subfigs[(0, 1)].clone()),
    ])
}

/// `fig.subplots_adjust(left=0.05, bottom=0.05, right=0.95, top=0.95)`.
fn spread_out(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.05),
        bottom: Some(0.05),
        right: Some(0.95),
        top: Some(0.95),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

pub(super) fn colorbar_extend_drawedges(fig: &Figure) -> CaseResult {
    let params = [
        (Extend::Both, 6),
        (Extend::Min, 5),
        (Extend::Max, 5),
        (Extend::Neither, 4),
    ];
    let viridis = colormaps::get("viridis")?;
    for (orientation, subfig) in halves(fig)? {
        let axs = match orientation {
            Orientation::Horizontal => subfig.subplots(4, 1)?,
            Orientation::Vertical => subfig.subplots(1, 4)?,
        };
        spread_out(fig)?;
        for (ax, (extend, nb_colors)) in axs.as_slice().iter().zip(params) {
            // `cmap(np.linspace(100, 255, nb_colors).astype(int))`
            let colors: Vec<Color> = linspace(100.0, 255.0, nb_colors)
                .into_iter()
                .map(|i| Color::from(viridis.lut()[i as usize]))
                .collect();
            let (cmap, norm) = from_levels_and_colors(&[0.0, 1.0, 2.0, 3.0, 4.0], &colors, extend)?;
            let mappable = ScalarMappable::with_norm(cmap, Box::new(norm));
            subfig.colorbar(
                &mappable,
                ColorbarOptions {
                    orientation: Some(orientation),
                    drawedges: true,
                    ..inside(ax)
                },
            )?;
            match orientation {
                Orientation::Horizontal => ax.set_xlim(1.1, 2.9)?,
                Orientation::Vertical => ax.set_ylim(1.1, 2.9)?,
            };
        }
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn colorbar_contourf_extend_patches(fig: &Figure) -> CaseResult {
    let params: [(Extend, i64, &[&str]); 4] = [
        (Extend::Both, 5, &["\\", "//"]),
        (Extend::Min, 7, &["+"]),
        (Extend::Max, 2, &["|", "-", "/", "\\", "//"]),
        (Extend::Neither, 10, &["//", "\\", "||"]),
    ];
    let halves = halves(fig)?;
    spread_out(fig)?;
    let x = linspace(-2.0, 3.0, 50);
    let y = linspace(-2.0, 3.0, 30);
    let z = Grid::from_fn(30, 50, |r, c| x[c].cos() + y[r].sin());
    let field = QuadField::new(z).with_coords(QuadCoords::Axes { x, y });
    let viridis = colormaps::get("viridis")?;
    for (orientation, subfig) in halves {
        let axs = subfig.subplots(2, 2)?;
        for (ax, (extend, levels, hatches)) in axs.as_slice().iter().zip(params) {
            let options = ContourOptions {
                colorizer: ColorizerOptions {
                    cmap: Some(viridis.clone()),
                    ..ColorizerOptions::default()
                },
                levels: Levels::Count(levels),
                hatches: hatches.iter().map(|&h| Some(h.to_owned())).collect(),
                extend,
                ..ContourOptions::default()
            };
            let cs = ax.contourf(&field, options)?;
            subfig.colorbar(
                &cs,
                ColorbarOptions {
                    orientation: Some(orientation),
                    fraction: 0.4,
                    extendfrac: ExtendFrac::Both(0.2),
                    aspect: 5.0,
                    ..ColorbarOptions::beside(ax)
                },
            )?;
        }
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn nonorm(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    fig.subplots_adjust(&SubplotOverrides {
        bottom: Some(0.5),
        ..SubplotOverrides::default()
    })?;
    let norm = NoNorm::default().with_vmin(1.0).with_vmax(5.0);
    let cmap = colormaps::get("viridis")?.resampled(5)?;
    let mappable = ScalarMappable::with_norm(cmap, Box::new(norm));
    fig.colorbar(
        &mappable,
        ColorbarOptions {
            orientation: Some(Orientation::Horizontal),
            ..inside(&ax)
        },
    )?;
    Ok(())
}

pub(super) fn boundaries(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let values = Grid::from_vec(10, 10, SpecRng::new(19_680_808).normal(100, 0.0, 1.0))?;
    let pc = mesh(
        &ax,
        values,
        ColorizerOptions::default().cmap(colormaps::get("RdBu_r")?),
    )?;
    fig.colorbar(
        &pc,
        ColorbarOptions {
            boundaries: Some(linspace(-3.0, 3.0, 7)),
            ..ColorbarOptions::beside(&ax)
        },
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn colorbar_extend_alpha(fig: &Figure) -> CaseResult {
    let options = ImshowOptions {
        alpha: Some(ImageAlpha::Uniform(0.3)),
        interpolation: Some(Interpolation::None),
        ..ImshowOptions::default()
    };
    let im = fig.subplot().imshow(two_by_two(), options)?;
    fig.colorbar(
        &im,
        ColorbarOptions {
            extend: Some(Extend::Both),
            boundaries: Some(vec![0.5, 1.5, 2.5]),
            ..ColorbarOptions::default()
        },
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn passing_location(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(two_by_two(), ImshowOptions::default())?;
    fig.colorbar(
        &im,
        ColorbarOptions {
            location: Some(SpineSide::Top),
            ..inside(&ax.inset_axes([0.0, 1.05, 1.0, 0.05]))
        },
    )?;
    Ok(())
}
