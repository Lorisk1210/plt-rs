//! `figures/cases_afm.py`: text in the standard PostScript fonts, which the
//! PDF backend (`pdf.use14corefonts`) and the PostScript backend
//! (`ps.useafm`) measure, lay out and draw with Adobe Font Metrics.

use plt::fonts::{FontStyle, FontWeight};
use plt::layout::LayoutEngine;
use plt::legend::LegendOptions;
use plt::offsetbox::AnchorLoc;
use plt::prelude::*;
use plt::text::{HAlign, TextRotationMode, VAlign};

use crate::figures::{CaseResult, FigureCase};

/// `CORE_FONTS`: the PDF backend's 14 core fonts.
const CORE_FONTS: &[(&str, &str)] = &[("pdf.use14corefonts", "True")];

/// `AFM_FONTS`: the PostScript backend's AFM fonts.
const AFM_FONTS: &[(&str, &str)] = &[("ps.useafm", "True")];

/// `_gallery`: families, weights and styles, kerning pairs, Latin-1 and
/// other characters, alignment, rotation, two lines and a mathtext line.
fn gallery(fig: &Figure) {
    fig.text(0.02, 0.92, "AVATAR Toyota WAVE")
        .set_fontsize(14.0);
    fig.text(0.02, 0.8, "serif: AVATAR Toyota")
        .set_family(["serif"])
        .set_fontsize(12.0);
    fig.text(0.02, 0.69, "monospace bold")
        .set_family(["monospace"])
        .set_weight(FontWeight::BOLD);
    fig.text(0.02, 0.59, "Helvetica bold oblique")
        .set_family(["Helvetica"])
        .set_weight(FontWeight::BOLD)
        .set_style(FontStyle::Oblique);
    fig.text(0.02, 0.49, "Times italic")
        .set_family(["Times"])
        .set_style(FontStyle::Italic)
        .set_fontsize(11.0);
    fig.text(
        0.02,
        0.38,
        "caf\u{e9} na\u{ef}ve \u{c5}ngstr\u{f6}m \u{b1}\u{bd}\u{d7}\u{a9}",
    )
    .set_fontsize(11.0);
    fig.text(
        0.02,
        0.27,
        "\u{20ac} \u{2013} \u{201c}quotes\u{201d} \u{2192} \u{3b1} \u{2212}",
    )
    .set_fontsize(11.0);
    fig.text(0.02, 0.1, r"math $\alpha^2+\frac{1}{2}$")
        .set_fontsize(11.0);
    fig.text(0.98, 0.95, "top right")
        .set_halign(HAlign::Right)
        .set_valign(VAlign::Top);
    fig.text(0.75, 0.62, "centred\ntwo lines")
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_multialign(Some(HAlign::Center))
        .set_fontsize(12.0);
    fig.text(0.7, 0.25, "rotated")
        .set_rotation(30.0)
        .set_halign(HAlign::Center);
    fig.text(0.92, 0.1, "anchor")
        .set_rotation(90.0)
        .set_rotation_mode(TextRotationMode::Anchor)
        .set_valign(VAlign::Bottom);
}

/// `_layout`: a title, labels, tick labels, a legend and a suptitle placed
/// by the constrained layout engine, which measures text at draw time.
fn layout(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let x = linspace(-3.0, 3.0, 40);
    let sin: Vec<f64> = x.iter().map(|v| v.sin()).collect();
    let cos: Vec<f64> = x.iter().map(|v| v.cos()).collect();
    ax.plot_with(x.clone(), sin, LineProps::default().label("sin AVATAR"))?;
    ax.plot_with(x, cos, LineProps::default().label("cos Toyota"))?;
    ax.set_title("Title: WAVE");
    ax.set_xlabel("time (s)").set_ylabel("amplitude");
    ax.legend(LegendOptions::default().loc(AnchorLoc::UpperLeft))?;
    fig.suptitle("Suptitle");
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("afm/core_fonts_text", |fig| {
        gallery(fig);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0)
    .with_rc(CORE_FONTS),
    FigureCase::new("afm/core_fonts_layout", layout)
        .sized(4.0, 3.0, 100.0)
        .with_rc(CORE_FONTS),
    FigureCase::new("afm/ps_fonts_text", |fig| {
        gallery(fig);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0)
    .with_rc(AFM_FONTS),
    FigureCase::new("afm/ps_fonts_layout", layout)
        .sized(4.0, 3.0, 100.0)
        .with_rc(AFM_FONTS),
];
