//! Placement, masks and transparency: `image_exact_placement`,
//! `image_masked`, `image_nonuniform` and `image_transparency_blend`.

use plt::axes::{Axes, AxisName, ImshowOptions, NonUniformOptions, TickParams};
use plt::colorbar::{ColorbarOptions, Spacing};
use plt::colors::{
    BoundaryNorm, Color, Colorize, ColorizerOptions, Extend, Normalize, colormaps, to_rgba,
};
use plt::contour::{ContourOptions, Levels, QuadField};
use plt::figure::{Figure, Share};
use plt::fonts::{FontSize, RelativeSize};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::image::{Extent, ImageAlpha, Interpolation, NonUniformInterpolation, Origin, ScalarField};
use plt::layout::LayoutEngine;
use plt::lines::LineStyle;
use plt::numeric::{Grid, linspace};
use plt::patches::{PatchProps, Rectangle};
use plt::rc::Aspect;
use plt::transforms::Anchor;

use super::data::{chirp, mesh_field};
use crate::figures::cases::common::two_bumps;
use crate::figures::{CaseResult, FigureCase};

/// `image_exact_placement`'s images: A (100 × 300) and B (40 × 200) cut
/// from the corner of a chirp.
fn exact_placement_images() -> (Grid<f64>, Grid<f64>) {
    let (_, a) = chirp(450, 0.0);
    let a_part = Grid::from_fn(100, 300, |r, c| a[(r, c)]);
    let b_part = Grid::from_fn(40, 200, |r, c| a[(r, c)]);
    (a_part, b_part)
}

/// `imshow(data, vmin=-1, vmax=1)`.
fn show_unit(ax: &Axes, data: Grid<f64>) -> CaseResult {
    let unit = ColorizerOptions::default().limits(Some(-1.0), Some(1.0));
    ax.imshow(data, ImshowOptions::default().colorizer(unit))?;
    Ok(())
}

/// `_annotate_rect(ax)`: B's outline in red.
fn annotate_rect(ax: &Axes) {
    let props = PatchProps::default()
        .linewidth(1.0)
        .edgecolor("r")
        .facecolor("none");
    ax.add_patch_with(Rectangle::new((0.0, 0.0), 200.0, 40.0), props);
}

/// The figure background of `image_exact_placement`.
fn alice_blue(fig: &Figure) -> CaseResult {
    fig.set_facecolor(to_rgba("aliceblue", None)?);
    Ok(())
}

fn image_exact_placement_1(fig: &Figure) -> CaseResult {
    let (a, b) = exact_placement_images();
    alice_blue(fig)?;
    let axs = fig.subplots(1, 2)?;
    show_unit(&axs[(0, 0)], a)?;
    show_unit(&axs[(0, 1)], b)?;
    annotate_rect(&axs[(0, 0)]);
    Ok(())
}

/// `_exact_placement_ratios`: A and B side by side with columns as wide as
/// the images, anchored at `anchor` if given.
fn exact_placement_ratios(fig: &Figure, anchor: Option<Anchor>) -> CaseResult {
    let (a, b) = exact_placement_images();
    alice_blue(fig)?;
    let grid = GridSpec::with_ratios(
        1,
        2,
        Some(vec![300.0 / 200.0, 1.0]),
        None,
        SubplotOverrides::default(),
    )?;
    let axs = fig.subplots_on(&grid, Share::None, Share::None)?;
    let (left, right) = (&axs[(0, 0)], &axs[(0, 1)]);
    show_unit(left, a)?;
    annotate_rect(left);
    if let Some(anchor) = anchor {
        left.set_anchor(anchor);
    }
    show_unit(right, b)?;
    if let Some(anchor) = anchor {
        right.set_anchor(anchor);
    }
    Ok(())
}

fn image_exact_placement_2(fig: &Figure) -> CaseResult {
    exact_placement_ratios(fig, None)
}

fn image_exact_placement_3(fig: &Figure) -> CaseResult {
    exact_placement_ratios(fig, Some(Anchor::NW))
}

fn image_exact_placement_4(fig: &Figure) -> CaseResult {
    let (a, b) = exact_placement_images();
    alice_blue(fig)?;
    let (width, height) = (605.0, 170.0);
    let ax = fig.add_axes([35.0 / width, 35.0 / height, 300.0 / width, 100.0 / height]);
    show_unit(&ax, a)?;
    annotate_rect(&ax);
    let ax = fig.add_axes([370.0 / width, 95.0 / height, 200.0 / width, 40.0 / height]);
    show_unit(&ax, b)?;
    Ok(())
}

fn image_masked(fig: &Figure) -> CaseResult {
    let (x0, x1, y0, y1) = (-5.0, 5.0, -3.0, 3.0);
    let (_, _, z) = mesh_field(&linspace(x0, x1, 500), &linspace(y0, y1, 500), two_bumps);
    let mask = z.map(|&v| v > 1.2);
    let palette = colormaps::get("gray")?
        .with_over("r")?
        .with_under("g")?
        .with_bad("b")?;
    let axs = fig.subplots(2, 1)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    let image = |colorizer: ColorizerOptions, interpolation| {
        ImshowOptions {
            aspect: Some(Aspect::Auto),
            origin: Some(Origin::Lower),
            extent: Some(Extent::new(x0, x1, y0, y1)),
            ..ImshowOptions::default()
        }
        .colorizer(colorizer)
        .interpolation(interpolation)
    };
    let uniform = ColorizerOptions::default()
        .cmap(palette.clone())
        .norm(Normalize::new(-1.0, 1.0));
    let im = ax1.imshow(
        ScalarField::masked(z.clone(), &mask)?,
        image(uniform, Interpolation::Bilinear),
    )?;
    ax1.set_title("Green=low, Red=high, Blue=masked");
    fig.colorbar(
        &im,
        ColorbarOptions {
            extend: Some(Extend::Both),
            shrink: 0.9,
            ..ColorbarOptions::beside(ax1)
        },
    )?
    .set_label("uniform", None);
    ax1.tick_params(&TickParams {
        axis: Some(AxisName::X),
        labelbottom: Some(false),
        ..TickParams::default()
    });
    let bins = BoundaryNorm::new(
        [-1.0, -0.5, -0.2, 0.0, 0.2, 0.5, 1.0],
        palette.n(),
        Extend::Neither,
    )?;
    let binned = ColorizerOptions::default().cmap(palette).norm(bins);
    let im = ax2.imshow(
        ScalarField::masked(z, &mask)?,
        image(binned, Interpolation::Nearest),
    )?;
    ax2.set_title("With BoundaryNorm");
    fig.colorbar(
        &im,
        ColorbarOptions {
            extend: Some(Extend::Both),
            spacing: Spacing::Proportional,
            shrink: 0.9,
            ..ColorbarOptions::beside(ax2)
        },
    )?
    .set_label("proportional", None);
    fig.suptitle("imshow, with out-of-range and masked data");
    Ok(())
}

fn image_nonuniform(fig: &Figure) -> CaseResult {
    let x = linspace(-4.0, 4.0, 9);
    let x2: Vec<f64> = x.iter().map(|v| v.powf(3.0)).collect();
    let y = linspace(-4.0, 4.0, 9);
    let z = Grid::from_fn(9, 9, |r, c| (x[c] * x[c] + y[r] * y[r]).sqrt());
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let axs = fig.subplots(2, 2)?;
    fig.suptitle("NonUniformImage class")
        .set_fontsize(FontSize::Relative(RelativeSize::Large));
    for (row, (interpolation, name)) in [
        (NonUniformInterpolation::Nearest, "nearest"),
        (NonUniformInterpolation::Bilinear, "bilinear"),
    ]
    .into_iter()
    .enumerate()
    {
        for (col, (xs, half_width)) in [(&x, 4.0), (&x2, 64.0)].into_iter().enumerate() {
            let ax = &axs[(row, col)];
            let options = NonUniformOptions {
                colorizer: ColorizerOptions::default().cmap(colormaps::get("Purples")?),
                interpolation,
            };
            ax.nonuniform_image(xs, &y, z.clone(), options)?;
            ax.set_xlim(-half_width, half_width);
            ax.set_ylim(-4.0, 4.0);
            ax.set_title(name);
        }
    }
    Ok(())
}

/// `image_transparency_blend`'s difference of two 2-D Gaussian blobs.
fn blend_weights() -> Grid<f64> {
    let normal_pdf = |x: f64, mean: f64, var: f64| (-(x - mean) * (x - mean) / (2.0 * var)).exp();
    let xx = linspace(0.0, 100.0, 100);
    let high = |r: usize, c: usize| normal_pdf(xx[r], 50.0, 150.0) * normal_pdf(xx[c], 20.0, 150.0);
    let low = |r: usize, c: usize| normal_pdf(xx[r], 60.0, 200.0) * normal_pdf(xx[c], 50.0, 200.0);
    Grid::from_fn(100, 100, |r, c| high(r, c) - low(r, c))
}

/// `_blend`: a grey background, then the weights with per-pixel `alphas`
/// (or none); returns the axes and the weights.
fn blend(
    fig: &Figure,
    alphas: impl Fn(&Grid<f64>) -> Option<Grid<f64>>,
) -> Result<(Axes, Grid<f64>), Box<dyn std::error::Error>> {
    let weights = blend_weights();
    let greys = Grid::filled(100, 100, [70_u8; 3]);
    let vmax = weights
        .as_slice()
        .iter()
        .fold(0.0, |m: f64, v| m.max(v.abs()));
    let ax = fig.subplot();
    ax.imshow(greys, ImshowOptions::default())?;
    let options = ImshowOptions {
        colorizer: ColorizerOptions::default()
            .cmap(colormaps::get("RdYlBu")?)
            .limits(Some(-vmax), Some(vmax)),
        alpha: alphas(&weights).map(ImageAlpha::Each),
        extent: Some(Extent::new(0.0, 100.0, 0.0, 100.0)),
        ..ImshowOptions::default()
    };
    ax.imshow(weights.clone(), options)?;
    Ok((ax, weights))
}

fn image_transparency_blend_1(fig: &Figure) -> CaseResult {
    let (ax, _) = blend(fig, |_| None)?;
    ax.set_axis_off();
    Ok(())
}

fn image_transparency_blend_2(fig: &Figure) -> CaseResult {
    let fading = |weights: &Grid<f64>| {
        let ramp = linspace(1.0, 0.0, 70);
        Some(Grid::from_fn(weights.rows(), weights.cols(), |_, c| {
            if c < 30 { 1.0 } else { ramp[c - 30] }
        }))
    };
    let (ax, _) = blend(fig, fading)?;
    ax.set_axis_off();
    Ok(())
}

fn image_transparency_blend_3(fig: &Figure) -> CaseResult {
    // `Normalize(0, .3, clip=True)(|w|)` clipped below at 0.4.
    let by_magnitude =
        |weights: &Grid<f64>| Some(weights.map(|w| (w.abs().min(0.3) / 0.3).clamp(0.4, 1.0)));
    let (ax, weights) = blend(fig, by_magnitude)?;
    let flipped = Grid::from_fn(100, 100, |r, c| weights[(99 - r, c)]);
    let options = ContourOptions {
        levels: Levels::Values(vec![-0.1, 0.1]),
        colors: Some(vec![Color::from("k")]),
        linestyles: Some(vec![LineStyle::Solid]),
        ..ContourOptions::default()
    };
    ax.contour(&QuadField::new(flipped), options)?;
    ax.set_axis_off();
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_images/image_exact_placement_1",
        image_exact_placement_1,
    ),
    FigureCase::new(
        "gallery_images/image_exact_placement_2",
        image_exact_placement_2,
    )
    .sized(6.4, 2.0, 100.0),
    FigureCase::new(
        "gallery_images/image_exact_placement_3",
        image_exact_placement_3,
    )
    .sized(6.4, 2.0, 100.0),
    FigureCase::new(
        "gallery_images/image_exact_placement_4",
        image_exact_placement_4,
    )
    .sized(6.05, 1.7, 100.0),
    FigureCase::new("gallery_images/image_masked", image_masked).sized(6.0, 5.4, 100.0),
    FigureCase::new("gallery_images/image_nonuniform", image_nonuniform),
    FigureCase::new(
        "gallery_images/image_transparency_blend_1",
        image_transparency_blend_1,
    ),
    FigureCase::new(
        "gallery_images/image_transparency_blend_2",
        image_transparency_blend_2,
    ),
    FigureCase::new(
        "gallery_images/image_transparency_blend_3",
        image_transparency_blend_3,
    ),
];
