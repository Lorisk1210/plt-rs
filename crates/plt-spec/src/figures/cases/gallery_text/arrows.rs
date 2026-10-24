//! Arrows: bracket arrows turned by their angles, a graph of arrows whose
//! length, width or opacity encodes a flux, and every arrow style with
//! its default parameters (`angles_on_bracket_arrows`, `arrow_demo` and
//! `fancyarrow_demo`).

use std::collections::BTreeMap;

use plt::artist::ArtistHandle;
use plt::figure::{Mosaic, MosaicCell, MosaicOptions, Share};
use plt::fonts::{FontWeight, RelativeSize};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::LayoutEngine;
use plt::patches::{ArrowShape, ArrowStyle, ConnectionStyle, FancyArrow, FancyArrowPatch};
use plt::prelude::*;
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps, HAlign, VAlign};

use crate::figures::cases::common::{arrow, styled_box, tickless};
use crate::figures::{CaseResult, FigureCase};

/// The end of a vertical line of `length` from `origin` turned by
/// `degrees` (the example's `get_point_of_rotated_vertical`).
fn rotated_vertical_top((x, y): (f64, f64), length: f64, degrees: i32) -> (f64, f64) {
    let (sin, cos) = f64::from(-degrees).to_radians().sin_cos();
    (x + length * sin, y + length * cos)
}

fn angles_on_bracket_arrows(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 6.0).set_ylim(-1.0, 5.0);
    ax.set_title("Orientation of the bracket arrows relative to angleA and angleB");
    for (i, angle) in [-40, 0, 60].into_iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "three rows")]
        let y = 2.0 * i as f64;
        let centers = [(1.0, y), (5.0, y)];
        let vlines = [(1.0, y + 0.5), (5.0, y + 0.5)];
        let angles = [angle, -angle];
        let bracketstyle = format!("]-[, angleA={angle}, angleB={}", -angle);
        let bracket = FancyArrowPatch::between(centers[0], centers[1])
            .with_arrowstyle(bracketstyle.parse::<ArrowStyle>()?)
            .with_mutation_scale(42.0);
        ax.add_patch(bracket);
        ax.text(3.0, y + 0.05, &bracketstyle)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Bottom)
            .set_fontsize(14.0);
        ax.vlines([1.0, 5.0], [y, y], [y + 0.5, y + 0.5])?
            .set_linestyles(vec![LineStyle::Dashed])
            .set_edgecolor("C0");
        let directions = if angle > 0 { [1.0, -1.0] } else { [-1.0, 1.0] };
        for ((vline, direction), (center, angle)) in vlines
            .into_iter()
            .zip(directions)
            .zip(centers.into_iter().zip(angles))
        {
            let top = rotated_vertical_top(center, 0.5, angle);
            let curved = FancyArrowPatch::between(vline, top)
                .with_connectionstyle(
                    format!("arc3,rad={}", direction * 0.5).parse::<ConnectionStyle>()?,
                )
                .with_arrowstyle(
                    "Simple, tail_width=0.5, head_width=4, head_length=8".parse::<ArrowStyle>()?,
                );
            ax.add_patch(curved).set_color("C0");
            ax.text(vline.0 - direction * 0.15, y + 0.7, format!("{angle}°"))
                .set_halign(HAlign::Center)
                .set_valign(VAlign::Center);
        }
    }
    Ok(())
}

/// The bases of `arrow_demo`'s graph, with their corners and colours.
const BASES: [(char, (i32, i32), &str); 4] = [
    ('A', (0, 1), "r"),
    ('T', (1, 1), "k"),
    ('G', (0, 0), "g"),
    ('C', (1, 0), "b"),
];

/// `arrow_demo`'s probabilities of the bases and of the transitions
/// between them.
const FLUXES: [(&str, f64); 16] = [
    ("A", 0.4),
    ("T", 0.3),
    ("G", 0.6),
    ("C", 0.2),
    ("AT", 0.4),
    ("AC", 0.3),
    ("AG", 0.2),
    ("TA", 0.2),
    ("TC", 0.3),
    ("TG", 0.4),
    ("CT", 0.2),
    ("CG", 0.3),
    ("CA", 0.2),
    ("GA", 0.1),
    ("GT", 0.4),
    ("GC", 0.1),
];

/// What the arrows of a graph encode.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Display {
    Length,
    Width,
    Alpha,
}

impl Display {
    fn name(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Width => "width",
            Self::Alpha => "alpha",
        }
    }
}

/// The example's `make_arrow_graph` with `size=4`, `shape='right'`,
/// `linewidth=0.001` and `normalize_data=True`, which rescales `data` in
/// place (the three graphs share it, so the later ones see it rescaled).
fn arrow_graph(ax: &Axes, data: &mut BTreeMap<String, f64>, display: Display) {
    const SIZE: f64 = 4.0;
    const MAX_ARROW_WIDTH: f64 = 0.03;
    const ARROW_SEP: f64 = 0.02;
    const SF: f64 = 0.6;
    ax.set_xlim(-0.25, 1.25).set_ylim(-0.25, 1.25);
    tickless(ax);
    ax.set_title(format!("flux encoded as arrow {}", display.name()));
    let (max_text_size, min_text_size, label_text_size) = (SIZE * 12.0, SIZE, SIZE * 4.0);
    for (base, (x, y), color) in BASES {
        let (x, y) = (f64::from(x), f64::from(y));
        let fontsize =
            (max_text_size * data[&base.to_string()].powf(0.5)).clamp(min_text_size, max_text_size);
        ax.text(x, y, format!("${base}_3$"))
            .set_color(color)
            .set_fontsize(fontsize)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center)
            .set_weight(FontWeight::BOLD);
    }
    let max_arrow_length = 1.0 - 2.0 * 0.25;
    let max_head_width = 2.5 * MAX_ARROW_WIDTH;
    let max_head_length = 2.0 * MAX_ARROW_WIDTH;
    let max_val = data
        .iter()
        .filter(|(k, _)| k.len() == 2)
        .map(|(_, v)| *v)
        .fold(0.0, f64::max);
    for v in data.values_mut() {
        *v = *v / max_val * SF;
    }
    let mut alpha: f64 = 0.5;
    for (from, cp0, fc) in BASES {
        for (to, cp1, _) in BASES.into_iter().filter(|(to, ..)| *to != from) {
            let flux = data[&format!("{from}{to}")];
            let length = match display {
                Display::Length => {
                    max_head_length + flux / SF * (max_arrow_length - max_head_length)
                }
                _ => max_arrow_length,
            };
            if display == Display::Alpha {
                alpha = alpha.min(flux / SF);
            }
            let scale = if display == Display::Width {
                flux / SF
            } else {
                1.0
            };
            let (dx, dy) = (f64::from(cp1.0 - cp0.0), f64::from(cp1.1 - cp0.1));
            let (cos, sin) = (dx / dx.hypot(dy), dy / dx.hypot(dy));
            let middle = |a: i32, b: i32| f64::from(a + b) / 2.0;
            let x_pos = middle(cp0.0, cp1.0) - cos * length / 2.0 + -sin * ARROW_SEP;
            let y_pos = middle(cp0.1, cp1.1) - sin * length / 2.0 + cos * ARROW_SEP;
            ax.arrow(FancyArrow {
                width: MAX_ARROW_WIDTH * scale,
                head_width: Some(max_head_width * scale),
                head_length: Some(max_head_length * scale),
                shape: ArrowShape::Right,
                length_includes_head: true,
                ..FancyArrow::new(x_pos, y_pos, cos * length, sin * length)
            })
            .update_props(
                PatchProps::default()
                    .facecolor(fc)
                    .edgecolor(fc)
                    .alpha(alpha)
                    .linewidth(0.001),
            );
            let along = if cp0.0 != cp1.0 && cp0.1 != cp1.1 {
                3.0 * MAX_ARROW_WIDTH
            } else {
                length / 2.0
            };
            let across = 3.0 * MAX_ARROW_WIDTH;
            let x = (cos * along + -sin * across) + x_pos;
            let y = (sin * along + cos * across) + y_pos;
            ax.text(x, y, format!(r"$r_{{_{{\mathrm{{{from}{to}}}}}}}$"))
                .set_fontsize(label_text_size)
                .set_halign(HAlign::Center)
                .set_valign(VAlign::Center)
                .set_color(fc);
        }
    }
}

fn arrow_demo(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let mut data: BTreeMap<String, f64> = FLUXES
        .iter()
        .map(|&(key, value)| (key.to_owned(), value))
        .collect();
    let displays = [Display::Length, Display::Width, Display::Alpha];
    let mosaic = Mosaic::new(vec![
        displays
            .iter()
            .map(|display| MosaicCell::Label(display.name().into()))
            .collect(),
    ])?;
    let axs = fig.subplot_mosaic(&mosaic, MosaicOptions::default())?;
    for display in displays {
        let ax = axs.get(display.name()).ok_or("a graph without axes")?;
        arrow_graph(ax, &mut data, display);
    }
    Ok(())
}

/// matplotlib's arrow styles with their default parameters as
/// `fancyarrow_demo` prints them (`inspect.signature` of the style
/// class, broken at every comma, or every second one when there are more
/// than three).
const ARROW_STYLES: [(&str, &str); 16] = [
    ("-", ""),
    ("<-", CURVE_PARAMETERS),
    ("->", CURVE_PARAMETERS),
    ("<->", CURVE_PARAMETERS),
    ("<|-", CURVE_PARAMETERS),
    ("-|>", CURVE_PARAMETERS),
    ("<|-|>", CURVE_PARAMETERS),
    ("]-", "widthA=1.0\nlengthA=0.2\nangleA=0"),
    ("-[", "widthB=1.0\nlengthB=0.2\nangleB=0"),
    (
        "]-[",
        "widthA=1.0, lengthA=0.2\nangleA=0, widthB=1.0\nlengthB=0.2, angleB=0",
    ),
    ("|-|", "widthA=1.0\nangleA=0\nwidthB=1.0\nangleB=0"),
    ("]->", "widthA=1.0\nlengthA=0.2\nangleA=None"),
    ("<-[", "widthB=1.0\nlengthB=0.2\nangleB=None"),
    ("simple", "head_length=0.5\nhead_width=0.5\ntail_width=0.2"),
    ("fancy", "head_length=0.4\nhead_width=0.4\ntail_width=0.4"),
    ("wedge", "tail_width=0.3\nshrink_factor=0.5"),
];

/// The default parameters of the head styles.
const CURVE_PARAMETERS: &str = "head_length=0.4, head_width=0.2\nwidthA=1.0, widthB=1.0\n\
                                lengthA=0.2, lengthB=0.2\nangleA=0, angleB=0\n\
                                scaleA=None, scaleB=None";

fn fancyarrow_demo(fig: &Figure) -> CaseResult {
    const NCOL: usize = 2;
    let nrow = ARROW_STYLES.len().div_ceil(NCOL);
    let grid = GridSpec::with_ratios(
        1 + nrow,
        NCOL,
        None,
        None,
        SubplotOverrides {
            wspace: Some(0.0),
            hspace: Some(0.05),
            left: Some(0.0),
            right: Some(1.0),
            bottom: Some(0.0),
            top: Some(1.0),
        },
    )?;
    let axs = fig.subplots_on(&grid, Share::None, Share::None)?;
    for ax in axs.as_slice() {
        ax.set_xlim(-0.1, 4.0);
        ax.set_axis_off();
    }
    for col in 0..NCOL {
        let ax = &axs[(0, col)];
        ax.text(0.0, 0.5, "arrowstyle")
            .set_fontsize(RelativeSize::Large)
            .set_color("tab:blue");
        ax.text(1.4, 0.5, "default parameters")
            .set_fontsize(RelativeSize::Large);
    }
    let cells = (0..NCOL).flat_map(|col| (1..=nrow).map(move |row| (row, col)));
    for ((name, parameters), cell) in ARROW_STYLES.into_iter().zip(cells) {
        let ax = &axs[cell];
        let point = ax.plot_with([1.25], [0.0], LineProps::fmt("o")?.color("darkgrey"))?;
        let props = FancyArrowProps {
            shrink_a: 5.0,
            shrink_b: 5.0,
            patch_b: Some(point.id()),
            patch: PatchProps::default().color("black"),
            ..arrow(name, "arc3,rad=0")?
        };
        ax.annotate(
            name,
            (1.25, 0.0),
            AnnotateOptions {
                xytext: Some((0.0, 0.0).into()),
                arrowprops: Some(ArrowProps::Fancy(props)),
                ..AnnotateOptions::default()
            },
        )?
        .set_fontsize(RelativeSize::Large)
        .set_color("tab:blue")
        .set_valign(VAlign::Center)
        .set_family(["monospace"])
        .set_bbox(styled_box(
            "square",
            PatchProps::default().facecolor("w").edgecolor("darkgrey"),
        )?);
        ax.text(1.4, 0.0, parameters)
            .set_valign(VAlign::Center)
            .set_color("0.3");
    }
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_text/angles_on_bracket_arrows",
        angles_on_bracket_arrows,
    ),
    FigureCase::new("gallery_text/arrow_demo", arrow_demo).sized(12.0, 4.0, 100.0),
    FigureCase::new("gallery_text/fancyarrow_demo", fancyarrow_demo).sized(8.0, 9.0, 100.0),
];
