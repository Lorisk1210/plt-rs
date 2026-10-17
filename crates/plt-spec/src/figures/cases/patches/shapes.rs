//! Simple shape cases of `figures/cases_patches.py`.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use plt::axes::StairsOptions;
use plt::patches::{
    Annulus, Arc, Circle, CirclePolygon, Ellipse, PatchStyle, PathShape, Polygon, Rectangle,
    RegularPolygon, RotationPoint, Shadow, StepBaseline, StepPatch, Wedge,
};
use plt::prelude::*;
use plt::transforms::Point;
use plt_core::{Path, PathCode};
use plt_render::{CapStyle, JoinStyle};

use super::square_canvas;
use crate::figures::FigureCase;

fn points(coords: &[(f64, f64)]) -> Vec<Point> {
    coords.iter().map(|&(x, y)| Point::new(x, y)).collect()
}

/// The ten-pointed star of `polygon_closed_open`.
fn star() -> Vec<Point> {
    let mut angles = linspace(FRAC_PI_2, FRAC_PI_2 + TAU, 11);
    angles.pop();
    angles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let r = if i % 2 == 0 { 1.0 } else { 0.4 };
            Point::new(7.5 + 2.0 * t.cos() * r, 7.0 + 2.0 * t.sin() * r)
        })
        .collect()
}

/// The curved and holed paths of `pathpatch_curves`.
fn curve_paths() -> Result<[Path; 3], plt_core::PathError> {
    use PathCode::{ClosePoly, Curve3, Curve4, LineTo, MoveTo};
    let curves = Path::new(
        points(&[
            (1.0, 1.0),
            (2.0, 5.0),
            (5.0, 5.0),
            (5.0, 2.0),
            (4.0, 1.0),
            (2.5, 0.5),
            (1.0, 1.0),
            (1.0, 1.0),
        ]),
        vec![
            MoveTo, Curve4, Curve4, Curve4, LineTo, Curve3, Curve3, ClosePoly,
        ],
    )?;
    let square = [MoveTo, LineTo, LineTo, LineTo, ClosePoly];
    let hole = Path::new(
        points(&[
            (6.0, 6.0),
            (9.0, 6.0),
            (9.0, 9.0),
            (6.0, 9.0),
            (6.0, 6.0),
            (7.0, 7.0),
            (7.0, 8.0),
            (8.0, 8.0),
            (8.0, 7.0),
            (7.0, 7.0),
        ]),
        [square, square].concat(),
    )?;
    let open = Path::new(
        points(&[(6.0, 1.0), (9.0, 4.0), (6.0, 4.0)]),
        vec![MoveTo, Curve3, Curve3],
    )?;
    Ok([curves, hole, open])
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("patches/rectangle_default", |fig| {
        fig.subplot()
            .add_patch(Rectangle::new((0.2, 0.3), 0.5, 0.4));
        Ok(())
    }),
    FigureCase::new("patches/rectangle_angle_rotation_point", |fig| {
        let ax = square_canvas(fig);
        let pivots = [
            (RotationPoint::Xy, "C0"),
            (RotationPoint::Center, "C1"),
            (RotationPoint::At(Point::new(5.0, 5.0)), "C2"),
        ];
        for (pivot, color) in pivots {
            ax.add_patch(Rectangle::new((3.0, 4.0), 4.0, 2.0).rotated(30.0, pivot))
                .update_props(
                    PatchProps::default()
                        .facecolor(color)
                        .alpha(0.5)
                        .edgecolor("k"),
                );
        }
        ax.add_patch(Rectangle::new((3.0, 4.0), 4.0, 2.0))
            .update_props(
                PatchProps::default()
                    .fill(false)
                    .linestyle(LineStyle::Dashed),
            );
        ax.plot_fmt([3.0, 5.0, 5.0], [4.0, 5.0, 5.0], "k.")?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/circle_ellipse", |fig| {
        let ax = square_canvas(fig);
        ax.add_patch(Circle::new((2.5, 7.0), 2.0));
        ax.add_patch(Circle::new((7.5, 7.0), 1.5)).update_props(
            PatchProps::default()
                .facecolor("none")
                .edgecolor("tab:red")
                .linewidth(3.0),
        );
        ax.add_patch(Ellipse {
            angle: 20.0,
            ..Ellipse::new((3.0, 2.5), 5.0, 2.0)
        })
        .set_facecolor("tab:green");
        ax.add_patch(Ellipse {
            angle: -30.0,
            ..Ellipse::new((7.5, 2.5), 1.5, 3.5)
        })
        .set_facecolor("gold")
        .set_edgecolor("k");
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/wedge_width", |fig| {
        let ax = square_canvas(fig);
        ax.add_patch(Wedge::new((2.5, 7.5), 2.0, 30.0, 270.0));
        ax.add_patch(Wedge {
            width: Some(0.7),
            ..Wedge::new((7.5, 7.5), 2.0, 0.0, 360.0)
        })
        .set_facecolor("tab:orange");
        ax.add_patch(Wedge {
            width: Some(1.2),
            ..Wedge::new((2.5, 2.5), 2.0, -45.0, 45.0)
        })
        .set_facecolor("tab:green")
        .set_edgecolor("k");
        ax.add_patch(Wedge::new((7.5, 2.5), 2.0, 300.0, 60.0))
            .set_facecolor("tab:purple")
            .set_alpha(Some(0.6));
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/arc", |fig| {
        let ax = square_canvas(fig);
        ax.add_patch(Arc::new((2.5, 7.5), 4.0, 3.0).between(0.0, 270.0))
            .set_linewidth(2.0);
        ax.add_patch(Arc::new((7.5, 7.5), 4.0, 2.0).with_angle(30.0))
            .set_color("tab:red")
            .set_linewidth(3.0);
        ax.add_patch(Arc::new((2.5, 2.5), 4.0, 4.0).between(200.0, 20.0))
            .set_linestyle(LineStyle::Dashed);
        ax.add_patch(
            Arc::new((7.5, 2.5), 3.0, 4.0)
                .with_angle(-45.0)
                .between(45.0, 135.0),
        )
        .set_linewidth(4.0)
        .set_color("tab:green")
        .set_capstyle(CapStyle::Round);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/polygon_closed_open", |fig| {
        let ax = square_canvas(fig);
        let pts = points(&[(1.0, 1.0), (4.0, 2.0), (3.0, 4.5), (1.5, 3.5)]);
        ax.add_patch(Polygon::new(pts.clone(), true))
            .set_facecolor("tab:blue")
            .set_edgecolor("k");
        let shifted = pts.iter().map(|p| Point::new(p.x + 5.0, p.y));
        ax.add_patch(Polygon::new(shifted, false))
            .set_fill(false)
            .set_linewidth(3.0);
        ax.add_patch(Polygon::new(star(), true))
            .set_facecolor("gold")
            .set_edgecolor("tab:red")
            .set_linewidth(2.0)
            .set_joinstyle(JoinStyle::Miter);
        let crossed = points(&[(1.0, 6.0), (4.0, 9.0), (1.0, 9.0), (4.0, 6.0)]);
        ax.add_patch(Polygon::new(crossed, true))
            .set_facecolor("tab:green")
            .set_alpha(Some(0.6));
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/regular_and_circle_polygon", |fig| {
        let ax = square_canvas(fig);
        ax.add_patch(RegularPolygon::new((2.0, 7.5), 3, 1.5));
        ax.add_patch(RegularPolygon {
            orientation: PI / 5.0,
            ..RegularPolygon::new((5.0, 7.5), 5, 1.5)
        })
        .set_facecolor("C1");
        ax.add_patch(RegularPolygon::new((8.0, 7.5), 6, 1.5))
            .set_facecolor("C2")
            .set_edgecolor("k");
        ax.add_patch(CirclePolygon::new((2.0, 2.5), 1.5).with_resolution(8))
            .set_facecolor("C3");
        ax.add_patch(CirclePolygon::new((5.0, 2.5), 1.5))
            .set_facecolor("C4");
        ax.add_patch(CirclePolygon::new((8.0, 2.5), 1.5).with_resolution(5))
            .set_fill(false)
            .set_linewidth(2.0);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/annulus", |fig| {
        let ax = square_canvas(fig);
        ax.add_patch(Annulus::new((3.0, 5.0), 2.5, 0.8)?);
        ax.add_patch(Annulus::elliptical((7.5, 5.0), (2.0, 1.0), 0.5, 30.0)?)
            .set_facecolor("tab:red")
            .set_edgecolor("k")
            .set_hatch("//");
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/pathpatch_curves", |fig| {
        let ax = square_canvas(fig);
        let [curves, hole, open] = curve_paths()?;
        ax.add_patch(PathShape(curves))
            .set_facecolor("tab:orange")
            .set_edgecolor("k")
            .set_linewidth(2.0);
        ax.add_patch(PathShape(hole))
            .set_facecolor("tab:blue")
            .set_edgecolor("k");
        ax.add_patch(PathShape(open))
            .set_fill(false)
            .set_linewidth(3.0)
            .set_edgecolor("tab:green");
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/steppatch_and_stairs", |fig| {
        let ax = fig.subplot();
        let edges = arange(0.0, 7.0, 1.0);
        let step = StepPatch::new(vec![1.0, 3.0, 2.0, 5.0, 4.0, 2.0], edges.clone())?
            .with_baseline(StepBaseline::Value(0.5))?;
        ax.add_patch(step)
            .update_props(PatchProps::default().fill(true).alpha(0.4));
        let red = StairsOptions {
            edges: Some(edges.clone()),
            color: Some("tab:red".into()),
            ..StairsOptions::default()
        };
        ax.stairs([2.0, 4.0, 3.0, 6.0, 5.0, 3.0], red)?
            .set_linewidth(2.0);
        let open = StairsOptions {
            edges: Some(edges),
            baseline: StepBaseline::None,
            ..StairsOptions::default()
        };
        ax.stairs([4.0, 1.0, 2.0, 3.0, 1.0, 5.0], open)?
            .set_linestyle(LineStyle::Dashed);
        // `ax.autoscale_view()`.
        ax.autoscale(None, None);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/shadow", |fig| {
        let ax = square_canvas(fig);
        let rect = ax.add_patch(Rectangle::new((1.0, 5.0), 3.0, 3.0));
        rect.set_facecolor("tab:blue");
        ax.add_patch(Shadow::of(&rect, 0.3, -0.3)?);
        let circle = ax.add_patch(Circle::new((7.0, 3.0), 2.0));
        circle.set_facecolor("tab:orange");
        ax.add_patch(Shadow::of(&circle, 0.3, -0.3)?);
        // A shadow of a patch never added anywhere takes the axes' data
        // coordinates.
        let mut green = PatchStyle::from_rc(&plt::rc::params());
        green.facecolor = Some("tab:green".into());
        let loose = Rectangle::new((1.0, 1.0), 2.0, 2.0);
        ax.add_patch(Shadow::new(loose, &green, None, (0.5, -0.5), 0.3)?);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
