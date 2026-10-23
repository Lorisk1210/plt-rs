//! Specifying colours: transparency against the drawing order, `"CN"`
//! colours under two styles, and the X11/CSS4 colours beside the xkcd
//! colours of the same names (`colors`).

use std::f64::consts::PI;

use plt::colors::{ColorTable, to_rgba};
use plt::fonts::FontWeight;
use plt::layout::LayoutEngine;
use plt::legend::LegendOptions;
use plt::patches::Rectangle;
use plt::prelude::*;
use plt::text::{HorizontalAlignment as HAlign, VerticalAlignment as VAlign};
use plt_core::Rgba;

use crate::figures::{CaseResult, FigureCase};

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn colors_1(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    ax.add_patch_with(
        Rectangle::new((-0.2, -0.35), 11.2, 0.7),
        PatchProps::default().color("C1").alpha(0.8),
    );
    for (i, alpha) in linspace(0.0, 1.0, 11).into_iter().enumerate() {
        let i = i as f64;
        ax.add_patch_with(
            Rectangle::new((i, 0.05), 0.8, 0.6),
            PatchProps::default().alpha(alpha).zorder(0.0),
        );
        ax.text(i + 0.4, 0.85, format!("{alpha:.1}"))
            .set_halign(HAlign::Center);
        ax.add_patch_with(
            Rectangle::new((i, -0.05), 0.8, -0.6),
            PatchProps::default().alpha(alpha).zorder(2.0),
        );
    }
    ax.set_xlim(-0.2, 13.0).set_ylim(-1.0, 1.0);
    ax.set_title("alpha values");
    for (y, text, color) in [
        (0.6, "zorder=1", "C0"),
        (0.0, "zorder=2\nalpha=0.8", "C1"),
        (-0.6, "zorder=3", "C0"),
    ] {
        ax.text(11.3, y, text)
            .set_valign(VAlign::Center)
            .set_color(color);
    }
    ax.set_axis_off();
    Ok(())
}

/// `demo`: the second and third colours of the style's cycle, the title in
/// its first.
fn style_demo(fig: &Figure, style: &str) -> CaseResult {
    let th = linspace(0.0, 2.0 * PI, 128);
    let ax = fig.subplot();
    ax.set_title(format!("style: '{style}'")).set_color("C0");
    for (f, color) in [(f64::cos as fn(f64) -> f64, "C1"), (f64::sin, "C2")] {
        let y: Vec<f64> = th.iter().map(|&t| f(t)).collect();
        ax.plot_with(&th, y, LineProps::fmt(color)?.label(color))?;
    }
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn colors_2(fig: &Figure) -> CaseResult {
    style_demo(fig, "default")
}

fn colors_3(fig: &Figure) -> CaseResult {
    style_demo(fig, "seaborn-v0_8")
}

/// `'k'` on colours lighter than half, `'w'` on the others (by perceived
/// luminance).
fn text_color(color: Rgba) -> &'static str {
    let luma = 0.299 * color.r + 0.587 * color.g + 0.114 * color.b;
    if luma > 0.5 { "k" } else { "w" }
}

fn colors_4(fig: &Figure) -> CaseResult {
    let xkcd = ColorTable::Xkcd.hex_entries();
    let mut overlap: Vec<(&str, &str, String)> = ColorTable::Css4
        .hex_entries()
        .iter()
        .filter_map(|&(name, css4)| {
            let key = format!("xkcd:{name}");
            let (_, hex) = xkcd.iter().find(|(xkcd_name, _)| *xkcd_name == key)?;
            Some((name, css4, hex.to_uppercase()))
        })
        .collect();
    overlap.sort_by_key(|&(name, ..)| name);
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    let n_groups = 3;
    let n_rows = overlap.len() / n_groups + 1;
    for (j, (color_name, css4, xkcd)) in overlap.iter().enumerate() {
        let col_shift = (j / n_rows * 3) as f64;
        let y_pos = (j % n_rows) as f64;
        let bold = css4 == xkcd;
        let styled = |text: plt::text::Text| {
            text.set_fontsize(10.0);
            if bold {
                text.set_weight(FontWeight::BOLD);
            }
        };
        for (column, hex) in [(0.0, *css4), (1.0, xkcd.as_str())] {
            ax.add_patch_with(
                Rectangle::new((column + col_shift, y_pos), 1.0, 1.0),
                PatchProps::default().color(hex),
            );
        }
        for (column, hex) in [(0.5, *css4), (1.5, xkcd.as_str())] {
            let text = ax.text(column + col_shift, y_pos + 0.7, hex);
            text.set_color(text_color(to_rgba(hex, None)?))
                .set_halign(HAlign::Center);
            styled(text);
        }
        styled(ax.text(2.0 + col_shift, y_pos + 0.7, format!("  {color_name}")));
    }
    let rows: Vec<f64> = (0..n_rows).map(|row| row as f64).collect();
    for g in 0..n_groups {
        let g = g as f64;
        ax.hlines(&rows, [3.0 * g], [3.0 * g + 2.8])?
            .set_edgecolor("0.7")
            .set_linewidths(vec![1.0]);
        for (x, label) in [(0.5, "X11/CSS4"), (1.5, "xkcd")] {
            ax.text(x + 3.0 * g, -0.3, label).set_halign(HAlign::Center);
        }
    }
    ax.set_xlim(0.0, 3.0 * n_groups as f64)
        .set_ylim(n_rows as f64, -1.0);
    ax.set_axis_off();
    Ok(())
}

/// The font family pinned on top of the seaborn style, which prefers Arial
/// and Liberation Sans, fonts hosts may or may not have.
const PINNED_FONT: &[(&str, &str)] = &[("font.family", "DejaVu Sans")];

/// The cases of this module; the style `colors` switches to for its third
/// figure stays in force for the fourth.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_colors_text/colors_1", colors_1).sized(6.5, 1.65, 100.0),
    FigureCase::new("gallery_guides_colors_text/colors_2", colors_2).sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/colors_3", colors_3)
        .sized(3.0, 3.0, 100.0)
        .with_style("seaborn-v0_8")
        .with_rc(PINNED_FONT),
    FigureCase::new("gallery_guides_colors_text/colors_4", colors_4)
        .sized(9.0, 5.0, 100.0)
        .with_style("seaborn-v0_8")
        .with_rc(PINNED_FONT),
];
