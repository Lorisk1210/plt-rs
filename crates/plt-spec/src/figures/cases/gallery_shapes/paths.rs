//! Paths drawn as patches: compound paths, holes made by orientation,
//! Bézier curves and a clipped image (`compound_path`, `donut`,
//! `path_patch`, `quad_bezier` and `dolphin`).

use std::error::Error;
use std::f64::consts::PI;

use plt::axes::{AxisMode, ImshowOptions};
use plt::colors::{ColorizerOptions, colormaps};
use plt::image::{Extent, Origin};
use plt::numeric::Grid;
use plt::patches::{Circle, PathShape};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{AnnotateOptions, HAlign, VAlign};
use plt::transforms::Point;
use plt_core::{Affine2D, Path, PathCode};

use super::SEED;
use crate::figures::cases::common::path_patch_curve;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

use PathCode::{ClosePoly, Curve3, Curve4, LineTo, MoveTo};

/// Points from coordinate pairs.
fn points(coords: &[(f64, f64)]) -> Vec<Point> {
    coords.iter().copied().map(Point::from).collect()
}

fn compound_path(fig: &Figure) -> CaseResult {
    let vertices = points(&[
        (1.0, 1.0),
        (1.0, 2.0),
        (2.0, 2.0),
        (2.0, 1.0),
        (0.0, 0.0),
        (4.0, 4.0),
        (5.0, 5.0),
        (5.0, 4.0),
        (0.0, 0.0),
    ]);
    let codes = vec![
        MoveTo, LineTo, LineTo, LineTo, ClosePoly, MoveTo, LineTo, LineTo, ClosePoly,
    ];
    let ax = fig.subplot();
    let outline = PatchProps::default().facecolor("none").edgecolor("green");
    ax.add_patch_with(PathShape(Path::new(vertices, codes)?), outline);
    ax.set_title("A compound path");
    ax.autoscale_view(None);
    Ok(())
}

/// The dolphin outline, in SVG path syntax.
const DOLPHIN: &str = "
M -0.59739425,160.18173 C -0.62740401,160.18885 -0.57867129,160.11183
-0.57867129,160.11183 C -0.57867129,160.11183 -0.5438361,159.89315
-0.39514638,159.81496 C -0.24645668,159.73678 -0.18316813,159.71981
-0.18316813,159.71981 C -0.18316813,159.71981 -0.10322971,159.58124
-0.057804323,159.58725 C -0.029723983,159.58913 -0.061841603,159.60356
-0.071265813,159.62815 C -0.080250183,159.65325 -0.082918513,159.70554
-0.061841203,159.71248 C -0.040763903,159.7194 -0.0066711426,159.71091
0.077336307,159.73612 C 0.16879567,159.76377 0.28380306,159.86448
0.31516668,159.91533 C 0.3465303,159.96618 0.5011127,160.1771
0.5011127,160.1771 C 0.63668998,160.19238 0.67763022,160.31259
0.66556395,160.32668 C 0.65339985,160.34212 0.66350443,160.33642
0.64907098,160.33088 C 0.63463742,160.32533 0.61309688,160.297
0.5789627,160.29339 C 0.54348657,160.28968 0.52329693,160.27674
0.50728856,160.27737 C 0.49060916,160.27795 0.48965803,160.31565
0.46114204,160.33673 C 0.43329696,160.35786 0.4570711,160.39871
0.43309565,160.40685 C 0.4105108,160.41442 0.39416631,160.33027
0.3954995,160.2935 C 0.39683269,160.25672 0.43807996,160.21522
0.44567915,160.19734 C 0.45327833,160.17946 0.27946869,159.9424
-0.061852613,159.99845 C -0.083965233,160.0427 -0.26176109,160.06683
-0.26176109,160.06683 C -0.30127962,160.07028 -0.21167141,160.09731
-0.24649368,160.1011 C -0.32642366,160.11569 -0.34521187,160.06895
-0.40622293,160.0819 C -0.467234,160.09485 -0.56738444,160.17461
-0.59739425,160.18173
";

/// The example's parse of [`DOLPHIN`]: each command letter takes the
/// number of points of its code, and the outline is shifted down by 160.
fn dolphin_path() -> Result<Path, Box<dyn Error>> {
    let mut vertices = Vec::new();
    let mut codes = Vec::new();
    let mut parts = DOLPHIN.split_whitespace();
    while let Some(command) = parts.next() {
        let (code, count) = match command {
            "M" => (MoveTo, 1),
            "C" => (Curve4, 3),
            "L" => (LineTo, 1),
            other => return Err(format!("unknown path command {other:?}").into()),
        };
        for _ in 0..count {
            let pair = parts.next().ok_or("truncated dolphin path")?;
            let (x, y) = pair.split_once(',').ok_or("malformed dolphin point")?;
            vertices.push(Point::new(x.parse()?, y.parse::<f64>()? - 160.0));
            codes.push(code);
        }
    }
    Ok(Path::new(vertices, codes)?)
}

fn dolphin(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let r = rng.uniform(50, 0.0, 1.0);
    let t = rng.uniform(50, 0.0, 1.0);
    let (x, y): (Vec<f64>, Vec<f64>) = r
        .iter()
        .zip(&t)
        .map(|(r, t)| {
            let t = t * PI * 2.0;
            (r * t.cos(), r * t.sin())
        })
        .unzip();
    let ax = fig.subplot();
    let circle = Circle::new((0.0, 0.0), 1.0);
    let ring = PatchProps::default()
        .facecolor("none")
        .edgecolor((0.0, 0.8, 0.8))
        .linewidth(3.0)
        .alpha(0.5);
    ax.add_patch_with(circle.clone(), ring);
    let noise = Grid::from_vec(100, 100, rng.uniform(100 * 100, 0.0, 1.0))?;
    let options = ImshowOptions {
        origin: Some(Origin::Lower),
        extent: Some(Extent::new(-1.0, 1.0, -1.0, 1.0)),
        ..ImshowOptions::default()
    }
    .colorizer(ColorizerOptions::default().cmap(colormaps::get("winter")?))
    .interpolation("spline36".parse()?);
    ax.imshow(noise, options)?
        .set_clip_patch(&circle, ax.trans_data());
    let dots = LineProps::fmt("o")?.color((0.9, 0.9, 1.0)).alpha(0.8);
    ax.plot_with(x, y, dots)?;
    let path = dolphin_path()?;
    let gray = |level: f64| {
        PatchProps::default()
            .facecolor((level, level, level))
            .edgecolor((0.0, 0.0, 0.0))
    };
    let turned = path.transformed(&Affine2D::IDENTITY.rotate_deg(60.0));
    ax.add_patch_with(PathShape(path), gray(0.6));
    ax.add_patch_with(PathShape(turned), gray(0.5));
    Ok(())
}

/// `make_circle`: a circle of radius `r` through angles 0, 0.01, ...
fn circle_points(r: f64) -> Vec<Point> {
    arange(0.0, PI * 2.0, 0.01)
        .into_iter()
        .map(|t| Point::new(r * t.cos(), r * t.sin()))
        .collect()
}

/// `wise`: the name of an orientation.
fn wise(counterclockwise: bool) -> &'static str {
    if counterclockwise { "CCW" } else { "CW" }
}

fn donut(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let inside_vertices = circle_points(0.5);
    let outside_vertices = circle_points(1.0);
    let ring_codes: Vec<PathCode> = (0..inside_vertices.len())
        .map(|i| if i == 0 { MoveTo } else { LineTo })
        .collect();
    let oriented = |vertices: &[Point], ccw: bool| -> Vec<Point> {
        if ccw {
            vertices.to_vec()
        } else {
            vertices.iter().rev().copied().collect()
        }
    };
    for (i, (inside, outside)) in [(true, true), (true, false), (false, true), (false, false)]
        .into_iter()
        .enumerate()
    {
        let shift = i as f64 * 2.5;
        let vertices = [
            oriented(&outside_vertices, outside),
            oriented(&inside_vertices, inside),
        ]
        .concat()
        .into_iter()
        .map(|p| Point::new(p.x + shift, p.y))
        .collect();
        let path = Path::new(vertices, [ring_codes.clone(), ring_codes.clone()].concat())?;
        let chocolate = PatchProps::default()
            .facecolor("#885500")
            .edgecolor("black");
        ax.add_patch_with(PathShape(path), chocolate);
        let label = format!("Outside {},\nInside {}", wise(outside), wise(inside));
        ax.annotate(label, (shift, -1.5), AnnotateOptions::default())?
            .set_valign(VAlign::Top)
            .set_halign(HAlign::Center);
    }
    ax.set_xlim(-2.0, 10.0).set_ylim(-3.0, 2.0);
    ax.set_aspect(Aspect::Ratio(1.0))?;
    ax.set_title("Mmm, donuts!");
    Ok(())
}

fn path_patch(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let curve = path_patch_curve()?;
    let (x, y): (Vec<f64>, Vec<f64>) = curve.vertices().iter().map(|p| (p.x, p.y)).unzip();
    let red = PatchProps::default().facecolor("r").alpha(0.5);
    ax.add_patch_with(PathShape(curve), red);
    ax.plot_fmt(x, y, "go-")?;
    ax.grid(true);
    ax.axis_mode(AxisMode::Equal);
    Ok(())
}

fn quad_bezier(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let curve = Path::new(
        points(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 0.0)]),
        vec![MoveTo, Curve3, Curve3, ClosePoly],
    )?;
    ax.add_patch_with(PathShape(curve), PatchProps::default().facecolor("none"));
    ax.plot_fmt([0.75], [0.25], "ro")?;
    ax.set_title("The red point should be on the path");
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_shapes/compound_path", compound_path),
    FigureCase::new("gallery_shapes/dolphin", dolphin).sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_shapes/donut", donut),
    FigureCase::new("gallery_shapes/path_patch", path_patch),
    FigureCase::new("gallery_shapes/quad_bezier", quad_bezier),
];
