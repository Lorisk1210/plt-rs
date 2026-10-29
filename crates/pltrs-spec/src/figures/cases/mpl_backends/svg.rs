//! The image tests of `test_backend_svg.py`: translucent black markers, an
//! image drawn without scaling, bold labels (also as SVG text), rasterized
//! lines among vector ones, an artist kept out of a rasterized group, and
//! text needing fallback fonts.

use pltrs::axes::{ImshowOptions, PlotError, ScatterColors, ScatterOptions};
use pltrs::colors::{Alpha, ColorizerOptions, colormaps};
use pltrs::fonts::FontWeight;
use pltrs::image::Interpolation;
use pltrs::numeric::Grid;
use pltrs::offsetbox::{AnnotationBboxOptions, TextArea};
use pltrs::prelude::*;
use pltrs::text::TextProps;

use super::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// `test_fill_black_with_alpha`: large, overlapping translucent black
/// discs.
fn fill_black_with_alpha(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = ScatterOptions {
        sizes: Some(vec![10_000.0]),
        colors: ScatterColors::Single(Color::from("k")),
        alpha: Alpha::Uniform(0.1),
        ..ScatterOptions::default()
    };
    ax.scatter_with(vec![0.0, 0.1, 1.0], vec![0.0; 3], options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_noscale`: `sin(y²)` over a 10 × 10 grid, not interpolated.
fn noscale(fig: &Figure) -> CaseResult {
    let z = Grid::from_fn(10, 10, |r, _| (r as f64 - 5.0).powi(2).sin());
    let ax = fig.subplot();
    ax.imshow(
        z,
        ImshowOptions {
            interpolation: Some(Interpolation::None),
            colorizer: ColorizerOptions::default().cmap(colormaps::get("gray")?),
            ..ImshowOptions::default()
        },
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The `bold_font_output*` tests: a bold y label and a title of weight
/// 600.
fn bold_font_output(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x: Vec<f64> = (0..10).map(f64::from).collect();
    ax.plot(x.clone(), x)?;
    ax.set_xlabel("nonbold-xlabel");
    ax.set_ylabel("bold-ylabel");
    ax.yaxis().set_label_weight(FontWeight::BOLD);
    ax.set_title("bold-title")
        .set_weight("600".parse::<FontWeight>()?);
    Ok(())
}

/// The rasterization tests' 100 points on the unit circle, `t = 2.3 i`,
/// shifted right by `shift`.
fn circle_points(shift: f64) -> (Vec<f64>, Vec<f64>) {
    (0..100)
        .map(|i| f64::from(i) * 2.3)
        .map(|t| (t.cos() + shift, t.sin()))
        .unzip()
}

/// `plot(x + shift, y, "-", c=color, lw=10)` through [`circle_points`].
fn circle_line(ax: &Axes, shift: f64, color: &str, zorder: f64) -> Result<Line2D, PlotError> {
    let (x, y) = circle_points(shift);
    let props = LineProps::default()
        .linestyle(LineStyle::Solid)
        .color(color)
        .linewidth(10.0)
        .zorder(zorder);
    ax.plot_with(x, y, props)
}

/// The z-order of lines.
const LINE_ZORDER: f64 = 2.0;

/// `test_rasterized`: two rasterized lines.
fn rasterized(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    circle_line(&ax, 0.0, "r", LINE_ZORDER)?.set_rasterized(true);
    circle_line(&ax, 1.0, "b", LINE_ZORDER)?.set_rasterized(true);
    Ok(())
}

/// `test_rasterized_ordering`: a vector line between rasterized ones, by
/// z-order rather than by the order they were added.
fn rasterized_ordering(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 3.0)?.set_ylim(-1.1, 1.1)?;
    circle_line(&ax, 0.0, "r", 1.1)?.set_rasterized(true);
    circle_line(&ax, 2.0, "g", 1.3)?.set_rasterized(true);
    circle_line(&ax, 3.0, "m", 1.4)?.set_rasterized(true);
    circle_line(&ax, 1.0, "b", 1.2)?;
    Ok(())
}

/// `test_prevent_rasterization`: a rasterized marker under an annotation
/// box that stays vector.
fn prevent_rasterization(fig: &Figure) -> CaseResult {
    let loc = (0.05, 0.05);
    let ax = fig.subplot();
    let marker = LineProps::default()
        .marker("x".parse::<Marker>()?)
        .color("black")
        .zorder(2.0);
    ax.plot_with([loc.0], [loc.1], marker)?.set_rasterized(true);
    ax.add_annotation_bbox(
        TextArea::from_text("X", TextProps::default()),
        loc,
        AnnotationBboxOptions::default(),
    )?
    .set_zorder(2.1);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic(
        "mpl_backends/svg_fill_black_with_alpha",
        fill_black_with_alpha,
    ),
    classic("mpl_backends/svg_noscale", noscale),
    FigureCase::new("mpl_backends/svg_bold_font_output", bold_font_output),
    classic(
        "mpl_backends/svg_bold_font_output_with_none_fonttype",
        bold_font_output,
    )
    .with_rc_setup(|rc| Ok(rc.set("svg.fonttype", "none")?)),
    classic("mpl_backends/svg_rasterized", rasterized),
    classic("mpl_backends/svg_rasterized_ordering", rasterized_ordering),
    classic(
        "mpl_backends/svg_prevent_rasterization",
        prevent_rasterization,
    ),
];
