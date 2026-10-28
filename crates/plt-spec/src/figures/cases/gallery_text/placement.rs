//! Placing text: alignment against an anchor point and a box, rotation
//! relative to the screen or to the data and the two rotation modes,
//! multi-line text, automatic wrapping and styled boxes
//! (`text_alignment`, `text_rotation_relative_to_line`,
//! `demo_text_rotation_mode`, `multiline`, `autowrap` and
//! `fancytextbox_demo`).

use plt::axes::{GridOptions, TickLevel, TickParams};
use plt::figure::{Share, SubfigureOptions};
use plt::fonts::{FontSize, FontStyle, RelativeSize};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::patches::Rectangle;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{HAlign, RotationMode, Text, TextBoxProps, VAlign};
use plt::transforms::Point;

use crate::figures::cases::common::{blank, styled_box, tickless};
use crate::figures::{CaseResult, FigureCase};

/// The long sentence `autowrap` wraps.
const LONG_TEXT: &str = "This is a really long string that I'd rather have wrapped so that it \
                         doesn't go outside of the figure, but if it's long enough it will go \
                         off the top or bottom!";

/// Aligns `text` against its anchor.
fn align(text: &Text, halign: HAlign, valign: VAlign) -> &Text {
    text.set_halign(halign).set_valign(valign)
}

/// The figure of `text_alignment`'s docstring: anchor points of every
/// alignment around a big word.
fn text_alignment_1(fig: &Figure) -> CaseResult {
    let y = [0.22, 0.34, 0.5, 0.56, 0.78];
    let x = [0.17, 0.5, 0.855];
    let ax = fig.subplot();
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    tickless(&ax);
    for spine in ax.spines().iter() {
        spine.set_visible(false);
    }
    align(&ax.text(0.5, 0.5, "plot"), HAlign::Center, VAlign::Center)
        .set_fontsize(128.0)
        .set_zorder(1.0);
    ax.hlines(y, [x[0]], [x[2]])?.set_edgecolor("grey");
    ax.vlines(x, [y[0]], [y[4]])?.set_edgecolor("grey");
    let grid: Vec<(f64, f64)> = y
        .iter()
        .flat_map(|&yi| x.iter().map(move |&xi| (xi, yi)))
        .collect();
    let (gx, gy): (Vec<f64>, Vec<f64>) = grid.into_iter().unzip();
    ax.plot_fmt(gx, gy, "o")?;
    let (pad_x, pad_y) = (0.02, 0.04);
    for (yi, label) in y
        .iter()
        .zip(["bottom", "baseline", "center", "center_baseline", "top"])
    {
        align(
            &ax.text(x[0] - pad_x, *yi, label),
            HAlign::Right,
            VAlign::Center,
        );
    }
    for (xi, label) in x.iter().zip(["left", "center", "right"]) {
        align(
            &ax.text(*xi, y[0] - pad_y, label),
            HAlign::Center,
            VAlign::Top,
        );
    }
    ax.set_xlabel("horizontalalignment");
    ax.xaxis().set_label_fontsize(FontSize::Points(14.0));
    ax.set_ylabel("verticalalignment");
    ax.yaxis()
        .set_label_fontsize(FontSize::Points(14.0))
        .set_label_pad(35.0);
    ax.set_title("Relative position of text anchor point depending on alignment");
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn text_alignment_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let (left, width, bottom, height) = (0.25, 0.5, 0.25, 0.5);
    let (right, top) = (left + width, bottom + height);
    ax.add_patch(Rectangle::new((left, bottom), width, height))
        .set_fill(false)
        .set_transform(ax.trans_axes())
        .set_clip_on(false);
    let middle = f64::midpoint(bottom, top);
    let texts = [
        (left, bottom, "left top", HAlign::Left, VAlign::Top, 0.0),
        (
            left,
            bottom,
            "left bottom",
            HAlign::Left,
            VAlign::Bottom,
            0.0,
        ),
        (
            right,
            top,
            "right bottom",
            HAlign::Right,
            VAlign::Bottom,
            0.0,
        ),
        (right, top, "right top", HAlign::Right, VAlign::Top, 0.0),
        (
            right,
            bottom,
            "center top",
            HAlign::Center,
            VAlign::Top,
            0.0,
        ),
        (
            left,
            middle,
            "right center",
            HAlign::Right,
            VAlign::Center,
            90.0,
        ),
        (
            left,
            middle,
            "left center",
            HAlign::Left,
            VAlign::Center,
            90.0,
        ),
        (
            f64::midpoint(left, right),
            middle,
            "middle",
            HAlign::Center,
            VAlign::Center,
            0.0,
        ),
        (
            right,
            middle,
            "centered",
            HAlign::Center,
            VAlign::Center,
            90.0,
        ),
        (
            left,
            top,
            "rotated\nwith newlines",
            HAlign::Center,
            VAlign::Center,
            45.0,
        ),
    ];
    for (x, y, label, halign, valign, rotation) in texts {
        align(&ax.text(x, y, label), halign, valign)
            .set_rotation(rotation)
            .set_transform(ax.trans_axes());
    }
    ax.set_axis_off();
    Ok(())
}

fn text_rotation_relative_to_line(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let ramp: Vec<f64> = (0..8).map(f64::from).collect();
    ax.plot(&ramp, &ramp)?;
    ax.set_xlim(-10.0, 10.0)?;
    ax.text(-8.0, 0.0, "text 45° in screen coordinates")
        .set_fontsize(18.0)
        .set_rotation(45.0)
        .set_rotation_mode(RotationMode::Anchor);
    ax.text(0.0, 0.0, "text 45° in data coordinates")
        .set_fontsize(18.0)
        .set_rotation(45.0)
        .set_rotation_mode(RotationMode::Anchor)
        .set_transform_rotates_text(true);
    Ok(())
}

/// One side of `demo_text_rotation_mode`: "Tpg" at 40° in every
/// alignment, with its box (anchor mode) or its extent (default mode)
/// shaded.
fn rotation_mode(fig: &Figure, mode: RotationMode, name: &str) -> CaseResult {
    let halignments = [
        (HAlign::Left, "left"),
        (HAlign::Center, "center"),
        (HAlign::Right, "right"),
    ];
    let valignments = [
        (VAlign::Top, "top"),
        (VAlign::Center, "center"),
        (VAlign::Baseline, "baseline"),
        (VAlign::Bottom, "bottom"),
    ];
    let grid = GridSpec::with_ratios(
        valignments.len(),
        halignments.len(),
        None,
        None,
        SubplotOverrides {
            hspace: Some(0.0),
            wspace: Some(0.0),
            ..SubplotOverrides::default()
        },
    )?;
    let axs = fig.subplots_on(&grid, Share::All, Share::All)?;
    for ax in axs.as_slice() {
        ax.set_aspect(Aspect::Ratio(1.0))?;
    }
    let last = valignments.len() - 1;
    for (col, (_, label)) in halignments.iter().enumerate() {
        axs[(last, col)].set_xlabel(*label);
    }
    for (row, (_, label)) in valignments.iter().enumerate() {
        axs[(row, 0)].set_ylabel(*label);
    }
    axs[(0, 1)]
        .set_title(format!("rotation_mode='{name}'"))
        .set_fontsize(RelativeSize::Large);
    let shaded = || PatchProps::default().facecolor("C1").alpha(0.3);
    let mut texts = Vec::new();
    for (row, (valign, _)) in valignments.iter().enumerate() {
        for (col, (halign, _)) in halignments.iter().enumerate() {
            let ax = &axs[(row, col)];
            tickless(ax);
            ax.axvline(0.5, LineProps::default().color("skyblue").zorder(0.0));
            ax.axhline(0.5, LineProps::default().color("skyblue").zorder(0.0));
            ax.plot_with(
                [0.5],
                [0.5],
                LineProps::default()
                    .color("C0")
                    .marker(Marker::from(MarkerSymbol::Circle))
                    .zorder(1.0),
            )?;
            let text = ax.text(0.5, 0.5, "Tpg");
            align(&text, *halign, *valign)
                .set_fontsize(RelativeSize::XLarge)
                .set_rotation(40.0)
                .set_rotation_mode(mode);
            if mode == RotationMode::Anchor {
                text.set_bbox(styled_box("square,pad=0.", shaded().edgecolor("none"))?);
            }
            texts.push((ax.clone(), text));
        }
    }
    if mode == RotationMode::Default {
        fig.layout();
        for (ax, text) in texts {
            let extent = text.window_extent().ok_or("a text without extent")?;
            let to_data = fig.resolve_transform(&ax.trans_data().inverted())?;
            let low = to_data.transform_point(Point::new(extent.x0(), extent.y0()));
            let high = to_data.transform_point(Point::new(extent.x1(), extent.y1()));
            ax.add_patch(Rectangle::new(
                (low.x, low.y),
                high.x - low.x,
                high.y - low.y,
            ))
            .set_facecolor("C1")
            .set_alpha(Some(0.3))
            .set_zorder(2.0);
        }
    }
    Ok(())
}

fn demo_text_rotation_mode(fig: &Figure) -> CaseResult {
    let subfigs = fig.subfigures(1, 2, SubfigureOptions::default())?;
    rotation_mode(&subfigs[(0, 0)], RotationMode::Default, "default")?;
    rotation_mode(&subfigs[(0, 1)], RotationMode::Anchor, "anchor")
}

fn multiline_1(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(plt::layout::LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let x: Vec<f64> = (0..6).map(f64::from).collect();
    ax.bar(
        x,
        [0.3, 0.5, 0.8, 1.5, 2.5, 8.0],
        BarOptions {
            props: PatchProps::default().alpha(0.3),
            ..BarOptions::default()
        },
    )?;
    ax.set_ylabel("labels are centered,\nso is their multi-line text by default");
    ax.set_xlabel("xlabel with\nmultialignment=\"left\"");
    ax.xaxis().set_label_multialign(Some(HAlign::Left));
    ax.text(
        4.5,
        6.0,
        "these lines\nare right aligned\nbecause of horizontalalignment",
    )
    .set_halign(HAlign::Right);
    ax.text(
        4.5,
        4.0,
        "but\nwe can override this\nwith an explicit multialingment=\"left\"",
    )
    .set_halign(HAlign::Right)
    .set_multialign(Some(HAlign::Left));
    Ok(())
}

fn multiline_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let outline = || {
        Some(TextBoxProps {
            patch: PatchProps::default()
                .facecolor("none")
                .edgecolor("tab:orange"),
            ..TextBoxProps::default()
        })
    };
    for (x, text) in [
        (0.1, "Mat\nTTp\n123"),
        (0.3, "Mag\nTTT\n123"),
        (0.5, "Mag\nTTT$^{A^A}$\n123"),
    ] {
        ax.text(x, 0.4, text)
            .set_fontsize(18.0)
            .set_valign(VAlign::Baseline)
            .set_bbox(outline());
    }
    ax.axhline(0.4, LineProps::default());
    ax.set_title("test line spacing for multiline text");
    ax.set_xlim(0.0, 0.7)?;
    ax.yaxis().minorticks_on();
    ax.yaxis().set_tick_params(TickParams {
        which: TickLevel::Minor,
        grid_color: Some(Color::from("0.9")),
        ..TickParams::default()
    });
    ax.grid_with(
        true,
        GridOptions {
            which: TickLevel::Both,
            ..GridOptions::default()
        },
    );
    Ok(())
}

fn autowrap(fig: &Figure) -> CaseResult {
    let ax = blank(fig)?;
    let wrapped = |x: f64, y: f64| {
        let text = ax.text(x, y, LONG_TEXT);
        text.set_wrap(true);
        text
    };
    wrapped(4.0, 1.0)
        .set_halign(HAlign::Left)
        .set_rotation(15.0);
    wrapped(6.0, 5.0)
        .set_halign(HAlign::Left)
        .set_rotation(15.0);
    wrapped(5.0, 5.0)
        .set_halign(HAlign::Right)
        .set_rotation(-15.0);
    align(&wrapped(5.0, 10.0), HAlign::Center, VAlign::Top)
        .set_fontsize(18.0)
        .set_style(FontStyle::Oblique);
    wrapped(3.0, 4.0)
        .set_family(["serif"])
        .set_style(FontStyle::Italic)
        .set_halign(HAlign::Right);
    wrapped(-1.0, 0.0)
        .set_halign(HAlign::Left)
        .set_rotation(-15.0);
    Ok(())
}

fn fancytextbox_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let pink = || {
        PatchProps::default()
            .edgecolor((1.0, 0.5, 0.5))
            .facecolor((1.0, 0.8, 0.8))
    };
    align(&ax.text(0.6, 0.7, "eggs"), HAlign::Center, VAlign::Center)
        .set_fontsize(50.0)
        .set_rotation(30.0)
        .set_bbox(styled_box("round", pink())?);
    align(&ax.text(0.55, 0.6, "spam"), HAlign::Right, VAlign::Top)
        .set_fontsize(50.0)
        .set_rotation(-25.0)
        .set_bbox(styled_box("square", pink())?);
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_text/text_alignment_1", text_alignment_1).sized(6.0, 4.0, 100.0),
    FigureCase::new("gallery_text/text_alignment_2", text_alignment_2),
    FigureCase::new(
        "gallery_text/text_rotation_relative_to_line",
        text_rotation_relative_to_line,
    ),
    FigureCase::new(
        "gallery_text/demo_text_rotation_mode",
        demo_text_rotation_mode,
    )
    .sized(8.0, 5.0, 100.0),
    FigureCase::new("gallery_text/multiline_1", multiline_1),
    FigureCase::new("gallery_text/multiline_2", multiline_2),
    FigureCase::new("gallery_text/autowrap", autowrap),
    FigureCase::new("gallery_text/fancytextbox_demo", fancytextbox_demo),
];
