//! The font, multi-line, alignment, title and text box cases of
//! `test_text.py`.

use crate::figures::cases::common::tickless;
use pltrs::axes::TitleLocation;
use pltrs::fonts::{FontExt, FontManager, FontProperties, FontStyle, FontWeight};
use pltrs::patches::{BoxStyle, Rectangle};
use pltrs::prelude::*;
use pltrs::text::{AnnotateOptions, HAlign, RotationMode, TextBoxProps, VAlign};
use pltrs::transforms::Point;

use super::remove_ticks_and_titles;
use crate::figures::{CaseResult, FigureCase};

/// One font request of `test_font_styles`: text, family, style, weight
/// and stretch.
type FontRequest = (
    &'static str,
    &'static str,
    FontStyle,
    &'static str,
    &'static str,
);

/// `test_font_styles`: annotations in the font files `findfont` finds
/// among the bundled fonts for five requests (`FontProperties(fname=path)`,
/// so the texts keep the default family, size, style and weight).
fn font_styles(fig: &Figure) -> CaseResult {
    const REQUESTS: [FontRequest; 5] = [
        (
            "Normal Font",
            "sans-serif",
            FontStyle::Normal,
            "normal",
            "normal",
        ),
        ("Bold Font", "Foo", FontStyle::Normal, "bold", "500"),
        (
            "Bold Italic Font",
            "sans serif",
            FontStyle::Italic,
            "750",
            "500",
        ),
        ("Light Font", "sans-serif", FontStyle::Normal, "200", "500"),
        (
            "Condensed Font",
            "sans-serif",
            FontStyle::Normal,
            "500",
            "100",
        ),
    ];
    let rc = pltrs::rc::params();
    let ax = fig.subplot();
    for (i, (text, family, style, weight, stretch)) in REQUESTS.into_iter().enumerate() {
        let wanted = FontProperties {
            families: vec![family.to_owned()],
            style,
            weight: weight.parse()?,
            stretch: stretch.parse()?,
            ..FontProperties::from_rc(&rc)
        };
        let file = FontManager::bundled().find_font(&wanted, &rc, FontExt::Ttf, true)?;
        let options = AnnotateOptions {
            xycoords: "axes fraction".parse()?,
            ..AnnotateOptions::default()
        };
        ax.annotate(text, (0.1, 0.1 * (i + 1) as f64), options)?
            .set_font(FontProperties::from_file(file, &rc)?);
    }
    tickless(&ax);
    Ok(())
}

/// `test_multiline`: three-line texts with mathtext of growing height in
/// the middle line.
fn multiline(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    ax.set_title("multiline\ntext alignment");
    for (x, math) in [
        (0.2, "$M$"),
        (0.5, "$M^{M^{M^{M}}}$"),
        (0.8, "$M_{q_{q_{q}}}$"),
    ] {
        ax.text(x, 0.5, format!("TpTpTp\n{math}\nTpTpTp"))
            .set_fontsize(20.0)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Top);
    }
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 0.8)?;
    tickless(&ax);
    Ok(())
}

/// `test_multiline2`: texts with and without mathtext in four vertical
/// alignments, each framed by a rectangle at its window extent.
fn multiline2(fig: &Figure) -> CaseResult {
    const TEXTS: [&str; 6] = [
        "Line",
        "2 Lineg\n 2 Lg",
        "$\\sum_i x $",
        "hi $\\sum_i x $\ntest",
        "test\n $\\sum_i x $",
        "$\\sum_i x $\n $\\sum_i x $",
    ];
    let rows = [
        (0.5, VAlign::Bottom, 0.0, "Bottom align"),
        (1.3, VAlign::Top, 0.0, "Top align"),
        (1.8, VAlign::Baseline, 0.0, "Baseline align"),
        (0.1, VAlign::Bottom, 20.0, "Bot align, rot20"),
    ];
    let ax = fig.subplot();
    ax.set_xlim(0.0, 1.4)?.set_ylim(0.0, 2.0)?;
    for (y, valign, rotation, label) in rows {
        ax.axhline(y, LineProps::default().color("C2").linewidth(0.3));
        for (nn, text) in TEXTS.into_iter().enumerate() {
            let tt = ax.text(0.2 * nn as f64 + 0.1, y, text);
            tt.set_halign(HAlign::Left)
                .set_valign(valign)
                .set_rotation(rotation);
            let extent = tt.window_extent().ok_or("a text without extent")?;
            let to_axes = fig.resolve_transform(&ax.trans_axes().inverted())?;
            let low = to_axes.transform_point(Point::new(extent.x0(), extent.y0()));
            let high = to_axes.transform_point(Point::new(extent.x1(), extent.y1()));
            ax.add_patch(Rectangle::new(
                (low.x, low.y),
                high.x - low.x,
                high.y - low.y,
            ))
            .set_transform(ax.trans_axes())
            .set_clip_on(false);
        }
        ax.text(1.2, y, label).set_color("C2");
    }
    Ok(())
}

/// `test_antialiasing`: texts and mathtext with anti-aliasing on and off,
/// each against the opposite `text.antialiased`.
fn antialiasing(fig: &Figure) -> CaseResult {
    for (x, text, antialiased) in [(0.3, "antialiased", true), (0.7, "not antialiased", false)] {
        pltrs::rc::set(
            "text.antialiased",
            if antialiased { "False" } else { "True" },
        )?;
        for (y, s) in [(0.75, text), (0.25, r"$\sqrt{x}$")] {
            fig.text(x, y, s)
                .set_halign(HAlign::Center)
                .set_valign(VAlign::Center)
                .set_antialiased(antialiased);
        }
    }
    pltrs::rc::set("text.antialiased", "False")?;
    Ok(())
}

/// `test_contains`: a grid of points, yellow where they hit a rotated
/// text and red elsewhere. The test resets the view after each point; the
/// limits are fixed up front instead.
fn contains(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    let txt = ax.text(0.5, 0.4, "hello world");
    txt.set_halign(HAlign::Center)
        .set_fontsize(30.0)
        .set_rotation(30.0);
    let to_display = fig.resolve_transform(&ax.trans_axes())?;
    let grid = linspace(0.25, 0.75, 30);
    for &y in &grid {
        for &x in &grid {
            let hit = txt.contains(to_display.transform_point(Point::new(x, y)));
            let color = if hit.is_some() { "yellow" } else { "red" };
            ax.plot_with([x], [y], LineProps::fmt("o")?.color(color))?;
        }
    }
    Ok(())
}

/// `test_titles`: left and right titles.
fn titles(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    ax.set_title_at(TitleLocation::Left, "left title");
    ax.set_title_at(TitleLocation::Right, "right title");
    tickless(&ax);
    Ok(())
}

/// `test_alignment`: boxed texts and mathtext in each vertical alignment,
/// upright and rotated by 30°.
fn alignment(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    let alignments = [
        ("top", VAlign::Top),
        ("bottom", VAlign::Bottom),
        ("baseline", VAlign::Baseline),
        ("center", VAlign::Center),
    ];
    let mut x = 0.1;
    for rotation in [0.0, 30.0] {
        for (name, valign) in alignments {
            let wheat = PatchProps::default().facecolor("wheat").alpha(0.5);
            ax.text(x, 0.5, format!("{name} Tj"))
                .set_valign(valign)
                .set_rotation(rotation)
                .set_bbox(Some(
                    TextBoxProps::new("round".parse::<BoxStyle>()?).with_patch(wheat),
                ));
            ax.text(x, 1.0, r"$\sum_{i=0}^{j}$")
                .set_valign(valign)
                .set_rotation(rotation);
            x += 0.1;
        }
    }
    ax.plot([0.0, 1.0], [0.5, 0.5])?;
    ax.plot([0.0, 1.0], [1.0, 1.0])?;
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.5)?;
    tickless(&ax);
    Ok(())
}

/// `test_rotation_mode_anchor`: large texts turned about their anchor in
/// quarter turns.
fn rotation_anchor(fig: &Figure) -> CaseResult {
    const N: u32 = 4;
    let ax = fig.subplot();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().linewidth(0.0))?;
    ax.axvline(0.5, LineProps::default().linewidth(0.5).color(".5"));
    ax.axhline(0.5, LineProps::default().linewidth(0.5).color(".5"));
    for r in 0..N {
        ax.text(0.5, 0.5, "pP")
            .set_color(format!("C{r}"))
            .set_fontsize(100.0)
            .set_rotation(f64::from(r) / f64::from(N) * 360.0)
            .set_rotation_mode(RotationMode::Anchor)
            .set_valign(VAlign::CenterBaseline);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_axes_titles`: a large bold centre title between plain left and
/// right ones.
fn axes_titles(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    for (location, text, size, weight) in [
        (TitleLocation::Center, "center", 20.0, 700),
        (TitleLocation::Left, "left", 12.0, 400),
        (TitleLocation::Right, "right", 12.0, 400),
    ] {
        ax.set_title_at(location, text)
            .set_fontsize(size)
            .set_weight(FontWeight::numeric(weight));
    }
    Ok(())
}

/// `test_bbox_clipping`: a text background and a fancy box cut by the
/// axes.
fn bbox_clipping(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.text(0.9, 0.2, "Is bbox clipped?")
        .set_backgroundcolor("r")
        .set_clip_on(true);
    ax.text(0.9, 0.5, "Is fancy bbox clipped?")
        .set_clip_on(true)
        .set_bbox(Some(TextBoxProps::new(
            "round, pad=0.1".parse::<BoxStyle>()?,
        )));
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_text/font_styles", font_styles),
    FigureCase::new("mpl_text/multiline", multiline),
    FigureCase::new("mpl_text/multiline2", multiline2),
    FigureCase::new("mpl_text/antialiasing", antialiasing).sized(5.25, 0.75, 100.0),
    FigureCase::new("mpl_text/contains", contains).sized(8.0, 6.0, 100.0),
    FigureCase::new("mpl_text/titles", titles),
    FigureCase::new("mpl_text/text_alignment", alignment),
    FigureCase::new("mpl_text/rotation_anchor", rotation_anchor),
    FigureCase::new("mpl_text/axes_titles", axes_titles),
    FigureCase::new("mpl_text/text_bboxclip", bbox_clipping),
];
