//! Images placed through non-linear scales and affine transforms, every
//! origin and extent combination, and clipping.

use plt::axes::ImshowOptions;
use plt::colors::colormaps;
use plt::figure::Figure;
use plt::gridspec::SubplotOverrides;
use plt::image::{ColorizerOptions, Extent, Interpolation, Origin};
use plt::numeric::{Grid, arange};
use plt::patches::{Rectangle, RotationPoint};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::transforms::Transform;
use plt_core::Affine2D;

use super::data::bump;
use crate::figures::CaseResult;

pub(super) fn log_axes_resampled(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let stripes = Grid::from_fn(10, 10, |r, _| if r % 2 == 0 { 1.0 } else { 0.0 });
    axs[(0, 0)].imshow(
        stripes,
        ImshowOptions {
            colors: ColorizerOptions::default()
                .cmap(colormaps::get("viridis")?)
                .limits(Some(-1.0), Some(1.0)),
            extent: Some(Extent::new(1.0, 100.0, 1.0, 100.0)),
            aspect: Some(Aspect::Auto),
            ..ImshowOptions::default()
        },
    )?;
    axs[(0, 0)].set_yscale("log")?;
    axs[(0, 1)].imshow(
        bump(30),
        ImshowOptions {
            extent: Some(Extent::new(1.0, 1000.0, 0.0, 1.0)),
            aspect: Some(Aspect::Auto),
            interpolation: Some(Interpolation::Bilinear),
            ..ImshowOptions::default()
        },
    )?;
    axs[(0, 1)].set_xscale("log")?;
    Ok(())
}

/// `_affine_image()`: two Gaussian bumps of opposite sign.
fn affine_image() -> Grid<f64> {
    let x = arange(-3.0, 3.0, 0.25);
    Grid::from_fn(x.len(), x.len(), |r, c| {
        let (x, y) = (x[c], x[r]);
        (-x * x - y * y).exp() - (-(x - 1.0).powi(2) - (y - 1.0).powi(2)).exp()
    })
}

/// `_affine_plot(ax, transform)`: the image drawn through `transform`
/// with its intended outline.
fn affine_plot(ax: &Axes, transform: Affine2D) -> CaseResult {
    let image = ax.imshow(
        affine_image(),
        ImshowOptions {
            interpolation: Some(Interpolation::None),
            origin: Some(Origin::Lower),
            extent: Some(Extent::new(-2.0, 4.0, -3.0, 2.0)),
            ..ImshowOptions::default()
        },
    )?;
    let trans_data = Transform::Affine(transform).then(ax.trans_data());
    image.set_transform(trans_data.clone());
    let Extent {
        left: x1,
        right: x2,
        bottom: y1,
        top: y2,
    } = image.extent();
    ax.plot_fmt([x1, x2, x2, x1, x1], [y1, y1, y2, y2, y1], "y--")?
        .set_transform(trans_data);
    ax.set_xlim(-5.0, 5.0).set_ylim(-4.0, 4.0);
    Ok(())
}

pub(super) fn affine_transforms(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let identity = Affine2D::IDENTITY;
    affine_plot(&axs[(0, 0)], identity.rotate_deg(30.0))?;
    affine_plot(&axs[(0, 1)], identity.skew_deg(30.0, 15.0))?;
    affine_plot(&axs[(1, 0)], identity.scale(-1.0, 0.5))?;
    affine_plot(
        &axs[(1, 1)],
        identity
            .rotate_deg(30.0)
            .skew_deg(30.0, 15.0)
            .scale(-1.0, 0.5)
            .translate(0.5, -1.0),
    )?;
    Ok(())
}

pub(super) fn origin_extent_grid(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 3)?;
    let data = Grid::from_fn(3, 4, |r, c| (r * 4 + c) as f64);
    let extents = [
        (None, "None"),
        (Some(Extent::new(0.0, 10.0, 0.0, 6.0)), "(0, 10, 0, 6)"),
        (Some(Extent::new(10.0, 0.0, 6.0, 0.0)), "(10, 0, 6, 0)"),
    ];
    for (row, (origin, name)) in [(Origin::Upper, "upper"), (Origin::Lower, "lower")]
        .into_iter()
        .enumerate()
    {
        for (col, (extent, label)) in extents.iter().enumerate() {
            let ax = &axs[(row, col)];
            ax.imshow(
                data.clone(),
                ImshowOptions {
                    origin: Some(origin),
                    extent: *extent,
                    ..ImshowOptions::default()
                },
            )?;
            ax.set_title(format!("{name}, {label}")).set_fontsize(8.0);
        }
    }
    Ok(())
}

pub(super) fn clip_off_beyond_axes(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.imshow(
        bump(20),
        ImshowOptions {
            extent: Some(Extent::new(-1.0, 21.0, -1.0, 21.0)),
            origin: Some(Origin::Lower),
            ..ImshowOptions::default()
        },
    )?
    .set_clip_on(false);
    ax.set_xlim(2.0, 18.0).set_ylim(2.0, 18.0);
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.2),
        right: Some(0.8),
        bottom: Some(0.2),
        top: Some(0.8),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

pub(super) fn clip_path_rotated_rectangle(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let image = ax.imshow(
        bump(40),
        ImshowOptions {
            colors: ColorizerOptions::default().cmap(colormaps::get("plasma")?),
            ..ImshowOptions::default()
        },
    )?;
    let rect = Rectangle::new((8.0, 8.0), 24.0, 12.0).rotated(30.0, RotationPoint::Xy);
    image.set_clip_patch(&rect, ax.trans_data());
    Ok(())
}
