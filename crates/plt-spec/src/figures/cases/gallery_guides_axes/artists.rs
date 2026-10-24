//! `color_cycle`, `patheffects_guide` and `paths`: property cycles, path
//! effects on text and lines, and paths of lines, curves and compound
//! rectangles.

use std::f64::consts::PI;

use plt::patches::PathShape;
use plt::patheffects::{LineShadow, PatchShadow, PathEffect};
use plt::prelude::*;
use plt::rc::{CycleValue, Cycler};
use plt::stats::{Bins, HistogramOptions, histogram};
use plt::text::{HAlign, Text, VAlign};
use plt::transforms::Point;
use plt_core::Path;
use plt_core::path::PathCode;

use super::SEED;
use crate::figures::cases::common::{hatch_shadow_text, map, stroke};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

fn color_cycle(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 2.0 * PI, 50);
    let offsets = (0..4).map(|i| f64::from(i) * (2.0 * PI / 4.0));
    let axs = fig.subplots(2, 1)?;
    let (ax0, ax1) = (&axs[(0, 0)], &axs[(1, 0)]);
    let colors = ["c", "m", "y", "k"].map(|c| CycleValue::Color(c.into()));
    let widths = [1.0, 2.0, 3.0, 4.0].map(CycleValue::Number);
    ax1.set_prop_cycle(Some(
        Cycler::new("color", colors.to_vec()).zip(Cycler::new("linewidth", widths.to_vec()))?,
    ));
    for (ax, title) in [
        (ax0, "Set default color cycle to rgby"),
        (ax1, "Set axes color cycle to cmyk"),
    ] {
        for phi in offsets.clone() {
            ax.plot_y(map(&x, |x| (x + phi).sin()));
        }
        ax.set_title(title);
    }
    fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
        hspace: Some(0.3),
        ..plt::gridspec::SubplotOverrides::default()
    })?;
    Ok(())
}

/// `text` centred on its position at `points`.
fn centred_text(text: &Text, points: f64) -> &Text {
    text.set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_fontsize(points)
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn patheffects_guide_1(fig: &Figure) -> CaseResult {
    centred_text(
        &fig.text(
            0.5,
            0.5,
            "Hello path effects world!\nThis is the normal path effect.\nPretty dull, huh?",
        ),
        20.0,
    )
    .set_path_effects(vec![PathEffect::normal()]);
    Ok(())
}

fn patheffects_guide_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.text(0.5, 0.5, "Hello path effects world!")
        .set_path_effects(vec![PathEffect::with_simple_patch_shadow(
            PatchShadow::default(),
        )]);
    ax.plot_with(
        [0.0, 1.0, 2.0, 3.0],
        [0.0, 3.0, 2.0, 5.0],
        LineProps::default()
            .linewidth(5.0)
            .color("blue")
            .path_effects(vec![
                PathEffect::simple_line_shadow(LineShadow::default()),
                PathEffect::normal(),
            ]),
    )?;
    Ok(())
}

fn patheffects_guide_3(fig: &Figure) -> CaseResult {
    centred_text(
        &fig.text(
            0.5,
            0.5,
            "This text stands out because of\nits black border.",
        ),
        30.0,
    )
    .set_color("white")
    .set_path_effects(vec![
        PathEffect::stroke(stroke(3.0, "black")?),
        PathEffect::normal(),
    ]);
    Ok(())
}

fn patheffects_guide_4(fig: &Figure) -> CaseResult {
    hatch_shadow_text(&fig.text(0.02, 0.5, "Hatch shadow"))
}

fn paths_1(fig: &Figure) -> CaseResult {
    let verts = [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)].map(Point::from);
    let codes = [
        PathCode::MoveTo,
        PathCode::LineTo,
        PathCode::LineTo,
        PathCode::LineTo,
        PathCode::ClosePoly,
    ];
    let ax = fig.subplot();
    ax.add_patch_with(
        PathShape(Path::new(verts.to_vec(), codes.to_vec())?),
        PatchProps::default().facecolor("orange").linewidth(2.0),
    );
    ax.set_xlim(-2.0, 2.0).set_ylim(-2.0, 2.0);
    Ok(())
}

fn paths_2(fig: &Figure) -> CaseResult {
    let verts = [(0.0, 0.0), (0.2, 1.0), (1.0, 0.8), (0.8, 0.0)];
    let codes = [
        PathCode::MoveTo,
        PathCode::Curve4,
        PathCode::Curve4,
        PathCode::Curve4,
    ];
    let ax = fig.subplot();
    ax.add_patch_with(
        PathShape(Path::new(verts.map(Point::from).to_vec(), codes.to_vec())?),
        PatchProps::default().facecolor("none").linewidth(2.0),
    );
    let (xs, ys): (Vec<f64>, Vec<f64>) = verts.into_iter().unzip();
    ax.plot_with(
        xs,
        ys,
        LineProps::fmt("x--")?
            .linewidth(2.0)
            .color("black")
            .markersize(10.0),
    )?;
    for (x, y, label) in [
        (-0.05, -0.05, "P0"),
        (0.15, 1.05, "P1"),
        (1.05, 0.85, "P2"),
        (0.85, -0.05, "P3"),
    ] {
        ax.text(x, y, label);
    }
    ax.set_xlim(-0.1, 1.1).set_ylim(-0.1, 1.1);
    Ok(())
}

fn paths_3(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let data = SpecRng::new(SEED).normal(1000, 0.0, 1.0);
    let hist = histogram(
        &data,
        HistogramOptions {
            bins: Bins::Count(100),
            ..HistogramOptions::default()
        },
    )?;
    let (left, right) = (&hist.edges[..100], &hist.edges[1..]);
    let mut verts = Vec::with_capacity(5 * 100);
    let mut codes = Vec::with_capacity(5 * 100);
    for ((&l, &r), &top) in left.iter().zip(right).zip(&hist.counts) {
        verts.extend([(l, 0.0), (l, top), (r, top), (r, 0.0), (0.0, 0.0)].map(Point::from));
        codes.extend([
            PathCode::MoveTo,
            PathCode::LineTo,
            PathCode::LineTo,
            PathCode::LineTo,
            PathCode::ClosePoly,
        ]);
    }
    ax.add_patch_with(
        PathShape(Path::new(verts, codes)?),
        PatchProps::default()
            .facecolor("green")
            .edgecolor("yellow")
            .alpha(0.5),
    );
    let top = hist
        .counts
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    ax.set_xlim(left[0], right[99]).set_ylim(0.0, top);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_axes/color_cycle", color_cycle).with_rc_setup(|rc| {
        rc.set("lines.linewidth", "4")?;
        rc.set(
            "axes.prop_cycle",
            "cycler('color', ['r', 'g', 'b', 'y']) + cycler('linestyle', ['-', '--', ':', '-.'])",
        )?;
        Ok(())
    }),
    FigureCase::new(
        "gallery_guides_axes/patheffects_guide_1",
        patheffects_guide_1,
    )
    .sized(5.0, 1.5, 100.0),
    FigureCase::new(
        "gallery_guides_axes/patheffects_guide_2",
        patheffects_guide_2,
    ),
    FigureCase::new(
        "gallery_guides_axes/patheffects_guide_3",
        patheffects_guide_3,
    )
    .sized(7.0, 1.0, 100.0),
    FigureCase::new(
        "gallery_guides_axes/patheffects_guide_4",
        patheffects_guide_4,
    )
    .sized(8.5, 1.0, 100.0),
    FigureCase::new("gallery_guides_axes/paths_1", paths_1),
    FigureCase::new("gallery_guides_axes/paths_2", paths_2),
    FigureCase::new("gallery_guides_axes/paths_3", paths_3),
];
