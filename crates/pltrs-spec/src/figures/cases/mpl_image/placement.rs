//! Placement and clipping: clip paths and rectangles, extents, origins,
//! compositing over axes backgrounds, rasterized artists at a low dpi,
//! images stretched over boxes, zoomed views, log axes, rotated images
//! and images lined up with a frame drawn around them.

use pltrs::artist::ArtistHandle;
use pltrs::axes::ImshowOptions;
use pltrs::colors::{Colorize, ColorizerOptions, LogNorm, colormaps};
use pltrs::figure::Figure;
use pltrs::image::{
    BboxImageOptions, Extent, ImageAlpha, ImageData, Interpolation, Origin, ScalarField,
};
use pltrs::lines::LineProps;
use pltrs::numeric::{Grid, arange};
use pltrs::patches::{Circle, Rectangle};
use pltrs::rc::Aspect;
use pltrs::transforms::{Affine2D, Bbox, Point, Transform};
use pltrs_core::Path;

use super::{classic, counting, gaussians, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_image/image_clip", |fig| {
        let ax = fig.subplot();
        let image = ax.imshow(two_by_two(), ImshowOptions::default())?;
        image.set_clip_patch(&Circle::new((0.0, 0.0), 1.0), ax.trans_data());
        Ok(())
    }),
    FigureCase::new("mpl_image/image_cliprect", |fig| {
        let ax = fig.subplot();
        let image = ax.imshow(two_by_two(), extent(0.0, 5.0, 0.0, 5.0))?;
        image.set_clip_patch(&Rectangle::new((1.0, 1.0), 2.0, 2.0), ax.trans_data());
        Ok(())
    }),
    classic("mpl_image/imshow_10_10_1", |fig| {
        let ax = fig.subplot();
        ax.imshow(
            counting(10, 10),
            ImshowOptions {
                extent: Some(Extent::new(1.0, 2.0, 1.0, 2.0)),
                interpolation: Some(Interpolation::Bilinear),
                ..ImshowOptions::default()
            },
        )?;
        ax.set_xlim(0.0, 3.0)?.set_ylim(0.0, 3.0)?;
        Ok(())
    }),
    classic("mpl_image/no_interpolation_origin", |fig| {
        let axs = fig.subplots(2, 1)?;
        axs[(0, 0)].imshow(
            counting(2, 50),
            ImshowOptions {
                origin: Some(Origin::Lower),
                interpolation: Some(Interpolation::None),
                ..ImshowOptions::default()
            },
        )?;
        axs[(1, 0)].imshow(
            counting(2, 50),
            ImshowOptions {
                interpolation: Some(Interpolation::None),
                ..ImshowOptions::default()
            },
        )?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_image/image_shift", |fig| {
        let data = Grid::from_fn(99, 99, |r, c| 1.0 / (c + 1) as f64 + 1.0 / (r + 1) as f64);
        let ax = fig.subplot();
        ax.imshow(
            data,
            ImshowOptions {
                colorizer: ColorizerOptions::default().norm(LogNorm::default()),
                extent: Some(Extent::new(734_717.945_208, 734_717.946_366, 1.0, 100.0)),
                interpolation: Some(Interpolation::None),
                ..ImshowOptions::default()
            },
        )?;
        ax.set_aspect(Aspect::Auto)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_image/image_composite_background", |fig| {
        let ax = fig.subplot();
        for left in [0.0, 4.0] {
            ax.imshow(counting(4, 3), extent(left, left + 2.0, 15.0, 0.0))?;
        }
        ax.set_facecolor((1.0, 0.0, 0.0, 0.5));
        ax.set_xlim(0.0, 12.0)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_image/image_composite_alpha", image_composite_alpha),
    classic("mpl_image/clip_path_disables_compositing", |fig| {
        let ax = fig.subplot();
        for corner in [(0.0, 0.0), (1.0, 1.0)] {
            let (x, y) = corner;
            let triangle = Path::polyline(vec![
                Point::new(x, y),
                Point::new(x, y + 1.0),
                Point::new(x + 1.0, y),
            ]);
            ax.imshow(
                counting(3, 3),
                ImshowOptions {
                    clip_path: Some((triangle, ax.trans_data())),
                    ..ImshowOptions::default()
                },
            )?;
        }
        Ok(())
    }),
    FigureCase::new("mpl_image/rasterize_dpi", rasterize_dpi).sized(3.0, 1.0, 10.0),
    FigureCase::new("mpl_image/bbox_image_inverted", bbox_image_inverted),
    FigureCase::new("mpl_image/zoom_and_clip_upper_origin", |fig| {
        let ax = fig.subplot();
        ax.imshow(counting(10, 10), ImshowOptions::default())?;
        ax.set_ylim(2.0, -0.5)?.set_xlim(-0.5, 2.0)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_image/log_scale_image", |fig| {
        let z = Grid::from_fn(10, 10, |r, _| f64::from(u8::from(r % 2 == 0)));
        let ax = fig.subplot();
        ax.imshow(
            z,
            ImshowOptions {
                colorizer: ColorizerOptions::default()
                    .cmap(colormaps::get("viridis")?)
                    .limits(Some(-1.0), Some(1.0)),
                aspect: Some(Aspect::Auto),
                extent: Some(Extent::new(1.0, 100.0, 1.0, 100.0)),
                ..ImshowOptions::default()
            },
        )?;
        ax.set_yscale("log")?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_image/rotate_image", rotate_image),
    FigureCase::new("mpl_image/image_placement", |fig| {
        let ax = fig.subplot();
        ax.plot_with(
            [0.0, 0.0, 1.0, 1.0, 0.0],
            [0.0, 1.0, 1.0, 0.0, 0.0],
            LineProps::default().color("r").linewidth(0.1),
        )?;
        let noise = SpecRng::new(19_680_801).normal(256, 0.0, 1.0);
        ax.imshow(
            Grid::from_vec(16, 16, noise)?,
            ImshowOptions {
                colorizer: ColorizerOptions::default()
                    .cmap(colormaps::get("Blues")?)
                    .limits(Some(-1.0), Some(1.0)),
                extent: Some(Extent::new(0.0, 1.0, 0.0, 1.0)),
                interpolation: Some(Interpolation::None),
                ..ImshowOptions::default()
            },
        )?;
        ax.set_xlim(-0.1, 1.0 + 0.1)?.set_ylim(-0.1, 1.0 + 0.1)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
];

/// The integer image `[[1, 2], [3, 4]]`.
fn two_by_two() -> ImageData {
    let values = Grid::from_fn(2, 2, |r, c| (2 * r + c + 1) as i64);
    ScalarField::integers(&values).into()
}

/// `imshow` options with only the extent set.
fn extent(left: f64, right: f64, bottom: f64, top: f64) -> ImshowOptions {
    ImshowOptions {
        extent: Some(Extent::new(left, right, bottom, top)),
        ..ImshowOptions::default()
    }
}

/// `test_image_composite_alpha`: red and yellow alpha ramps at three
/// opacities over a green background.
fn image_composite_alpha(fig: &Figure) -> CaseResult {
    let mut ramp = arange(0.0, 1.1, 0.1);
    ramp.extend(arange(0.0, 1.0, 0.1).into_iter().rev());
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
    let ax = fig.subplot();
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
    ax.set_xlim(0.0, 5.0)?.set_ylim(5.0, 0.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_rasterize_dpi`: an image, a rasterized line and a vector line
/// without ticks or spines.
fn rasterize_dpi(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    axs[(0, 0)].imshow(two_by_two(), ImshowOptions::default())?;
    for (ax, rasterized) in [(&axs[(0, 1)], true), (&axs[(0, 2)], false)] {
        let line = ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().linewidth(20.0))?;
        line.set_rasterized(rasterized);
        ax.set_xlim(0.0, 1.0)?.set_ylim(-1.0, 2.0)?;
    }
    for ax in axs.as_slice() {
        ax.set_xticks(Vec::<f64>::new());
        ax.set_yticks(Vec::<f64>::new());
        for spine in ax.spines().iter() {
            spine.set_visible(false);
        }
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_bbox_image_inverted`: an image over an inverted box in data
/// coordinates and the identity over a box in figure coordinates.
fn bbox_image_inverted(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let nearest = || BboxImageOptions {
        interpolation: Some(Interpolation::Nearest),
        ..BboxImageOptions::default()
    };
    let image = ax.add_bbox_image(
        counting(10, 10),
        Bbox::from_extents(100.0, 100.0, 0.0, 0.0),
        ax.trans_data(),
        nearest(),
    )?;
    image.set_clip_on(false);
    ax.set_xlim(0.0, 100.0)?.set_ylim(0.0, 100.0)?;
    let identity = Grid::from_fn(10, 10, |r, c| f64::from(u8::from(r == c)));
    let image = ax.add_bbox_image(
        identity,
        Bbox::from_extents(0.1, 0.2, 0.3, 0.25),
        Transform::Figure(fig.id()),
        nearest(),
    )?;
    image.set_clip_on(false);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_rotate_image`: an image rotated by 30° about the data origin,
/// framed by a dashed outline of its extent.
fn rotate_image(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let image = ax.imshow(
        gaussians(0.25, 1.0),
        ImshowOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("viridis")?),
            origin: Some(Origin::Lower),
            extent: Some(Extent::new(-2.0, 4.0, -3.0, 2.0)),
            interpolation: Some(Interpolation::None),
            ..ImshowOptions::default()
        },
    )?;
    let rotated = Transform::Affine(Affine2D::identity().rotate_deg(30.0)).then(ax.trans_data());
    image.set_transform(rotated.clone());
    let Extent {
        left,
        right,
        bottom,
        top,
    } = image.extent();
    let outline = ax.plot_with(
        [left, right, right, left, left],
        [bottom, bottom, top, top, bottom],
        LineProps::fmt("r--")?.linewidth(3.0),
    )?;
    outline.set_transform(rotated);
    ax.set_xlim(2.0, 5.0)?.set_ylim(0.0, 4.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}
