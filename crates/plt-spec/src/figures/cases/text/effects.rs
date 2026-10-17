//! Path effect cases of `figures/cases_text.py`.

use std::f64::consts::TAU;

use plt::axes::ImshowOptions;
use plt::colors::colormaps;
use plt::image::{ColorizerOptions, Extent, ImageData, ScalarField};
use plt::numeric::Grid;
use plt::patches::Rectangle;
use plt::patheffects::{GcUpdate, LineShadow, PatchShadow, PathEffect, Stroke, TickedStroke};
use plt::prelude::*;

use super::blank;
use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("text/patheffects_withstroke", |fig| {
        let ax = fig.subplot();
        let ramp = Grid::from_fn(10, 10, |r, c| (r * 10 + c) as i64);
        let options = ImshowOptions {
            colors: ColorizerOptions::default().cmap(colormaps::get("gray")?),
            extent: Some(Extent::new(0.0, 10.0, 0.0, 10.0)),
            ..ImshowOptions::default()
        };
        ax.imshow(ImageData::from(ScalarField::integers(&ramp)), options)?;
        let black = Stroke::new(GcUpdate::default().linewidth(4.0).foreground("k")?);
        ax.text(1.0, 6.0, "stroked text")
            .set_fontsize(24.0)
            .set_color("w")
            .set_path_effects(vec![PathEffect::with_stroke(black)]);
        let red = Stroke::new(GcUpdate::default().linewidth(6.0).foreground("r")?);
        ax.plot_with(
            [0.0, 10.0],
            [2.0, 4.0],
            LineProps::default().color("w").linewidth(2.0),
        )?
        .set_path_effects(vec![PathEffect::with_stroke(red)]);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("text/patheffects_stroke_normal", |fig| {
        let ax = blank(fig);
        let navy = Stroke::new(GcUpdate::default().linewidth(5.0).foreground("navy")?);
        ax.text(1.0, 6.0, "Stroke + Normal")
            .set_fontsize(24.0)
            .set_color("gold")
            .set_path_effects(vec![PathEffect::stroke(navy), PathEffect::normal()]);
        let grey = Stroke::new(GcUpdate::default().linewidth(8.0).foreground("0.7")?);
        ax.plot_with([1.0, 9.0], [2.0, 3.0], LineProps::default().linewidth(3.0))?
            .set_path_effects(vec![PathEffect::stroke(grey), PathEffect::normal()]);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("text/patheffects_shadows", |fig| {
        let ax = blank(fig);
        ax.text(0.5, 7.0, "simple shadow")
            .set_fontsize(22.0)
            .set_path_effects(vec![PathEffect::with_simple_patch_shadow(
                PatchShadow::default(),
            )]);
        ax.plot_with([1.0, 9.0], [5.0, 5.5], LineProps::default().linewidth(4.0))?
            .set_path_effects(vec![
                PathEffect::simple_line_shadow(LineShadow::default()),
                PathEffect::normal(),
            ]);
        let black = PatchShadow {
            offset: (4.0, -4.0),
            alpha: 0.4,
            ..PatchShadow::default()
        }
        .with_color("k")?;
        ax.add_patch(Rectangle::new((1.0, 1.0), 3.0, 2.0))
            .set_facecolor("tab:blue")
            .set_path_effects(vec![
                PathEffect::simple_patch_shadow(black),
                PathEffect::normal(),
            ]);
        let red = PatchShadow {
            offset: (3.0, -3.0),
            alpha: 0.6,
            ..PatchShadow::default()
        }
        .with_color("r")?;
        ax.text(6.0, 1.5, "offset")
            .set_fontsize(18.0)
            .set_path_effects(vec![PathEffect::with_simple_patch_shadow(red)]);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("text/patheffects_ticked_stroke", |fig| {
        let ax = blank(fig);
        let ticks = TickedStroke {
            angle: 135.0,
            spacing: 10.0,
            length: 1.0,
            ..TickedStroke::default()
        };
        ax.plot([1.0, 9.0], [2.0, 8.0])?
            .set_path_effects(vec![PathEffect::with_ticked_stroke(ticks)]);
        let theta = linspace(0.0, TAU, 60);
        let x: Vec<f64> = theta.iter().map(|t| 5.0 + 2.0 * t.cos()).collect();
        let y: Vec<f64> = theta.iter().map(|t| 5.0 + 2.0 * t.sin()).collect();
        ax.plot_with(x, y, LineProps::default().color("tab:red"))?
            .set_path_effects(vec![
                PathEffect::with_ticked_stroke(TickedStroke::default()),
            ]);
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
];
