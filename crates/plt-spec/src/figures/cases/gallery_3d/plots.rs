//! `fill_between`, error bars, stems, quivers and polygons:
//! `fillbetween3d`, `fillunder3d`, `errorbar3d`, `stem3d_demo_1` to `_3`,
//! `quiver3d` and `polys3d`.

use std::f64::consts::PI;

use plt::axes::Adjustable;
use plt::collections::CollectionProps;
use plt::colors::colormaps;
use plt::figure::Figure;
use plt::lines::MarkEvery;
use plt::mplot3d::{
    Aspect3D, Axis3, Errorbar3DOptions, FillBetween3DOptions, Poly3DOptions, Quiver3DOptions,
    Stem3DOptions, Vec3,
};
use plt::numeric::{arange, linspace, linspace_with};

use super::gamma::gamma;
use super::label_xyz;
use crate::figures::cases::common::{map, subplots_3d};
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_3d/fillbetween3d", fillbetween3d),
    FigureCase::new("gallery_3d/fillunder3d", fillunder3d),
    FigureCase::new("gallery_3d/errorbar3d", errorbar3d),
    FigureCase::new("gallery_3d/stem3d_demo_1", stem3d_demo_1),
    FigureCase::new("gallery_3d/stem3d_demo_2", stem3d_demo_2),
    FigureCase::new("gallery_3d/stem3d_demo_3", stem3d_demo_3),
    FigureCase::new("gallery_3d/quiver3d", quiver3d),
    FigureCase::new("gallery_3d/polys3d", polys3d),
];

fn fillbetween3d(fig: &Figure) -> CaseResult {
    let theta = linspace(0.0, 2.0 * PI, 50);
    let x1 = map(&theta, f64::cos);
    let y1 = map(&theta, f64::sin);
    let z1 = map(&theta, |t| 0.1 * (6.0 * t).sin());
    let x2 = map(&theta, |t| 0.6 * t.cos());
    let y2 = map(&theta, |t| 0.6 * t.sin());
    let options = FillBetween3DOptions {
        // The alpha fades the faces only; matplotlib also fades the edges
        // here because `edgecolor` follows `alpha` among the keywords.
        props: CollectionProps::default()
            .alpha(0.5)
            .edgecolor((0.0, 0.0, 0.0, 0.5)),
        ..FillBetween3DOptions::default()
    };
    fig.subplot_3d()
        .fill_between((&x1, &y1, &z1), (&x2, &y2, &[2.0]), options)?;
    Ok(())
}

fn fillunder3d(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 10.0, 31);
    let lambdas: Vec<f64> = (1..9).map(f64::from).collect();
    let ax = fig.subplot_3d();
    let viridis_r = colormaps::get("viridis_r")?;
    let shades = linspace(0.0, 1.0, lambdas.len());
    for (&lambda, shade) in lambdas.iter().zip(shades) {
        let probability = map(&x, |x| lambda.powf(x) * (-lambda).exp() / gamma(x + 1.0));
        let options = FillBetween3DOptions {
            props: CollectionProps::default()
                .facecolor(viridis_r.sample(shade))
                .alpha(0.7),
            ..FillBetween3DOptions::default()
        };
        ax.fill_between(
            (&x, &[lambda], &probability),
            (&x, &[lambda], &[0.0]),
            options,
        )?;
    }
    ax.set_xlim(0.0, 10.0)?
        .set_ylim(1.0, 9.0)?
        .set_zlim(0.0, 0.35)?;
    label_xyz(&ax, ["x", r"$\lambda$", "probability"]);
    Ok(())
}

fn errorbar3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let t = arange(0.0, 2.0 * PI + 0.1, 0.01);
    let x = map(&t, f64::sin);
    let y = map(&t, |t| (3.0 * t).cos());
    let z = map(&t, |t| (5.0 * t).sin());
    let estep = 15;
    let limits = |phase: usize| -> Vec<bool> {
        (0..t.len())
            .map(|i| i % estep == 0 && (i / estep) % 3 == phase)
            .collect()
    };
    let options = Errorbar3DOptions {
        errors: [None, None, Some(0.2.into())],
        uplims: [Vec::new(), Vec::new(), limits(0)],
        lolims: [Vec::new(), Vec::new(), limits(2)],
        errorevery: Some(MarkEvery::Every {
            start: 0,
            step: estep as i64,
        }),
        ..Errorbar3DOptions::default()
    };
    ax.errorbar(&x, &y, &z, options)?;
    label_xyz(&ax, ["X label", "Y label", "Z label"]);
    Ok(())
}

/// `stem3d_demo`'s data: a unit circle rising with its angle.
fn stem_curve() -> [Vec<f64>; 3] {
    let theta = linspace(0.0, 2.0 * PI, 50);
    [
        map(&theta, |t| (t - PI / 2.0).cos()),
        map(&theta, |t| (t - PI / 2.0).sin()),
        theta,
    ]
}

fn stem3d_demo_1(fig: &Figure) -> CaseResult {
    let [x, y, z] = stem_curve();
    subplots_3d(fig, 1, 1)?[0].stem(&x, &y, &z, Stem3DOptions::default())?;
    Ok(())
}

fn stem3d_demo_2(fig: &Figure) -> CaseResult {
    let [x, y, z] = stem_curve();
    let options = Stem3DOptions {
        linefmt: "grey".parse()?,
        markerfmt: "D".parse()?,
        bottom: PI,
        ..Stem3DOptions::default()
    };
    let stem = subplots_3d(fig, 1, 1)?[0].stem(&x, &y, &z, options)?;
    stem.markerline.set_markerfacecolor("none");
    Ok(())
}

fn stem3d_demo_3(fig: &Figure) -> CaseResult {
    let [x, y, z] = stem_curve();
    let options = Stem3DOptions {
        bottom: -1.0,
        orientation: Axis3::X,
        ..Stem3DOptions::default()
    };
    let ax = &subplots_3d(fig, 1, 1)?[0];
    ax.stem(&x, &y, &z, options)?;
    label_xyz(ax, ["x", "y", "z"]);
    Ok(())
}

fn quiver3d(fig: &Figure) -> CaseResult {
    let grid = arange(-0.8, 1.0, 0.2);
    let heights = arange(-0.8, 1.0, 0.8);
    // np.meshgrid(grid, grid, heights), raveled: y slowest, then x, then z.
    let mut positions: Vec<Vec3> = Vec::new();
    for &y in &grid {
        for &x in &grid {
            positions.extend(heights.iter().map(|&z| [x, y, z]));
        }
    }
    let vectors: Vec<Vec3> = positions
        .iter()
        .map(|&[x, y, z]| {
            let (px, py, pz) = (PI * x, PI * y, PI * z);
            [
                px.sin() * py.cos() * pz.cos(),
                -px.cos() * py.sin() * pz.cos(),
                (2.0_f64 / 3.0).sqrt() * px.cos() * py.cos() * pz.sin(),
            ]
        })
        .collect();
    let options = Quiver3DOptions {
        length: 0.1,
        normalize: true,
        ..Quiver3DOptions::default()
    };
    fig.subplot_3d().quiver(&positions, &vectors, options)?;
    Ok(())
}

fn polys3d(fig: &Figure) -> CaseResult {
    let angles = linspace_with(0.0, 2.0 * PI, 6, false);
    // The hexagon, closed by repeating its first vertex.
    let corners: Vec<(f64, f64)> = angles
        .iter()
        .chain(&angles[..1])
        .map(|a| (a.cos(), a.sin()))
        .collect();
    let verts: Vec<Vec<Vec3>> = [-3.0, -2.0, -1.0]
        .iter()
        .map(|&z| corners.iter().map(|&(x, y)| [x * z, y * z, z]).collect())
        .collect();
    let ax = fig.subplot_3d();
    let options = Poly3DOptions {
        props: CollectionProps::default().alpha(0.7),
        ..Poly3DOptions::default()
    };
    ax.add_poly_collection3d_with(verts, options)?;
    ax.set_aspect(Aspect3D::EqualXY, Adjustable::Box)?;
    Ok(())
}
