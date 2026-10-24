//! The cases from `test_marker.py` and `test_artist.py`.

use plt::artist::{ArtistHandle, ClipPathSpec};
use plt::axes::{Axes, BarAlign, BarOptions, ScatterOptions};
use plt::collections::{CollectionProps, CollectionSpec, EdgeColors};
use plt::colors::Color;
use plt::figure::{Figure, Share};
use plt::markers::{FillStyle, Marker, MarkerStyle, PolygonStyle};
use plt::patches::{PatchProps, PathShape, Rectangle};
use plt::prelude::*;
use plt::text::TextBoxProps;
use plt::transforms::{Affine2D, Point};
use plt_core::{Path, PathCode};

use super::{CLASSIC_TEST_PATCH, remove_text};
use crate::figures::{CaseResult, FigureCase};

/// The marker area of `test_marker.py`: 20 points across.
const MARKER_AREA: f64 = 400.0;

/// One scatter point at `(x, y)` with `marker` of [`MARKER_AREA`].
fn scatter_one(ax: &Axes, (x, y): (f64, f64), marker: Marker) -> CaseResult {
    let options = ScatterOptions {
        sizes: Some(vec![MARKER_AREA]),
        marker: Some(MarkerStyle::new(marker, FillStyle::Full)?),
        ..ScatterOptions::default()
    };
    ax.scatter_with([x], [y], options)?;
    Ok(())
}

/// `test_poly_marker`: polygon markers with and without rotation.
fn poly_marker(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let polygons = [
        ((0.0, 0.0), 4, 45.0),
        ((1.0, 1.0), 4, 0.0),
        ((1.0, 1.5), 4, 0.0),
        ((2.0, 2.0), 5, 0.0),
        ((2.0, 2.5), 5, 0.0),
        ((3.0, 3.0), 6, 0.0),
        ((3.0, 3.5), 6, 0.0),
        ((4.0, 4.0), 6, 30.0),
        ((5.0, 5.0), 8, 22.5),
    ];
    for (at, sides, angle) in polygons {
        scatter_one(
            &ax,
            at,
            Marker::polygon(sides, PolygonStyle::Regular, angle),
        )?;
    }
    ax.set_xlim(-0.5, 5.5)?.set_ylim(-0.5, 5.5)?;
    Ok(())
}

/// `test_asterisk_marker`: plus and cross asterisks.
fn asterisk_marker(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    for (at, angle) in [((0.0, 0.0), 0.0), ((0.5, 0.5), 0.0), ((1.0, 1.0), 45.0)] {
        scatter_one(&ax, at, Marker::polygon(4, PolygonStyle::Asterisk, angle))?;
    }
    ax.set_xlim(-0.5, 1.5)?.set_ylim(-0.5, 1.5)?;
    Ok(())
}

/// `matplotlib.markers.MarkerStyle.markers`, in order.
fn every_marker() -> Result<Vec<Marker>, Box<dyn std::error::Error>> {
    let named = [
        ".", ",", "o", "v", "^", "<", ">", "1", "2", "3", "4", "8", "s", "p", "*", "h", "H", "+",
        "x", "D", "d", "|", "_", "P", "X",
    ];
    let mut markers = named
        .iter()
        .map(|name| name.parse::<Marker>())
        .collect::<Result<Vec<_>, _>>()?;
    for code in 0..=11 {
        markers.push(Marker::from_code(code)?);
    }
    for nothing in ["None", "none", " ", ""] {
        markers.push(nothing.parse()?);
    }
    Ok(markers)
}

/// `test_marker_clipping`: thick-edged markers on short lines, rows of
/// [`CLIP_COLUMNS`].
fn marker_clipping(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    for (i, marker) in every_marker()?.into_iter().enumerate() {
        let x = (i % CLIP_COLUMNS) as f64;
        let y = (i / CLIP_COLUMNS * 2) as f64;
        let props = LineProps::default()
            .color("k")
            .linestyle(LineStyle::Solid)
            .linewidth(3.0)
            .marker(marker)
            .markersize(50.0)
            .markeredgewidth(10.0)
            .fillstyle(FillStyle::Full)
            .markerfacecolor(Color::from("white"));
        ax.plot_with([x, x], [y, y + 1.0], props)?;
    }
    ax.set_xlim(-0.5, CLIP_COLUMNS as f64)?
        .set_ylim(-0.5, 2.0 * CLIP_ROWS as f64)?;
    ax.set_axis_off();
    Ok(())
}

/// Markers per row of `test_marker_clipping`.
const CLIP_COLUMNS: usize = 7;
/// Rows of `test_marker_clipping`: 41 markers.
const CLIP_ROWS: usize = 41 / CLIP_COLUMNS + 1;

/// `test_clipping`: a hatched star clipped to a square with a round hole,
/// as a collection and as a patch.
fn clip_path_clipping(fig: &Figure) -> CaseResult {
    let exterior = Path::unit_rectangle()
        .transformed(&Affine2D::identity().scale(4.0, 4.0).translate(-2.0, -2.0));
    let mut interior = Path::unit_circle();
    interior.vertices_mut().reverse();
    let clip_path = Path::compound([&exterior, &interior]);
    let star = Path::unit_regular_star(6, 0.5).transformed(&Affine2D::identity().scale(2.6, 2.6));
    let axs = fig.subplots_shared(1, 2, Share::All, Share::All)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    // Like the test, the clip paths are set before the artists are
    // added, so the axes do not clip them to their box.
    let clip = |ax: &plt::axes::Axes| ClipPathSpec::Path {
        path: clip_path.clone(),
        transform: ax.trans_data(),
    };
    let props = CollectionProps {
        linewidths: Some(vec![5.0]),
        edgecolors: Some(EdgeColors::Colors(vec![Color::from("blue")])),
        facecolors: Some(vec![Color::from("red")]),
        hatch: Some("*".into()),
        ..CollectionProps::default()
    }
    .alpha(0.7)
    .clip_path(clip(ax1));
    ax1.add_collection(CollectionSpec::paths(vec![star.clone()]).with_props(props));
    let props = PatchProps::default()
        .linewidth(5.0)
        .edgecolor("blue")
        .facecolor("red")
        .hatch("*")
        .alpha(0.7)
        .clip_path(clip(ax2));
    ax2.add_patch_with(PathShape(star), props);
    ax1.set_xlim(-3.0, 3.0)?.set_ylim(-3.0, 3.0)?;
    remove_text(fig);
    Ok(())
}

/// `test_clipping_zoom`: a line clipped to a path outside the figure.
fn clipping_zoom(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    let line = ax.plot([-3.0, 3.0], [-3.0, 3.0])?;
    let square = Path::polyline(
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.0, 0.0)]
            .map(|(x, y)| Point::new(x, y))
            .to_vec(),
    );
    line.set_clip_path(Some(ClipPathSpec::Path {
        path: square,
        transform: ax.trans_data(),
    }));
    ax.set_xlim(0.5, 0.75)?.set_ylim(0.5, 0.75)?;
    Ok(())
}

/// `test_hatching`: hatches in the default colour and with edge colours,
/// on patches and collections.
fn hatching(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    for (x, edge) in [(0.0, None), (4.0, Some("C1"))] {
        let rect = ax.add_patch(Rectangle::new((x, 0.0), 3.0, 4.0));
        rect.set_hatch("/");
        let mut props = CollectionProps {
            hatch: Some("/".into()),
            ..CollectionProps::default()
        };
        if let Some(edge) = edge {
            rect.set_edgecolor(edge);
            props.edgecolors = Some(EdgeColors::Colors(vec![Color::from(edge)]));
        }
        ax.add_collection(
            CollectionSpec::regular_polygons(4, 0.0, vec![16000.0])
                .with_offsets(vec![Point::new(x + 1.5, 6.5)], ax.trans_data())
                .with_props(props),
        );
    }
    ax.set_xlim(0.0, 7.0)?.set_ylim(0.0, 9.0)?;
    remove_text(fig);
    Ok(())
}

/// `test_default_edges`: markers, edge-aligned bars, a sawtooth text box
/// and a quadratic patch in the default style.
fn default_edges(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let range: Vec<f64> = (0..10).map(f64::from).collect();
    let shifted: Vec<f64> = range.iter().map(|x| x + 1.0).collect();
    axs[(0, 0)].plot_fmt(&range, &range, "x")?;
    axs[(0, 0)].plot_fmt(&shifted, &range, "o")?;
    axs[(0, 1)].bar(
        range.clone(),
        &range,
        BarOptions {
            align: BarAlign::Edge,
            ..BarOptions::default()
        },
    )?;
    let ax3 = &axs[(1, 0)];
    ax3.text(0.0, 0.0, "BOX")
        .set_fontsize(24.0)
        .set_bbox(Some(TextBoxProps::new("sawtooth".parse()?)));
    ax3.set_xlim(-1.0, 1.0)?.set_ylim(-1.0, 1.0)?;
    let curve = Path::new(
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 0.0)]
            .map(|(x, y)| Point::new(x, y))
            .to_vec(),
        vec![
            PathCode::MoveTo,
            PathCode::Curve3,
            PathCode::Curve3,
            PathCode::ClosePoly,
        ],
    )?;
    axs[(1, 1)]
        .add_patch(PathShape(curve))
        .set_facecolor("none");
    remove_text(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_small/poly_marker", poly_marker),
    FigureCase::new("mpl_small/asterisk_marker", asterisk_marker),
    FigureCase::new("mpl_small/text_marker", |fig| {
        let props = LineProps::default()
            .marker(r"$\bullet$".parse::<Marker>()?)
            .markersize(100.0)
            .markeredgewidth(0.0);
        fig.subplot().plot_with([0.0], [0.0], props)?;
        Ok(())
    }),
    FigureCase::new("mpl_small/marker_clipping", marker_clipping).sized(
        (2 * 50 * CLIP_COLUMNS) as f64 / 100.0,
        (2 * 50 * CLIP_ROWS * 2) as f64 / 100.0,
        100.0,
    ),
    FigureCase::new("mpl_small/clip_path_clipping", clip_path_clipping)
        .with_style("classic")
        .with_rc(CLASSIC_TEST_PATCH),
    FigureCase::new("mpl_small/clipping_zoom", clipping_zoom),
    FigureCase::new("mpl_small/hatching", hatching),
    FigureCase::new("mpl_small/default_edges", default_edges),
];
