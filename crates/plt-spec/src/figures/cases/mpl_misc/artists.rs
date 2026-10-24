//! The cases from `test_agg.py`, `test_agg_filter.py` and `test_skew.py`.

use plt::artist::{AggFilter, FilterImage, Filtered};
use plt::axes::{Adjustable, Mesh, PcolorOptions};
use plt::colors::{ColorizerOptions, colormaps};
use plt::figure::Share;
use plt::gridspec::SubplotOverrides;
use plt::numeric::Grid;
use plt::patches::Rectangle;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::transforms::Transform;
use plt_core::Affine2D;

use super::{classic, remove_text};
use crate::figures::cases::common::smooth2d;
use crate::figures::{CaseResult, FigureCase};

/// `test_agg_filter`'s `DropShadowFilter(4)`: the artist's opacity at
/// 30 %, blurred over 4 points, in black.
fn drop_shadow(image: &FilterImage, dpi: f64) -> Filtered {
    let sigma = 4.0;
    let pad = (sigma * 3.0 / 72.0 * dpi) as usize;
    let (width, height) = (image.width() + 2 * pad, image.height() + 2 * pad);
    let alpha: Vec<f64> = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let source = x.checked_sub(pad).zip(y.checked_sub(pad));
                source
                    .and_then(|(x, y)| image.pixel(x, y))
                    .map_or(0.0, |pixel| pixel[3] * 0.3)
            })
        })
        .collect();
    let shadow = smooth2d(&alpha, width, sigma / 72.0 * dpi)
        .into_iter()
        .map(|a| [0.0, 0.0, 0.0, a])
        .collect();
    let offset = -(pad as f64);
    Filtered {
        image: FilterImage::new(width, height, shadow).expect("one pixel per position"),
        offset: (offset, offset),
    }
}

/// `test_agg_filter`: two lines with blurred shadows offset by
/// (4, -6) points.
fn agg_filter(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = [0.1, 0.5, 0.9];
    let shadow = AggFilter::new(drop_shadow);
    for (y, fmt, edge) in [([0.1, 0.9, 0.5], "bo-", "b"), ([0.5, 0.2, 0.7], "ro-", "r")] {
        let props = LineProps::fmt(fmt)?
            .markeredgecolor(edge)
            .markerfacecolor("w")
            .linewidth(5.0)
            .markeredgewidth(3.0)
            .markersize(10.0)
            .label("Line 1");
        let line = ax.plot_with(x, y, props.clone())?;
        ax.plot_with(x, y, props)?
            .set_transform(ax.trans_data().offset_points(4.0, -6.0))
            .set_zorder(line.zorder() - 0.5)
            .set_agg_filter(Some(shadow.clone()))
            .set_rasterized(true);
    }
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    ax.xaxis().set_visible(false);
    ax.yaxis().set_visible(false);
    remove_text(fig);
    Ok(())
}

/// `test_agg_filter_alpha`: a mesh made translucent by a filter.
fn agg_filter_alpha(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    let data = Grid::from_fn(7, 8, |i, j| (i * i) as f64 - (j * j) as f64);
    let mesh = ax.pcolormesh(
        Mesh::new(data),
        PcolorOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("Reds")?),
            ..PcolorOptions::default()
        },
    )?;
    let manual_alpha = AggFilter::new(|image, _dpi| {
        let mut faded = image.clone();
        faded
            .pixels_mut()
            .iter_mut()
            .for_each(|pixel| pixel[3] *= 0.6);
        Filtered {
            image: faded,
            offset: (0.0, 0.0),
        }
    });
    mesh.set_zorder(5.0)
        .set_agg_filter(Some(manual_alpha))
        .set_rasterized(true);
    ax.plot([0.0, 4.0, 7.0], [1.0, 3.0, 8.0])?;
    Ok(())
}

/// `test_skew_rectangle`: a square skewed by multiples of 45° along x and
/// y in a grid of shared, equal-aspect axes.
fn skew_rectangle(fig: &Figure) -> CaseResult {
    let axes = fig.subplots_shared(5, 5, Share::All, Share::All)?;
    let axes = axes.as_slice();
    axes[0].set_xlim(-3.0, 3.0)?.set_ylim(-3.0, 3.0)?;
    axes[0].set_shared_aspect(Aspect::Equal, Adjustable::Box)?;
    let rotations = [-3.0, -1.0, 0.0, 1.0, 3.0];
    let skews = rotations
        .iter()
        .flat_map(|&x| rotations.iter().map(move |&y| (45.0 * x, 45.0 * y)));
    for (ax, (xdeg, ydeg)) in axes.iter().zip(skews) {
        ax.set_title(format!("Skew of {xdeg} in X and {ydeg} in Y"));
        let skew = Transform::Affine(Affine2D::identity().skew_deg(xdeg, ydeg));
        ax.add_patch_with(
            Rectangle::new((-1.0, -1.0), 2.0, 2.0),
            PatchProps::default()
                .alpha(0.5)
                .facecolor("coral")
                .transform(skew.then(ax.trans_data())),
        );
    }
    fig.subplots_adjust(&SubplotOverrides {
        wspace: Some(0.0),
        left: Some(0.01),
        right: Some(0.99),
        bottom: Some(0.01),
        top: Some(0.99),
        ..SubplotOverrides::default()
    })?;
    remove_text(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_misc/agg_filter", agg_filter),
    FigureCase::new("mpl_misc/agg_filter_alpha", agg_filter_alpha),
    classic("mpl_misc/skew_rectangle", skew_rectangle).sized(8.0, 8.0, 100.0),
];
