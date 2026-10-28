//! `figures/cases_patches_gallery.py`: collections with offsets and their
//! own transforms, ellipse collections in x units, multicoloured lines,
//! connection patches between coordinate systems, arrows between patches,
//! hatched collections and arcs on unequal axes.

use std::f64::consts::PI;

use plt::artist::ArtistHandle;
use plt::collections::{
    CollectionPatch, CollectionProps, CollectionSpec, ColorMapping, EdgeColors, EllipseUnits,
};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{BoundaryNorm, Colormap, Extend, Normalize, ScalarMappable, colormaps};
use plt::coords::Coords;
use plt::figure::Share;
use plt::gridspec::SubplotOverrides;
use plt::patches::{
    Arc, ArrowStyle, Circle, ConnectionEnd, ConnectionPatch, ConnectionStyle, Ellipse,
    FancyArrowPatch, PatchProps, Rectangle, Wedge,
};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::transforms::{Point, Transform};
use plt_core::Affine2D;
use plt_render::CapStyle;

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The default property cycle's colours.
const CYCLE: [&str; 10] = [
    "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2", "#7f7f7f",
    "#bcbd22", "#17becf",
];

/// The default property cycle's colours.
fn cycle() -> Vec<Color> {
    CYCLE.iter().map(|&c| Color::from(c)).collect()
}

/// `col.set_color(CYCLE)` of a filled collection: faces and edges.
fn cycle_colors(collection: &plt::collections::Collection) {
    collection
        .set_facecolors(cycle())
        .set_edgecolors(EdgeColors::Colors(cycle()));
}

/// `col.set_color(CYCLE)` of a line collection, which only colours the
/// lines (matplotlib's `LineCollection.set_color`).
fn cycle_line_colors(collection: &plt::collections::Collection) {
    collection.set_edgecolors(EdgeColors::Colors(cycle()));
}

fn gallery_collections(fig: &Figure) -> CaseResult {
    let r: Vec<f64> = (0..50).map(f64::from).collect();
    let theta = linspace(0.0, 2.0 * PI, 50);
    let spiral: Vec<Point> = r
        .iter()
        .zip(&theta)
        .map(|(r, t)| Point::new(r * t.sin(), r * t.cos()))
        .collect();
    let mut rng = SpecRng::new(19_680_801);
    let (xo, yo) = (rng.normal(100, 0.0, 1.0), rng.normal(100, 0.0, 1.0));
    let xyo: Vec<Point> = xo
        .iter()
        .zip(&yo)
        .map(|(&x, &y)| Point::new(x, y))
        .collect();
    let axs = fig.subplots(2, 2)?;
    fig.subplots_adjust(SubplotOverrides {
        top: Some(0.92),
        left: Some(0.07),
        right: Some(0.97),
        hspace: Some(0.3),
        wspace: Some(0.3),
        ..SubplotOverrides::default()
    })?;
    // `Affine2D().scale(fig.dpi / 72.0)`: points to pixels.
    let points_to_pixels =
        Transform::Affine(Affine2D::IDENTITY.scale(fig.dpi() / 72.0, fig.dpi() / 72.0));
    let ax1 = &axs[(0, 0)];
    let lines = CollectionSpec::lines(vec![spiral.clone()])
        .with_offsets(xyo.clone(), ax1.trans_data())
        .with_transform(Transform::Inches.then(Transform::Affine(
            Affine2D::IDENTITY.scale(1.0 / 72.0, 1.0 / 72.0),
        )));
    cycle_line_colors(&ax1.add_collection(lines));
    ax1.autoscale_view(None);
    ax1.set_title("LineCollection using offsets");
    let ax2 = &axs[(0, 1)];
    let polys = CollectionSpec::polygons(vec![spiral])
        .with_offsets(xyo.clone(), ax2.trans_data())
        .with_transform(points_to_pixels.clone());
    cycle_colors(&ax2.add_collection(polys));
    ax2.autoscale_view(None);
    ax2.set_title("PolyCollection using offsets");
    let ax3 = &axs[(1, 0)];
    let sizes = theta
        .iter()
        .zip(&r)
        .map(|(t, r)| (r * t.sin()).abs() * 10.0)
        .collect();
    let heptagons = CollectionSpec::regular_polygons(7, 0.0, sizes)
        .with_offsets(xyo, ax3.trans_data())
        .with_transform(points_to_pixels);
    cycle_colors(&ax3.add_collection(heptagons));
    ax3.autoscale_view(None);
    ax3.set_title("RegularPolyCollection using offsets");
    let ax4 = &axs[(1, 1)];
    let yy = linspace(0.0, 2.0 * PI, 60);
    let ym = yy[59];
    let xx: Vec<f64> = yy
        .iter()
        .map(|y| (0.2 + (ym - y) / ym).powi(2) * (y - 0.4).cos() * 0.5)
        .collect();
    let segments: Vec<Vec<Point>> = (0..20)
        .map(|_| {
            let noise = rng.normal(60, 0.0, 1.0);
            xx.iter()
                .zip(&noise)
                .zip(&yy)
                .map(|((x, n), y)| Point::new(x + 0.02 * n, y * 100.0))
                .collect()
        })
        .collect();
    let waterfall = CollectionSpec::lines(segments)
        .with_offsets(vec![Point::new(0.1, 0.0)], Transform::Display);
    cycle_line_colors(&ax4.add_collection(waterfall));
    ax4.autoscale_view(None);
    ax4.set_title("Successive data offsets");
    ax4.set_xlabel("Zonal velocity component (m/s)");
    ax4.set_ylabel("Depth (m)");
    let (bottom, top) = ax4.ylim();
    ax4.set_ylim(top, bottom)?;
    Ok(())
}

fn ellipsecollection_x_units_colorbar(fig: &Figure) -> CaseResult {
    let cells: Vec<(f64, f64)> = (0..15)
        .flat_map(|y| (0..10).map(move |x| (f64::from(x), f64::from(y))))
        .collect();
    let ax = fig.subplot();
    let widths: Vec<f64> = cells.iter().map(|(x, _)| x / 10.0).collect();
    let heights: Vec<f64> = cells.iter().map(|(_, y)| y / 15.0).collect();
    let angles: Vec<f64> = cells.iter().map(|(x, _)| x * 9.0).collect();
    let offsets = cells.iter().map(|&(x, y)| Point::new(x, y)).collect();
    let spec = CollectionSpec::ellipses(&widths, &heights, &angles, EllipseUnits::X)
        .with_offsets(offsets, ax.trans_data())
        .with_mapping(ColorMapping {
            values: cells.iter().map(|(x, y)| Some(x + y)).collect(),
            mappable: ScalarMappable::new(colormaps::get("viridis")?),
        });
    let ellipses = ax.add_collection(spec);
    ax.autoscale_view(None);
    ax.set_xlabel("X").set_ylabel("y");
    let cbar = fig.colorbar(&ellipses, ColorbarOptions::default())?;
    cbar.set_label("X+Y", None);
    Ok(())
}

fn multicolored_line(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 3.0 * PI, 500);
    let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
    let dydx: Vec<f64> = x
        .windows(2)
        .map(|w| f64::midpoint(w[0], w[1]).cos())
        .collect();
    let segments: Vec<Vec<Point>> = (0..499)
        .map(|i| vec![Point::new(x[i], y[i]), Point::new(x[i + 1], y[i + 1])])
        .collect();
    let axs = fig.subplots_shared(2, 1, Share::All, Share::All)?;
    let (low, high) = dydx
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    let values: Vec<Option<f64>> = dydx.iter().copied().map(Some).collect();
    let smooth = CollectionSpec::lines(segments.clone())
        .with_props(CollectionProps::default().linewidth(2.0))
        .with_mapping(ColorMapping {
            values: values.clone(),
            mappable: ScalarMappable::with_norm(
                colormaps::get("viridis")?,
                Box::new(Normalize::new(low, high)),
            ),
        });
    let line = axs[(0, 0)].add_collection(smooth);
    fig.colorbar(
        &line,
        ColorbarOptions {
            axes: ColorbarAxes::Parent(axs[(0, 0)].clone()),
            ..ColorbarOptions::default()
        },
    )?;
    let cmap = Colormap::listed("from_list", vec!["r", "g", "b"])?;
    let norm = BoundaryNorm::new([-1.0, -0.5, 0.5, 1.0], cmap.n(), Extend::Neither)?;
    let stepped = CollectionSpec::lines(segments)
        .with_props(CollectionProps {
            linewidths: Some(linspace(0.5, 4.0, 499)),
            ..CollectionProps::default()
        })
        .with_mapping(ColorMapping {
            values,
            mappable: ScalarMappable::with_norm(cmap, Box::new(norm)),
        });
    let line = axs[(1, 0)].add_collection(stepped);
    fig.colorbar(
        &line,
        ColorbarOptions {
            axes: ColorbarAxes::Parent(axs[(1, 0)].clone()),
            ..ColorbarOptions::default()
        },
    )?;
    axs[(0, 0)].set_xlim(x[0], x[499])?.set_ylim(-1.1, 1.1)?;
    Ok(())
}

fn connectionpatch_coordinate_systems(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let mut simple = ConnectionPatch::new(
        ConnectionEnd::new((0.2, 0.2), Coords::Data, None),
        ConnectionEnd::new((0.8, 0.8), Coords::Data, None),
    );
    simple.spec.arrowstyle = "-|>".parse()?;
    simple.spec.shrink_a = 5.0;
    simple.spec.shrink_b = 5.0;
    simple.spec.mutation_scale = 20.0;
    ax1.plot_fmt([0.2, 0.8], [0.2, 0.8], "o")?;
    ax1.add_artist(simple).set_facecolor("w");
    let mut across = ConnectionPatch::new(
        ConnectionEnd::new((0.3, 0.2), Coords::Transform(ax2.trans_data()), None),
        ConnectionEnd::new((0.3, 0.2), Coords::Transform(ax1.trans_data()), None),
    );
    across.spec.arrowstyle = "->".parse()?;
    across.spec.shrink_b = 5.0;
    fig.add_artist(across);
    let mut mixed = ConnectionPatch::new(
        ConnectionEnd::new((0.6, 1.0), Coords::Transform(ax2.trans_axes()), None),
        ConnectionEnd::new((0.0, 0.2), Coords::Transform(ax2.trans_yaxis()), None),
    );
    mixed.spec.arrowstyle = "-".parse()?;
    ax2.add_artist(mixed);
    ax1.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    ax2.set_xlim(0.0, 0.5)?.set_ylim(0.0, 0.5)?;
    Ok(())
}

fn fancyarrowpatch_between_patches(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 6.0)?;
    ax.set_aspect(Aspect::Equal)?;
    let styles = [
        ("arc3,rad=0.3", "fancy", 1.0),
        ("angle3,angleA=90,angleB=0", "simple", 0.6),
        ("angle,angleA=-90,angleB=180,rad=8", "-|>", 1.0),
        ("arc,angleA=-90,angleB=0,armA=30,armB=30,rad=6", "->", 1.0),
        ("bar,fraction=-0.2", "<|-|>", 0.5),
    ];
    for (i, (connection, arrow, aspect)) in styles.into_iter().enumerate() {
        let x = 1.0 + 2.0 * i as f64;
        let start = ax.add_patch(Circle::new((x - 0.4, 1.2), 0.4));
        start.set_facecolor("lightsteelblue").set_edgecolor("k");
        let end = ax.add_patch(Circle::new((x + 0.4, 4.5), 0.5));
        end.set_facecolor("moccasin").set_edgecolor("k");
        let mut patch = FancyArrowPatch::between((x - 0.4, 1.2), (x + 0.4, 4.5))
            .with_patches(Some(start.id()), Some(end.id()))
            .with_connectionstyle(connection.parse::<ConnectionStyle>()?)
            .with_arrowstyle(arrow.parse::<ArrowStyle>()?)
            .with_mutation_scale(15.0)
            .with_shrink(2.0, 4.0);
        patch.spec.mutation_aspect = aspect;
        ax.add_patch(patch)
            .set_facecolor("tab:red")
            .set_edgecolor("k")
            .set_linewidth(1.0);
    }
    Ok(())
}

/// A 2 × 2 square with its lower-left corner at `(x, 0.5)`.
fn square(x: f64) -> Vec<Point> {
    vec![
        Point::new(x, 0.5),
        Point::new(x + 2.0, 0.5),
        Point::new(x + 2.0, 2.5),
        Point::new(x, 2.5),
    ]
}

/// A polygon collection of one square with `props`.
fn hatched_square(x: f64, props: CollectionProps) -> CollectionSpec {
    CollectionSpec::polygons(vec![square(x)]).with_props(props)
}

fn collection_hatch_alpha_and_edges(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let mut faded = CollectionProps::default().facecolor("lightblue");
    faded.hatch = Some("//".into());
    faded.base.alpha = Some(0.5);
    ax.add_collection(hatched_square(0.5, faded));
    let mut edged = CollectionProps::default()
        .facecolor("w")
        .edgecolor("tab:red")
        .linewidth(2.0);
    edged.hatch = Some("xx".into());
    ax.add_collection(hatched_square(3.0, edged));
    let mut hollow = CollectionProps::default()
        .facecolor("none")
        .edgecolor("tab:green");
    hollow.hatch = Some("..".into());
    hollow.base.alpha = Some(0.6);
    ax.add_collection(hatched_square(5.5, hollow));
    let shapes = [
        CollectionPatch::new(&Circle::new((9.0, 1.5), 0.9), PatchProps::default()),
        CollectionPatch::new(&Rectangle::new((8.2, 2.7), 1.6, 0.6), PatchProps::default()),
    ];
    let mut mixed = CollectionProps {
        facecolors: Some(vec![Color::from("gold"), Color::from("tab:purple")]),
        ..CollectionProps::default().edgecolor("k")
    };
    mixed.hatch = Some("\\\\".into());
    mixed.base.alpha = Some(0.7);
    ax.add_collection(CollectionSpec::patches(&shapes, false)?.with_props(mixed));
    ax.set_xlim(0.0, 10.5)?.set_ylim(0.0, 3.5)?;
    Ok(())
}

fn collection_hatch_rc_color(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let mut plain = CollectionProps::default().facecolor("w");
    plain.hatch = Some("//".into());
    ax.add_collection(hatched_square(0.5, plain));
    let mut faded = CollectionProps::default().facecolor("w").edgecolor("k");
    faded.hatch = Some("++".into());
    faded.base.alpha = Some(0.4);
    ax.add_collection(hatched_square(3.0, faded));
    let mut circles = CollectionProps::default()
        .facecolor("0.9")
        .edgecolor("tab:orange");
    circles.hatch = Some("o".into());
    ax.add_collection(hatched_square(5.5, circles));
    ax.set_xlim(0.0, 8.0)?.set_ylim(0.0, 3.0)?;
    Ok(())
}

fn arcs_wedges_unequal_aspect(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    for ax in [ax1, ax2] {
        ax.set_xlim(-2.0, 2.0)?.set_ylim(-1.0, 1.0)?;
    }
    ax1.add_patch(
        Arc::new((0.0, 0.0), 3.0, 1.5)
            .with_angle(20.0)
            .between(30.0, 300.0),
    )
    .set_linewidth(3.0)
    .set_color("tab:blue");
    ax1.add_patch(Arc::new((0.0, 0.0), 1.5, 1.5).between(-45.0, 45.0))
        .set_linewidth(2.0)
        .set_color("tab:red")
        .set_capstyle(CapStyle::Round);
    ax1.add_patch(Ellipse {
        angle: -30.0,
        ..Ellipse::new((1.2, 0.5), 1.0, 0.4)
    })
    .set_facecolor("none")
    .set_edgecolor("tab:green")
    .set_linewidth(2.0)
    .set_linestyle(LineStyle::Dashed);
    ax2.add_patch(Wedge {
        width: Some(0.3),
        ..Wedge::new((0.0, 0.0), 0.9, 300.0, 60.0)
    })
    .set_facecolor("tab:orange")
    .set_edgecolor("k");
    ax2.add_patch(Wedge::new((-1.0, -0.2), 0.6, 90.0, 450.0))
        .set_facecolor("tab:cyan")
        .set_alpha(Some(0.6));
    ax2.add_patch(Wedge {
        width: Some(0.5),
        ..Wedge::new((1.2, 0.3), 0.5, -30.0, 30.0)
    })
    .set_hatch("//")
    .set_facecolor("w")
    .set_edgecolor("tab:purple");
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("patches/gallery_collections", gallery_collections),
    FigureCase::new(
        "patches/ellipsecollection_x_units_colorbar",
        ellipsecollection_x_units_colorbar,
    )
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("patches/multicolored_line", multicolored_line),
    FigureCase::new(
        "patches/connectionpatch_coordinate_systems",
        connectionpatch_coordinate_systems,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new(
        "patches/fancyarrowpatch_between_patches",
        fancyarrowpatch_between_patches,
    )
    .sized(6.0, 4.0, 100.0),
    FigureCase::new(
        "patches/collection_hatch_alpha_and_edges",
        collection_hatch_alpha_and_edges,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new(
        "patches/collection_hatch_rc_color",
        collection_hatch_rc_color,
    )
    .sized(5.0, 3.0, 100.0)
    .with_rc(&[("hatch.color", "tab:purple"), ("hatch.linewidth", "1.5")]),
    FigureCase::new(
        "patches/arcs_wedges_unequal_aspect",
        arcs_wedges_unequal_aspect,
    )
    .sized(6.0, 3.0, 100.0),
];
