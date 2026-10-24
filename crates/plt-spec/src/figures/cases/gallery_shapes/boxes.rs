//! Fancy boxes: every box style with its default parameters, and the
//! effect of padding, rounding size and mutation scale and aspect
//! (`fancybox_demo`).

use plt::figure::Share;
use plt::fonts::RelativeSize;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::TightLayout;
use plt::patches::{BoxStyle, FancyBboxPatch, Patch, PatchShape};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{HAlign, Text, VAlign};
use plt_core::Bbox;

use crate::figures::cases::common::styled_box;
use crate::figures::{CaseResult, FigureCase};

/// matplotlib's box styles in `BoxStyle.get_styles()` order, each with
/// its parameters' defaults one per line (`inspect.signature`).
const BOX_STYLES: [(&str, &str); 10] = [
    ("square", "pad=0.3"),
    ("circle", "pad=0.3"),
    ("ellipse", "pad=0.3"),
    ("rarrow", "pad=0.3\nhead_width=1.5\nhead_angle=90"),
    ("larrow", "pad=0.3\nhead_width=1.5\nhead_angle=90"),
    ("darrow", "pad=0.3\nhead_width=1.5\nhead_angle=90"),
    ("round", "pad=0.3\nrounding_size=None"),
    ("round4", "pad=0.3\nrounding_size=None"),
    ("sawtooth", "pad=0.3\ntooth_size=None"),
    ("roundtooth", "pad=0.3\ntooth_size=None"),
];

/// A text at `x` in axes coordinates, vertically centred.
fn axes_text(ax: &Axes, x: f64, text: &str, halign: HAlign) -> Text {
    let text = ax.text(x, 0.5, text);
    text.set_transform(ax.trans_axes());
    text.set_halign(halign).set_valign(VAlign::Center);
    text
}

/// A large blue label right-aligned at 0.2 (the style column).
fn style_label(ax: &Axes, text: &str) -> Text {
    let label = axes_text(ax, 0.2, text, HAlign::Right);
    label
        .set_fontsize(RelativeSize::Large)
        .set_color(Color::from("tab:blue"));
    label
}

fn fancybox_demo_1(fig: &Figure) -> CaseResult {
    let spaced = SubplotOverrides {
        wspace: Some(0.5),
        ..SubplotOverrides::default()
    };
    let grid = GridSpec::with_ratios(6, 2, None, None, spaced)?;
    let axs = fig.subplots_on(&grid, Share::None, Share::None)?;
    for ax in axs.as_slice() {
        ax.set_axis_off();
    }
    for col in 0..2 {
        let ax = &axs[(0, col)];
        style_label(ax, "boxstyle");
        axes_text(ax, 0.4, "default parameters", HAlign::Left);
    }
    // `axs[1:, :].T.flat`: down the first column, then the second.
    let cells = (0..2).flat_map(|col| (1..6).map(move |row| (row, col)));
    for (cell, (name, signature)) in cells.zip(BOX_STYLES) {
        let ax = &axs[cell];
        let boxed = PatchProps::default().facecolor("w").edgecolor("k");
        style_label(ax, name).set_bbox(styled_box(name, boxed)?);
        axes_text(ax, 0.4, signature, HAlign::Left);
    }
    Ok(())
}

/// `add_fancy_patch_around`: a translucent pink box (unless `props` says
/// otherwise) around `bb`.
fn fancy_patch_around(
    ax: &Axes,
    bb: &Bbox,
    boxstyle: &str,
    shape: impl FnOnce(FancyBboxPatch) -> FancyBboxPatch,
    props: PatchProps,
) -> Result<Patch<FancyBboxPatch>, Box<dyn std::error::Error>> {
    let fancy = FancyBboxPatch::new(bb.p0, bb.width(), bb.height(), boxstyle.parse()?);
    let pink = PatchProps::default()
        .facecolor((1.0, 0.8, 1.0, 0.5))
        .edgecolor((1.0, 0.5, 1.0, 0.5));
    Ok(ax.add_patch_with(shape(fancy), pink.overlay(props)))
}

/// The original `bb`, drawn as a black square box with no padding above
/// everything else.
fn bare_box(ax: &Axes, bb: &Bbox) -> CaseResult {
    let black = PatchProps::default()
        .edgecolor("black")
        .facecolor("none")
        .zorder(10.0);
    fancy_patch_around(ax, bb, "square,pad=0", |f| f, black)?;
    Ok(())
}

/// Shows `[0, 1] × [0, 1]` with `title`.
fn unit_view(ax: &Axes, title: &str) {
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    ax.set_title(title);
}

fn fancybox_demo_2(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let bb = Bbox::from_extents(0.3, 0.4, 0.7, 0.6);
    let round = "round,pad=0.1";
    let plain = PatchProps::default;
    let patches = [
        fancy_patch_around(&axs[(0, 0)], &bb, round, |f| f, plain())?,
        fancy_patch_around(&axs[(0, 1)], &bb, round, |f| f, plain())?,
        fancy_patch_around(
            &axs[(1, 0)],
            &bb,
            round,
            |f| f.with_mutation_scale(2.0),
            plain(),
        )?,
        fancy_patch_around(
            &axs[(1, 1)],
            &bb,
            round,
            |f| f.with_mutation_aspect(0.5),
            plain(),
        )?,
    ];
    unit_view(&axs[(0, 0)], "boxstyle=\"round,pad=0.1\"");
    let rounded: BoxStyle = "round,pad=0.1,rounding_size=0.2".parse()?;
    patches[1].update_shape(|f| f.boxstyle = rounded);
    unit_view(&axs[(0, 1)], "boxstyle=\"round,pad=0.1,rounding_size=0.2\"");
    unit_view(
        &axs[(1, 0)],
        "boxstyle=\"round,pad=0.1\"\n mutation_scale=2",
    );
    unit_view(
        &axs[(1, 1)],
        "boxstyle=\"round,pad=0.1\"\nmutation_aspect=0.5",
    );
    for ax in [&axs[(0, 0)], &axs[(0, 1)], &axs[(1, 0)]] {
        ax.set_aspect(Aspect::Ratio(1.0))?;
    }
    for (ax, patch) in axs.as_slice().iter().zip(&patches) {
        let path = patch.shape().path();
        let (x, y): (Vec<f64>, Vec<f64>) = path.vertices().iter().map(|p| (p.x, p.y)).unzip();
        let edge = patch.colors()?.edge;
        ax.plot_with(x, y, LineProps::fmt(".")?.color(edge))?;
        bare_box(ax, &bb)?;
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn fancybox_demo_3(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let bb = Bbox::from_extents(-0.5, -0.5, 0.5, 0.5);
    for ax in [ax1, ax2] {
        bare_box(ax, &bb)?;
        ax.set_xlim(-1.5, 1.5).set_ylim(-1.5, 1.5);
        ax.set_aspect(Aspect::Ratio(2.0))?;
    }
    let round = "round,pad=0.5";
    fancy_patch_around(ax1, &bb, round, |f| f, PatchProps::default())?;
    ax1.set_title("aspect=2\nmutation_aspect=1");
    fancy_patch_around(
        ax2,
        &bb,
        round,
        |f| f.with_mutation_aspect(0.5),
        PatchProps::default(),
    )?;
    ax2.set_title("aspect=2\nmutation_aspect=0.5");
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_shapes/fancybox_demo_1", fancybox_demo_1).sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_shapes/fancybox_demo_2", fancybox_demo_2).sized(8.0, 8.0, 100.0),
    FigureCase::new("gallery_shapes/fancybox_demo_3", fancybox_demo_3).sized(6.5, 5.0, 100.0),
];
