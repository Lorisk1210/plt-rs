//! Pixel alignment: nearest-neighbour pixels against their grid lines,
//! images anchored on whole and half display pixels, the bounds of
//! rotated images, clean RGBA edges over backgrounds, and mirrored
//! images filling their extent — through affine transforms and through
//! the identity disguised as a non-affine transform.

use plt::artist::ArtistHandle;
use plt::axes::{Axes, ImshowOptions};
use plt::collections::Collection;
use plt::colors::{Color, ColormapError};
use plt::figure::Figure;
use plt::image::{ImageData, Interpolation, InterpolationStage, Origin, ScalarField};
use plt::lines::LineStyle;
use plt::numeric::{Grid, arange};
use plt::patches::Rectangle;
use plt::transforms::{Affine2D, Point, PointMap, Transform};

use super::{classic, colormapped, uniform};
use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    classic("mpl_image/nn_pixel_alignment", nn_pixel_alignment),
    FigureCase::new(
        "mpl_image/alignment_half_display_pixels",
        alignment_half_display_pixels,
    )
    .sized(5.0, 2.0, 100.0),
    classic("mpl_image/image_bounds_handling", image_bounds_handling),
    classic("mpl_image/rgba_clean_edges", rgba_clean_edges),
    classic("mpl_image/affine_fill_to_edges", affine_fill_to_edges),
];

/// The `nonaffine_identity` fixture: the identity, as a transform plt
/// must treat as non-affine.
#[derive(Debug)]
struct NonAffineIdentity;

impl PointMap for NonAffineIdentity {
    fn forward(&self, point: Point) -> Point {
        point
    }

    fn inverse(&self, point: Point) -> Point {
        point
    }
}

/// `nonaffine_identity + transform`.
fn non_affine(transform: Transform) -> Transform {
    Transform::custom(NonAffineIdentity).then(transform)
}

/// `np.arange(n * n).reshape(n, n) % modulus` as integers.
fn modular(n: usize, modulus: usize) -> ImageData {
    let values = Grid::from_fn(n, n, |r, c| ((r * n + c) % modulus) as i64);
    ScalarField::integers(&values).into()
}

/// `imshow` options with a colormap, an interpolation and a transform.
fn shown(
    cmap: &str,
    interpolation: Interpolation,
    transform: Transform,
) -> Result<ImshowOptions, ColormapError> {
    Ok(ImshowOptions {
        interpolation: Some(interpolation),
        transform: Some(transform),
        ..colormapped(cmap)?
    })
}

/// Styles the segments of `vlines` or `hlines`: width 0.5, red, `style`.
fn red(lines: &Collection, style: Option<LineStyle>) {
    lines
        .set_linewidths(vec![0.5])
        .set_edgecolor(Color::from("red"));
    if let Some(style) = style {
        lines.set_linestyles(vec![style]);
    }
}

/// Red dashed lines at `seps` across `low..high`, both ways.
fn grid_lines(ax: &Axes, seps: &[f64], low: f64, high: f64, style: &LineStyle) -> CaseResult {
    red(&ax.vlines(seps, [low], [high])?, Some(style.clone()));
    red(&ax.hlines(seps, [low], [high])?, Some(style.clone()));
    Ok(())
}

/// `test_nn_pixel_alignment`: nearest-neighbour images of 3, 7 and 11
/// pixels under their pixel edges, through affine (top) and non-affine
/// (bottom) transforms.
fn nn_pixel_alignment(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 3)?;
    for (j, n) in [3, 7, 11].into_iter().enumerate() {
        let seps = arange(-0.5, n as f64, 1.0);
        for i in 0..2 {
            let ax = &axs[(i, j)];
            let transform = if i == 1 {
                non_affine(ax.trans_data())
            } else {
                ax.trans_data()
            };
            ax.imshow(
                modular(n, 4),
                shown("Grays", Interpolation::Nearest, transform)?,
            )?;
            ax.set_axis_off();
            grid_lines(ax, &seps, -1.0, n as f64, &LineStyle::Dashed)?;
        }
    }
    Ok(())
}

/// `test_alignment_half_display_pixels`: images filling axes whose
/// corners lie on whole or half display pixels, in both origins, through
/// affine and non-affine transforms, nearest and Hanning-filtered.
fn alignment_half_display_pixels(fig: &Figure) -> CaseResult {
    fig.set_facecolor(Color::from("g").resolve()?);
    let corner_x = [0.01, 0.199, 0.41, 0.599, 0.809];
    let mut axs = Vec::new();
    for cy in [0.05, 0.53] {
        for (ix, cx) in corner_x.into_iter().enumerate() {
            let my = if [1, 3, 4].contains(&ix) {
                cy + 0.0125
            } else {
                cy
            };
            let ax = fig.add_axes([cx, my, 0.17, 0.425]);
            ax.set_xticks(Vec::<f64>::new());
            ax.set_yticks(Vec::<f64>::new());
            axs.push(ax);
        }
    }
    let n = 10;
    let seps = arange(-0.5, n as f64, 1.0);
    let dashes = LineStyle::dashes(0.0, vec![2.0, 4.0]);
    for (i, ax) in axs.iter().enumerate() {
        ax.set_facecolor("m");
        let transform = if i % 4 >= 2 {
            non_affine(ax.trans_data())
        } else {
            ax.trans_data()
        };
        let interpolation = if i % 5 == 4 {
            Interpolation::Hanning
        } else {
            Interpolation::Nearest
        };
        ax.imshow(
            modular(n, 9),
            ImshowOptions {
                origin: Some(if i >= 5 { Origin::Upper } else { Origin::Lower }),
                ..shown("Blues", interpolation, transform)?
            },
        )?;
        grid_lines(ax, &seps, -0.5, n as f64 - 0.5, &dashes)?;
        for spine in ax.spines().iter() {
            spine.set_linestyle(LineStyle::dashes(0.0, vec![5.0, 10.0]));
        }
    }
    Ok(())
}

/// `test_image_bounds_handling`: images rotated by one radian about their
/// centre inside a dashed outline of their bounds, nearest, Hanning and
/// bilinear, through affine (top) and non-affine (bottom) transforms.
fn image_bounds_handling(fig: &Figure) -> CaseResult {
    const N: usize = 11;
    let axs = fig.subplots(2, 3)?;
    let interpolations = [
        Interpolation::Nearest,
        Interpolation::Hanning,
        Interpolation::Bilinear,
    ];
    let count = (N * N) as f64;
    let data = Grid::from_fn(N, N, |r, c| {
        let k = r * N + c;
        k as f64 / count + (k % 4) as f64 / 6.0
    });
    let centre = N as f64 / 2.0 - 0.5;
    let rotation = Affine2D::identity().rotate_around(centre, centre, 1.0);
    for (j, interpolation) in interpolations.into_iter().enumerate() {
        for i in 0..2 {
            let ax = &axs[(i, j)];
            let rotated = Transform::Affine(rotation).then(ax.trans_data());
            let transform = if i == 1 {
                non_affine(rotated.clone())
            } else {
                rotated.clone()
            };
            ax.imshow(data.clone(), shown("Grays", interpolation, transform)?)?;
            ax.set_axis_off();
            let side = N as f64;
            let outline = ax.add_artist(Rectangle::new((-0.5, -0.5), side, side));
            outline
                .set_edgecolor("red")
                .set_facecolor("none")
                .set_linewidth(0.5)
                .set_linestyle(LineStyle::Dashed);
            outline.set_transform(rotated);
        }
    }
    Ok(())
}

/// `test_rgba_clean_edges`: a rotated, bilinearly interpolated RGBA-stage
/// image (the second with a NaN block) fading cleanly into black, grey
/// and half-transparent black backgrounds.
fn rgba_clean_edges(fig: &Figure) -> CaseResult {
    let data = uniform(19_680_801 + 9, 8, 8);
    let mut holed = data.clone();
    for r in 2..4 {
        for c in 2..4 {
            holed[(r, c)] = f64::NAN;
        }
    }
    let rotation = Affine2D::identity().rotate_around(3.5, 3.5, 1.0);
    let axs = fig.subplots(1, 2)?;
    for (ax, image) in axs.as_slice().iter().zip([data, holed]) {
        ax.add_patch(Rectangle::new((3.75, 2.0), 5.0, 5.0))
            .set_color("black")
            .set_zorder(0.0);
        ax.add_patch(Rectangle::new((0.0, -2.0), 3.75, 4.0))
            .set_color("gray")
            .set_zorder(0.0);
        ax.add_patch(Rectangle::new((3.75, -2.0), 5.0, 4.0))
            .set_facecolor("black")
            .set_edgecolor("none")
            .set_alpha(Some(0.5))
            .set_zorder(0.0);
        ax.imshow(
            image,
            ImshowOptions {
                interpolation: Some(Interpolation::Bilinear),
                interpolation_stage: Some(InterpolationStage::Rgba),
                transform: Some(Transform::Affine(rotation).then(ax.trans_data())),
                ..ImshowOptions::default()
            },
        )?;
        ax.set_axis_off();
        ax.set_xlim(-2.5, 9.5).set_ylim(-2.5, 9.5);
    }
    Ok(())
}

/// `test_affine_fill_to_edges`: Hanning-filtered images, as they are and
/// mirrored either way, filling exactly the corners marked in red, in
/// both origins.
fn affine_fill_to_edges(fig: &Figure) -> CaseResult {
    const N: usize = 7;
    let n = N as f64;
    let mirrors = [
        Affine2D::identity(),
        Affine2D::identity()
            .translate(0.0, -n + 1.0)
            .scale(1.0, -1.0),
        Affine2D::identity()
            .translate(-n + 1.0, 0.0)
            .scale(-1.0, 1.0),
    ];
    let axs = fig.subplots(2, 3)?;
    for (j, mirror) in mirrors.into_iter().enumerate() {
        for (i, origin) in [Origin::Upper, Origin::Lower].into_iter().enumerate() {
            let ax = &axs[(i, j)];
            let transform = Transform::Affine(mirror).then(ax.trans_data());
            ax.imshow(
                modular(N, 3),
                ImshowOptions {
                    origin: Some(origin),
                    ..shown("Grays", Interpolation::Hanning, transform)?
                },
            )?;
            ax.set_axis_off();
            let edges = [-0.5, n - 0.5];
            for (low, high) in [(-1.0, 2.0), (n - 3.0, n)] {
                red(&ax.vlines(edges, [low], [high])?, None);
            }
            for (low, high) in [(-1.0, 2.0), (n - 3.0, n)] {
                red(&ax.hlines(edges, [low], [high])?, None);
            }
        }
    }
    Ok(())
}
