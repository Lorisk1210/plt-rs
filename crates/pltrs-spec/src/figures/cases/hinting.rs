//! Hinted text, `figures/cases_hinting.py`: `text.hinting` "default",
//! "native" and "`force_autohint`"/"auto" at several sizes and resolutions,
//! with tick labels, titles, rotated text and mathtext (STIX mathtext is
//! autohinted under "default"), also without anti-aliasing.

use pltrs::fonts::{FontStyle, FontWeight};
use pltrs::legend::LegendOptions;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;
use pltrs::text::{HAlign, VAlign};

use crate::figures::{CaseResult, FigureCase};

const DEFAULT: &[(&str, &str)] = &[("text.hinting", "default")];
const NATIVE: &[(&str, &str)] = &[("text.hinting", "native")];
const AUTOHINT: &[(&str, &str)] = &[("text.hinting", "force_autohint")];
const ALIASED: &[(&str, &str)] = &[("text.antialiased", "False")];

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
        text.set_fontsize(f64::from(size)).set_valign(VAlign::Top);
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
    ax.legend(LegendOptions::default().loc(AnchorLoc::UpperRight))?;
    Ok(())
}

/// The mathtext cases: the formula at four sizes.
fn math(fig: &Figure) {
    for (i, size) in [8.0, 11.0, 15.0, 20.0].into_iter().enumerate() {
        fig.text(0.03, 0.9 - 0.23 * i as f64, MATH)
            .set_fontsize(size)
            .set_valign(VAlign::Top);
    }
}

/// The serif and monospaced cases: both families in three styles, each
/// label followed by `suffix`.
fn serif_and_mono(fig: &Figure, suffix: &str) {
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
                format!("{family} {size}{suffix}"),
            )
            .set_fontsize(size)
            .set_family([family])
            .set_weight(weight)
            .set_style(style)
            .set_valign(VAlign::Top);
        }
    }
}

/// The rotated cases: labels at eight angles.
fn rotated(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 10.0)?;
    ax.set_axis_off();
    for (i, angle) in [0, 15, 30, 45, 90, 135, 180, 270].into_iter().enumerate() {
        ax.text(0.6 + 1.2 * i as f64, 5.0, format!("rot {angle} Wa"))
            .set_rotation(f64::from(angle))
            .set_fontsize(10.0 + i as f64)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center);
    }
    Ok(())
}

/// The STIX cases: the sample in STIX General at three sizes, then a
/// formula.
fn stix(fig: &Figure, formula: &str) {
    for (i, size) in [7.0, 10.0, 14.0].into_iter().enumerate() {
        fig.text(0.03, 0.92 - 0.2 * i as f64, format!("{size} pt {SAMPLE}"))
            .set_fontsize(size)
            .set_family(["STIXGeneral"])
            .set_valign(VAlign::Top);
    }
    fig.text(0.03, 0.3, formula)
        .set_fontsize(12.0)
        .set_valign(VAlign::Top);
}

/// The aliased large case: glyphs too tall for high precision, and a W
/// whose profiles overflow `FreeType`'s render pool.
fn large(fig: &Figure) {
    fig.text(0.02, 0.1, "BW%").set_fontsize(150.0);
    fig.text(0.62, 0.05, "W").set_fontsize(300.0);
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
        serif_and_mono(fig, "");
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
    FigureCase::new("hinting/rotated", rotated)
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
        stix(
            fig,
            r"$x^2 + y_i = \alpha\beta\gamma\,\frac{a}{b}$ and DejaVu text",
        );
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(&[("text.hinting", "native"), ("mathtext.fontset", "stix")]),
    FigureCase::new("hinting/native_axes", axes)
        .sized(5.0, 3.75, 100.0)
        .with_rc(NATIVE),
    FigureCase::new("hinting/autohint_sizes_100dpi", |fig| {
        sizes(fig, &SIZES, None);
        Ok(())
    })
    .sized(6.0, 4.5, 100.0)
    .with_rc(AUTOHINT),
    FigureCase::new("hinting/autohint_sizes_150dpi", |fig| {
        sizes(fig, &[6, 8, 10, 12, 14, 18, 24], None);
        Ok(())
    })
    .sized(4.5, 3.4, 150.0)
    .with_rc(&[("text.hinting", "auto")]),
    FigureCase::new("hinting/autohint_axes_72dpi", axes)
        .sized(5.0, 3.75, 72.0)
        .with_rc(AUTOHINT),
    FigureCase::new("hinting/autohint_serif_and_mono", |fig| {
        serif_and_mono(fig, " 0123 \u{c3}\u{f1}");
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(AUTOHINT),
    FigureCase::new("hinting/autohint_rotated", rotated)
        .sized(6.0, 3.0, 100.0)
        .with_rc(AUTOHINT),
    FigureCase::new("hinting/mathtext_stix", |fig| {
        math(fig);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(&[("text.hinting", "default"), ("mathtext.fontset", "stix")]),
    FigureCase::new("hinting/stix_default", |fig| {
        stix(
            fig,
            r"$\sum_{i=0}^{n} \sqrt{x_i} \int f\,dx$ and DejaVu text",
        );
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(&[("text.hinting", "default"), ("mathtext.fontset", "stix")]),
    FigureCase::new("hinting/aliased_sizes", |fig| {
        sizes(fig, &SIZES, None);
        Ok(())
    })
    .sized(6.0, 4.5, 100.0)
    .with_rc(&[("text.hinting", "default"), ("text.antialiased", "False")]),
    FigureCase::new("hinting/aliased_unhinted_sizes", |fig| {
        sizes(fig, &SIZES, None);
        Ok(())
    })
    .sized(6.0, 4.5, 100.0)
    .with_rc(ALIASED),
    FigureCase::new("hinting/aliased_unhinted_rotated", rotated)
        .sized(6.0, 3.0, 100.0)
        .with_rc(ALIASED),
    FigureCase::new("hinting/aliased_autohint_rotated", rotated)
        .sized(6.0, 3.0, 100.0)
        .with_rc(&[
            ("text.hinting", "force_autohint"),
            ("text.antialiased", "False"),
        ]),
    FigureCase::new("hinting/aliased_mathtext_cm", |fig| {
        math(fig);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(&[
        ("text.hinting", "default"),
        ("text.antialiased", "False"),
        ("mathtext.fontset", "cm"),
    ]),
    FigureCase::new("hinting/aliased_native_stix", |fig| {
        stix(
            fig,
            r"$x^2 + y_i = \alpha\beta\gamma\,\frac{a}{b}$ and DejaVu text",
        );
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(&[
        ("text.hinting", "native"),
        ("text.antialiased", "False"),
        ("mathtext.fontset", "stix"),
    ]),
    FigureCase::new("hinting/aliased_large", |fig| {
        large(fig);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0)
    .with_rc(ALIASED),
];
