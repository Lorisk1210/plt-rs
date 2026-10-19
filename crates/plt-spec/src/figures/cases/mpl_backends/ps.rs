//! The image tests of `test_backend_ps.py`: fully transparent artists, the
//! standard PostScript fonts, hatches of edgeless ellipses, text clipped
//! out of the figure, Type 42 fonts, text needing fallback fonts, path
//! collections at pixel offsets and a colorbar saved with a tight box.

use plt::axes::{MappedColors, ScatterOptions};
use plt::collections::{CollectionProps, CollectionSpec, EdgeColors};
use plt::colorbar::ColorbarOptions;
use plt::colors::{BoundaryNorm, Colormap, Extend};
use plt::patches::Ellipse;
use plt::prelude::*;
use plt::transforms::{Point, Transform};
use plt_core::Path;

use super::{SEED, classic};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `test_transparency`: a line and a text of zero opacity.
fn transparency(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_axis_off();
    ax.plot_with(
        [0.0, 1.0],
        [0.0, 1.0],
        LineProps::default().color("r").alpha(0.0),
    )?;
    ax.text(0.5, 0.5, "foo")
        .set_color(Color::from("r"))
        .set_alpha(Some(0.0));
    Ok(())
}

/// `test_useafm`: a text in the standard PostScript fonts over a line.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn useafm(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_axis_off();
    ax.axhline(0.5, LineProps::default());
    ax.text(0.5, 0.5, "qk");
    Ok(())
}

/// `test_colored_hatch_zero_linewidth`: hatched ellipses, two without
/// edges.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn colored_hatch_zero_linewidth(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let ellipses = [
        ((0.0, 0.0), 1.0, 1.0, "/", "r", 0.0),
        ((0.5, 0.5), 0.5, 0.5, "+", "g", 0.2),
        ((1.0, 1.0), 0.3, 0.8, "\\", "b", 0.0),
    ];
    for (center, width, height, hatch, edge, linewidth) in ellipses {
        ax.add_patch_with(
            Ellipse::new(center, width, height),
            PatchProps::default()
                .hatch(hatch)
                .facecolor("none")
                .edgecolor(edge)
                .linewidth(linewidth),
        );
    }
    ax.set_axis_off();
    Ok(())
}

/// `test_text_clip`: a text clipped to the axes outside of them.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn text_clip(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.text(0.0, 0.0, "hello")
        .set_transform(Transform::Figure(fig.id()))
        .set_clip_on(true);
    Ok(())
}

/// `test_type42_font_without_prep`: text and mathtext in STIX.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn type42_font_without_prep(fig: &Figure) -> CaseResult {
    fig.text(0.5, 0.5, "Mass $m$");
    Ok(())
}

/// `test_path_collection`: a scatter plot of left triangles over yellow
/// polygons in data units at pixel offsets, left out of the limits.
fn path_collection(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let xvals = rng.uniform(10, 0.0, 1.0);
    let yvals = rng.uniform(10, 0.0, 1.0);
    let sizes = rng.uniform(10, 30.0, 100.0);
    let ax = fig.subplot();
    let options = ScatterOptions {
        sizes: Some(sizes),
        marker: Some("<".parse()?),
        edgecolors: Some(EdgeColors::Colors(vec![Color::from((0.9, 0.2, 0.1))])),
        ..ScatterOptions::default()
    };
    ax.scatter_with(xvals, yvals, options)?;
    ax.set_axis_off();
    let paths = (3..7).map(Path::unit_regular_polygon).collect();
    let offsets = rng
        .uniform(20, 0.0, 200.0)
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[x, y]| Point::new(x, y))
        .collect();
    let polygons = CollectionSpec::paths(paths)
        .with_offsets(offsets, Transform::Display)
        .with_props(CollectionProps::default().facecolor("yellow").zorder(-1.0))
        .without_autolim();
    ax.add_collection(polygons).set_sizes(vec![0.02, 0.04]);
    ax.set_xlim(0.0, 1.0);
    Ok(())
}

/// `test_colorbar_shift`: three colours over uneven boundaries.
fn colorbar_shift(fig: &Figure) -> CaseResult {
    let cmap = Colormap::listed("from_list", vec!["r", "g", "b"])?;
    let norm = BoundaryNorm::new([-1.0, -0.5, 0.5, 1.0], cmap.n(), Extend::Neither)?;
    let options = ScatterOptions {
        colors: MappedColors::new(vec![0.0, 1.0])
            .with_cmap(cmap)
            .with_norm(norm)
            .into(),
        ..ScatterOptions::default()
    };
    let points = fig
        .gca()
        .scatter_with(vec![0.0, 1.0], vec![1.0, 1.0], options)?;
    fig.colorbar(&points, ColorbarOptions::default())?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_backends/ps_transparency", transparency),
    classic("mpl_backends/ps_useafm", useafm).with_rc_setup(|rc| Ok(rc.set("ps.useafm", "True")?)),
    classic(
        "mpl_backends/ps_colored_hatch_zero_linewidth",
        colored_hatch_zero_linewidth,
    ),
    classic("mpl_backends/ps_text_clip", text_clip),
    FigureCase::new(
        "mpl_backends/ps_type42_font_without_prep",
        type42_font_without_prep,
    )
    .with_rc(&[("ps.fonttype", "42"), ("mathtext.fontset", "stix")]),
    classic("mpl_backends/ps_path_collection", path_collection),
    FigureCase::new("mpl_backends/ps_colorbar_shift", colorbar_shift)
        .with_rc(&[("savefig.bbox", "tight")]),
];
