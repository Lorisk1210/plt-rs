//! Interpolation and opacity: interpolating the alpha channel, `none`
//! against `nearest`, figure images, adaptive (`auto`) interpolation
//! against the kernel it must pick, and scalar or per-pixel alpha on
//! scalar, RGB and RGBA images in both interpolation stages.

use plt::axes::ImshowOptions;
use plt::colors::{Colorize, ColorizerOptions, colormaps};
use plt::figure::Figure;
use plt::image::{FigimageOptions, ImageAlpha, Interpolation, InterpolationStage, Origin};
use plt::numeric::{Grid, linspace};

use super::{classic, remove_ticks_and_titles, uniform};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(crate) const CASES: &[FigureCase] = &[
    classic("mpl_image/alpha_interp", |fig| {
        let axs = fig.subplots(1, 2)?;
        // Full green, transparent above the main diagonal.
        let img = Grid::from_fn(5, 5, |r, c| [0.0, 1.0, 0.0, f64::from(u8::from(c <= r))]);
        axs[(0, 0)].imshow(
            img.clone(),
            ImshowOptions::default().interpolation(Interpolation::None),
        )?;
        axs[(0, 1)].imshow(
            img,
            ImshowOptions::default().interpolation(Interpolation::Bilinear),
        )?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_image/interp_nearest_vs_none", |fig| {
        let pixels = Grid::from_vec(
            2,
            2,
            vec![
                [218, 165, 32],
                [122, 103, 238],
                [127, 255, 0],
                [255, 99, 71],
            ],
        )?;
        let axs = fig.subplots(1, 2)?;
        for (ax, interpolation) in axs
            .as_slice()
            .iter()
            .zip([Interpolation::None, Interpolation::Nearest])
        {
            ax.imshow(
                pixels.clone(),
                ImshowOptions::default().interpolation(interpolation),
            )?;
            ax.set_title(format!("interpolation {interpolation}"));
        }
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .sized(6.4, 4.8, 3.0),
    classic("mpl_image/figimage", |fig| figimage(fig, false)).sized(2.0, 2.0, 100.0),
    classic("mpl_image/figimage_suppress_composite", |fig| {
        figimage(fig, true)
    })
    .sized(2.0, 2.0, 100.0),
    classic("mpl_image/imshow_antialiased_downsample", |fig| {
        antialiased(fig, 5)
    })
    .sized(2.0, 2.0, 100.0),
    classic("mpl_image/imshow_antialiased_exact", |fig| {
        antialiased(fig, 5)
    })
    .sized(5.0, 5.0, 100.0),
    classic("mpl_image/imshow_antialiased_double", |fig| {
        antialiased(fig, 5)
    })
    .sized(10.0, 10.0, 100.0),
    classic("mpl_image/imshow_antialiased_upsample_below_3", |fig| {
        antialiased(fig, 3)
    })
    .sized(2.9, 2.9, 100.0),
    classic("mpl_image/imshow_antialiased_upsample_above_3", |fig| {
        antialiased(fig, 3)
    })
    .sized(9.1, 9.1, 100.0),
    classic("mpl_image/imshow_zoom", |fig| {
        let ax = fig.subplot();
        ax.imshow(
            uniform(19_680_801, 300, 300),
            ImshowOptions::default().interpolation(Interpolation::Auto),
        )?;
        ax.set_xlim(10.0, 20.0)?.set_ylim(10.0, 20.0)?;
        Ok(())
    })
    .sized(2.9, 2.9, 100.0),
    classic("mpl_image/image_alpha", |fig| {
        let z = uniform(0, 6, 6);
        let axs = fig.subplots(1, 3)?;
        let settings = [
            (1.0, Interpolation::None),
            (0.5, Interpolation::None),
            (0.5, Interpolation::Nearest),
        ];
        for (ax, (alpha, interpolation)) in axs.as_slice().iter().zip(settings) {
            ax.imshow(
                z.clone(),
                ImshowOptions {
                    alpha: Some(ImageAlpha::Uniform(alpha)),
                    ..ImshowOptions::default().interpolation(interpolation)
                },
            )?;
        }
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_image/imshow_alpha", imshow_alpha),
    classic("mpl_image/image_array_alpha", |fig| {
        let x = linspace(0.0, 1.0, 50);
        let zz = Grid::from_fn(50, 50, |r, c| {
            (-3.0 * (x[c] - 0.5).powi(2) + (x[r] - 0.7_f64.powi(2))).exp()
        });
        let max = zz.as_slice().iter().copied().fold(f64::MIN, f64::max);
        let alpha = zz.map(|&v| v / max);
        fig.subplot().imshow(
            zz,
            ImshowOptions {
                colorizer: ColorizerOptions::default().cmap(colormaps::get("viridis")?),
                alpha: Some(ImageAlpha::Each(alpha)),
                ..ImshowOptions::default().interpolation(Interpolation::Nearest)
            },
        )?;
        Ok(())
    }),
    classic(
        "mpl_image/interpolation_stage_rgba_respects_alpha_param_data",
        |fig| alpha_param(fig, InterpolationStage::Data),
    ),
    classic(
        "mpl_image/interpolation_stage_rgba_respects_alpha_param_rgba",
        |fig| alpha_param(fig, InterpolationStage::Rgba),
    ),
];

/// `test_figimage`: four mirrored copies of one image tiling the canvas.
fn figimage(fig: &Figure, suppress_composite: bool) -> CaseResult {
    fig.set_suppress_composite(Some(suppress_composite));
    let img = Grid::from_fn(100, 100, |r, c| {
        let (x, y) = (r as f64 / 100.0, c as f64 / 100.0);
        (x * x + y * y - x * y).sin() + (20.0 * x * x + 50.0 * y * y).sin() / 5.0
    });
    let tiles = [
        (0.0, 0.0, false, false),
        (0.0, 100.0, true, false),
        (100.0, 0.0, false, true),
        (100.0, 100.0, true, true),
    ];
    for (xo, yo, flip_rows, flip_cols) in tiles {
        let tile = Grid::from_fn(100, 100, |r, c| {
            img[(
                if flip_rows { 99 - r } else { r },
                if flip_cols { 99 - c } else { c },
            )]
        });
        fig.figimage(
            tile,
            FigimageOptions {
                xo,
                yo,
                origin: Some(Origin::Lower),
                ..FigimageOptions::default()
            },
        )?;
    }
    Ok(())
}

/// `test_imshow_antialiased`: a random image `100 × size` pixels square
/// filling the canvas, with adaptive interpolation.
fn antialiased(fig: &Figure, size: usize) -> CaseResult {
    let n = 100 * size;
    let ax = fig.subplot();
    ax.set_position([0.0, 0.0, 1.0, 1.0]);
    ax.imshow(
        uniform(19_680_801, n, n),
        ImshowOptions::default().interpolation(Interpolation::Auto),
    )?;
    Ok(())
}

/// `test_imshow_alpha`: uniform alpha on float and byte RGB images.
fn imshow_alpha(fig: &Figure) -> CaseResult {
    let draws = SpecRng::new(19_680_801).uniform(6 * 6 * 3, 0.0, 1.0);
    // numpy multiplies the single-precision array in single precision.
    let rgbf: Vec<[f32; 3]> = draws
        .as_chunks::<3>()
        .0
        .iter()
        .map(|rgb| rgb.map(|v| v as f32))
        .collect();
    let rgbu: Vec<[u8; 3]> = rgbf
        .iter()
        .map(|rgb| rgb.map(|v| (v * 255.0) as u8))
        .collect();
    let axs = fig.subplots(2, 2)?;
    let with_alpha = |alpha: f64| ImshowOptions {
        alpha: Some(ImageAlpha::Uniform(alpha)),
        ..ImshowOptions::default()
    };
    axs[(0, 0)].imshow(Grid::from_vec(6, 6, rgbf.clone())?, with_alpha(0.5))?;
    axs[(0, 1)].imshow(Grid::from_vec(6, 6, rgbf)?, with_alpha(0.75))?;
    axs[(1, 0)].imshow(
        Grid::from_vec(6, 6, rgbu.clone())?,
        with_alpha(127.0 / 255.0),
    )?;
    axs[(1, 1)].imshow(Grid::from_vec(6, 6, rgbu)?, with_alpha(191.0 / 255.0))?;
    Ok(())
}

/// `test_interpolation_stage_rgba_respects_alpha_param`: scalar and
/// array alpha on RGB and RGBA images resampled in `stage`.
fn alpha_param(fig: &Figure, stage: InterpolationStage) -> CaseResult {
    let mut draws = SpecRng::new(19_680_801);
    let mut grid = |n: usize| draws.uniform(n, 0.0, 1.0);
    let array_alpha = Grid::from_vec(3, 3, grid(9))?;
    let rgb_values = grid(27);
    let new_array_alpha = Grid::from_vec(3, 3, grid(9))?;
    let im_rgb = Grid::from_fn(3, 3, |r, c| {
        let at = 3 * (3 * r + c);
        [rgb_values[at], rgb_values[at + 1], rgb_values[at + 2]]
    });
    let im_rgba = Grid::from_fn(3, 3, |r, c| {
        let [red, green, blue] = im_rgb[(r, c)];
        [red, green, blue, array_alpha[(r, c)]]
    });
    let staged = |alpha: ImageAlpha| ImshowOptions {
        interpolation_stage: Some(stage),
        alpha: Some(alpha),
        ..ImshowOptions::default()
    };
    let axs = fig.subplots(2, 3)?;
    axs[(0, 0)].imshow(im_rgb.clone(), ImshowOptions::default())?;
    axs[(0, 1)].imshow(im_rgb.clone(), staged(ImageAlpha::Uniform(0.5)))?;
    axs[(0, 2)].imshow(im_rgb, staged(ImageAlpha::Each(array_alpha)))?;
    axs[(1, 0)].imshow(im_rgba.clone(), ImshowOptions::default())?;
    axs[(1, 1)].imshow(im_rgba.clone(), staged(ImageAlpha::Uniform(0.5)))?;
    axs[(1, 2)].imshow(im_rgba, staged(ImageAlpha::Each(new_array_alpha)))?;
    Ok(())
}
