//! Alignment, rotation, layout, font, colour, wrapping and coordinate
//! system cases of `figures/cases_text.py`.

use plt::fonts::FontStyle;
use plt::fonts::{FontStretch, FontWeight, RelativeSize};
use plt::prelude::*;
use plt::text::{HAlign, TextRotationMode, VAlign};

use super::{LOREM, blank};
use crate::figures::{CaseResult, FigureCase};

/// `ax.plot(x, y, "r+", ms=10)`: the red cross marking a text anchor.
fn mark(ax: &Axes, x: f64, y: f64) -> CaseResult {
    ax.plot_with([x], [y], LineProps::fmt("r+")?.markersize(10.0))?;
    Ok(())
}

/// The rotated texts of the two rotation cases, in `mode`.
fn rotations(fig: &Figure, mode: TextRotationMode) -> CaseResult {
    let ax = blank(fig);
    for (i, rotation) in [0, 30, 45, 90, 135, 180, 270].into_iter().enumerate() {
        let x = 0.8 + 1.4 * i as f64;
        mark(&ax, x, 5.0)?;
        ax.text(x, 5.0, format!("rot {rotation}"))
            .set_rotation(f64::from(rotation))
            .set_rotation_mode(mode)
            .set_valign(VAlign::Bottom);
    }
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("text/text_default", |fig| {
        let ax = fig.subplot();
        ax.text(0.5, 0.5, "default text");
        ax.text(0.1, 0.8, "second\nline");
        Ok(())
    }),
    FigureCase::new("text/ha_va_grid", |fig| {
        let ax = blank(fig);
        let valigns = [
            (VAlign::Top, "top"),
            (VAlign::Center, "ctr"),
            (VAlign::Baseline, "base"),
            (VAlign::Bottom, "bot"),
            (VAlign::CenterBaseline, "cbase"),
        ];
        let haligns = [
            (HAlign::Left, 'l'),
            (HAlign::Center, 'c'),
            (HAlign::Right, 'r'),
        ];
        for (i, (halign, initial)) in haligns.into_iter().enumerate() {
            for (j, (valign, short)) in valigns.into_iter().enumerate() {
                let (x, y) = (1.5 + 3.5 * i as f64, 9.0 - 2.0 * j as f64);
                mark(&ax, x, y)?;
                ax.text(x, y, format!("{initial}/{short}g"))
                    .set_halign(halign)
                    .set_valign(valign)
                    .set_fontsize(11.0);
            }
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/rotation_default_mode", |fig| {
        rotations(fig, TextRotationMode::Default)
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("text/rotation_mode_anchor", |fig| {
        rotations(fig, TextRotationMode::Anchor)
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("text/font_sizes", |fig| {
        let ax = blank(fig);
        let sizes = [
            RelativeSize::XxSmall,
            RelativeSize::XSmall,
            RelativeSize::Small,
            RelativeSize::Medium,
            RelativeSize::Large,
            RelativeSize::XLarge,
            RelativeSize::XxLarge,
        ];
        for (i, size) in sizes.into_iter().enumerate() {
            ax.text(0.3, 9.3 - 1.3 * i as f64, format!("size {}", size.name()))
                .set_fontsize(size);
        }
        for (i, size) in [6, 9, 14, 20, 28].into_iter().enumerate() {
            ax.text(5.5, 9.3 - 1.9 * i as f64, format!("{size} pt"))
                .set_fontsize(f64::from(size));
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/font_families_styles", |fig| {
        let ax = blank(fig);
        let rows = [
            ("sans-serif", FontStyle::Normal),
            ("serif", FontStyle::Normal),
            ("monospace", FontStyle::Normal),
            ("sans-serif", FontStyle::Italic),
            ("serif", FontStyle::Italic),
            ("monospace", FontStyle::Oblique),
            ("DejaVu Serif", FontStyle::Normal),
        ];
        for (i, (family, style)) in rows.into_iter().enumerate() {
            let name = plt::fonts::style_name(style);
            ax.text(
                0.5,
                9.2 - 1.3 * i as f64,
                format!("{family} {name} Quick fox 0123"),
            )
            .set_family([family])
            .set_style(style)
            .set_fontsize(13.0);
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/font_weights_stretch", |fig| {
        let ax = blank(fig);
        let weights = [
            "ultralight",
            "light",
            "normal",
            "medium",
            "semibold",
            "bold",
            "heavy",
            "black",
        ];
        for (i, weight) in weights.into_iter().enumerate() {
            ax.text(0.3, 9.3 - 1.15 * i as f64, format!("weight {weight}"))
                .set_weight(weight.parse::<FontWeight>()?)
                .set_fontsize(12.0);
        }
        let stretches = ["ultra-condensed", "condensed", "normal", "expanded"];
        for (i, stretch) in stretches.into_iter().enumerate() {
            ax.text(5.3, 9.3 - 1.5 * i as f64, stretch)
                .set_stretch(stretch.parse::<FontStretch>()?)
                .set_fontsize(12.0);
        }
        ax.text(5.3, 2.5, "serif bold")
            .set_family(["serif"])
            .set_weight(FontWeight::BOLD)
            .set_fontsize(12.0);
        ax.text(5.3, 1.0, "mono bold")
            .set_family(["monospace"])
            .set_weight(FontWeight::BOLD)
            .set_fontsize(12.0);
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/rc_text_color", |fig| {
        let ax = fig.subplot();
        ax.text(0.2, 0.5, "inherits text.color");
        ax.set_title("title colour");
        ax.set_xlabel("label (axes.labelcolor)");
        fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
            bottom: Some(0.18),
            ..Default::default()
        })?;
        Ok(())
    })
    .sized(4.0, 3.5, 100.0)
    .with_rc(&[("text.color", "tab:green")]),
    FigureCase::new("text/wrap", |fig| {
        let ax = fig.subplot();
        ax.text(0.05, 0.95, LOREM)
            .set_wrap(true)
            .set_valign(VAlign::Top)
            .set_transform(ax.trans_axes())
            .set_fontsize(9.0);
        ax.text(0.05, 0.1, &LOREM[..120])
            .set_wrap(true)
            .set_rotation(15.0)
            .set_fontsize(9.0)
            .set_transform(ax.trans_axes());
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("text/fig_text_wrap_paragraph", |fig| {
        fig.text(0.02, 0.98, format!("{LOREM} {LOREM}"))
            .set_wrap(true)
            .set_valign(VAlign::Top)
            .set_fontsize(10.0);
        fig.text(0.5, 0.1, format!("centred: {}", &LOREM[..90]))
            .set_wrap(true)
            .set_halign(HAlign::Center);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("text/coordinate_systems", |fig| {
        let ax = fig.subplot();
        ax.set_xlim(0.0, 4.0).set_ylim(0.0, 2.0);
        ax.text(1.0, 1.0, "data (1, 1)");
        ax.text(0.98, 0.02, "axes (0.98, 0.02)")
            .set_transform(ax.trans_axes())
            .set_halign(HAlign::Right);
        ax.text(0.5, 1.02, "axes top")
            .set_transform(ax.trans_axes())
            .set_halign(HAlign::Center);
        fig.text(0.01, 0.01, "figure (0.01, 0.01)");
        fig.text(0.99, 0.99, "fig top right")
            .set_halign(HAlign::Right)
            .set_valign(VAlign::Top);
        fig.suptitle("suptitle");
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
