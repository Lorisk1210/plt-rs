//! Reference sheets: every marker, line style and hatch pattern, named
//! as they are written in code.

use pltrs::fonts::FontWeight;
use pltrs::patches::Rectangle;
use pltrs::prelude::*;
use pltrs::rc::Aspect;
use pltrs::text::{HAlign, Text, VAlign};

use super::{Built, Entry, background};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure("markers", "Markers: filled, unfilled and mathtext", markers),
    Entry::figure(
        "line_styles",
        "Line styles: named and dash patterns",
        line_styles,
    ),
    Entry::figure("hatches", "Hatch patterns and their densities", hatches),
];

/// A label in monospace, so that it reads as code, with its dollar signs
/// taken literally rather than as mathtext.
fn code_label(ax: &Axes, x: f64, y: f64, text: &str) -> Text {
    let label = ax.text(x, y, text);
    label
        .set_family(["DejaVu Sans Mono"])
        .set_fontsize(9.0)
        .set_parse_math(false);
    label
}

/// Every marker shape on a grid of cells, each named by the string that
/// selects it.
fn markers() -> Built {
    let fig = Figure::with_size(7.5, 4.8, 100.0);
    let ax = fig.subplot();
    let filled = [
        "o", "v", "^", "<", ">", "8", "s", "p", "*", "h", "H", "D", "d", "P", "X",
    ];
    let unfilled = [".", ",", "1", "2", "3", "4", "+", "x", "|", "_"];
    let math = [r"$\clubsuit$", r"$\heartsuit$", r"$\star$", r"$\checkmark$"];
    // Each family with its number of columns: mathtext names are longer.
    let rows: [(&str, &[&str], usize); 3] = [
        ("filled", &filled, 8),
        ("unfilled", &unfilled, 8),
        ("mathtext", &math, 4),
    ];
    let mut y = 0.0;
    for (family, names, columns) in rows {
        let heading = ax.text(-0.5, y + 0.55, family);
        heading.set_fontsize(10.0).set_weight(FontWeight::BOLD);
        let step = 8.0 / columns as f64;
        for (i, name) in names.iter().enumerate() {
            let at = ((i % columns) as f64 * step, y - (i / columns) as f64);
            let style = LineProps::default()
                .marker(name.parse::<Marker>()?)
                .markersize(14.0)
                .linestyle(LineStyle::None);
            ax.plot_with([at.0], [at.1], style)?;
            code_label(&ax, at.0 + 0.25, at.1, &format!("\"{name}\"")).set_valign(VAlign::Center);
        }
        y -= 1.0 + names.len().div_ceil(columns) as f64;
    }
    ax.set_xlim(-0.6, 8.0)?.set_ylim(y + 0.6, 1.0)?;
    ax.set_axis_off();
    ax.set_title("Markers");
    Ok(fig)
}

/// The named line styles and a few dash patterns `(offset, on/off
/// lengths)`, measured in line widths.
fn line_styles() -> Built {
    let fig = Figure::with_size(7.0, 4.5, 100.0);
    let ax = fig.subplot();
    let styles = [
        ("\"-\"  solid", LineStyle::Solid),
        ("\"--\" dashed", LineStyle::Dashed),
        ("\"-.\" dashdot", LineStyle::DashDot),
        ("\":\"  dotted", LineStyle::Dotted),
        (
            "(0, [1, 1]) densely dotted",
            LineStyle::dashes(0.0, vec![1.0, 1.0]),
        ),
        (
            "(0, [5, 10]) loosely dashed",
            LineStyle::dashes(0.0, vec![5.0, 10.0]),
        ),
        (
            "(0, [3, 1, 1, 1]) dash-dotted",
            LineStyle::dashes(0.0, vec![3.0, 1.0, 1.0, 1.0]),
        ),
        (
            "(0, [3, 5, 1, 5, 1, 5]) dash-dot-dotted",
            LineStyle::dashes(0.0, vec![3.0, 5.0, 1.0, 5.0, 1.0, 5.0]),
        ),
        (
            "(5, [10, 3]) offset dashes",
            LineStyle::dashes(5.0, vec![10.0, 3.0]),
        ),
    ];
    for (i, (name, style)) in styles.into_iter().enumerate() {
        let y = -(i as f64);
        ax.plot_with(
            [0.0, 1.0],
            [y, y],
            LineProps::default()
                .linestyle(style)
                .linewidth(2.0)
                .color("C0"),
        )?;
        code_label(&ax, 1.05, y, name).set_valign(VAlign::Center);
    }
    ax.set_xlim(-0.05, 2.0)?.set_ylim(-8.6, 0.6)?;
    ax.set_axis_off();
    ax.set_title("Line styles");
    Ok(fig)
}

/// Each hatch pattern in a square, its density doubling with each repeat
/// of the character, and two patterns combined.
fn hatches() -> Built {
    let fig = Figure::with_size(7.5, 3.0, 100.0);
    let ax = fig.subplot();
    let patterns = ["/", "\\", "|", "-", "+", "x", "o", "O", ".", "*"];
    for (i, pattern) in patterns.iter().enumerate() {
        for (row, repeat) in [1, 2, 3].into_iter().enumerate() {
            let hatch = pattern.repeat(repeat);
            let (x, y) = (i as f64 * 1.2, -(row as f64) * 1.4);
            let square = PatchProps::default()
                .facecolor(background())
                .edgecolor("C0")
                .hatch(&hatch);
            ax.add_patch_with(Rectangle::new((x, y), 1.0, 1.0), square);
            let label = code_label(&ax, x + 0.5, y - 0.12, &format!("{hatch:?}"));
            label
                .set_halign(HAlign::Center)
                .set_valign(VAlign::Top)
                .set_fontsize(8.0);
        }
    }
    let combined = PatchProps::default()
        .facecolor(background())
        .edgecolor("C1")
        .hatch("/o");
    ax.add_patch_with(Rectangle::new((12.0, -1.4), 1.0, 1.0), combined);
    code_label(&ax, 12.5, -1.52, "\"/o\"")
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Top);
    ax.set_xlim(-0.2, 13.2)?.set_ylim(-3.2, 1.2)?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_axis_off();
    ax.set_title("Hatches");
    Ok(fig)
}
