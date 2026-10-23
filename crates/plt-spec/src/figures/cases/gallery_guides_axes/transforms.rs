//! `transforms_tutorial`: data, display, axes and figure coordinates,
//! blended transforms, physical sizes and offset transforms.

use std::f64::consts::PI;

use plt::axes::HistPlotOptions;
use plt::patches::{Circle, Ellipse, Rectangle};
use plt::prelude::*;
use plt::stats::{Bins, HistOptions};
use plt::text::{AnnotateOptions, ArrowProps, TextProps, VerticalAlignment};
use plt::ticker::pyfmt;
use plt::transforms::{Point, Transform};

use super::SEED;
use crate::figures::cases::common::{arrow, map, styled_box};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The tutorial's damped sine over `[0, 10)` in limits `(0, 10)` ×
/// `(-1, 1)`.
fn damped_sine(ax: &Axes) -> CaseResult {
    let x = arange(0.0, 10.0, 0.005);
    let y = map(&x, |x| (-x / 2.0).exp() * (2.0 * PI * x).sin());
    ax.plot(&x, y)?;
    ax.set_xlim(0.0, 10.0).set_ylim(-1.0, 1.0);
    Ok(())
}

/// 1000 faint green dots in `[0, 10) × [0, 10 yscale)`.
fn green_cloud(ax: &Axes, yscale: f64) -> CaseResult {
    let xy = SpecRng::new(SEED).uniform(2000, 0.0, 10.0);
    let (x, y) = xy.split_at(1000);
    ax.plot_with(x, map(y, |y| y * yscale), LineProps::fmt("go")?.alpha(0.2))?;
    Ok(())
}

/// A cloud with a blue circle of 1 in radius 2.5 in right of and 2 in
/// above the figure's corner.
fn inch_circle(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    green_cloud(&ax, 10.0)?;
    ax.add_patch_with(
        Circle::new((2.5, 2.0), 1.0),
        PatchProps::default()
            .transform(Transform::Inches)
            .facecolor("blue")
            .alpha(0.75),
    );
    Ok(())
}

fn transforms_tutorial_1(fig: &Figure) -> CaseResult {
    damped_sine(&fig.subplot())
}

fn transforms_tutorial_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    damped_sine(&ax)?;
    let (xdata, ydata) = (5.0, 0.0);
    let display = fig
        .resolve_transform(&ax.trans_data())?
        .transform_point(Point::new(xdata, ydata));
    let offset = 72.0;
    let annotate = |text: String, xy: Point, xytext: (f64, f64), xycoords: &str| -> CaseResult {
        ax.annotate(
            text,
            xy,
            AnnotateOptions {
                xytext: Some(xytext.into()),
                xycoords: xycoords.parse()?,
                textcoords: Some("offset points".parse()?),
                arrowprops: Some(ArrowProps::Fancy(arrow(
                    "->",
                    "angle,angleA=0,angleB=90,rad=10",
                )?)),
                ..AnnotateOptions::default()
            },
        )?
        .set_bbox(styled_box("round", PatchProps::default().facecolor("0.8"))?);
        Ok(())
    };
    annotate(
        format!(
            "data = ({}, {})",
            pyfmt::fixed(xdata, 1),
            pyfmt::fixed(ydata, 1)
        ),
        Point::new(xdata, ydata),
        (-2.0 * offset, offset),
        "data",
    )?;
    annotate(
        format!(
            "display = ({}, {})",
            pyfmt::fixed(display.x, 1),
            pyfmt::fixed(display.y, 1)
        ),
        display,
        (0.5 * offset, -offset),
        "figure pixels",
    )
}

fn transforms_tutorial_3(fig: &Figure) -> CaseResult {
    for (i, label) in ["A", "B", "C", "D"].into_iter().enumerate() {
        let ax = fig.add_subplot(2, 2, i + 1)?;
        ax.text(0.05, 0.95, label)
            .set_transform(ax.trans_axes())
            .update_props(TextProps::default().valign(VerticalAlignment::Top))
            .set_fontsize(16.0)
            .set_weight(plt::fonts::FontWeight::BOLD);
    }
    Ok(())
}

fn transforms_tutorial_4(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    green_cloud(&ax, 1.0)?;
    ax.add_patch_with(
        Circle::new((0.5, 0.5), 0.25),
        PatchProps::default()
            .transform(ax.trans_axes())
            .facecolor("blue")
            .alpha(0.75),
    );
    Ok(())
}

fn transforms_tutorial_5(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Count(30)),
            ..HistOptions::default()
        },
        ..HistPlotOptions::default()
    };
    ax.hist(&[SpecRng::new(SEED).normal(1000, 0.0, 1.0)], &options)?;
    ax.set_title(r"$\sigma=1 \/ \dots \/ \sigma=2$")
        .set_fontsize(16.0);
    let trans = Transform::blended(ax.trans_data(), ax.trans_axes());
    ax.add_patch_with(
        Rectangle::new((1.0, 0.0), 1.0, 1.0),
        PatchProps::default()
            .transform(trans)
            .color("yellow")
            .alpha(0.5),
    );
    Ok(())
}

fn transforms_tutorial_8(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let (xdata, ydata) = ([0.2, 0.7], [0.5, 0.5]);
    ax.plot_fmt(xdata, ydata, "o")?;
    ax.set_xlim(0.0, 1.0);
    let trans = Transform::Inches.then(Transform::Translation {
        dx: xdata[0],
        dy: ydata[0],
        units: Box::new(ax.trans_data()),
    });
    let circle = Ellipse {
        angle: 40.0,
        ..Ellipse::new((0.0, 0.0), 150.0 / 72.0, 130.0 / 72.0)
    };
    ax.add_patch_with(circle, PatchProps::default().fill(false).transform(trans));
    Ok(())
}

fn transforms_tutorial_9(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = arange(0.0, 2.0, 0.01);
    let y = map(&x, |x| (2.0 * PI * x).sin());
    let line = ax.plot_with(&x, &y, LineProps::default().linewidth(3.0).color("blue"))?;
    let shadow_transform = ax.trans_data().then(Transform::Translation {
        dx: 2.0 / 72.0,
        dy: -2.0 / 72.0,
        units: Box::new(Transform::Inches),
    });
    ax.plot_with(
        &x,
        &y,
        LineProps::default()
            .linewidth(3.0)
            .color("gray")
            .transform(shadow_transform)
            .zorder(0.5 * line.zorder()),
    )?;
    ax.set_title("creating a shadow effect with an offset transform");
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_guides_axes/transforms_tutorial_1",
        transforms_tutorial_1,
    ),
    FigureCase::new(
        "gallery_guides_axes/transforms_tutorial_2",
        transforms_tutorial_2,
    ),
    FigureCase::new(
        "gallery_guides_axes/transforms_tutorial_3",
        transforms_tutorial_3,
    ),
    FigureCase::new(
        "gallery_guides_axes/transforms_tutorial_4",
        transforms_tutorial_4,
    ),
    FigureCase::new(
        "gallery_guides_axes/transforms_tutorial_5",
        transforms_tutorial_5,
    ),
    FigureCase::new("gallery_guides_axes/transforms_tutorial_6", inch_circle)
        .sized(5.0, 4.0, 100.0),
    FigureCase::new("gallery_guides_axes/transforms_tutorial_7", inch_circle)
        .sized(7.0, 2.0, 100.0),
    FigureCase::new(
        "gallery_guides_axes/transforms_tutorial_8",
        transforms_tutorial_8,
    ),
    FigureCase::new(
        "gallery_guides_axes/transforms_tutorial_9",
        transforms_tutorial_9,
    ),
];
