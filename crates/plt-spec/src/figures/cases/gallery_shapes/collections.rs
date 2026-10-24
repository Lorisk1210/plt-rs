//! Collections: ellipses sized in x units, arcs coloured by name or by a
//! colormap, circles, wedges and polygons mapped to colours, and a
//! scatter plot (`ellipse_collection`, `line_collection`,
//! `patch_collection` and `scatter`).

use std::f64::consts::PI;

use plt::collections::{
    CollectionPatch, CollectionProps, CollectionSpec, ColorMapping, EdgeColors, EllipseUnits,
};
use plt::colorbar::ColorbarOptions;
use plt::colors::{Alpha, ScalarMappable, colormaps};
use plt::patches::{Circle, Polygon, Wedge};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::transforms::Point;

use super::SEED;
use crate::figures::cases::common::{colors, map};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `set_array(values)` with the default colormap.
fn mapped(values: &[f64]) -> Result<ColorMapping, Box<dyn std::error::Error>> {
    Ok(ColorMapping {
        values: values.iter().map(|&v| Some(v)).collect(),
        mappable: ScalarMappable::new(colormaps::get("viridis")?),
    })
}

fn ellipse_collection(fig: &Figure) -> CaseResult {
    let (xs, ys): (Vec<f64>, Vec<f64>) = (0..15)
        .flat_map(|y| (0..10).map(move |x| (f64::from(x), f64::from(y))))
        .unzip();
    let ax = fig.subplot();
    let offsets = xs
        .iter()
        .zip(&ys)
        .map(|(&x, &y)| Point::new(x, y))
        .collect();
    let spec = CollectionSpec::ellipses(
        &map(&xs, |x| x / 10.0),
        &map(&ys, |y| y / 15.0),
        &map(&xs, |x| x * 9.0),
        EllipseUnits::X,
    )
    .with_offsets(offsets, ax.trans_data())
    .with_mapping(mapped(
        &xs.iter().zip(&ys).map(|(x, y)| x + y).collect::<Vec<_>>(),
    )?);
    let ellipses = ax.add_collection(spec);
    ax.set_xlabel("X").set_ylabel("y");
    fig.colorbar(&ellipses, ColorbarOptions::default())?
        .set_label("X+Y", None);
    Ok(())
}

/// Half circles of the given radii, 36 points each.
fn arcs(radii: &[f64]) -> Vec<Vec<Point>> {
    let theta = linspace(0.0, PI, 36);
    radii
        .iter()
        .map(|r| {
            theta
                .iter()
                .map(|t| Point::new(r * t.cos(), r * t.sin()))
                .collect()
        })
        .collect()
}

/// An axes showing `[-6, 6] × [0, 6]` at equal aspect.
fn arc_axes(fig: &Figure) -> Result<Axes, Box<dyn std::error::Error>> {
    let ax = fig.subplot();
    ax.set_xlim(-6.0, 6.0)?.set_ylim(0.0, 6.0)?;
    ax.set_aspect(Aspect::Equal)?;
    Ok(ax)
}

fn line_collection_1(fig: &Figure) -> CaseResult {
    let names = ["indigo", "blue", "green", "yellow", "orange", "red"];
    let ax = arc_axes(fig)?;
    let props = CollectionProps {
        edgecolors: Some(EdgeColors::Colors(colors(&names))),
        linewidths: Some(vec![4.0]),
        ..CollectionProps::default()
    };
    let spec = CollectionSpec::lines(arcs(&linspace(4.0, 5.0, names.len()))).with_props(props);
    ax.add_collection(spec);
    Ok(())
}

fn line_collection_2(fig: &Figure) -> CaseResult {
    let radii = linspace(4.0, 5.5, 15);
    let ax = arc_axes(fig)?;
    let spec = CollectionSpec::lines(arcs(&radii)).with_mapping(ColorMapping {
        values: radii.iter().map(|&r| Some(r)).collect(),
        mappable: ScalarMappable::new(colormaps::get("rainbow")?),
    });
    let lines = ax.add_collection(spec);
    let options = ColorbarOptions {
        label: "Radius".into(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&lines, options)?;
    ax.set_title("Line Collection with mapped colors");
    Ok(())
}

fn patch_collection(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let ax = fig.subplot();
    let n = 3;
    let plain = PatchProps::default;
    let (x, y) = (rng.uniform(n, 0.0, 1.0), rng.uniform(n, 0.0, 1.0));
    let radii = rng.uniform(n, 0.0, 1.0);
    let mut patches: Vec<CollectionPatch> = (0..n)
        .map(|i| CollectionPatch::new(&Circle::new((x[i], y[i]), 0.1 * radii[i]), plain()))
        .collect();
    let (x, y) = (rng.uniform(n, 0.0, 1.0), rng.uniform(n, 0.0, 1.0));
    let radii = rng.uniform(n, 0.0, 1.0);
    let (theta1, theta2) = (rng.uniform(n, 0.0, 1.0), rng.uniform(n, 0.0, 1.0));
    patches.extend((0..n).map(|i| {
        let wedge = Wedge::new(
            (x[i], y[i]),
            0.1 * radii[i],
            360.0 * theta1[i],
            360.0 * theta2[i],
        );
        CollectionPatch::new(&wedge, plain())
    }));
    let ring = |center: (f64, f64), r, theta1, theta2, width| Wedge {
        width,
        ..Wedge::new(center, r, theta1, theta2)
    };
    for wedge in [
        ring((0.3, 0.7), 0.1, 0.0, 360.0, None),
        ring((0.7, 0.8), 0.2, 0.0, 360.0, Some(0.05)),
        ring((0.8, 0.3), 0.2, 0.0, 45.0, None),
        ring((0.8, 0.3), 0.2, 45.0, 90.0, Some(0.10)),
    ] {
        patches.push(CollectionPatch::new(&wedge, plain()));
    }
    for _ in 0..n {
        let corners = rng.uniform(n * 2, 0.0, 1.0);
        let polygon = Polygon::new(corners.chunks(2).map(|c| Point::new(c[0], c[1])), true);
        patches.push(CollectionPatch::new(&polygon, plain()));
    }
    let values = map(&rng.uniform(patches.len(), 0.0, 1.0), |v| 100.0 * v);
    let spec = CollectionSpec::patches(&patches, false)?
        .with_props(CollectionProps::default().alpha(0.4))
        .with_mapping(mapped(&values)?);
    let collection = ax.add_collection(spec);
    fig.colorbar(&collection, ColorbarOptions::beside(&ax))?;
    Ok(())
}

fn scatter(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let n = 50;
    let (x, y) = (rng.uniform(n, 0.0, 1.0), rng.uniform(n, 0.0, 1.0));
    let colors = rng.uniform(n, 0.0, 1.0);
    let area = map(&rng.uniform(n, 0.0, 1.0), |v| (30.0 * v).powi(2));
    let options = ScatterOptions {
        sizes: Some(area),
        colors: MappedColors::new(colors).into(),
        alpha: Alpha::Uniform(0.5),
        ..ScatterOptions::default()
    };
    fig.subplot().scatter_with(x, y, options)?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_shapes/ellipse_collection", ellipse_collection),
    FigureCase::new("gallery_shapes/line_collection_1", line_collection_1).sized(6.4, 3.2, 100.0),
    FigureCase::new("gallery_shapes/line_collection_2", line_collection_2).sized(6.4, 3.0, 100.0),
    FigureCase::new("gallery_shapes/patch_collection", patch_collection),
    FigureCase::new("gallery_shapes/scatter", scatter),
];
