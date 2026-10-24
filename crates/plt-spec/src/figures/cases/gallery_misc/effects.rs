//! Path effects and vector-output features: strokes and shadows, ticked
//! strokes, shadows made for SVG filters and rasterized meshes
//! (`patheffect_demo`, `tickedstroke_demo`, `svg_filter_line`,
//! `svg_filter_pie` and `rasterization_demo`).

use std::f64::consts::PI;

use plt::axes::{AxisMode, ImshowOptions, Mesh, PcolorOptions, PieLabelFormat, PieOptions};
use plt::contour::{ClabelOptions, ContourOptions, LabelFormat, QuadField};
use plt::image::{ImageData, ScalarField};
use plt::layout::LayoutEngine;
use plt::legend::{LegendHandle, LegendOptions};
use plt::numeric::Grid;
use plt::patches::{PathShape, Shadow};
use plt::patheffects::{PatchShadow, PathEffect, TickedStroke};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::VAlign;
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps, HAlign};
use plt::ticker::pyfmt::PrintfFormat;
use plt::transforms::Point;
use plt_core::Path;

use crate::figures::cases::common::{contour_effects, stroke, ticked_stroke, with_stroke};
use crate::figures::{CaseResult, FigureCase};

fn patheffect_demo(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    let (ax1, ax2, ax3) = (&axs[(0, 0)], &axs[(0, 1)], &axs[(0, 2)]);
    let image = Grid::from_vec(2, 2, vec![1_i64, 2, 2, 3])?;
    ax1.imshow(
        ImageData::from(ScalarField::integers(&image)),
        ImshowOptions::default(),
    )?;
    let arrow = FancyArrowProps {
        connectionstyle: "angle3".parse()?,
        patch: PatchProps::default().linewidth(2.0).path_effects(vec![
            PathEffect::stroke(stroke(5.0, "w")?),
            PathEffect::normal(),
        ]),
        ..FancyArrowProps::new("->".parse()?)
    };
    ax1.annotate(
        "test",
        (1.0, 1.0),
        AnnotateOptions {
            xytext: Some(Point::new(0.0, 0.0)),
            arrowprops: Some(ArrowProps::Fancy(arrow)),
            ..AnnotateOptions::default()
        },
    )?
    .set_fontsize(20.0)
    .set_halign(HAlign::Center)
    .set_path_effects(with_stroke(3.0, "w")?);
    ax1.grid_with(
        true,
        plt::axes::GridOptions {
            linestyle: Some(LineStyle::Solid),
            path_effects: Some(with_stroke(3.0, "w")?),
            ..plt::axes::GridOptions::default()
        },
    );
    let arr = Grid::from_fn(5, 5, |i, j| (i * 5 + j) as i64);
    ax2.imshow(
        ImageData::from(ScalarField::integers(&arr)),
        ImshowOptions::default(),
    )?;
    let contours = ax2.contour(
        &QuadField::new(arr.map(|&v| v as f64)),
        ContourOptions {
            colors: Some(vec!["k".into()]),
            ..ContourOptions::default()
        },
    )?;
    contour_effects(&contours, &with_stroke(3.0, "w")?);
    let labels = contours.clabel(ClabelOptions {
        fmt: LabelFormat::Printf(PrintfFormat::parse("%2.0f")?),
        use_clabeltext: true,
        ..ClabelOptions::default()
    })?;
    for label in labels {
        label.set_path_effects(with_stroke(3.0, "w")?);
    }
    let line = ax3.plot([0.0, 1.0], [0.0, 1.0])?;
    let legend = ax3.legend(LegendOptions {
        handles: Some(vec![LegendHandle::from(&line)]),
        labels: Some(vec!["Line 1".into()]),
        fancybox: Some(true),
        ..LegendOptions::default().loc("upper left")
    })?;
    legend
        .frame()
        .set_path_effects(vec![PathEffect::with_simple_patch_shadow(
            PatchShadow::default(),
        )]);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn tickedstroke_demo_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.add_patch(PathShape(Path::unit_circle()))
        .update_props(PatchProps::default().facecolor("none").linewidth(2.0))
        .set_path_effects(ticked_stroke(-90.0, 10.0, 1.0));
    ax.axis_mode(AxisMode::Equal);
    ax.set_xlim(-2.0, 2.0).set_ylim(-2.0, 2.0);
    Ok(())
}

fn tickedstroke_demo_4(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let default_length = TickedStroke::default().length;
    for (label, angle) in [("Line", 135.0), ("Opposite side", -135.0)] {
        ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label(label))?
            .set_path_effects(ticked_stroke(angle, 7.0, default_length));
    }
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn svg_filter_line(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
    let x = [0.1, 0.5, 0.9];
    for (y, fmt, edge, label) in [
        ([0.1, 0.9, 0.5], "bo-", "b", "Line 1"),
        ([0.5, 0.2, 0.7], "rs-", "r", "Line 2"),
    ] {
        let props = LineProps::fmt(fmt)?
            .markeredgecolor(Color::from(edge))
            .linewidth(5.0)
            .markersize(10.0)
            .label(label);
        let line = ax.plot_with(x, y, props.clone())?;
        let shadow = ax.plot_with(x, y, props)?;
        shadow
            .set_color("0.2")
            .set_zorder(line.zorder() - 0.5)
            .set_transform(ax.trans_data().offset_points(4.0, -6.0))
            .set_gid(format!("{label}_shadow"));
    }
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    Ok(())
}

fn svg_filter_pie(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
    let pie = ax.pie(
        &[15.0, 30.0, 45.0, 10.0],
        PieOptions {
            explode: vec![0.0, 0.05, 0.0, 0.0],
            labels: ["Frogs", "Hogs", "Dogs", "Logs"].map(String::from).to_vec(),
            autopct: Some(PieLabelFormat::Printf("%1.1f%%".into())),
            ..PieOptions::default()
        },
    )?;
    for wedge in &pie.wedges {
        wedge.set_gid(wedge.label());
        wedge.set_edgecolor("none");
    }
    for wedge in &pie.wedges {
        let shadow = ax.add_patch(Shadow::of(wedge, -0.01, -0.01)?);
        shadow
            .set_gid(format!("{}_shadow", wedge.label()))
            .set_zorder(wedge.zorder() - 0.1);
    }
    Ok(())
}

/// The mesh of `rasterization_demo`: the 11 × 11 index grid rotated by
/// 45° and the values `0, ..., 99` on its cells.
fn rotated_mesh() -> Mesh {
    let theta = 0.25 * PI;
    let (sin, cos) = theta.sin_cos();
    let xx = Grid::from_fn(11, 11, |y, x| x as f64 * cos - y as f64 * sin);
    let yy = Grid::from_fn(11, 11, |y, x| x as f64 * sin + y as f64 * cos);
    let d = Grid::from_fn(10, 10, |r, c| (r * 10 + c) as f64);
    Mesh::from_coords(xx, yy, d)
}

/// The big faint "Text" in the middle of `ax`.
fn faint_text(ax: &Axes) {
    let text = ax.text(0.5, 0.5, "Text");
    text.set_alpha(Some(0.2));
    text.set_valign(VAlign::Center)
        .set_halign(HAlign::Center)
        .set_fontsize(50.0)
        .set_transform(ax.trans_axes());
}

fn rasterization_demo(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(2, 2)?;
    let (ax1, ax2, ax3, ax4) = (&axs[(0, 0)], &axs[(0, 1)], &axs[(1, 0)], &axs[(1, 1)]);
    ax1.set_aspect(Aspect::Ratio(1.0))?;
    ax1.pcolormesh(rotated_mesh(), PcolorOptions::default())?;
    ax1.set_title("No Rasterization");
    ax2.set_aspect(Aspect::Ratio(1.0))?;
    ax2.set_title("Rasterization");
    ax2.pcolormesh(rotated_mesh(), PcolorOptions::default())?
        .set_rasterized(true);
    ax3.set_aspect(Aspect::Ratio(1.0))?;
    ax3.pcolormesh(rotated_mesh(), PcolorOptions::default())?;
    faint_text(ax3);
    ax3.set_title("No Rasterization");
    ax4.set_aspect(Aspect::Ratio(1.0))?;
    ax4.pcolormesh(rotated_mesh(), PcolorOptions::default())?
        .set_zorder(-10.0);
    faint_text(ax4);
    ax4.set_rasterization_zorder(Some(0.0));
    ax4.set_title("Rasterization z$<-10$");
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_misc/patheffect_demo", patheffect_demo).sized(8.0, 3.0, 100.0),
    FigureCase::new("gallery_misc/tickedstroke_demo_1", tickedstroke_demo_1).sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_misc/tickedstroke_demo_4", tickedstroke_demo_4).sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_misc/svg_filter_line", svg_filter_line),
    FigureCase::new("gallery_misc/svg_filter_pie", svg_filter_pie).sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_misc/rasterization_demo", rasterization_demo),
];
