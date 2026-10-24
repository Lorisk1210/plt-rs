//! Collection cases of `figures/cases_patches.py`.

use std::f64::consts::PI;

use plt::collections::{
    CollectionPatch, CollectionProps, CollectionSpec, ColorMapping, EdgeColors, EllipseUnits,
    TriMeshSpec,
};
use plt::colors::{ScalarMappable, colormaps};
use plt::orientation::Orientation;
use plt::patches::{Circle, Ellipse, Rectangle, RegularPolygon, Wedge};
use plt::prelude::*;
use plt::transforms::Point;
use plt::tri::Triangulation;

use super::square_canvas;
use crate::figures::cases::common::colors;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `cmap=name` with `set_array(values)`.
fn mapping(cmap: &str, values: &[f64]) -> Result<ColorMapping, Box<dyn std::error::Error>> {
    Ok(ColorMapping {
        values: values.iter().map(|&v| Some(v)).collect(),
        mappable: ScalarMappable::new(colormaps::get(cmap)?),
    })
}

/// `xy`: the 4 × 3 grid of `ellipsecollection_units`.
fn grid_points() -> Vec<Point> {
    (0..4)
        .flat_map(|x| (0..3).map(move |y| Point::new(f64::from(x), f64::from(y))))
        .collect()
}

/// The two panels of `ellipsecollection_units`.
fn ellipse_panels(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let xy = grid_points();
    for (ax, (units, name)) in axes
        .as_slice()
        .iter()
        .zip([(EllipseUnits::Xy, "xy"), (EllipseUnits::Points, "points")])
    {
        let widths: Vec<f64> = xy
            .iter()
            .map(|p| match units {
                EllipseUnits::Xy => 0.3 + 0.1 * p.x,
                _ => 10.0 + 5.0 * p.x,
            })
            .collect();
        let heights: Vec<f64> = widths.iter().map(|w| w / 2.0).collect();
        let angles: Vec<f64> = xy.iter().map(|p| 30.0 * p.y).collect();
        let values: Vec<f64> = xy.iter().map(|p| p.x + p.y).collect();
        let ellipses = CollectionSpec::ellipses(&widths, &heights, &angles, units)
            .with_offsets(xy.clone(), ax.trans_data())
            .with_mapping(mapping("viridis", &values)?);
        ax.add_collection(ellipses);
        ax.set_xlim(-1.0, 4.0)?.set_ylim(-1.0, 3.0)?;
        ax.set_title(name);
    }
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("patches/patchcollection_cmap", |fig| {
        let ax = square_canvas(fig)?;
        let plain = PatchProps::default;
        let shapes = [
            CollectionPatch::new(&Circle::new((2.0, 2.0), 1.5), plain()),
            CollectionPatch::new(&Rectangle::new((5.0, 1.0), 3.0, 2.0), plain()),
            CollectionPatch::new(&Wedge::new((3.0, 7.0), 2.0, 30.0, 300.0), plain()),
            CollectionPatch::new(&RegularPolygon::new((7.5, 7.0), 6, 1.8), plain()),
            CollectionPatch::new(
                &Ellipse {
                    angle: 45.0,
                    ..Ellipse::new((5.0, 4.5), 2.0, 1.0)
                },
                plain(),
            ),
        ];
        let collection = CollectionSpec::patches(&shapes, false)?
            .with_props(CollectionProps::default().alpha(0.8).edgecolor("k"))
            .with_mapping(mapping("viridis", &arange(0.0, 5.0, 1.0))?);
        ax.add_collection(collection);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/patchcollection_match_original", |fig| {
        let ax = square_canvas(fig)?;
        let shapes = [
            CollectionPatch::new(
                &Circle::new((3.0, 3.0), 2.0),
                PatchProps::default().facecolor("tab:red").hatch("//"),
            ),
            CollectionPatch::new(
                &Rectangle::new((5.0, 5.0), 3.0, 3.0),
                PatchProps::default()
                    .facecolor("tab:blue")
                    .edgecolor("k")
                    .linewidth(3.0),
            ),
        ];
        ax.add_collection(CollectionSpec::patches(&shapes, true)?);
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("patches/linecollection_styles", |fig| {
        let ax = fig.subplot();
        let x = linspace(0.0, 4.0 * PI, 60);
        let segments = (0..5)
            .map(|i| {
                x.iter()
                    .map(|&x| Point::new(x, x.sin() + f64::from(i)))
                    .collect()
            })
            .collect();
        let props = CollectionProps {
            edgecolors: Some(EdgeColors::Colors(colors(&["C0", "C1", "C2", "C3", "C4"]))),
            linewidths: Some(vec![0.5, 1.0, 2.0, 3.0, 4.0]),
            linestyles: Some(vec![
                LineStyle::Solid,
                LineStyle::Dashed,
                LineStyle::Dotted,
                LineStyle::DashDot,
                LineStyle::Solid,
            ]),
            ..CollectionProps::default()
        };
        ax.add_collection(CollectionSpec::lines(segments).with_props(props));
        // `ax.autoscale_view()`.
        ax.autoscale(None, None);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/linecollection_array", |fig| {
        let ax = fig.subplot();
        let x = linspace(0.0, 4.0 * PI, 100);
        let points: Vec<Point> = x.iter().map(|&x| Point::new(x, x.sin())).collect();
        let segments = points.windows(2).map(<[Point]>::to_vec).collect();
        let values: Vec<f64> = x[..x.len() - 1].iter().map(|x| x.cos()).collect();
        let lines = CollectionSpec::lines(segments)
            .with_props(CollectionProps::default().linewidth(3.0))
            .with_mapping(mapping("plasma", &values)?);
        ax.add_collection(lines);
        ax.autoscale(None, None);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/polycollection", |fig| {
        let ax = fig.subplot();
        let p = Point::new;
        let verts = vec![
            vec![p(0.0, 0.0), p(1.0, 0.0), p(0.5, 1.0)],
            vec![p(1.0, 1.0), p(2.0, 1.0), p(2.0, 2.0), p(1.0, 2.0)],
            vec![p(2.5, 0.0), p(3.5, 0.5), p(3.0, 1.5), p(2.2, 1.0)],
        ];
        let props = CollectionProps {
            facecolors: Some(colors(&["tab:red", "tab:green", "tab:blue"])),
            ..CollectionProps::default()
        }
        .edgecolor("k")
        .linewidth(2.0)
        .alpha(0.6);
        ax.add_collection(CollectionSpec::polygons(verts).with_props(props));
        ax.autoscale(None, None);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/polycollection_nan_vertices", |fig| {
        let ax = fig.subplot();
        let mut verts = vec![Point::new(f64::NAN, f64::NAN)];
        verts.extend(
            linspace(0.0, 1.0, 20)
                .into_iter()
                .map(|x| Point::new(x, 0.5 + 0.3 * (2.0 * PI * x).sin())),
        );
        verts.push(Point::new(f64::NAN, f64::NAN));
        let props = CollectionProps::default()
            .facecolor("tab:orange")
            .edgecolor("k")
            .linewidth(0.5);
        ax.add_collection(CollectionSpec::polygons(vec![verts]).with_props(props));
        ax.set_xlim(-0.1, 1.1)?.set_ylim(0.0, 1.0)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("patches/ellipsecollection_units", ellipse_panels).sized(6.0, 3.0, 100.0),
    FigureCase::new("patches/circlecollection", |fig| {
        let ax = fig.subplot();
        let mut g = SpecRng::new(51);
        let xs = g.uniform(20, 0.0, 1.0);
        let ys = g.uniform(20, 0.0, 1.0);
        let offsets = xs
            .into_iter()
            .zip(ys)
            .map(|(x, y)| Point::new(x, y))
            .collect();
        let sizes = g.uniform(20, 20.0, 400.0);
        let circles = CollectionSpec::circles(sizes)
            .with_offsets(offsets, ax.trans_data())
            .with_props(
                CollectionProps::default()
                    .facecolor("tab:blue")
                    .edgecolor("k")
                    .alpha(0.6),
            );
        ax.add_collection(circles);
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("patches/regularpolycollection", |fig| {
        let ax = fig.subplot();
        let offsets = (0..12_u32)
            .map(|i| Point::new(f64::from(i % 4), f64::from(i / 4)))
            .collect();
        let sizes = arange(0.0, 12.0, 1.0)
            .into_iter()
            .map(|i| 50.0 + 30.0 * i)
            .collect();
        let polygons = CollectionSpec::regular_polygons(5, 0.3, sizes)
            .with_offsets(offsets, ax.trans_data())
            .with_props(CollectionProps::default().edgecolor("k"))
            .with_mapping(mapping("cool", &arange(0.0, 12.0, 1.0))?);
        ax.add_collection(polygons);
        ax.set_xlim(-1.0, 4.0)?.set_ylim(-1.0, 3.0)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("patches/eventcollection", |fig| {
        let ax = fig.subplot();
        let mut g = SpecRng::new(52);
        let horizontal = Orientation::Horizontal;
        ax.add_collection(CollectionSpec::events(
            &g.uniform(30, 0.0, 10.0),
            horizontal,
            1.0,
            0.8,
        ));
        let dashed = CollectionProps::default()
            .edgecolor("tab:red")
            .linewidth(2.0)
            .linestyle(LineStyle::Dashed);
        ax.add_collection(
            CollectionSpec::events(&g.uniform(20, 0.0, 10.0), horizontal, 3.0, 1.5)
                .with_props(dashed),
        );
        ax.add_collection(
            CollectionSpec::events(&g.uniform(15, 0.0, 4.0), Orientation::Vertical, 8.0, 1.0)
                .with_props(CollectionProps::default().edgecolor("tab:green")),
        );
        ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 4.5)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/trimesh", |fig| {
        let ax = fig.subplot();
        let mut g = SpecRng::new(53);
        let x = g.uniform(40, 0.0, 1.0);
        let y = g.uniform(40, 0.0, 1.0);
        let triangulation = Triangulation::delaunay(&x, &y)?;
        let values: Vec<f64> = x
            .iter()
            .zip(&y)
            .map(|(x, y)| (4.0 * x).sin() * (3.0 * y).cos())
            .collect();
        ax.add_trimesh(TriMeshSpec::new(
            &triangulation,
            mapping("viridis", &values)?,
        )?);
        ax.autoscale(None, None);
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
];
