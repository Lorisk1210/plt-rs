//! `test_patches.py`: rotated and dashed rectangles, paths clipped to a
//! box, cut-out stars as a patch and as a collection, invisible edges,
//! wedges over every range of angles, hatch colours, connection patches
//! and a shadow.

use plt::collections::{CollectionProps, CollectionSpec};
use plt::coords::Coords;
use plt::patches::{ConnectionEnd, ConnectionPatch, PathShape, Rectangle, Shadow, Wedge};
use plt::prelude::*;
use plt::transforms::Point;
use plt_core::{Affine2D, Bbox, Path, PathError, Rgba};

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

/// `(x, y)` shifted by `d` in both directions.
fn shifted(points: &[Point], d: f64) -> Vec<Point> {
    points
        .iter()
        .map(|p| Point::new(p.x + d, p.y + d))
        .collect()
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn rotate_rect_draw(fig: &Figure) -> CaseResult {
    let rect = fig
        .subplot()
        .add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
    rect.update_shape(|r| r.angle = 30.0);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn dash_offset_patch_draw(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    for (color, offset) in [("b", 0.0), ("r", 6.0)] {
        ax.add_patch(Rectangle::new((0.1, 0.1), 0.8, 0.8))
            .update_props(
                PatchProps::default()
                    .linewidth(3.0)
                    .edgecolor(color)
                    .linestyle(LineStyle::dashes(offset, vec![6.0, 6.0])),
            );
    }
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn clip_to_bbox(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(-18.0, 20.0).set_ylim(-150.0, 100.0);
    let star = Path::unit_regular_star(8, 0.5)
        .transformed(&Affine2D::scaling(10.0, 100.0).translate(-5.0, -25.0));
    let circle =
        Path::unit_circle().transformed(&Affine2D::scaling(10.0, 100.0).translate(10.0, -25.0));
    let combined = Path::compound([&star, &circle]);
    let clipped = combined.clip_to_bbox(&Bbox::from_extents(-12.0, -77.5, 50.0, -110.0), true);
    ax.add_patch(PathShape(combined)).update_props(
        PatchProps::default()
            .alpha(0.5)
            .facecolor("coral")
            .edgecolor("none"),
    );
    ax.add_patch(PathShape(clipped)).update_props(
        PatchProps::default()
            .alpha(0.5)
            .facecolor("green")
            .linewidth(4.0)
            .edgecolor("black"),
    );
    Ok(())
}

/// A unit circle with a six-pointed star cut out (the star's vertices
/// reversed, its codes kept), and the same moved by `(1, 1)`.
pub(in crate::figures::cases) fn cut_stars() -> Result<(Path, Path), PathError> {
    let star = Path::unit_regular_star(6, 0.5);
    let circle = Path::unit_circle();
    let vertices: Vec<Point> = circle
        .vertices()
        .iter()
        .chain(star.vertices().iter().rev())
        .copied()
        .collect();
    let codes = [circle.resolved_codes(), star.resolved_codes()].concat();
    let moved = shifted(&vertices, 1.0);
    Ok((
        Path::new(vertices, codes.clone())?,
        Path::new(moved, codes)?,
    ))
}

/// A cut star as a path collection styled by `collection` and another as
/// a path patch styled by `patch`, both with 5-point edges.
fn cut_star_pair(fig: &Figure, collection: CollectionProps, patch: PatchProps) -> CaseResult {
    let (star1, star2) = cut_stars()?;
    let ax = fig.subplot();
    ax.add_collection(CollectionSpec::paths(vec![star2]).with_props(collection.linewidth(5.0)));
    ax.add_patch(PathShape(star1))
        .update_props(patch.linewidth(5.0));
    ax.set_xlim(-1.0, 2.0).set_ylim(-1.0, 2.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// Half-transparent red faces and three-quarter-opaque blue edges.
const FACE: Rgba = Rgba::new(1.0, 0.0, 0.0, 0.5);
const EDGE: Rgba = Rgba::new(0.0, 0.0, 1.0, 0.75);

pub(super) fn patch_alpha_coloring(fig: &Figure) -> CaseResult {
    let collection = CollectionProps::default()
        .linestyle(LineStyle::DashDot)
        .facecolor(FACE)
        .edgecolor(EDGE);
    let patch = PatchProps::default()
        .linestyle(LineStyle::DashDot)
        .facecolor(FACE)
        .edgecolor(EDGE);
    cut_star_pair(fig, collection, patch)
}

pub(super) fn patch_alpha_override(fig: &Figure) -> CaseResult {
    let collection = CollectionProps::default()
        .linestyle(LineStyle::DashDot)
        .alpha(0.25)
        .facecolor(FACE)
        .edgecolor(EDGE);
    let patch = PatchProps::default()
        .linestyle(LineStyle::DashDot)
        .alpha(0.25)
        .facecolor(FACE)
        .edgecolor(EDGE);
    cut_star_pair(fig, collection, patch)
}

pub(super) fn patch_custom_linestyle(fig: &Figure) -> CaseResult {
    let dashes = LineStyle::dashes(0.0, vec![5.0, 7.0, 10.0, 7.0]);
    let (red, blue) = (Rgba::opaque(1.0, 0.0, 0.0), Rgba::opaque(0.0, 0.0, 1.0));
    let collection = CollectionProps::default()
        .linestyle(dashes.clone())
        .facecolor(red)
        .edgecolor(blue);
    let patch = PatchProps::default()
        .linestyle(dashes)
        .facecolor(red)
        .edgecolor(blue);
    cut_star_pair(fig, collection, patch)
}

pub(super) fn patch_linestyle_none(fig: &Figure) -> CaseResult {
    let circle = Path::unit_circle();
    let ax = fig.subplot();
    let styles = ["none", "None", " ", ""];
    for (i, style) in styles.into_iter().enumerate() {
        let moved = Path::new(
            shifted(circle.vertices(), i as f64),
            circle.resolved_codes(),
        )?;
        ax.add_patch(PathShape(moved)).update_props(
            PatchProps::default()
                .linewidth(3.0)
                .linestyle(style.parse::<LineStyle>()?)
                .facecolor(Rgba::opaque(1.0, 0.0, 0.0))
                .edgecolor(Rgba::opaque(0.0, 0.0, 1.0)),
        );
    }
    let last = (styles.len() - 1) as f64;
    ax.set_xlim(-1.0, last + 1.0).set_ylim(-1.0, last + 1.0);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn wedge_range(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t1 = 2.313_869_244_286_224;
    let angles = [
        (52.313_869_24, 232.313_869_24),
        (52.313_869_244_286_224, 232.313_869_244_286_22),
        (t1, t1 + 180.0),
        (0.0, 360.0),
        (90.0, 90.0 + 360.0),
        (-180.0, 180.0),
        (0.0, 380.0),
        (45.0, 46.0),
        (46.0, 45.0),
    ];
    for (i, (theta1, theta2)) in angles.into_iter().enumerate() {
        let (x, y) = ((i % 3) as f64, (i / 3) as f64);
        ax.add_artist(Wedge::new((x * 3.0, y * 3.0), 1.0, theta1, theta2))
            .update_props(
                PatchProps::default()
                    .facecolor("none")
                    .edgecolor("k")
                    .linewidth(3.0),
            );
    }
    ax.set_xlim(-2.0, 8.0).set_ylim(-2.0, 9.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn multi_color_hatch(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let bars = ax.bar(
        vec![0.0, 1.0, 2.0, 3.0, 4.0],
        [1.0, 2.0, 3.0, 4.0, 5.0],
        BarOptions::default(),
    )?;
    for (i, rect) in bars.patches.iter().enumerate() {
        rect.set_facecolor("none")
            .set_edgecolor(format!("C{i}"))
            .set_hatch("/");
    }
    ax.autoscale_view(None);
    ax.autoscale_with(Some(false), None, None);
    for i in 0..5 {
        let x = f64::from(i) - 0.8 / 2.0;
        // The rectangle takes `hatch.color` when it is made.
        plt::rc::context(|| {
            plt::rc::set("hatch.color", &format!("C{i}"))?;
            ax.add_patch(Rectangle::new((x, 5.0), 0.8, 1.0))
                .update_props(PatchProps::default().hatch("//").facecolor("none"));
            Ok::<_, plt::rc::RcError>(())
        })?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// A connection patch from `a` to `b` drawn with `arrowstyle`.
fn connection(
    a: ConnectionEnd,
    b: ConnectionEnd,
    arrowstyle: &str,
) -> Result<ConnectionPatch, Box<dyn std::error::Error>> {
    let mut patch = ConnectionPatch::new(a, b);
    patch.spec.arrowstyle = arrowstyle.parse()?;
    Ok(patch)
}

pub(super) fn connection_patch(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax2.add_artist(connection(
        ConnectionEnd::new((0.1, 0.1), Coords::Data, Some(ax2.id())),
        ConnectionEnd::new((0.9, 0.9), Coords::Data, Some(ax1.id())),
        "->",
    )?);
    // x in axes coordinates, y in data coordinates.
    ax2.add_artist(connection(
        ConnectionEnd::new((0.6, 1.0), "axes fraction".parse()?, None),
        ConnectionEnd::new((0.0, 0.2), Coords::Transform(ax2.trans_yaxis()), None),
        "-",
    )?);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// A figure-level arrow from `(0.3, 0.2)` in `coords_a` of the left of two
/// axes to `(-30, -20)` in `coords_b` (counted from the top right corner),
/// shrunk by 5 points at its head.
fn figure_connection(fig: &Figure, coords_a: &str, coords_b: &str) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let mut patch = connection(
        ConnectionEnd::new((0.3, 0.2), coords_a.parse()?, Some(axs[(0, 0)].id())),
        ConnectionEnd::new((-30.0, -20.0), coords_b.parse()?, None),
        "->",
    )?;
    patch.spec.shrink_b = 5.0;
    fig.add_artist(patch);
    Ok(())
}

pub(super) fn connection_patch_fig(fig: &Figure) -> CaseResult {
    figure_connection(fig, "data", "figure pixels")
}

pub(super) fn connection_patch_pixel_points(fig: &Figure) -> CaseResult {
    figure_connection(fig, "axes points", "figure points")
}

pub(super) fn shadow(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let rect = ax.add_patch(Rectangle::new((0.2, 0.3), 0.5, 0.5));
    ax.add_patch(Shadow::of(&rect, 0.1, 0.2)?);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn set_and_get_hatch_linewidth(fig: &Figure) -> CaseResult {
    fig.subplot()
        .add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0))
        .set_hatch("x")
        .set_hatch_linewidth(2.0);
    Ok(())
}

pub(super) fn patch_edgegapcolor_visual(fig: &Figure) -> CaseResult {
    use plt::patches::{Ellipse, Polygon};
    use plt::rc::Aspect;

    let ax = fig.subplot();
    let style = |edge: &str, style: LineStyle| {
        PatchProps::default()
            .fill(false)
            .edgecolor(edge)
            .linestyle(style)
            .linewidth(3.0)
    };
    ax.add_patch(Rectangle::new((0.1, 0.1), 0.3, 0.3))
        .update_props(style("blue", LineStyle::Dashed))
        .set_edgegapcolor("orange");
    ax.add_patch(Ellipse::new((0.7, 0.3), 0.3, 0.2))
        .update_props(style("red", LineStyle::Dotted))
        .set_edgegapcolor("yellow");
    let triangle = [(0.1, 0.6), (0.3, 0.9), (0.4, 0.6)].map(Point::from);
    ax.add_patch(Polygon::new(triangle, true))
        .update_props(style("green", LineStyle::DashDot))
        .set_edgegapcolor("purple");
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    ax.set_aspect(Aspect::Equal)?;
    remove_ticks_and_titles(fig);
    Ok(())
}
