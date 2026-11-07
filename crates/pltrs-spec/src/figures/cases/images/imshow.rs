//! The `imshow`, `matshow`, `spy` and `figimage` cases.

use pltrs::axes::ImshowOptions;
use pltrs::colors::{
    BoundaryNorm, Colorize, ColorizerOptions, Extend, LogNorm, PowerNorm, SymLogNorm, colormaps,
};
use pltrs::figure::Figure;
use pltrs::image::{
    Extent, FigimageOptions, ImageAlpha, ImageData, Interpolation, InterpolationStage, Origin,
    ScalarField,
};
use pltrs::lines::LineProps;
use pltrs::numeric::{Grid, linspace, pow};
use pltrs::patches::Circle;
use pltrs::rc::Aspect;

use super::data::{bump, bytes, column_ramp, rgb_float, rings, small};
use crate::figures::CaseResult;
use crate::rng::SpecRng;

/// `_interp_case`: the small pattern upsampled and the rings downsampled.
pub(super) fn interpolation(fig: &Figure, interpolation: Interpolation) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    for (ax, data) in axes.as_slice().iter().zip([small(), rings(400)]) {
        let options = ImshowOptions {
            interpolation: Some(interpolation),
            ..ImshowOptions::default()
        };
        ax.imshow(data, options)?;
    }
    Ok(())
}

/// `interp_stage_data` / `interp_stage_rgba`.
pub(super) fn stage(fig: &Figure, stage: InterpolationStage) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let pairs = [
        (small(), Interpolation::Bilinear),
        (rings(400), Interpolation::Hanning),
    ];
    for (ax, (data, interpolation)) in axes.as_slice().iter().zip(pairs) {
        let options = ImshowOptions {
            interpolation: Some(interpolation),
            interpolation_stage: Some(stage),
            ..ImshowOptions::default().cmap(colormaps::get("RdBu")?)
        };
        ax.imshow(data, options)?;
    }
    Ok(())
}

pub(super) fn filterrad_sinc(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let sinc = ImshowOptions {
        interpolation: Some(Interpolation::Sinc),
        filterrad: Some(2.0),
        ..ImshowOptions::default()
    };
    axes[(0, 0)].imshow(small(), sinc)?;
    let lanczos = ImshowOptions {
        interpolation: Some(Interpolation::Lanczos),
        filternorm: Some(false),
        ..ImshowOptions::default()
    };
    axes[(0, 1)].imshow(small(), lanczos)?;
    Ok(())
}

pub(super) fn imshow_default(fig: &Figure) -> CaseResult {
    let values = SpecRng::new(1).uniform(120, 0.0, 1.0);
    fig.subplot()
        .imshow(Grid::from_vec(10, 12, values)?, ImshowOptions::default())?;
    Ok(())
}

pub(super) fn origin_lower(fig: &Figure) -> CaseResult {
    let options = ImshowOptions {
        origin: Some(Origin::Lower),
        ..ImshowOptions::default()
    };
    fig.subplot().imshow(small(), options)?;
    Ok(())
}

pub(super) fn extent(fig: &Figure) -> CaseResult {
    let options = ImshowOptions {
        extent: Some(Extent::new(-3.0, 3.0, -2.0, 2.0)),
        ..ImshowOptions::default()
    };
    fig.subplot().imshow(bump(30), options)?;
    Ok(())
}

pub(super) fn extent_origin_lower_and_reversed(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let lower = ImshowOptions {
        origin: Some(Origin::Lower),
        extent: Some(Extent::new(0.0, 16.0, 10.0, 30.0)),
        ..ImshowOptions::default()
    };
    axes[(0, 0)].imshow(small(), lower)?;
    let reversed = ImshowOptions {
        extent: Some(Extent::new(8.0, 0.0, 0.0, 8.0)),
        ..ImshowOptions::default()
    };
    axes[(0, 1)].imshow(small(), reversed)?;
    Ok(())
}

pub(super) fn aspect_auto_and_numeric(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    for (ax, (data, aspect)) in axes
        .as_slice()
        .iter()
        .zip([(bump(20), Aspect::Auto), (small(), Aspect::Ratio(0.5))])
    {
        let options = ImshowOptions {
            aspect: Some(aspect),
            ..ImshowOptions::default()
        };
        ax.imshow(data, options)?;
    }
    Ok(())
}

pub(super) fn xlim_zoom(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.imshow(bump(40), ImshowOptions::default())?;
    ax.set_xlim(10.0, 25.0)?.set_ylim(30.0, 5.0)?;
    Ok(())
}

pub(super) fn rgb_float_uint8_rgba(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 3)?;
    axes[(0, 0)].imshow(rgb_float(6), ImshowOptions::default())?;
    axes[(0, 1)].imshow(bytes(&rgb_float(6)), ImshowOptions::default())?;
    let ax = &axes[(0, 2)];
    ax.plot_with(
        [0.0, 5.0],
        [0.0, 5.0],
        LineProps::default().color("k").linewidth(4.0),
    )?;
    let alpha = column_ramp(6, 0.1, 1.0, 6);
    let rgb = rgb_float(6);
    let rgba = Grid::from_fn(6, 6, |r, c| {
        let [red, green, blue] = rgb[(r, c)];
        [red, green, blue, alpha[(r, c)]]
    });
    ax.imshow(rgba, ImshowOptions::default())?;
    Ok(())
}

pub(super) fn rgba_uint8_downsampled(fig: &Figure) -> CaseResult {
    let rgb = rgb_float(300);
    let rings = rings(300);
    let rgba = Grid::from_fn(300, 300, |r, c| {
        let [red, green, blue] = rgb[(r, c)];
        let alpha = f64::from(u8::from(rings[(r, c)] > 0.0)) * 0.5 + 0.5;
        [red, green, blue, alpha]
    });
    fig.subplot()
        .imshow(bytes(&rgba), ImshowOptions::default())?;
    Ok(())
}

pub(super) fn alpha_scalar_and_array(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    for ax in axes.as_slice() {
        ax.plot_with(
            [-0.5, 7.5],
            [-0.5, 7.5],
            LineProps::default().color("r").linewidth(6.0),
        )?;
    }
    let alphas = [
        ImageAlpha::Uniform(0.5),
        ImageAlpha::Each(column_ramp(8, 0.0, 1.0, 8)),
    ];
    for (ax, alpha) in axes.as_slice().iter().zip(alphas) {
        let options = ImshowOptions {
            alpha: Some(alpha),
            ..ImshowOptions::default()
        };
        ax.imshow(small(), options)?;
    }
    Ok(())
}

pub(super) fn vmin_vmax(fig: &Figure) -> CaseResult {
    let colors = ColorizerOptions::default()
        .cmap(colormaps::get("coolwarm")?)
        .limits(Some(-0.3), Some(0.3));
    fig.subplot()
        .imshow(bump(20), ImshowOptions::default().colorizer(colors))?;
    Ok(())
}

pub(super) fn lognorm(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 4.0, 40);
    let data = Grid::from_fn(40, 40, |r, c| pow(10.0, x[r] + x[c] / 2.0));
    let colors = ColorizerOptions::default().norm(LogNorm::default());
    fig.subplot()
        .imshow(data, ImshowOptions::default().colorizer(colors))?;
    Ok(())
}

pub(super) fn symlognorm(fig: &Figure) -> CaseResult {
    let data = bump(30).map(|v| 100.0 * v.powi(3));
    let colors = ColorizerOptions::default()
        .cmap(colormaps::get("RdBu_r")?)
        .norm(SymLogNorm::new(0.5, 1.0, 10.0)?);
    fig.subplot()
        .imshow(data, ImshowOptions::default().colorizer(colors))?;
    Ok(())
}

pub(super) fn powernorm(fig: &Figure) -> CaseResult {
    let colors = ColorizerOptions::default()
        .cmap(colormaps::get("magma")?)
        .norm(PowerNorm::new(0.4));
    fig.subplot().imshow(
        bump(30).map(|v| v.abs()),
        ImshowOptions::default().colorizer(colors),
    )?;
    Ok(())
}

pub(super) fn boundarynorm(fig: &Figure) -> CaseResult {
    let norm = BoundaryNorm::new([-1.0, -0.5, -0.1, 0.1, 0.5, 1.0], 256, Extend::Neither)?;
    let colors = ColorizerOptions::default()
        .cmap(colormaps::get("RdBu")?)
        .norm(norm);
    fig.subplot()
        .imshow(bump(30), ImshowOptions::default().colorizer(colors))?;
    Ok(())
}

pub(super) fn cmap_resampled(fig: &Figure) -> CaseResult {
    let colors = ColorizerOptions::default().cmap(colormaps::get("viridis")?.resampled(5)?);
    fig.subplot()
        .imshow(bump(30), ImshowOptions::default().colorizer(colors))?;
    Ok(())
}

pub(super) fn cmap_extremes(fig: &Figure) -> CaseResult {
    let values = bump(20);
    let mask = values.map(|&v| v > 0.6);
    let mut values = values;
    values[(3, 3)] = f64::NAN;
    let mask = Grid::from_fn(20, 20, |r, c| mask[(r, c)] && (r, c) != (3, 3));
    let cmap = colormaps::get("viridis")?
        .with_bad("red")?
        .with_under("black")?
        .with_over("white")?;
    let colors = ColorizerOptions::default()
        .cmap(cmap)
        .limits(Some(-0.4), Some(0.4));
    fig.subplot().imshow(
        ScalarField::masked(values, &mask)?,
        ImshowOptions::default().colorizer(colors),
    )?;
    Ok(())
}

pub(super) fn masked_default(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_with(
        [0.0, 19.0],
        [0.0, 19.0],
        LineProps::default().color("k").linewidth(3.0),
    )?;
    let values = bump(20);
    let mask = values.map(|&v| (-0.1..=0.1).contains(&v));
    ax.imshow(
        ScalarField::masked(values, &mask)?,
        ImshowOptions::default(),
    )?;
    Ok(())
}

pub(super) fn matshow(fig: &Figure) -> CaseResult {
    let data = Grid::from_fn(5, 7, |r, c| ((r * 7 + c) % 6) as i64);
    fig.subplot().matshow(
        ImageData::from(ScalarField::integers(&data)),
        ImshowOptions::default(),
    )?;
    Ok(())
}

pub(super) fn spy_image_and_markers(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let values = SpecRng::new(2).uniform(400, 0.0, 1.0);
    let a = Grid::from_vec(20, 20, values)?.map(|&v| if v > 0.8 { 1.0 } else { 0.0 });
    axes[(0, 0)].spy(&a, pltrs::axes::SpyOptions::default())?;
    let markers = pltrs::axes::SpyOptions {
        markersize: Some(4.0),
        precision: 0.5,
        ..pltrs::axes::SpyOptions::default()
    };
    axes[(0, 1)].spy(&a, markers)?;
    Ok(())
}

pub(super) fn figimage(fig: &Figure) -> CaseResult {
    let viridis = FigimageOptions {
        xo: 20.0,
        yo: 30.0,
        colorizer: ColorizerOptions::default().cmap(colormaps::get("viridis")?),
        ..FigimageOptions::default()
    };
    fig.figimage(bump(80), viridis)?;
    let translucent = FigimageOptions {
        xo: 70.0,
        yo: 80.0,
        alpha: Some(ImageAlpha::Uniform(0.6)),
        ..FigimageOptions::default()
    };
    fig.figimage(bytes(&rgb_float(100)), translucent)?;
    let lower = FigimageOptions {
        xo: 300.0,
        yo: 200.0,
        origin: Some(Origin::Lower),
        ..FigimageOptions::default()
    };
    fig.figimage(bump(50), lower)?;
    Ok(())
}

pub(super) fn clip_path_circle(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let image = ax.imshow(bump(40), ImshowOptions::default())?;
    image.set_clip_patch(&Circle::new((20.0, 20.0), 15.0), ax.trans_data());
    Ok(())
}
