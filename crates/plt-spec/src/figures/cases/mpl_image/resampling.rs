//! Resampling: images on non-uniform grids (also on log axes), and
//! antialiasing when downsampling or upsampling in the data and RGBA
//! interpolation stages.

use std::f64::consts::PI;

use plt::axes::{AxisName, ImshowOptions, MeshCoords, NonUniformOptions, PcolorfastOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{
    Colorize, ColorizerOptions, Colormap, ColormapError, Extend, LogNorm, colormaps,
};
use plt::figure::{Figure, Share};
use plt::image::{Interpolation, InterpolationStage, NonUniformInterpolation};
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::scale::ScaleOptions;

use super::{classic, colormapped, counting, remove_ticks_and_titles, uniform};
use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_image/nonuniform_and_pcolor", nonuniform_and_pcolor)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("mpl_image/nonuniform_logscale", nonuniform_logscale),
    FigureCase::new("mpl_image/rgba_antialias", rgba_antialias).sized(3.5, 3.5, 100.0),
    classic("mpl_image/upsample_interpolation_stage", |fig| {
        fig.subplot().imshow(
            uniform(19_680_801 + 9, 4, 4),
            ImshowOptions {
                interpolation: Some(Interpolation::Bilinear),
                interpolation_stage: Some(InterpolationStage::Auto),
                ..colormapped("viridis")?
            },
        )?;
        Ok(())
    }),
    classic("mpl_image/downsample_interpolation_stage", |fig| {
        fig.subplot().imshow(
            uniform(19_680_801, 400, 400),
            ImshowOptions {
                interpolation: Some(Interpolation::Auto),
                interpolation_stage: Some(InterpolationStage::Auto),
                ..colormapped("viridis")?
            },
        )?;
        Ok(())
    }),
    FigureCase::new("mpl_image/downsampling", downsampling).sized(7.0, 6.0, 100.0),
    FigureCase::new("mpl_image/downsampling_speckle", downsampling_speckle).sized(5.0, 2.7, 100.0),
    FigureCase::new("mpl_image/upsampling", upsampling).sized(6.5, 3.0, 100.0),
];

/// The test array of the antialiasing tests, `n` pixels square: rows
/// alternating between -1 and 1 left of column `split`, aliased
/// concentric rings of frequency `f0` and chirp `k` right of it, with a
/// striped disc in the upper half.
fn rings(n: usize, f0: f64, k: f64, split: usize) -> Grid<f64> {
    let coordinate = |i: usize| i as f64 / n as f64 - 0.5;
    Grid::from_fn(n, n, |r, c| {
        if c < split {
            return if r % 2 == 0 { -1.0 } else { 1.0 };
        }
        let radius = (coordinate(c).powi(2) + coordinate(r).powi(2)).sqrt();
        if r < n / 2 && radius < 0.3 {
            1.0
        } else if r < n / 2 && radius < 0.4 {
            -1.0
        } else {
            (PI * 2.0 * (f0 * radius + k * radius.powi(2) / 2.0)).sin()
        }
    })
}

/// `test_nonuniform_and_pcolor`: nearest and bilinear non-uniform images
/// and a `PcolorImage` on the same squared grid.
fn nonuniform_and_pcolor(fig: &Figure) -> CaseResult {
    let axs = fig.subplots_shared(3, 1, Share::All, Share::All)?;
    let centres = [0.0, 1.0, 4.0];
    let interpolations = [
        NonUniformInterpolation::Nearest,
        NonUniformInterpolation::Bilinear,
    ];
    for (ax, interpolation) in axs.as_slice().iter().zip(interpolations) {
        ax.nonuniform_image(
            &centres,
            &centres,
            counting(3, 3),
            NonUniformOptions {
                interpolation,
                ..NonUniformOptions::default()
            },
        )?;
    }
    let edges = || MeshCoords::from([0.0, 1.0, 4.0, 9.0]);
    axs[(2, 0)].pcolorfast(
        Some((edges(), edges())),
        Grid::from_fn(3, 3, |r, c| (3 * r + c) as f64),
        PcolorfastOptions::default(),
    )?;
    for ax in axs.as_slice() {
        ax.set_axis_off();
        ax.set_xlim(0.0, 10.0);
    }
    Ok(())
}

/// `test_nonuniform_logscale`: a non-uniform image on linear, base-2 and
/// base-4 log axes.
fn nonuniform_logscale(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    let centres = [1.0, 4.0, 9.0];
    for (i, ax) in axs.as_slice().iter().enumerate() {
        ax.set_xlim(1.0, 16.0).set_ylim(1.0, 16.0);
        ax.set_box_aspect(Some(1.0))?;
        if i > 0 {
            let base = ScaleOptions {
                base: Some(if i == 1 { 2.0 } else { 4.0 }),
                ..ScaleOptions::default()
            };
            for axis in [AxisName::X, AxisName::Y] {
                ax.set_scale_with(axis, "log", base.clone())?;
            }
        }
        ax.nonuniform_image(
            &centres,
            &centres,
            counting(3, 3),
            NonUniformOptions::default(),
        )?;
    }
    Ok(())
}

/// `RdBu_r` with yellow and cyan extremes.
fn rdbu_extremes() -> Result<Colormap, ColormapError> {
    colormaps::get("RdBu_r")?
        .with_over("yellow")?
        .with_under("cyan")
}

/// `test_rgba_antialias`: stripes, rings, NaNs and extreme values zoomed,
/// without antialiasing and antialiased in either stage.
fn rgba_antialias(fig: &Figure) -> CaseResult {
    let n = 250;
    let mut aa = rings(n, 10.0, 75.0, n / 2);
    let blocks = [
        (20..50, 20..50, f64::NAN),
        (70..90, 70..90, 1e6),
        (70..90, 20..30, -1e6),
        (70..90, 195..215, 1e6),
        (20..30, 195..215, -1e6),
    ];
    for (rows, cols, value) in blocks {
        for r in rows {
            for c in cols.clone() {
                aa[(r, c)] = value;
            }
        }
    }
    let axs = fig.subplots(2, 2)?;
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let settings = [
        (Interpolation::Nearest, None),
        (Interpolation::Nearest, None),
        (Interpolation::Auto, Some(InterpolationStage::Data)),
        (Interpolation::Auto, Some(InterpolationStage::Rgba)),
    ];
    for (ax, (interpolation, stage)) in axs.as_slice().iter().zip(settings) {
        ax.imshow(
            aa.clone(),
            ImshowOptions {
                colorizer: ColorizerOptions::default()
                    .cmap(rdbu_extremes()?)
                    .limits(Some(-1.2), Some(1.2)),
                interpolation: Some(interpolation),
                interpolation_stage: stage,
                ..ImshowOptions::default()
            },
        )?;
    }
    let half = n as f64 / 2.0;
    axs[(0, 0)]
        .set_xlim(half - 25.0, half + 25.0)
        .set_ylim(half + 50.0, half - 10.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_downsampling`: a zoomed view, then nearest and Hanning
/// downsampling in both stages and the adaptive default.
fn downsampling(fig: &Figure) -> CaseResult {
    let n = 450;
    let a = rings(n, 5.0, 100.0, n / 3);
    let axs = fig.subplots(2, 3)?;
    fig.set_layout_engine(LayoutEngine::compressed())?;
    let zoom = &axs[(0, 0)];
    zoom.imshow(
        a.clone(),
        ImshowOptions {
            interpolation: Some(Interpolation::Nearest),
            interpolation_stage: Some(InterpolationStage::Rgba),
            ..colormapped("RdBu_r")?
        },
    )?;
    zoom.set_xlim(125.0, 175.0).set_ylim(250.0, 200.0);
    zoom.set_title("Zoom");
    let settings = [
        (Interpolation::Nearest, InterpolationStage::Data),
        (Interpolation::Nearest, InterpolationStage::Rgba),
        (Interpolation::Hanning, InterpolationStage::Data),
        (Interpolation::Hanning, InterpolationStage::Rgba),
        (Interpolation::Auto, InterpolationStage::Auto),
    ];
    for (ax, (interpolation, stage)) in axs.as_slice()[1..].iter().zip(settings) {
        ax.imshow(
            a.clone(),
            ImshowOptions {
                interpolation: Some(interpolation),
                interpolation_stage: Some(stage),
                ..colormapped("RdBu_r")?
            },
        )?;
        ax.set_title(format!(
            "interpolation='{interpolation}'\nspace='{}'",
            stage_name(stage)
        ));
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// matplotlib's name of an interpolation stage.
fn stage_name(stage: InterpolationStage) -> &'static str {
    match stage {
        InterpolationStage::Data => "data",
        InterpolationStage::Rgba => "rgba",
        InterpolationStage::Auto => "auto",
    }
}

/// `test_downsampling_speckle`: a log-normed staircase downsampled in the
/// RGBA stage, explicitly and by default.
fn downsampling_speckle(fig: &Figure) -> CaseResult {
    let img = Grid::from_fn(720, 1024, |r, c| {
        if c >= r {
            (c as f64 / 50.0).floor()
        } else {
            0.0
        }
    });
    let cmap = colormaps::get("viridis")?.with_over("m")?;
    let axs = fig.subplots_shared(1, 2, Share::All, Share::All)?;
    fig.set_layout_engine(LayoutEngine::compressed())?;
    let stages = [
        (
            "interpolation='auto', stage='rgba'",
            Some(InterpolationStage::Rgba),
        ),
        ("interpolation='auto', stage='auto'", None),
    ];
    for (ax, (title, stage)) in axs.as_slice().iter().zip(stages) {
        ax.set_title(title);
        ax.imshow(
            img.clone(),
            ImshowOptions {
                colorizer: ColorizerOptions::default()
                    .cmap(cmap.clone())
                    .norm(LogNorm::new(3.0, 11.0)),
                interpolation_stage: stage,
                ..ImshowOptions::default()
            },
        )?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_upsampling`: a 4 × 4 image upsampled adaptively and with `sinc`
/// in both stages, with one extended colorbar for all three.
fn upsampling(fig: &Figure) -> CaseResult {
    let a = uniform(19_680_801 + 9, 4, 4);
    let axs = fig.subplots(1, 3)?;
    fig.set_layout_engine(LayoutEngine::compressed())?;
    let panels = [
        (
            None,
            None,
            "interpolation='auto'\nstage='antialaised'\n(default for upsampling)",
        ),
        (
            Some(Interpolation::Sinc),
            None,
            "interpolation='sinc'\nstage='auto'\n(default for upsampling)",
        ),
        (
            Some(Interpolation::Sinc),
            Some(InterpolationStage::Rgba),
            "interpolation='sinc'\nstage='rgba'",
        ),
    ];
    let mut first = None;
    for (ax, (interpolation, stage, title)) in axs.as_slice().iter().zip(panels) {
        let image = ax.imshow(
            a.clone(),
            ImshowOptions {
                interpolation,
                interpolation_stage: stage,
                ..colormapped("viridis")?
            },
        )?;
        first.get_or_insert(image);
        ax.set_title(title);
    }
    let image = first.expect("three panels");
    fig.colorbar(
        &image,
        ColorbarOptions {
            axes: ColorbarAxes::Parents(axs.as_slice().to_vec()),
            shrink: 0.7,
            extend: Some(Extend::Both),
            ..ColorbarOptions::default()
        },
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}
