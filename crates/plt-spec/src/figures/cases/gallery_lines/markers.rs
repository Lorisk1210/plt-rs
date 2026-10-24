//! The markers of the section: the marker reference sheets (unfilled,
//! filled, fill styles, mathtext, paths, rotations, cap and join styles),
//! `markevery` on linear, logarithmic, zoomed and polar axes, and markers
//! made of text paths.

use plt::axes::Axes;
use plt::colorbar::ColorbarOptions;
use plt::colors::{Normalize, ScalarMappable, colormaps};
use plt::fonts::FontProperties;
use plt::gridspec::GridSpec;
use plt::layout::LayoutEngine;
use plt::markers::MarkerStyle;
use plt::prelude::*;
use plt::text::{HAlign, VAlign, text_path};
use plt::transforms::{Affine2D, Point};
use plt_core::Path;
use plt_render::{CapStyle, JoinStyle};

use crate::figures::cases::common::markevery_sine;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The x of `ax.plot([y] * 3)`: the indices of the three markers.
const INDICES: [f64; 3] = [0.0, 1.0, 2.0];

/// The angles of the rotated marker sheets, in degrees.
const MARKER_ANGLES: [f64; 7] = [0.0, 10.0, 20.0, 30.0, 45.0, 60.0, 90.0];

/// `ax.text(x, y, label, **text_style)`: right-aligned, vertically
/// centred 12 pt monospace.
fn row_label(ax: &Axes, y: f64, label: &str) {
    ax.text(-0.5, y, label)
        .set_halign(HAlign::Right)
        .set_valign(VAlign::Center)
        .set_fontsize(12.0)
        .set_family(["monospace"]);
}

/// `marker_style`: markers on a dotted light grey line.
fn marker_style() -> LineProps {
    LineProps::default()
        .linestyle(LineStyle::Dotted)
        .color("0.8")
        .markersize(10.0)
        .markerfacecolor("tab:blue")
        .markeredgecolor("tab:blue")
}

/// `filled_marker_style` without its marker: half-filled-ready markers on
/// a dotted dark grey line.
fn filled_marker_style() -> LineProps {
    LineProps::default()
        .linestyle(LineStyle::Dotted)
        .markersize(15.0)
        .color("darkgrey")
        .markerfacecolor("tab:blue")
        .markerfacecoloralt("lightsteelblue")
        .markeredgecolor("brown")
}

/// `format_axes`: margins, no axis, rows from the top.
fn format_axes(ax: &Axes) -> CaseResult {
    ax.set_margins(0.2, 0.2)?;
    ax.set_axis_off();
    ax.yaxis().set_inverted(true);
    Ok(())
}

/// One labelled row of three markers, styled by `style`, per marker.
fn marker_rows(ax: &Axes, rows: &[(String, Marker)], style: &LineProps) -> CaseResult {
    for (y, (label, marker)) in (0_i32..).map(f64::from).zip(rows) {
        row_label(ax, y, label);
        ax.plot_with(INDICES, [y; 3], style.clone().marker(marker.clone()))?;
    }
    format_axes(ax)
}

/// Two columns of labelled marker rows, the first holding the first half.
fn marker_columns(fig: &Figure, title: &str, rows: &[(String, Marker)]) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    fig.suptitle(title).set_fontsize(14.0);
    let (first, second) = rows.split_at(rows.len() / 2);
    marker_rows(&axs[(0, 0)], first, &marker_style())?;
    marker_rows(&axs[(0, 1)], second, &marker_style())
}

/// Markers given by their string codes, labelled with Python's `repr`.
fn coded(codes: &[&str]) -> Result<Vec<(String, Marker)>, Box<dyn std::error::Error>> {
    codes
        .iter()
        .map(|code| Ok((format!("'{code}'"), code.parse()?)))
        .collect()
}

fn marker_reference_1(fig: &Figure) -> CaseResult {
    let mut rows = coded(&[",", "1", "2", "3", "4", "+", "x", "|", "_"])?;
    for code in 0..12 {
        let symbol = MarkerSymbol::from_code(code).ok_or("an integer marker code")?;
        rows.push((code.to_string(), symbol.into()));
    }
    marker_columns(fig, "Un-filled markers", &rows)
}

fn marker_reference_2(fig: &Figure) -> CaseResult {
    let filled = coded(&[
        ".", "o", "v", "^", "<", ">", "8", "s", "p", "*", "h", "H", "D", "d", "P", "X",
    ])?;
    marker_columns(fig, "Filled markers", &filled)
}

/// One axes for a sheet of marker rows, the figure titled `title` with
/// its subplots starting at `left`.
fn marker_sheet(fig: &Figure, title: &str, left: f64) -> Result<Axes, Box<dyn std::error::Error>> {
    let ax = fig.subplot();
    fig.suptitle(title).set_fontsize(14.0);
    fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
        left: Some(left),
        ..Default::default()
    })?;
    Ok(ax)
}

fn marker_reference_3(fig: &Figure) -> CaseResult {
    let ax = marker_sheet(fig, "Marker fillstyle", 0.4)?;
    let fillstyles = [
        ("full", FillStyle::Full),
        ("left", FillStyle::Left),
        ("right", FillStyle::Right),
        ("bottom", FillStyle::Bottom),
        ("top", FillStyle::Top),
        ("none", FillStyle::None),
    ];
    for (y, (name, fillstyle)) in (0_i32..).map(f64::from).zip(fillstyles) {
        row_label(&ax, y, &format!("'{name}'"));
        let style = filled_marker_style()
            .marker(MarkerSymbol::Circle)
            .fillstyle(fillstyle);
        ax.plot_with(INDICES, [y; 3], style)?;
    }
    format_axes(&ax)
}

fn marker_reference_4(fig: &Figure) -> CaseResult {
    let ax = marker_sheet(fig, "Mathtext markers", 0.4)?;
    let markers = [
        "$1$",
        r"$\frac{1}{2}$",
        "$f$",
        "$\u{266B}$",
        r"$\mathcal{A}$",
    ];
    // `repr(marker).replace("$", r"\$")`: the spelling, shown as plain text.
    let rows: Vec<(String, Marker)> = markers
        .iter()
        .map(|m| {
            let repr = format!("'{}'", m.replace('\\', r"\\"));
            Ok((repr.replace('$', r"\$"), m.parse()?))
        })
        .collect::<Result<_, plt::markers::MarkerError>>()?;
    let style = marker_style().markeredgecolor("none").markersize(15.0);
    marker_rows(&ax, &rows, &style)
}

fn marker_reference_5(fig: &Figure) -> CaseResult {
    let ax = marker_sheet(fig, "Path markers", 0.4)?;
    let star = Path::unit_regular_star(6, 0.5);
    let circle = Path::unit_circle();
    // The circle followed by the star walked backwards: a star cut out.
    let mut vertices = circle.vertices().to_vec();
    vertices.extend(star.vertices().iter().rev());
    let mut codes = circle.resolved_codes();
    codes.extend(star.resolved_codes());
    let cut_star = Path::new(vertices, codes)?;
    let rows = [("star", star), ("circle", circle), ("cut_star", cut_star)]
        .map(|(name, path)| (name.to_owned(), Marker::Path(path)));
    marker_rows(&ax, &rows, &marker_style())
}

/// `MarkerStyle(marker, fillstyle, Affine2D().rotate_deg(degrees))`.
fn rotated(
    marker: &str,
    fillstyle: FillStyle,
    degrees: f64,
) -> Result<MarkerStyle, plt::markers::MarkerError> {
    Ok(MarkerStyle::new(marker.parse::<Marker>()?, fillstyle)?.rotated_deg(degrees))
}

/// The degree labels above the columns of a rotated marker sheet.
fn angle_label(ax: &Axes, x: f64, y: f64, degrees: f64) {
    ax.text(x, y, format!("{degrees}°"))
        .set_halign(HAlign::Center);
}

fn marker_reference_6(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    fig.suptitle("Rotated markers").set_fontsize(14.0);
    let rows = [
        ("Filled marker", "o"),
        ("Un-filled marker", "1"),
        ("Equation marker", r"$\frac{1}{x}$"),
    ];
    for (y, (label, marker)) in (0_i32..).map(f64::from).zip(rows) {
        row_label(&ax, y, label);
        for (x, theta) in (0_i32..).map(f64::from).zip(MARKER_ANGLES) {
            let style = rotated(marker, FillStyle::Left, theta)?;
            ax.plot_with([x], [y], filled_marker_style().marker_style(&style))?;
        }
    }
    for (x, theta) in (0_i32..).map(f64::from).zip(MARKER_ANGLES) {
        angle_label(&ax, x, 2.5, theta);
    }
    format_axes(&ax)?;
    fig.tight_layout(&plt::layout::TightLayout::default())?;
    Ok(())
}

/// `marker_inner` and `marker_outer`: big markers edged `edgecolor`.
fn big_marker(edgecolor: &str, edgewidth: f64) -> LineProps {
    LineProps::default()
        .markersize(35.0)
        .markerfacecolor("tab:blue")
        .markerfacecoloralt("lightsteelblue")
        .markeredgecolor(edgecolor)
        .markeredgewidth(edgewidth)
}

fn marker_reference_7(fig: &Figure) -> CaseResult {
    let ax = marker_sheet(fig, "Marker CapStyle", 0.1)?;
    let styles = [
        ("butt", CapStyle::Butt),
        ("projecting", CapStyle::Projecting),
        ("round", CapStyle::Round),
    ];
    for (y, (name, capstyle)) in (0_i32..).map(f64::from).zip(styles) {
        row_label(&ax, y, name);
        for (x, theta) in (0_i32..).map(f64::from).zip(MARKER_ANGLES) {
            let style = MarkerStyle::with_styles(
                MarkerSymbol::TriDown,
                FillStyle::Full,
                Some(capstyle),
                None,
            )?
            .rotated_deg(theta);
            ax.plot_with([x], [y], big_marker("brown", 8.0).marker_style(&style))?;
            ax.plot_with([x], [y], big_marker("white", 1.0).marker_style(&style))?;
            angle_label(&ax, x, 2.5, theta);
        }
    }
    format_axes(&ax)
}

fn marker_reference_8(fig: &Figure) -> CaseResult {
    let ax = marker_sheet(fig, "Marker JoinStyle", 0.05)?;
    let styles = [
        ("miter", JoinStyle::Miter),
        ("round", JoinStyle::Round),
        ("bevel", JoinStyle::Bevel),
    ];
    for (y, (name, joinstyle)) in (0_i32..).map(f64::from).zip(styles) {
        row_label(&ax, y, name);
        for (x, theta) in (0_i32..).map(f64::from).zip(MARKER_ANGLES) {
            let style = MarkerStyle::with_styles(
                MarkerSymbol::Star,
                FillStyle::Full,
                None,
                Some(joinstyle),
            )?
            .rotated_deg(theta);
            ax.plot_with([x], [y], big_marker("brown", 8.0).marker_style(&style))?;
            angle_label(&ax, x, 2.5, theta);
        }
    }
    format_axes(&ax)
}

/// The `markevery` values of the demo with their Python spelling.
fn markevery_cases() -> [(Option<MarkEvery>, &'static str); 9] {
    let distance = |start, step| Some(MarkEvery::Distance { start, step });
    [
        (None, "None"),
        (Some(MarkEvery::Every { start: 0, step: 8 }), "8"),
        (Some(MarkEvery::Every { start: 30, step: 8 }), "(30, 8)"),
        (Some(MarkEvery::Indices(vec![16, 24, 32])), "[16, 24, 32]"),
        (Some(MarkEvery::Indices(vec![0, -1])), "[0, -1]"),
        (
            Some(MarkEvery::Slice {
                start: Some(100),
                stop: Some(200),
                step: Some(3),
            }),
            "slice(100, 200, 3)",
        ),
        (distance(0.0, 0.1), "0.1"),
        (distance(0.0, 0.4), "0.4"),
        (distance(0.2, 0.4), "(0.2, 0.4)"),
    ]
}

/// How the `markevery` grids set up their axes.
#[derive(Clone, Copy, PartialEq)]
enum GridAxes {
    /// Plain rectilinear axes.
    Linear,
    /// Log scales on both axes before plotting.
    Log,
    /// Rectilinear axes zoomed to `x` 6–6.7, `y` 1.1–1.7 after plotting.
    Zoomed,
    /// Polar axes.
    Polar,
}

/// The 3 × 3 constrained grid of the demo, one axes per `markevery`
/// value, titled with it, plotting `y` against `x` as `o-` with 4 pt
/// markers.
fn markevery_grid(fig: &Figure, x: &[f64], y: &[f64], kind: GridAxes) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let grid = GridSpec::new(3, 3)?;
    for (i, (markevery, spelling)) in markevery_cases().into_iter().enumerate() {
        let spec = grid.cell(i / 3, i % 3)?;
        let ax = if kind == GridAxes::Polar {
            (*fig.add_polar_subplot_spec(&spec)?).clone()
        } else {
            fig.add_subplot_spec(&spec)?
        };
        ax.set_title(format!("markevery={spelling}"));
        if kind == GridAxes::Log {
            ax.set_xscale("log")?.set_yscale("log")?;
        }
        let style = LineProps::fmt("o")?
            .linestyle(LineStyle::Solid)
            .markersize(4.0)
            .markevery(markevery);
        ax.plot_with(x, y, style)?;
        if kind == GridAxes::Zoomed {
            ax.set_xlim(6.0, 6.7)?.set_ylim(1.1, 1.7)?;
        }
    }
    Ok(())
}

fn markevery_demo_1(fig: &Figure) -> CaseResult {
    let (x, y) = markevery_sine();
    markevery_grid(fig, &x, &y, GridAxes::Linear)
}

fn markevery_demo_2(fig: &Figure) -> CaseResult {
    let (x, y) = markevery_sine();
    markevery_grid(fig, &x, &y, GridAxes::Log)
}

fn markevery_demo_3(fig: &Figure) -> CaseResult {
    let (x, y) = markevery_sine();
    markevery_grid(fig, &x, &y, GridAxes::Zoomed)
}

fn markevery_demo_4(fig: &Figure) -> CaseResult {
    let r = linspace(0.0, 3.0, 200);
    let theta: Vec<f64> = r.iter().map(|r| 2.0 * std::f64::consts::PI * r).collect();
    markevery_grid(fig, &theta, &r, GridAxes::Polar)
}

fn multivariate_marker_plot(fig: &Figure) -> CaseResult {
    let font = FontProperties::from_rc(&plt::rc::params());
    let symbols = ["\u{2639}", "\u{1F612}", "\u{263A}"]
        .map(|symbol| text_path(Point::new(0.0, 0.0), symbol, None, &font));
    let n = 25;
    let mut rng = SpecRng::new(42);
    let skills: Vec<f64> = rng
        .uniform(n, 5.0, 80.0)
        .iter()
        .map(|s| s * 0.1 + 5.0)
        .collect();
    let takeoff_angles = rng.normal(n, 0.0, 90.0);
    let thrusts = rng.uniform(n, 0.0, 1.0);
    let successful = rng.integers(n, 0, 3);
    let positions = rng.normal(2 * n, 0.0, 1.0);
    let cmap = colormaps::get("plasma")?;
    let ax = fig.subplot();
    fig.suptitle("Throwing success").set_fontsize(14.0);
    for i in 0..n {
        let transform = Affine2D::scaling(skills[i], skills[i]).rotate_deg(takeoff_angles[i]);
        let mood = usize::try_from(successful[i])?;
        let symbol = symbols[mood].clone()?;
        let style = MarkerStyle::new(symbol, FillStyle::Full)?.transformed(transform);
        let props = LineProps::default()
            .marker_style(&style)
            .color(Color::from(cmap.sample(thrusts[i])));
        ax.plot_with(
            [5.0 * positions[2 * i]],
            [5.0 * positions[2 * i + 1]],
            props,
        )?;
    }
    let thrust = ScalarMappable::with_norm(cmap, Box::new(Normalize::new(0.0, 1.0)));
    let options = ColorbarOptions {
        label: "Normalized Thrust [a.u.]".to_owned(),
        ..ColorbarOptions::beside(&ax)
    };
    fig.colorbar(&thrust, options)?;
    ax.set_xlabel("X position [m]");
    ax.set_ylabel("Y position [m]");
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_lines/marker_reference_1", marker_reference_1),
    FigureCase::new("gallery_lines/marker_reference_2", marker_reference_2),
    FigureCase::new("gallery_lines/marker_reference_3", marker_reference_3),
    FigureCase::new("gallery_lines/marker_reference_4", marker_reference_4),
    FigureCase::new("gallery_lines/marker_reference_5", marker_reference_5),
    FigureCase::new("gallery_lines/marker_reference_6", marker_reference_6),
    FigureCase::new("gallery_lines/marker_reference_7", marker_reference_7),
    FigureCase::new("gallery_lines/marker_reference_8", marker_reference_8),
    FigureCase::new("gallery_lines/markevery_demo_1", markevery_demo_1).sized(10.0, 6.0, 100.0),
    FigureCase::new("gallery_lines/markevery_demo_2", markevery_demo_2).sized(10.0, 6.0, 100.0),
    FigureCase::new("gallery_lines/markevery_demo_3", markevery_demo_3).sized(10.0, 6.0, 100.0),
    FigureCase::new("gallery_lines/markevery_demo_4", markevery_demo_4).sized(10.0, 6.0, 100.0),
    FigureCase::new(
        "gallery_lines/multivariate_marker_plot",
        multivariate_marker_plot,
    ),
];
