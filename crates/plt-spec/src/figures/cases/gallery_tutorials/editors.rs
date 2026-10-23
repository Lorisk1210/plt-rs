//! The interactive editors and explorers before any interaction: points
//! to lasso, a path and a polygon with their vertex markers, a
//! triangulation to search and the Mandelbrot set with its zoom window.
//!
//! The editors draw their patch and markers as animated artists from a
//! draw callback; here they are ordinary artists, drawn the same way in
//! every output format.

use std::f64::consts::PI;

use plt::axes::ImshowOptions;
use plt::collections::{CollectionSpec, ColorMapping};
use plt::colors::{Colormap, ScalarMappable};
use plt::image::{Extent, Origin};
use plt::markers::MarkerError;
use plt::numeric::Grid;
use plt::patches::{PathShape, Polygon, Rectangle};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::transforms::Point;

use super::{SEED, uniform_rows};
use crate::figures::cases::common::{Annulus, path_patch_curve};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

fn lasso_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    ax.set_title("Lasso points using left mouse button");
    let data = uniform_rows(&mut SpecRng::new(SEED), 100, 2);
    let offsets = data
        .iter_rows()
        .map(|xy| Point::new(xy[0], xy[1]))
        .collect();
    let mut mappable =
        ScalarMappable::new(Colormap::listed("from_list", vec!["tab:blue", "tab:red"])?);
    mappable.set_clim(Some(0.0), Some(1.0));
    let hexagons = CollectionSpec::regular_polygons(6, 0.0, vec![100.0])
        .with_offsets(offsets, ax.trans_data())
        .with_mapping(ColorMapping {
            values: vec![Some(0.0); data.rows()],
            mappable,
        });
    ax.add_collection(hexagons);
    Ok(())
}

/// The editors' markers: red-faced circles on a line through the
/// vertices of the outline being edited.
fn vertex_markers() -> Result<LineProps, MarkerError> {
    Ok(LineProps::default()
        .marker("o".parse::<Marker>()?)
        .markerfacecolor(Color::from("r")))
}

/// The x and y of `vertices`.
fn unzipped(vertices: &[Point]) -> (Vec<f64>, Vec<f64>) {
    vertices.iter().map(|p| (p.x, p.y)).unzip()
}

fn path_editor(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let curve = path_patch_curve()?;
    let (x, y) = unzipped(curve.vertices());
    let green = PatchProps::default()
        .facecolor("green")
        .edgecolor("yellow")
        .alpha(0.5);
    ax.add_patch_with(PathShape(curve), green);
    ax.plot_with(x, y, vertex_markers()?)?;
    ax.set_title("drag vertices to update path");
    ax.set_xlim(-3.0, 4.0).set_ylim(-3.0, 4.0);
    Ok(())
}

fn poly_editor(fig: &Figure) -> CaseResult {
    let r = 1.5;
    let circle = arange(0.0, 2.0 * PI, 0.1)
        .into_iter()
        .map(|theta| Point::new(r * theta.cos(), r * theta.sin()));
    let poly = Polygon::new(circle, true);
    let (x, y) = unzipped(poly.vertices());
    let ax = fig.subplot();
    ax.add_patch(poly);
    ax.add_line(&x, &y, vertex_markers()?)?;
    ax.set_title("Click and drag a point to move it");
    ax.set_xlim(-2.0, 2.0).set_ylim(-2.0, 2.0);
    Ok(())
}

fn trifinder_event_demo(fig: &Figure) -> CaseResult {
    let min_radius = 0.25;
    let annulus = Annulus::new(16, 5, min_radius);
    let triang = annulus.masked_delaunay(min_radius)?;
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Equal)?;
    ax.triplot(&triang, "bo-", LineProps::default())?;
    // The polygon of the triangle under the mouse, collapsed onto the
    // first point while the mouse is outside the triangulation.
    let first = Point::new(annulus.x[0], annulus.y[0]);
    ax.add_patch_with(
        Polygon::new([first; 3], true),
        PatchProps::default().facecolor("y"),
    );
    Ok(())
}

/// For each point of a `width × height` grid over `xlim × ylim`, the
/// number of the first 50 iterations of `z² + c` it stays within 2 of the
/// origin (`_mandelbrot`).
fn mandelbrot(xlim: (f64, f64), ylim: (f64, f64), width: usize, height: usize) -> Grid<f64> {
    let x = linspace(xlim.0, xlim.1, width);
    let y = linspace(ylim.0, ylim.1, height);
    Grid::from_fn(height, width, |row, col| {
        let (cr, ci) = (x[col], y[row]);
        let (mut zr, mut zi) = (0.0_f64, 0.0_f64);
        let mut count = 0.0;
        for _ in 0..50 {
            (zr, zi) = (zr * zr - zi * zi + cr, zr * zi + zi * zr + ci);
            if zr.hypot(zi) >= 2.0 {
                break;
            }
            count += 1.0;
        }
        count
    })
}

fn viewlims(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax_full, ax_zoom) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax_zoom.set_title("Zoom here");
    // What the example's limit callbacks do once the zoom axes shows
    // (-2, .5) × (-1.25, 1.25): track its bounds with a rectangle on the
    // full axes and draw the set there, a pixel per point of the axes.
    let (xlim, ylim) = ((-2.0, 0.5), (-1.25, 1.25));
    let extent = Extent::new(xlim.0, xlim.1, ylim.0, ylim.1);
    ax_full.add_patch_with(
        Rectangle::new((xlim.0, ylim.0), xlim.1 - xlim.0, ylim.1 - ylim.0),
        PatchProps::default()
            .facecolor("none")
            .edgecolor("black")
            .linewidth(1.0),
    );
    let (fig_width, fig_height) = fig.size_pixels();
    let zoom = ax_zoom.original_position();
    let width = (zoom.width() * fig_width).round_ties_even() as usize;
    let height = (zoom.height() * fig_height).round_ties_even() as usize;
    let set = mandelbrot(xlim, ylim, width, height);
    for ax in [ax_zoom, ax_full] {
        let options = ImshowOptions {
            extent: Some(extent),
            origin: Some(Origin::Lower),
            ..ImshowOptions::default()
        };
        ax.imshow(set.clone(), options)?;
    }
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_tutorials/lasso_demo", lasso_demo),
    FigureCase::new("gallery_tutorials/path_editor", path_editor),
    FigureCase::new("gallery_tutorials/poly_editor", poly_editor),
    FigureCase::new(
        "gallery_tutorials/trifinder_event_demo",
        trifinder_event_demo,
    ),
    FigureCase::new("gallery_tutorials/viewlims", viewlims),
];
