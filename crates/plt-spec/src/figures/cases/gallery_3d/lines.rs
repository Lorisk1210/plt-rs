//! Lines, scatter plots and texts: `2dcollections3d`, `lines3d`,
//! `lorenz_attractor`, `scales3d`, `scatter3d`, `text3d` and
//! `pathpatch3d`.

use std::f64::consts::{LN_10, PI};

use plt::artist::ArtistHandle;
use plt::axes::{ScatterColors, ScatterOptions};
use plt::colors::Color;
use plt::figure::Figure;
use plt::fonts::FontProperties;
use plt::legend::LegendOptions;
use plt::lines::LineProps;
use plt::mplot3d::{Axes3D, Axis3, Scatter3DOptions, Vec3};
use plt::numeric::linspace;
use plt::patches::{Circle, PatchProps, PathShape};
use plt::text::text_path;
use plt::transforms::{Affine2D, Point};

use super::{SEED, label_xyz};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_3d/2dcollections3d", collections_2d),
    FigureCase::new("gallery_3d/lines3d", lines3d),
    FigureCase::new("gallery_3d/lorenz_attractor", lorenz_attractor),
    FigureCase::new("gallery_3d/scales3d", scales3d),
    FigureCase::new("gallery_3d/scatter3d", scatter3d),
    FigureCase::new("gallery_3d/text3d", text3d),
    FigureCase::new("gallery_3d/pathpatch3d", pathpatch3d),
];

/// Sets every view limit to `(low, high)`.
fn limit_all(ax: &Axes3D, low: f64, high: f64) -> CaseResult {
    ax.set_xlim(low, high)?
        .set_ylim(low, high)?
        .set_zlim(low, high)?;
    Ok(())
}

fn collections_2d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let x = linspace(0.0, 1.0, 100);
    let y: Vec<f64> = x.iter().map(|x| (x * 2.0 * PI).sin() / 2.0 + 0.5).collect();
    let curve = LineProps::default().label("curve in (x, y)");
    ax.plot_with(&x, &y, [0.0], Axis3::Z, curve)?;
    let colors = ["r", "g", "b", "k"];
    let mut rng = SpecRng::new(SEED);
    let x = rng.uniform(20 * colors.len(), 0.0, 1.0);
    let y = rng.uniform(20 * colors.len(), 0.0, 1.0);
    let c_list = colors
        .iter()
        .flat_map(|&c| std::iter::repeat_n(Color::from(c), 20))
        .collect();
    let options = Scatter3DOptions {
        zdir: Axis3::Y,
        scatter: ScatterOptions {
            colors: ScatterColors::PerPoint(c_list),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(&x, &y, [0.0], options)?
        .set_label("points in (x, z)");
    ax.legend(LegendOptions::default())?;
    limit_all(&ax, 0.0, 1.0)?;
    label_xyz(&ax, ["X", "Y", "Z"]);
    ax.view_init(20.0, -35.0, 0.0);
    Ok(())
}

fn lines3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let theta = linspace(-4.0 * PI, 4.0 * PI, 100);
    let z = linspace(-2.0, 2.0, 100);
    let r: Vec<f64> = z.iter().map(|z| z.powi(2) + 1.0).collect();
    let x: Vec<f64> = r.iter().zip(&theta).map(|(r, t)| r * t.sin()).collect();
    let y: Vec<f64> = r.iter().zip(&theta).map(|(r, t)| r * t.cos()).collect();
    let curve = LineProps::default().label("parametric curve");
    ax.plot_with(&x, &y, &z, Axis3::Z, curve)?;
    ax.legend(LegendOptions::default())?;
    Ok(())
}

/// The time derivatives of the Lorenz system at `[x, y, z]`.
fn lorenz([x, y, z]: Vec3) -> Vec3 {
    let (s, r, b) = (10.0, 28.0, 2.667);
    [s * (y - x), r * x - y - x * z, x * y - b * z]
}

fn lorenz_attractor(fig: &Figure) -> CaseResult {
    let dt = 0.01;
    let steps = 10_000;
    let mut xyzs: Vec<Vec3> = Vec::with_capacity(steps + 1);
    xyzs.push([0.0, 1.0, 1.05]);
    for i in 0..steps {
        let (p, d) = (xyzs[i], lorenz(xyzs[i]));
        xyzs.push([0, 1, 2].map(|k| p[k] + d[k] * dt));
    }
    let [x, y, z] = [0, 1, 2].map(|k| xyzs.iter().map(|p| p[k]).collect::<Vec<f64>>());
    let ax = fig.subplot_3d();
    ax.plot_with(&x, &y, &z, Axis3::Z, LineProps::default().linewidth(0.5))?;
    label_xyz(&ax, ["X Axis", "Y Axis", "Z Axis"]);
    ax.set_title("Lorenz Attractor");
    Ok(())
}

fn scales3d(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 1.0, 400);
    let y: Vec<f64> = x.iter().map(|x| 10_f64.powf(2.0 * x)).collect();
    let z: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(x, y)| {
            let phase = 2.0 * PI * (y - 1.0) / (2.0 * LN_10);
            phase.sin() * x.powi(2) * 10.0
        })
        .collect();
    let ax = fig.subplot_3d();
    ax.plot(&x, &y, &z)?;
    label_xyz(
        &ax,
        ["Time (linear)", "Frequency, Hz (log)", "Amplitude (symlog)"],
    );
    ax.set_yscale("log")?;
    ax.set_zscale("symlog")?;
    Ok(())
}

fn scatter3d(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let ax = fig.subplot_3d();
    let n = 100;
    for (marker, zlow, zhigh) in [("o", -50.0, -25.0), ("^", -30.0, -5.0)] {
        let xs = rng.uniform(n, 23.0, 32.0);
        let ys = rng.uniform(n, 0.0, 100.0);
        let zs = rng.uniform(n, zlow, zhigh);
        let options = Scatter3DOptions {
            scatter: ScatterOptions {
                marker: Some(marker.parse()?),
                ..ScatterOptions::default()
            },
            ..Scatter3DOptions::default()
        };
        ax.scatter_with(&xs, &ys, &zs, options)?;
    }
    label_xyz(&ax, ["X Label", "Y Label", "Z Label"]);
    Ok(())
}

fn text3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    // (anchor, direction, how the example prints the direction)
    let texts: [(Vec3, Vec3, &str); 6] = [
        ([1.0, 2.0, 10.0], [0.0; 3], "None"),
        ([4.0, 5.0, 3.0], Axis3::X.unit(), "x"),
        ([4.0, 8.0, 8.0], Axis3::Y.unit(), "y"),
        ([9.0, 10.0, 9.0], Axis3::Z.unit(), "z"),
        ([4.0, 1.0, 1.0], [1.0, 1.0, 0.0], "(1, 1, 0)"),
        ([1.0, 2.0, 8.0], [1.0, 1.0, 1.0], "(1, 1, 1)"),
    ];
    for ([x, y, z], direction, zdir) in texts {
        let label = format!("({x}, {y}, {z}), dir={zdir}");
        ax.text_along(x, y, z, label, direction);
    }
    ax.text(9.0, 0.0, 0.0, "red", None).set_color("red");
    ax.text2d(0.05, 0.95, "2D Text");
    limit_all(&ax, 0.0, 10.0)?;
    label_xyz(&ax, ["X axis", "Y axis", "Z axis"]);
    Ok(())
}

/// `pathpatch3d`'s `text3d`: the black outline of `text`, `size` units
/// tall and turned by `angle`, laid in the plane normal to `zdir` through
/// `[x, y, z]`.
fn text_outline(
    ax: &Axes3D,
    [x, y, z]: Vec3,
    text: &str,
    zdir: Axis3,
    size: f64,
    angle: f64,
) -> CaseResult {
    let ((u, v), w) = match zdir {
        Axis3::X => ((y, z), x),
        Axis3::Y => ((x, z), y),
        Axis3::Z => ((x, y), z),
    };
    let font = FontProperties::from_rc(&plt::rc::params());
    let outline = text_path(Point::new(0.0, 0.0), text, Some(size), &font)?
        .transformed(&Affine2D::rotation(angle).translate(u, v));
    let props = PatchProps::default().edgecolor("none").facecolor("k");
    ax.add_path_patch3d(&PathShape(outline), props, &[w], zdir)?;
    Ok(())
}

fn pathpatch3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let circle = Circle::new((5.0, 5.0), 3.0);
    ax.add_path_patch3d(&circle, PatchProps::default(), &[0.0], Axis3::X)?;
    text_outline(&ax, [4.0, -2.0, 0.0], "X-axis", Axis3::Z, 0.5, 0.0)?;
    text_outline(&ax, [12.0, 4.0, 0.0], "Y-axis", Axis3::Z, 0.5, PI / 2.0)?;
    text_outline(&ax, [12.0, 10.0, 4.0], "Z-axis", Axis3::Y, 0.5, PI / 2.0)?;
    let formula = r"$G_{\mu\nu} + \Lambda g_{\mu\nu} = \frac{8\pi G}{c^4} T_{\mu\nu}  $";
    text_outline(&ax, [1.0, 5.0, 0.0], formula, Axis3::Z, 1.0, 0.0)?;
    limit_all(&ax, 0.0, 10.0)?;
    Ok(())
}
