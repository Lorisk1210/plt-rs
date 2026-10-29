//! The other collection tests: scatter limits on log axes, ellipses in data
//! units, polygons lifted into 3-D, a 3-D scatter recoloured after a draw,
//! rotated and data-scaled regular polygons, line caps and joins, alpha set
//! after the fact, new `fill_between` data, masked offsets, striped lines
//! and the hatch width a collection takes when it is made.

use std::f64::consts::PI;

use pltrs::axes::FillBetweenOptions;
use pltrs::collections::{CollectionProps, CollectionSpec, EdgeColors, EllipseUnits};
use pltrs::colors::colormaps;
use pltrs::dates::DateTime;
use pltrs::lines::StepWhere;
use pltrs::mplot3d::{Axes3DOptions, Axis3, Scatter3DOptions};
use pltrs::prelude::*;
use pltrs::transforms::{Point, Transform};
use pltrs_core::Affine2D;
use pltrs_render::{CapStyle, JoinStyle};

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;
use crate::figures::cases::common::points;

pub(super) fn collection_log_datalim(fig: &Figure) -> CaseResult {
    let x_vals = [
        4.38462e-6, 5.54929e-6, 7.02332e-6, 8.88889e-6, 1.12500e-5, 1.42383e-5, 1.80203e-5,
        2.28070e-5, 2.88651e-5, 3.65324e-5, 4.62363e-5, 5.85178e-5, 7.40616e-5, 9.37342e-5,
        1.18632e-4,
    ];
    let y_vals = [
        0.0, 0.1, 0.182, 0.332, 0.604, 1.1, 2.0, 3.64, 6.64, 12.1, 22.0, 39.6, 71.3,
    ];
    let (x, y) = meshgrid(&x_vals, &y_vals);
    let ax = fig.subplot();
    ax.set_xscale("log")?.set_yscale("log")?;
    ax.scatter(x.as_slice(), y.as_slice())?;
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn ellipse_collection(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let (xx, yy) = meshgrid(&[0.0, 1.0, 2.0, 3.0], &[0.0, 1.0, 2.0]);
    let offsets = xx
        .as_slice()
        .iter()
        .zip(yy.as_slice())
        .map(|(&x, &y)| Point::new(x, y))
        .collect();
    let widths: Vec<f64> = xx.as_slice().iter().map(|x| x / 3.0).collect();
    let heights: Vec<f64> = yy.as_slice().iter().map(|y| y / 2.0).collect();
    let angles = vec![20.0; widths.len()];
    let ellipses = CollectionSpec::ellipses(&widths, &heights, &angles, EllipseUnits::X)
        .with_offsets(offsets, ax.trans_data())
        .with_props(CollectionProps::default().facecolor("none"));
    ax.add_collection(ellipses);
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn polycollection_close(fig: &Figure) -> CaseResult {
    let quads = [
        points(&[(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)]),
        points(&[(0.0, 1.0), (2.0, 3.0), (2.0, 2.0), (1.0, 1.0)]),
        points(&[(2.0, 2.0), (2.0, 3.0), (4.0, 1.0), (3.0, 1.0)]),
        points(&[(3.0, 0.0), (3.0, 1.0), (4.0, 1.0), (4.0, 0.0)]),
    ];
    let colors = ["r", "g", "b", "y", "k"];
    let ax = fig.add_axes_3d([0.0, 0.0, 1.0, 1.0], Axes3DOptions::default())?;
    // `verts_quad * 5`, with one z-value and one colour per polygon.
    let n = quads.len();
    let verts: Vec<Vec<Point>> = colors.iter().flat_map(|_| quads.clone()).collect();
    let zs: Vec<f64> = (0..colors.len())
        .flat_map(|z| std::iter::repeat_n(z as f64, n))
        .collect();
    let cs: Vec<Color> = colors
        .iter()
        .flat_map(|&color| std::iter::repeat_n(Color::from(color), n))
        .collect();
    // `set_color`: faces and edges alike.
    let props = CollectionProps {
        facecolors: Some(cs.clone()),
        edgecolors: Some(EdgeColors::Colors(cs)),
        ..CollectionProps::default().linewidth(0.25).alpha(0.7)
    };
    ax.add_collection3d(
        CollectionSpec::polygons(verts).with_props(props),
        &zs,
        Axis3::Y,
        true,
    )?;
    ax.set_xlim(0.0, 4.0)?
        .set_zlim(0.0, 3.0)?
        .set_ylim(0.0, 4.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn scalarmap_change_cmap(fig: &Figure) -> CaseResult {
    let grid: Vec<[f64; 3]> = (0..5)
        .flat_map(|x| (0..5).flat_map(move |y| (0..5).map(move |z| [x, y, z].map(f64::from))))
        .collect();
    let coordinate = |i: usize| -> Vec<f64> { grid.iter().map(|p| p[i]).collect() };
    let (x, y, z) = (coordinate(0), coordinate(1), coordinate(2));
    let c: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x + y).collect();
    let ax = fig.subplot_3d();
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            sizes: Some(vec![40.0]),
            colors: MappedColors::new(c).cmap(colormaps::get("jet")?).into(),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    let scatter = ax.scatter_with(&x, &y, &z, options)?;
    fig.layout();
    scatter.set_cmap(colormaps::get("viridis")?);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn regularpolycollection_rotate(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let rotations = linspace(0.0, 2.0 * PI, 100);
    // `np.mgrid[:10, :10]` flattened: x is the slow index.
    let xy = (0..10).flat_map(|x| (0..10).map(move |y| Point::new(f64::from(x), f64::from(y))));
    for (xy, rotation) in xy.zip(rotations) {
        let squares = CollectionSpec::regular_polygons(4, rotation, vec![100.0])
            .with_offsets(vec![xy], ax.trans_data());
        ax.add_collection(squares);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn regularpolycollection_scale(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    // `SquareCollection.get_transform`: the pixels per data unit of the
    // axes times 72 / dpi at draw time (pixels to inches to points).
    let to_data = Transform::Inverted(Box::new(Transform::Inches))
        .then(Transform::Affine(Affine2D::scaling(72.0, 72.0)))
        .then(Transform::AffineDelta(Box::new(ax.trans_data())));
    // A unit square has a half-diagonal of 1/√2: its circle area is π/2.
    let squares = CollectionSpec::regular_polygons(4, PI / 4.0, vec![PI / 2.0])
        .with_offsets(vec![Point::ORIGIN], ax.trans_data())
        .with_transform(to_data);
    ax.add_collection(squares);
    ax.set_xlim(-1.0, 1.0)?.set_ylim(-1.0, 1.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn size_in_xy(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let ellipses = CollectionSpec::ellipses(&[10.0, 10.0], &[10.0], &[0.0], EllipseUnits::Xy)
        .with_offsets(points(&[(10.0, 10.0), (15.0, 15.0)]), ax.trans_data());
    ax.add_collection(ellipses);
    ax.set_xlim(0.0, 30.0)?.set_ylim(0.0, 30.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn cap_and_joinstyle_image(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(-0.5, 1.5)?.set_ylim(-0.5, 2.5)?;
    let segments = [0.0, 0.5, 1.0]
        .into_iter()
        .map(|y| points(&[(0.0, y), (1.0, y), (0.5, y + 1.0)]))
        .collect();
    let props = CollectionProps {
        linewidths: Some(vec![10.0, 15.0, 20.0]),
        ..CollectionProps::default()
    };
    ax.add_collection(CollectionSpec::lines(segments).with_props(props))
        .set_capstyle(CapStyle::Round)
        .set_joinstyle(JoinStyle::Miter);
    ax.set_title("Line collection with customized caps and joinstyle");
    Ok(())
}

pub(super) fn scatter_post_alpha(fig: &Figure) -> CaseResult {
    let values = [0.0, 1.0, 2.0, 3.0, 4.0];
    let options = ScatterOptions {
        colors: MappedColors::new(values).into(),
        ..ScatterOptions::default()
    };
    fig.subplot()
        .scatter_with(values, values, options)?
        .set_alpha(Some(0.1));
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_fill_between_poly_collection_set_data`: a fill from -1 to 1.2
/// given the curves `sin t` and `sin t + 0.2` afterwards.
fn fill_between_set_data(fig: &Figure, step: Option<StepWhere>) -> CaseResult {
    let t = linspace(0.0, 16.0, 50);
    let f1: Vec<f64> = t.iter().map(|t| t.sin()).collect();
    let f2: Vec<f64> = f1.iter().map(|f| f + 0.2).collect();
    let options = FillBetweenOptions {
        step,
        ..FillBetweenOptions::default()
    };
    fig.subplot()
        .fill_between(&t, [-1.0], [1.2], options)?
        .set_fill_between_data(&t, &f1, &f2, None)?;
    Ok(())
}

pub(super) fn fill_between_poly_collection_set_data(fig: &Figure) -> CaseResult {
    fill_between_set_data(fig, None)
}

pub(super) fn fill_between_poly_collection_set_data_step_pre(fig: &Figure) -> CaseResult {
    fill_between_set_data(fig, Some(StepWhere::Pre))
}

pub(super) fn check_masked_offsets(fig: &Figure) -> CaseResult {
    let x = (52..57)
        .map(|second| DateTime::new(2022, 12, 15, 4, 49, second, 0))
        .collect::<Result<Vec<_>, _>>()?;
    let masked_y = [1.0, f64::NAN, f64::NAN, 4.0, 5.0];
    fig.subplot().scatter(x, masked_y.as_slice())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn masked_set_offsets(fig: &Figure) -> CaseResult {
    let x = [1.0, 2.0, f64::NAN, f64::NAN, 5.0];
    let y = [1.0, 2.0, 3.0, 4.0, 5.0];
    let ax = fig.subplot();
    let offsets = x.iter().zip(&y).map(|(&x, &y)| Point::new(x, y)).collect();
    ax.scatter(x.as_slice(), y.as_slice())?.set_offsets(offsets);
    ax.set_xticks(Vec::<f64>::new());
    ax.set_yticks(Vec::<f64>::new());
    Ok(())
}

/// `test_striped_lines`: thick vertical lines dotted, solid and dashed in
/// turn, their gaps in `gapcolors`, half transparent.
fn striped_lines(fig: &Figure, gapcolors: Vec<Color>) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 6.0)?.set_ylim(0.0, 1.0)?;
    ax.vlines([1.0, 2.0, 3.0, 4.0, 5.0], [0.0], [1.0])?
        .set_linewidths(vec![20.0])
        .set_linestyles(vec![LineStyle::Dotted, LineStyle::Solid, LineStyle::Dashed])
        .set_gapcolors(Some(gapcolors))
        .set_alpha(Some(0.5));
    Ok(())
}

pub(super) fn striped_lines_orange(fig: &Figure) -> CaseResult {
    striped_lines(fig, vec!["orange".into()])
}

pub(super) fn striped_lines_r_k(fig: &Figure) -> CaseResult {
    striped_lines(fig, vec!["r".into(), "k".into()])
}

pub(super) fn hatch_linewidth(fig: &Figure) -> CaseResult {
    let polygons = vec![
        points(&[(0.1, 0.1), (0.1, 0.4), (0.4, 0.4), (0.4, 0.1)]),
        points(&[(0.6, 0.6), (0.6, 0.9), (0.9, 0.9), (0.9, 0.6)]),
    ];
    // The collection takes `hatch.linewidth` when it is made.
    let hatched = pltrs::rc::context(|| {
        pltrs::rc::set("hatch.linewidth", "2.0").map(|()| {
            CollectionSpec::polygons(polygons).with_props(CollectionProps::default().hatch("x"))
        })
    })?;
    fig.subplot().add_collection(hatched);
    Ok(())
}
