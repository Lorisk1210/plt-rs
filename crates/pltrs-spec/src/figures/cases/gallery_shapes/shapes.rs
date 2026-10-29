//! Arrows and single artists: `arrow_guide`, `artist_reference`,
//! `ellipse_arrow` and `ellipse_demo`.

use pltrs::colors::colormaps;
use pltrs::layout::LayoutEngine;
use pltrs::markers::MarkerStyle;
use pltrs::patches::{
    Arrow, BoxShape, BoxStyle, Circle, Ellipse, FancyArrow, FancyArrowPatch, FancyBboxPatch,
    PathShape, Rectangle, RegularPolygon, Wedge,
};
use pltrs::prelude::*;
use pltrs::rc::Aspect;
use pltrs::transforms::{Point, Transform};
use pltrs_core::{Affine2D, Path, PathCode};

use super::SEED;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The tail and head of the guide's arrows.
const TAIL: (f64, f64) = (0.1, 0.5);
const HEAD: (f64, f64) = (0.9, 0.8);

/// Two axes stacked, the second showing `[0, 2] × [0, 2]` once `fill`
/// has drawn into both.
fn arrow_pair(fig: &Figure, fill: impl Fn(&Axes)) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    for ax in axs.as_slice() {
        fill(ax);
    }
    axs[(1, 0)].set_xlim(0.0, 2.0)?.set_ylim(0.0, 2.0)?;
    Ok(())
}

/// The guide's display-space arrow, placed by `transform` (data
/// coordinates when `None`).
fn guide_arrow(ax: &Axes, transform: Option<Transform>) {
    let arrow = FancyArrowPatch::between(TAIL, HEAD).with_mutation_scale(100.0);
    let props = transform.map_or_else(PatchProps::default, |t| PatchProps::default().transform(t));
    ax.add_patch_with(arrow, props);
}

fn arrow_guide_1(fig: &Figure) -> CaseResult {
    arrow_pair(fig, |ax| guide_arrow(ax, None))
}

fn arrow_guide_2(fig: &Figure) -> CaseResult {
    arrow_pair(fig, |ax| guide_arrow(ax, Some(ax.trans_axes())))
}

fn arrow_guide_3(fig: &Figure) -> CaseResult {
    let (dx, dy) = (HEAD.0 - TAIL.0, HEAD.1 - TAIL.1);
    let fancy = |x: f64| FancyArrow {
        width: 0.1,
        length_includes_head: true,
        ..FancyArrow::new(x, TAIL.1 - 0.4, dx, dy)
    };
    arrow_pair(fig, |ax| {
        ax.add_patch(Arrow::new(TAIL.0, TAIL.1, dx, dy));
        ax.add_patch_with(fancy(TAIL.0), PatchProps::default().color("C1"));
        ax.arrow(fancy(TAIL.0 + 1.0)).set_color("C2");
    })
}

/// The `PathPatch` of `artist_reference`.
fn reference_path() -> Result<Path, pltrs_core::PathError> {
    use PathCode::{ClosePoly, Curve4, LineTo, MoveTo};
    let vertices = [
        (0.018, -0.11),
        (-0.031, -0.051),
        (-0.115, 0.073),
        (-0.03, 0.073),
        (-0.011, 0.039),
        (0.043, 0.121),
        (0.075, -0.005),
        (0.035, -0.027),
        (0.018, -0.11),
    ];
    let codes = vec![
        MoveTo, Curve4, Curve4, Curve4, LineTo, Curve4, Curve4, Curve4, ClosePoly,
    ];
    Path::new(vertices.into_iter().map(Point::from).collect(), codes)
}

/// Adds the `index`-th artist of `artist_reference` coloured `color` and
/// returns its class name.
fn add_reference_artist(
    ax: &Axes,
    index: usize,
    color: Color,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    let props = PatchProps::default().color(color.clone());
    let name = match index {
        0 => {
            ax.add_artist_with(Circle::new((0.0, 0.0), 0.1), props);
            "Circle"
        }
        1 => {
            ax.add_artist_with(Rectangle::new((-0.025, -0.05), 0.05, 0.1), props);
            "Rectangle"
        }
        2 => {
            ax.add_artist_with(Wedge::new((0.0, 0.0), 0.1, 30.0, 270.0), props);
            "Wedge"
        }
        3 => {
            ax.add_artist_with(RegularPolygon::new((0.0, 0.0), 5, 0.1), props);
            "RegularPolygon"
        }
        4 => {
            ax.add_artist_with(Ellipse::new((0.0, 0.0), 0.2, 0.1), props);
            "Ellipse"
        }
        5 => {
            ax.add_artist_with(Arrow::new(-0.05, -0.05, 0.1, 0.1).with_width(0.1), props);
            "Arrow"
        }
        6 => {
            ax.add_artist_with(PathShape(reference_path()?), props);
            "PathPatch"
        }
        7 => {
            let round = BoxStyle::new(BoxShape::Round {
                rounding_size: None,
            })
            .with_pad(0.02);
            ax.add_artist_with(
                FancyBboxPatch::new((-0.025, -0.05), 0.05, 0.1, round),
                props,
            );
            "FancyBboxPatch"
        }
        _ => {
            let line = LineProps::default().linewidth(5.0).color(color);
            ax.add_line(&[-0.06, 0.0, 0.1], &[0.05, -0.05, 0.05], line)?;
            "Line2D"
        }
    };
    Ok(name)
}

fn artist_reference(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let axs = fig.subplots(3, 3)?;
    let hsv = colormaps::get("hsv")?;
    for (i, ax) in axs.as_slice().iter().enumerate() {
        let name = add_reference_artist(ax, i, hsv.sample(i as f64 / 9.0).into())?;
        ax.set_title(name);
        ax.set_aspect(Aspect::Ratio(1.0))?;
        ax.set_xlim(-0.2, 0.2)?.set_ylim(-0.2, 0.2)?;
        ax.set_axis_off();
    }
    Ok(())
}

fn ellipse_arrow(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Equal)?;
    let ellipse = Ellipse {
        angle: 35.0,
        ..Ellipse::new((2.0, 4.0), 30.0, 20.0)
    };
    let props = PatchProps::default().facecolor("none").edgecolor("b");
    let vertex = ellipse.co_vertices()[0];
    let marker = MarkerStyle::new(">".parse::<Marker>()?, FillStyle::Full)?
        .transformed(Affine2D::IDENTITY.rotate_deg(ellipse.angle));
    ax.add_patch_with(ellipse, props);
    let line = LineProps::default()
        .color("b")
        .marker_style(&marker)
        .markersize(10.0);
    ax.plot_with([vertex.x], [vertex.y], line)?;
    Ok(())
}

fn ellipse_demo_1(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let ellipses: Vec<Ellipse> = (0..250)
        .map(|_| {
            let xy = rng.uniform(2, 0.0, 10.0);
            let size = rng.uniform(2, 0.0, 1.0);
            Ellipse {
                angle: rng.next_f64() * 360.0,
                ..Ellipse::new((xy[0], xy[1]), size[0], size[1])
            }
        })
        .collect();
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 10.0)?;
    ax.set_aspect(Aspect::Equal)?;
    for ellipse in ellipses {
        let alpha = rng.next_f64();
        let face = rng.uniform(3, 0.0, 1.0);
        ax.add_artist(ellipse)
            .set_alpha(Some(alpha))
            .set_facecolor([face[0], face[1], face[2]]);
    }
    Ok(())
}

fn ellipse_demo_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(-2.2, 2.2)?.set_ylim(-2.2, 2.2)?;
    ax.set_aspect(Aspect::Equal)?;
    for angle in arange(0.0, 180.0, 45.0) {
        let ellipse = Ellipse {
            angle,
            ..Ellipse::new((0.0, 0.0), 4.0, 2.0)
        };
        ax.add_artist_with(ellipse, PatchProps::default().alpha(0.1));
    }
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_shapes/arrow_guide_1", arrow_guide_1),
    FigureCase::new("gallery_shapes/arrow_guide_2", arrow_guide_2),
    FigureCase::new("gallery_shapes/arrow_guide_3", arrow_guide_3),
    FigureCase::new("gallery_shapes/artist_reference", artist_reference).sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_shapes/ellipse_arrow", ellipse_arrow),
    FigureCase::new("gallery_shapes/ellipse_demo_1", ellipse_demo_1),
    FigureCase::new("gallery_shapes/ellipse_demo_2", ellipse_demo_2),
];
