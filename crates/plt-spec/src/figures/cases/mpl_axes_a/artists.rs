//! Artists: an annotation offset in points, `FancyArrow`s of every shape,
//! dashed error bars, an Aitoff grid of markers, a thick polygon
//! collection, and elliptical arcs at many angles and scales.

use std::f64::consts::PI;

use plt::axes::{AxisName, ErrorSize, ErrorbarOptions};
use plt::collections::{CollectionProps, CollectionSpec};
use plt::geo::GeoProjection;
use plt::numeric::{arange, linspace, meshgrid};
use plt::patches::{Arc, ArrowShape, Ellipse, FancyArrow};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::AnnotateOptions;
use plt_core::Point;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

pub(super) fn basic_annotate(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 5.0, 0.01);
    let s: Vec<f64> = t.iter().map(|t| (2.0 * PI * t).cos()).collect();
    let ax = fig.subplot();
    ax.set_autoscale_on(false);
    ax.set_xlim(-1.0, 5.0)?.set_ylim(-3.0, 5.0)?;
    ax.plot_with(t, s, LineProps::default().linewidth(3.0).color("purple"))?;
    let options = AnnotateOptions {
        xytext: Some(Point::new(3.0, 3.0)),
        xycoords: "data".parse()?,
        textcoords: Some("offset points".parse()?),
        ..AnnotateOptions::default()
    };
    ax.annotate("local max", (3.0, 1.0), options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn arrow_simple(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(3, 4)?;
    let shapes = [ArrowShape::Full, ArrowShape::Left, ArrowShape::Right];
    // itertools.product(length_includes_head, shape, head_starts_at_zero)
    let kwargs = [true, false].into_iter().flat_map(|includes_head| {
        shapes.into_iter().flat_map(move |shape| {
            [true, false]
                .into_iter()
                .map(move |starts_at_zero| (includes_head, shape, starts_at_zero))
        })
    });
    for (i, (ax, (length_includes_head, shape, head_starts_at_zero))) in
        axs.as_slice().iter().zip(kwargs).enumerate()
    {
        ax.set_xlim(-2.0, 2.0)?.set_ylim(-2.0, 2.0)?;
        let theta = 2.0 * PI * i as f64 / 12.0;
        ax.arrow(FancyArrow {
            width: theta / 100.0,
            length_includes_head,
            shape,
            head_starts_at_zero,
            head_width: Some(theta / 10.0),
            head_length: Some(theta / 10.0),
            ..FancyArrow::new(0.0, 0.0, theta.sin(), theta.cos())
        });
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn errorbar_dashes(fig: &Figure) -> CaseResult {
    let x = [1.0, 2.0, 3.0, 4.0];
    let y: Vec<f64> = x.iter().map(|x: &f64| x.sin()).collect();
    let err: Vec<f64> = y.iter().map(|y| y.abs()).collect();
    let options = ErrorbarOptions {
        xerr: Some(ErrorSize::Symmetric(err.clone().into())),
        yerr: Some(ErrorSize::Symmetric(err.into())),
        line: LineProps::default().linestyle(LineStyle::dashes(0.0, vec![2.0, 2.0])),
        ..ErrorbarOptions::default()
    };
    fig.gca().errorbar(x, y, options)?;
    Ok(())
}

pub(super) fn aitoff_proj(fig: &Figure) -> CaseResult {
    let x = linspace(-PI, PI, 20);
    let y = linspace(-PI / 2.0, PI / 2.0, 20);
    let (xx, yy) = meshgrid(&x, &y);
    let ax = fig.geo_subplot(GeoProjection::Aitoff);
    ax.grid(true);
    ax.plot_with(
        xx.as_slice(),
        yy.as_slice(),
        LineProps::fmt("o")?.markersize(4.0),
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn polycollection_joinstyle(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let square = [(1.0, 1.0), (1.0, 2.0), (2.0, 2.0), (2.0, 1.0)]
        .into_iter()
        .map(Point::from)
        .collect();
    let props = CollectionProps {
        linewidths: Some(vec![40.0]),
        ..CollectionProps::default()
    };
    ax.add_collection(CollectionSpec::polygons(vec![square]).with_props(props));
    ax.set_bound(AxisName::X, Some(0.0), Some(3.0))?.set_bound(
        AxisName::Y,
        Some(0.0),
        Some(3.0),
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// A point at `angle` degrees and distance `scale` from `centre`.
fn polar_point(centre: (f64, f64), scale: f64, angle: f64) -> (f64, f64) {
    let angle = angle.to_radians();
    (
        scale * angle.cos() + centre.0,
        scale * angle.sin() + centre.1,
    )
}

pub(super) fn arc_angles(fig: &Figure) -> CaseResult {
    let (mut w, mut h) = (2.0, 1.0);
    let mut centre = (0.2, 0.5);
    let mut scale = 2.0;
    for (i, ax) in fig.subplots(3, 3)?.as_slice().iter().enumerate() {
        let theta2 = i as f64 * 360.0 / 9.0;
        let theta1 = theta2 - 45.0;
        ax.add_patch(Ellipse::new(centre, w, h))
            .set_alpha(Some(0.3));
        ax.add_patch(Arc::new(centre, w, h).between(theta1, theta2));
        let start = polar_point(centre, scale, theta1);
        let end = polar_point(centre, scale, theta2);
        ax.plot([start.0, centre.0, end.0], [start.1, centre.1, end.1])?;
        ax.set_xlim(-scale, scale)?.set_ylim(-scale, scale)?;
        w *= 10.0;
        h *= 10.0;
        centre = (centre.0 * 10.0, centre.1 * 10.0);
        scale *= 10.0;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn arc_ellipse(fig: &Figure) -> CaseResult {
    let (xcenter, ycenter) = (0.38, 0.52);
    let (width, height) = (1e-1, 3e-1);
    let angle: f64 = -30.0;
    let (sin, cos) = angle.to_radians().sin_cos();
    let (x, y): (Vec<f64>, Vec<f64>) = (0..360)
        .map(|degrees| {
            let theta = f64::from(degrees).to_radians();
            let (px, py) = (width / 2.0 * theta.cos(), height / 2.0 * theta.sin());
            (cos * px - sin * py + xcenter, sin * px + cos * py + ycenter)
        })
        .unzip();
    // The test passes linewidth=1 to the first fill only: classic's
    // patch.linewidth is 1 too.
    for (index, aspect, color) in [(1, Aspect::Auto, "yellow"), (2, Aspect::Equal, "green")] {
        let ax = fig.add_subplot(2, 1, index)?;
        ax.set_aspect(aspect)?;
        let props = PatchProps::default()
            .alpha(0.2)
            .facecolor(color)
            .edgecolor(color)
            .linewidth(1.0)
            .zorder(1.0);
        ax.fill(&x, &y, props)?;
        ax.add_patch(Arc::new((xcenter, ycenter), width, height).with_angle(angle))
            .set_linewidth(2.0)
            .set_fill(false)
            .set_zorder(2.0);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}
