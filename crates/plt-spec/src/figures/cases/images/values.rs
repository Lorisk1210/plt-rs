//! Image values: non-finite data through the interpolation stages,
//! resampling factors, alpha compositing, boolean or integer arrays and
//! the alpha of non-uniform `pcolorfast` images.

use plt::axes::{ImshowOptions, MeshCoords, PcolorfastOptions};
use plt::colors::{Colorize, ColorizerOptions, colormaps};
use plt::figure::Figure;
use plt::image::{Extent, ImageAlpha, ImageData, Interpolation, InterpolationStage, ScalarField};
use plt::lines::LineProps;
use plt::numeric::{Grid, arange};

use super::data::{bump, rings, small};
use crate::figures::CaseResult;

/// `_with_holes()`: the 12 × 12 bump with a NaN block and two infinities.
fn with_holes() -> Grid<f64> {
    let mut data = bump(12);
    for r in 2..4 {
        for c in 3..7 {
            data[(r, c)] = f64::NAN;
        }
    }
    data[(8, 8)] = f64::INFINITY;
    data[(9, 2)] = f64::NEG_INFINITY;
    data
}

pub(super) fn nan_inf_interpolation_stages(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    let cmap = colormaps::get("viridis")?
        .with_bad("magenta")?
        .with_under("k")?
        .with_over("w")?;
    let settings = [
        (Interpolation::Nearest, None),
        (Interpolation::Bilinear, Some(InterpolationStage::Data)),
        (Interpolation::Bilinear, Some(InterpolationStage::Rgba)),
    ];
    for (ax, (interpolation, stage)) in axs.as_slice().iter().zip(settings) {
        ax.imshow(
            with_holes(),
            ImshowOptions {
                colorizer: ColorizerOptions::default().cmap(cmap.clone()),
                interpolation: Some(interpolation),
                interpolation_stage: stage,
                ..ImshowOptions::default()
            },
        )?;
    }
    Ok(())
}

pub(super) fn resample_factors(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    axs[(0, 0)].imshow(rings(60), ImshowOptions::default())?;
    axs[(0, 1)].imshow(small(), ImshowOptions::default())?;
    axs[(0, 2)].imshow(
        rings(250),
        ImshowOptions {
            interpolation_stage: Some(InterpolationStage::Auto),
            ..ImshowOptions::default()
        },
    )?;
    axs[(0, 2)].set_xlim(0.0, 120.0);
    Ok(())
}

/// The alpha ramp of `composite_alpha`: up from 0 to 1, then down to 0.
fn alpha_ramp() -> Vec<f64> {
    let mut ramp = arange(0.0, 1.1, 0.1);
    ramp.extend(arange(0.0, 1.0, 0.1).into_iter().rev());
    ramp
}

pub(super) fn composite_alpha(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let ramp = alpha_ramp();
    let red = Grid::from_fn(11, 21, |_, c| [1.0, 0.0, 0.0, ramp[c]]);
    let yellow = Grid::from_fn(21, 11, |r, _| [1.0, 1.0, 0.0, ramp[r]]);
    let layers = [
        (&red, Extent::new(1.0, 2.0, 5.0, 0.0), Some(0.3)),
        (&red, Extent::new(2.0, 3.0, 5.0, 0.0), Some(0.6)),
        (&red, Extent::new(3.0, 4.0, 5.0, 0.0), None),
        (&yellow, Extent::new(0.0, 5.0, 1.0, 2.0), None),
        (&yellow, Extent::new(0.0, 5.0, 2.0, 3.0), Some(0.6)),
        (&yellow, Extent::new(0.0, 5.0, 3.0, 4.0), Some(0.3)),
    ];
    for (pixels, extent, alpha) in layers {
        ax.imshow(
            pixels.clone(),
            ImshowOptions {
                extent: Some(extent),
                alpha: alpha.map(ImageAlpha::Uniform),
                ..ImshowOptions::default()
            },
        )?;
    }
    ax.set_facecolor((0.0, 0.5, 0.0, 1.0));
    ax.set_xlim(0.0, 5.0).set_ylim(5.0, 0.0);
    Ok(())
}

pub(super) fn bool_and_integer_arrays(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    let pattern = small();
    axs[(0, 0)].imshow(
        ScalarField::booleans(&pattern.map(|&v| v > 0.5)),
        ImshowOptions::default(),
    )?;
    let codes = Grid::from_fn(6, 8, |r, c| ((r * 8 + c) * 37 % 101) as i64);
    axs[(0, 1)].imshow(
        ImageData::from(ScalarField::integers(&codes)),
        ImshowOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("tab20")?),
            ..ImshowOptions::default()
        },
    )?;
    let levels = pattern.map(|&v| (v * 60000.0) as i64);
    axs[(0, 2)].imshow(
        ScalarField::integers(&levels),
        ImshowOptions {
            colorizer: ColorizerOptions::default()
                .cmap(colormaps::get("gray")?)
                .limits(Some(0.0), Some(65535.0)),
            ..ImshowOptions::default()
        },
    )?;
    Ok(())
}

pub(super) fn pcolorfast_nonuniform_alpha(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_with(
        [0.0, 10.0],
        [0.0, 5.0],
        LineProps::default().color("k").linewidth(4.0),
    )?;
    let coords = (
        MeshCoords::from([0.0, 1.0, 3.0, 6.0, 10.0]),
        MeshCoords::from([0.0, 2.0, 3.0, 5.0]),
    );
    ax.pcolorfast(
        Some(coords),
        Grid::from_fn(3, 4, |r, c| (r * 4 + c) as f64),
        PcolorfastOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("plasma")?),
            alpha: Some(0.5),
            ..PcolorfastOptions::default()
        },
    )?;
    Ok(())
}
