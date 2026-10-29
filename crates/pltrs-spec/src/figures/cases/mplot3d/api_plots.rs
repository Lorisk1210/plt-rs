//! `figures/cases_mplot3d_api.py`, plotting part: errorbar,
//! `fill_between`, `add_collection3d`, `tricontour`, `tricontourf` and
//! extended contours.

use std::f64::consts::PI;
use std::num::NonZeroUsize;

use pltrs::collections::{CollectionProps, CollectionSpec};
use pltrs::contour::{ContourOptions, Levels};
use pltrs::lines::MarkEvery;
use pltrs::mplot3d::{Axis3, Contour3DOptions, Errorbar3DOptions, FillBetween3DOptions, FillMode};
use pltrs::numeric::linspace;
use pltrs::transforms::Point;
use pltrs::tri::Triangulation;

use super::contours::levels;
use super::surface;
use crate::figures::FigureCase;

/// `_polar_points()`: five rings of twelve points, each ring turned by
/// its radius, with `z = cos(3x) sin(2y) + x`.
pub(super) fn polar_points() -> [Vec<f64>; 3] {
    let step = 2.0 * PI / 12.0;
    let points: Vec<(f64, f64)> = linspace(0.25, 1.0, 5)
        .into_iter()
        .flat_map(|r| {
            (0..12).map(move |i| {
                let a = f64::from(i) * step + r;
                (r * a.cos(), r * a.sin())
            })
        })
        .collect();
    [
        points.iter().map(|p| p.0).collect(),
        points.iter().map(|p| p.1).collect(),
        points
            .iter()
            .map(|&(x, y)| (3.0 * x).cos() * (2.0 * y).sin() + x)
            .collect(),
    ]
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mplot3d/api_errorbar", |fig| {
        let ax = fig.subplot_3d();
        let t = linspace(0.0, 2.0 * PI, 9);
        let x: Vec<f64> = t.iter().map(|t| t.cos()).collect();
        let y: Vec<f64> = t.iter().map(|t| t.sin()).collect();
        let z: Vec<f64> = t.iter().map(|t| t / 2.0).collect();
        let yerr: Vec<f64> = [0.1, 0.2].repeat(4).into_iter().chain([0.1]).collect();
        let options = Errorbar3DOptions {
            errors: [Some(0.15.into()), Some(yerr.into()), Some(0.4.into())],
            fmt: "o".parse()?,
            capsize: Some(3.0),
            lolims: [Vec::new(), Vec::new(), [true, false, false].repeat(3)],
            errorevery: Some(MarkEvery::Every { start: 1, step: 2 }),
            ecolor: Some("tab:red".into()),
            ..Errorbar3DOptions::default()
        };
        ax.errorbar(&x, &y, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_fill_between_quad", |fig| {
        let ax = fig.subplot_3d();
        let t = linspace(0.0, 2.0 * PI, 30);
        let (c, s): (Vec<f64>, Vec<f64>) = t.iter().map(|t| (t.cos(), t.sin())).unzip();
        let top: Vec<f64> = t.iter().map(|t| t / 2.0).collect();
        let bottom: Vec<f64> = t.iter().map(|t| 0.2 * (3.0 * t).sin()).collect();
        let options = FillBetween3DOptions {
            props: CollectionProps::default().alpha(0.6),
            ..FillBetween3DOptions::default()
        };
        ax.fill_between((&c, &s, &top), (&c, &s, &bottom), options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_fill_between_polygon", |fig| {
        let ax = fig.subplot_3d();
        let x = linspace(0.0, 4.0, 25);
        let wave: Vec<f64> = x.iter().map(|x| (2.0 * x).sin() + 1.5).collect();
        let options = FillBetween3DOptions {
            mask: Some(x.iter().map(|&x| x < 1.5 || x > 2.2).collect()),
            props: CollectionProps::default()
                .facecolor("tab:orange")
                .edgecolor("k")
                .linewidth(0.5),
            ..FillBetween3DOptions::default()
        };
        ax.fill_between((&x, &[1.0], &wave), (&x, &[1.0], &[0.0]), options)?;
        let cosine: Vec<f64> = x.iter().map(|x| x.cos()).collect();
        let options = FillBetween3DOptions {
            mode: FillMode::Polygon,
            props: CollectionProps::default().facecolor("tab:blue"),
            ..FillBetween3DOptions::default()
        };
        ax.fill_between((&x, &[0.0], &cosine), (&x, &[0.0], &[0.0]), options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_add_collection3d", |fig| {
        let ax = fig.subplot_3d();
        let x = linspace(0.0, 3.0, 10);
        let polygons: Vec<Vec<Point>> = (0..3)
            .map(|k| {
                let k = f64::from(k);
                std::iter::once(Point::new(0.0, 0.0))
                    .chain(x.iter().map(|&x| Point::new(x, (x + k).sin().abs() + 0.2)))
                    .chain(std::iter::once(Point::new(3.0, 0.0)))
                    .collect()
            })
            .collect();
        let mut props = CollectionProps::default().alpha(0.6);
        props.facecolors = Some(vec!["r".into(), "g".into(), "b".into()]);
        let polys = CollectionSpec::polygons(polygons).with_props(props);
        ax.add_collection3d(polys, &[0.0, 1.0, 2.0], Axis3::Y, true)?;
        let lines: Vec<Vec<Point>> = (0..2)
            .map(|k| {
                let k = f64::from(k);
                x.iter().map(|&x| Point::new(x, (x + k).cos())).collect()
            })
            .collect();
        let lines = CollectionSpec::lines(lines)
            .with_props(CollectionProps::default().edgecolor("k").linewidth(1.5));
        ax.add_collection3d(lines, &[3.5, 4.0], Axis3::X, true)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_tricontour", |fig| {
        let ax = fig.subplot_3d();
        let [x, y, z] = polar_points();
        let tri = Triangulation::delaunay(&x, &y)?;
        let options = Contour3DOptions {
            contour: levels(8, "viridis")?,
            ..Contour3DOptions::default()
        };
        ax.tricontour(&tri, &z, options)?;
        let options = Contour3DOptions {
            zdir: Axis3::X,
            offset: Some(-1.2),
            contour: ContourOptions {
                levels: Levels::Count(6),
                colors: Some(vec!["k".into()]),
                ..ContourOptions::default()
            },
            ..Contour3DOptions::default()
        };
        ax.tricontour(&tri, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_tricontourf", |fig| {
        let ax = fig.subplot_3d();
        let [x, y, z] = polar_points();
        let tri = Triangulation::delaunay(&x, &y)?;
        let options = Contour3DOptions {
            contour: levels(7, "coolwarm")?,
            ..Contour3DOptions::default()
        };
        ax.tricontourf(&tri, &z, options)?;
        let options = Contour3DOptions {
            offset: Some(-2.0),
            contour: ContourOptions {
                alpha: Some(0.5),
                ..levels(5, "Greys")?
            },
            ..Contour3DOptions::default()
        };
        ax.tricontourf(&tri, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_contour_extend3d", |fig| {
        let (x, y, z) = surface(30);
        let options = Contour3DOptions {
            extend3d: NonZeroUsize::new(4),
            contour: levels(6, "plasma")?,
            ..Contour3DOptions::default()
        };
        fig.subplot_3d().contour(&x, &y, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
];
