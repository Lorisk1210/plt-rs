//! Hinted text, `figures/cases_hinting.py`: `text.hinting` "default" and
//! "native" at several sizes and resolutions, with tick labels, titles,
//! rotated text and mathtext.

use plt::fonts::{FontStyle, FontWeight};
use plt::legend::LegendOptions;
use plt::prelude::*;
use plt::text::{HorizontalAlignment, VerticalAlignment};

use crate::figures::{CaseResult, FigureCase};

const DEFAULT: &[(&str, &str)] = &[("text.hinting", "default")];
const NATIVE: &[(&str, &str)] = &[("text.hinting", "native")];

/// `SAMPLE`: letters, figures, accents, a ligature and kerning pairs.
const SAMPLE: &str = "Hamburgefonstiv 0123456789 \u{c5}\u{c9}\u{e7}\u{df} ffi Wa.To, (x)";

/// `SIZES`: the point sizes of `_sizes`.
const SIZES: [u32; 11] = [6, 7, 8, 9, 10, 11, 12, 14, 17, 20, 24];

/// `MATH`: the formula of the mathtext cases.
const MATH: &str = r"$x^2 + y_i = \alpha\beta\gamma\,\frac{a}{b} - \pi r^2$";

/// `_sizes(fig, sizes, family)`: one line of the sample per size, top to
/// bottom.
fn sizes(fig: &Figure, sizes: &[u32], family: Option<&str>) {
    for (i, &size) in sizes.iter().enumerate() {
        let text = fig.text(0.02, 0.96 - 0.085 * i as f64, format!("{size} pt {SAMPLE}"));
        text.set_fontsize(f64::from(size))
            .set_valign(VerticalAlignment::Top);
        if let Some(family) = family {
            text.set_family([family]);
        }
    }
}

/// `_axes(fig)`: a plot with title, labels, tick labels and a legend.
fn axes(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 10.0, 40);
    let sin: Vec<f64> = x.iter().map(|x| x.sin()).collect();
    let cos: Vec<f64> = x.iter().map(|x| 0.5 * x.cos()).collect();
    ax.plot_with(&x, sin, LineProps::default().label("sin(x)"))?;
    ax.plot_with(&x, cos, LineProps::default().label("0.5 cos(x)"))?;
    ax.set_title("Hinted title: Waveform AVA");
    ax.set_xlabel("time [s]").set_ylabel("amplitude");
    ax.legend(LegendOptions::default().loc("upper right"))?;
    Ok(())
}

/// The mathtext cases: the formula at four sizes.
fn math(fig: &Figure) {
    for (i, size) in [8.0, 11.0, 15.0, 20.0].into_iter().enumerate() {
        fig.text(0.03, 0.9 - 0.23 * i as f64, MATH)
            .set_fontsize(size)
            .set_valign(VerticalAlignment::Top);
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("hinting/sizes_100dpi", |fig| {
        sizes(fig, &SIZES, None);
        Ok(())
    })
    .sized(6.0, 4.5, 100.0)
    .with_rc(DEFAULT),
    FigureCase::new("hinting/sizes_72dpi", |fig| {
        sizes(fig, &SIZES, None);
        Ok(())
    })
    .sized(8.0, 6.0, 72.0)
    .with_rc(DEFAULT),
    FigureCase::new("hinting/sizes_150dpi", |fig| {
        sizes(fig, &[6, 8, 10, 12, 14, 18, 24], None);
        Ok(())
    })
    .sized(4.5, 3.4, 150.0)
    .with_rc(DEFAULT),
    FigureCase::new("hinting/serif_and_mono", |fig| {
        let styles = [
            (9.0, FontWeight::NORMAL, FontStyle::Normal),
            (11.0, FontWeight::BOLD, FontStyle::Normal),
            (13.0, FontWeight::NORMAL, FontStyle::Italic),
        ];
        for (i, family) in ["DejaVu Serif", "DejaVu Sans Mono"].into_iter().enumerate() {
            for (j, (size, weight, style)) in styles.into_iter().enumerate() {
                fig.text(
                    0.02 + 0.5 * i as f64,
                    0.9 - 0.3 * j as f64,
                    format!("{family} {size}"),
                )
                .set_fontsize(size)
                .set_family([family])
                .set_weight(weight)
                .set_style(style)
                .set_valign(VerticalAlignment::Top);
            }
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(DEFAULT),
    FigureCase::new("hinting/axes_100dpi", axes)
        .sized(5.0, 3.75, 100.0)
        .with_rc(DEFAULT),
    FigureCase::new("hinting/axes_72dpi", axes)
        .sized(5.0, 3.75, 72.0)
        .with_rc(DEFAULT),
    FigureCase::new("hinting/axes_150dpi", axes)
        .sized(4.0, 3.0, 150.0)
        .with_rc(DEFAULT),
    FigureCase::new("hinting/rotated", |fig| {
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        ax.set_axis_off();
        for (i, angle) in [0, 15, 30, 45, 90, 135, 180, 270].into_iter().enumerate() {
            ax.text(0.6 + 1.2 * i as f64, 5.0, format!("rot {angle} Wa"))
                .set_rotation(f64::from(angle))
                .set_fontsize(10.0 + i as f64)
                .set_halign(HorizontalAlignment::Center)
                .set_valign(VerticalAlignment::Center);
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(DEFAULT),
    FigureCase::new("hinting/mathtext_dejavusans", |fig| {
        math(fig);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(&[
        ("text.hinting", "default"),
        ("mathtext.fontset", "dejavusans"),
    ]),
    FigureCase::new("hinting/mathtext_cm", |fig| {
        math(fig);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(&[("text.hinting", "default"), ("mathtext.fontset", "cm")]),
    FigureCase::new("hinting/native_stix", |fig| {
        for (i, size) in [7.0, 10.0, 14.0].into_iter().enumerate() {
            fig.text(0.03, 0.92 - 0.2 * i as f64, format!("{size} pt {SAMPLE}"))
                .set_fontsize(size)
                .set_family(["STIXGeneral"])
                .set_valign(VerticalAlignment::Top);
        }
        fig.text(
            0.03,
            0.3,
            r"$x^2 + y_i = \alpha\beta\gamma\,\frac{a}{b}$ and DejaVu text",
        )
        .set_fontsize(12.0)
        .set_valign(VerticalAlignment::Top);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(&[("text.hinting", "native"), ("mathtext.fontset", "stix")]),
    FigureCase::new("hinting/native_axes", axes)
        .sized(5.0, 3.75, 100.0)
        .with_rc(NATIVE),
];
