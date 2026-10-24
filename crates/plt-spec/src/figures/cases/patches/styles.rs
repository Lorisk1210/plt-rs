//! Edge style, alpha/z-order, clipping, `broken_barh` and `fill` cases of
//! `figures/cases_patches.py`.

use std::f64::consts::TAU;

use plt::artist::ClipPathSpec;
use plt::axes::BandAlign;
use plt::patches::{Circle, PatchShape, Polygon, Rectangle};
use plt::prelude::*;
use plt::transforms::{Point, Transform};
use plt_render::{CapStyle, JoinStyle};

use super::{canvas, square_canvas};
use crate::figures::{CaseResult, FigureCase};

/// The three joins and caps of `edge_styles`.
fn joins(ax: &Axes) {
    let styles = [
        (JoinStyle::Miter, CapStyle::Butt),
        (JoinStyle::Round, CapStyle::Round),
        (JoinStyle::Bevel, CapStyle::Projecting),
    ];
    for (i, (join, cap)) in styles.into_iter().enumerate() {
        let dx = 3.3 * i as f64;
        let corners = [
            Point::new(0.5 + dx, 3.5),
            Point::new(1.5 + dx, 5.8),
            Point::new(2.5 + dx, 3.5),
        ];
        ax.add_patch(Polygon::new(corners, false))
            .set_fill(false)
            .set_linewidth(8.0)
            .set_joinstyle(join)
            .set_capstyle(cap);
    }
}

/// `clip_path_artists`: lines and a rectangle clipped to a circle.
fn clipped(ax: &Axes) -> CaseResult {
    let circle = Circle::new((5.0, 5.0), 3.5);
    let clip = || {
        Some(ClipPathSpec::Path {
            path: circle.path(),
            transform: Transform::Affine(circle.patch_transform()).then(ax.trans_data()),
        })
    };
    ax.add_patch(circle.clone())
        .set_fill(false)
        .set_linestyle(LineStyle::Dotted);
    for i in 0..10 {
        let i = f64::from(i);
        ax.plot_with(
            [0.0, 10.0],
            [i, 10.0 - i + 3.0],
            LineProps::default().linewidth(3.0),
        )?
        .set_clip_path(clip());
    }
    ax.add_patch(Rectangle::new((0.0, 0.0), 6.0, 6.0))
        .set_facecolor("tab:gray")
        .set_alpha(Some(0.5))
        .set_clip_path(clip());
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("patches/edge_styles", |fig| {
        let ax = canvas(fig, (0.0, 10.0), (0.0, 10.0), false)?;
        let styles = [
            LineStyle::Solid,
            LineStyle::Dashed,
            LineStyle::Dotted,
            LineStyle::DashDot,
            LineStyle::Dashes {
                offset: 0.0,
                pattern: Some(vec![5.0, 1.0, 1.0, 1.0]),
            },
        ];
        for (i, style) in styles.into_iter().enumerate() {
            ax.add_patch(Rectangle::new((0.3 + 1.9 * i as f64, 6.5), 1.5, 3.0))
                .set_fill(false)
                .set_linewidth(2.0)
                .set_linestyle(style);
        }
        joins(&ax);
        ax.add_patch(Rectangle::new((0.5, 0.5), 9.0, 2.0))
            .set_facecolor("w")
            .set_edgecolor("tab:red")
            .set_linewidth(3.0)
            .set_linestyle(LineStyle::Dashed)
            .set_edgegapcolor("tab:blue");
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("patches/alpha_zorder", |fig| {
        let ax = square_canvas(fig)?;
        let circles = [
            ((4.0, 6.0), "tab:red", 3.0),
            ((6.0, 6.0), "tab:green", 1.0),
            ((5.0, 4.0), "tab:blue", 2.0),
        ];
        for (center, color, zorder) in circles {
            ax.add_patch(Circle::new(center, 3.0))
                .set_facecolor(color)
                .set_alpha(Some(0.5))
                .set_zorder(zorder);
        }
        let line = LineProps::default().color("k").linewidth(4.0).zorder(2.5);
        ax.plot_with([0.0, 10.0], [5.0, 5.0], line)?;
        ax.add_patch(Rectangle::new((1.0, 1.0), 8.0, 1.0))
            .set_facecolor((1.0, 0.5, 0.0, 0.3))
            .set_edgecolor((0.0, 0.0, 0.0, 1.0))
            .set_linewidth(2.0);
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("patches/clip_path_artists", |fig| {
        let ax = square_canvas(fig)?;
        clipped(&ax)
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("patches/broken_barh", |fig| {
        let ax = fig.subplot();
        ax.broken_barh(
            &[(1.0, 2.0), (4.0, 1.0), (6.0, 3.0)],
            (1.0, 2.0),
            BandAlign::Bottom,
        );
        ax.broken_barh(&[(0.0, 3.0), (5.0, 4.0)], (4.0, 2.0), BandAlign::Bottom)
            .set_facecolors(vec!["tab:red".into(), "tab:green".into()])
            .set_edgecolor("k")
            .set_hatch("//");
        ax.set_ylim(0.0, 7.0)?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("patches/fill_polygons", |fig| {
        let ax = fig.subplot();
        let t = linspace(0.0, TAU, 7);
        let cos: Vec<f64> = t.iter().map(|t| t.cos()).collect();
        let sin: Vec<f64> = t.iter().map(|t| t.sin()).collect();
        let shifted: Vec<f64> = cos.iter().map(|c| c + 2.5).collect();
        ax.fill(
            &cos,
            &sin,
            PatchProps::default().facecolor("tab:blue").alpha(0.7),
        )?;
        ax.fill(
            &shifted,
            &sin,
            PatchProps::default().facecolor("tab:orange").alpha(0.7),
        )?;
        ax.fill(
            [0.0, 1.0, 0.5],
            [-2.0, -2.0, -1.0],
            PatchProps::default().fill(false).hatch("x").edgecolor("k"),
        )?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
