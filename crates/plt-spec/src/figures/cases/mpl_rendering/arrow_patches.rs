//! The cases from `test_arrow_patches.py`: annotation arrows, every box,
//! arrow and connection style, arrow heads at two resolutions and dashed
//! arrows.

use std::fmt::Write as _;

use plt::axes::TickParams;
use plt::patches::{ArrowStyle, BoxStyle, ConnectionStyle, FancyArrowPatch};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps, HAlign, Text, TextBoxProps, VAlign};
use plt::transforms::Point;

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// `test_fancyarrow`: `fancy` and `simple` arrows (and `Fancy()`, the same
/// as `fancy`) of five lengths, the last one zero.
fn fancyarrow(fig: &Figure) -> CaseResult {
    let lengths = [0.4, 0.3, 0.2, 0.1, 0.0];
    let styles = ["fancy", "simple", "fancy"];
    let axs = fig.subplots(styles.len(), lengths.len())?;
    for (i_r, r) in lengths.into_iter().enumerate() {
        for (i_t, style) in styles.into_iter().enumerate() {
            let ax = &axs[(i_t, i_r)];
            ax.set_aspect(Aspect::Equal)?;
            let arrow = FancyArrowProps {
                patch: PatchProps::default().facecolor("b").edgecolor("k"),
                ..FancyArrowProps::new(style.parse::<ArrowStyle>()?)
            };
            let options = AnnotateOptions {
                xytext: Some(Point::new(0.5, 0.5)),
                arrowprops: Some(ArrowProps::Fancy(arrow)),
                ..AnnotateOptions::default()
            };
            ax.annotate("", (0.5, 0.5 + r), options)?.set_fontsize(30.0);
            ax.tick_params(&TickParams {
                labelleft: Some(false),
                labelbottom: Some(false),
                ..TickParams::default()
            });
        }
    }
    Ok(())
}

/// The font size of the box style tests' labels: 0.3 in.
const BOX_FONT_SIZE: f64 = 0.3 * 72.0;

/// A centred label at figure fraction `(x, y)` in a white box of
/// `boxstyle`.
fn boxed_label(fig: &Figure, (x, y): (f64, f64), label: &str, boxstyle: BoxStyle) -> Text {
    let patch = PatchProps::default().facecolor("w").edgecolor("k");
    let text = fig.text(x, y, label);
    text.set_halign(HAlign::Center)
        .set_fontsize(BOX_FONT_SIZE)
        .set_bbox(Some(TextBoxProps::new(boxstyle).with_patch(patch)));
    text
}

/// matplotlib's box styles, sorted by name.
const BOX_STYLES: [&str; 10] = [
    "circle",
    "darrow",
    "ellipse",
    "larrow",
    "rarrow",
    "round",
    "round4",
    "roundtooth",
    "sawtooth",
    "square",
];

/// The spacing of the box style tests' labels, in inches at 1.5 scale.
const BOX_SPACING: f64 = 1.2;

/// `test_boxarrow`: every box style around its name.
fn boxarrow(fig: &Figure) -> CaseResult {
    let n = BOX_STYLES.len() as f64;
    let figheight = n * BOX_SPACING + 0.5;
    for (i, style) in BOX_STYLES.into_iter().enumerate() {
        let y = ((n - i as f64) * BOX_SPACING - 0.5) / figheight;
        boxed_label(fig, (0.5, y), style, style.parse()?);
    }
    Ok(())
}

/// `test_boxarrow_adjustment`: the arrow box styles with six head widths
/// and angles each.
fn boxarrow_adjustment(fig: &Figure) -> CaseResult {
    let styles = ["larrow", "rarrow", "darrow"];
    let heads = [
        (1.5, 90.0),
        (1.5, 170.0),
        (0.75, 30.0),
        (0.5, -10.0),
        (2.0, -90.0),
        (2.0, -15.0),
    ];
    let (spacing_horizontal, spacing_vertical) = (3.75, 1.6);
    let (m, n) = (styles.len() as f64, heads.len() as f64);
    let figwidth = m * spacing_horizontal;
    let figheight = n * spacing_vertical + 0.5;
    for (i, style) in styles.into_iter().enumerate() {
        for (j, (head_width, head_angle)) in heads.into_iter().enumerate() {
            let x = ((m - i as f64) * spacing_horizontal - 1.5) / figwidth;
            let y = ((n - j as f64) * spacing_vertical - 0.5) / figheight;
            let boxstyle = format!("{style},head_width={head_width},head_angle={head_angle}");
            boxed_label(fig, (x, y), style, boxstyle.parse()?).set_valign(VAlign::Center);
        }
    }
    Ok(())
}

/// `__prepare_fancyarrow_dpi_cor_test`: an arrow whose head must not
/// depend on the resolution.
fn fancyarrow_dpi_cor(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    let arrow = FancyArrowPatch::between((0.3, 0.4), (0.8, 0.6))
        .with_arrowstyle("->".parse()?)
        .with_mutation_scale(100.0);
    ax.add_patch(arrow).set_linewidth(3.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_fancyarrow_dash`: dashed and dotted arrows along `angle3` curves.
fn fancyarrow_dash(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let arrows = [
        ("angle3,angleA=0,angleB=90", LineStyle::Dashed),
        ("angle3", LineStyle::Dotted),
    ];
    for (connection, linestyle) in arrows {
        let arrow = FancyArrowPatch::between((0.0, 0.0), (0.5, 0.5))
            .with_arrowstyle("-|>".parse()?)
            .with_connectionstyle(connection.parse::<ConnectionStyle>()?)
            .with_mutation_scale(10.0);
        ax.add_patch(arrow).update_props(
            PatchProps::default()
                .linewidth(2.0)
                .linestyle(linestyle)
                .color("k"),
        );
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// matplotlib's arrow styles, sorted by name.
const ARROW_STYLES: [&str; 16] = [
    "-", "->", "-[", "-|>", "<-", "<->", "<-[", "<|-", "<|-|>", "]-", "]->", "]-[", "fancy",
    "simple", "wedge", "|-|",
];

/// `test_arrow_styles`: every arrow style, then the bracket styles with
/// tilted brackets.
fn arrow_styles(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 1.0)
        .set_ylim(-1.0, ARROW_STYLES.len() as f64);
    fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
        left: Some(0.0),
        right: Some(1.0),
        bottom: Some(0.0),
        top: Some(1.0),
        ..plt::gridspec::SubplotOverrides::default()
    })?;
    let arrow = |a: (f64, f64), b: (f64, f64), style: &str| -> CaseResult {
        let patch = FancyArrowPatch::between(a, b)
            .with_arrowstyle(style.parse()?)
            .with_mutation_scale(25.0);
        ax.add_patch(patch);
        Ok(())
    };
    for (i, style) in ARROW_STYLES.into_iter().enumerate() {
        let shift = (i % 2) as f64 * 0.05;
        arrow((0.1 + shift, i as f64), (0.45 + shift, i as f64), style)?;
    }
    for (i, style) in ["]-[", "]-", "-[", "|-|"].into_iter().enumerate() {
        for (j, angle) in [-30, 60].into_iter().enumerate() {
            let mut spec = style.to_owned();
            if !style.starts_with('-') {
                write!(spec, ",angleA={angle}")?;
            }
            if !style.ends_with('-') {
                write!(spec, ",angleB={angle}")?;
            }
            let y = (2 * i + j) as f64;
            arrow((0.55, y), (0.9, y), &spec)?;
        }
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_connection_styles`: an arrow along every connection style.
fn connection_styles(fig: &Figure) -> CaseResult {
    let styles = ["angle", "angle3", "arc", "arc3", "bar"];
    let ax = fig.subplot();
    ax.set_xlim(0.0, 1.0).set_ylim(-1.0, styles.len() as f64);
    for (i, style) in styles.into_iter().enumerate() {
        let y = i as f64;
        let patch = FancyArrowPatch::between((0.1, y), (0.8, y + 0.5))
            .with_arrowstyle("->".parse()?)
            .with_connectionstyle(style.parse()?)
            .with_mutation_scale(25.0);
        ax.add_patch(patch);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_rendering/fancyarrow", fancyarrow).sized(8.0, 4.5, 100.0),
    FigureCase::new("mpl_rendering/boxarrow", boxarrow).sized(
        4.0 / 1.5,
        (10.0 * BOX_SPACING + 0.5) / 1.5,
        100.0,
    ),
    FigureCase::new("mpl_rendering/boxarrow_adjustment", boxarrow_adjustment).sized(
        3.0 * 3.75 / 1.5,
        (6.0 * 1.6 + 0.5) / 1.5,
        100.0,
    ),
    classic(
        "mpl_rendering/fancyarrow_dpi_cor_100dpi",
        fancyarrow_dpi_cor,
    )
    .sized(4.0, 3.0, 100.0),
    classic(
        "mpl_rendering/fancyarrow_dpi_cor_200dpi",
        fancyarrow_dpi_cor,
    )
    .sized(4.0, 3.0, 200.0),
    FigureCase::new("mpl_rendering/fancyarrow_dash", fancyarrow_dash),
    FigureCase::new("mpl_rendering/arrow_styles", arrow_styles).sized(8.0, 8.0, 100.0),
    FigureCase::new("mpl_rendering/connection_styles", connection_styles).sized(6.0, 10.0, 100.0),
];
