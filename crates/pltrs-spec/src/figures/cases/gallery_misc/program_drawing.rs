//! Drawing defined by the program: a path effect of its own
//! (`patheffect_demo_custom`), drawn through pltrs's
//! [`CustomPathEffect`].

use pltrs::colors::Color;
use pltrs::lines::LineProps;
use pltrs::patheffects::{CustomEffect, CustomPathEffect, PathEffect};
use pltrs::prelude::*;
use pltrs::text::HAlign;
use pltrs_core::{Affine2D, Path, Rgba};
use pltrs_render::{GraphicsContext, Renderer};

use crate::figures::{CaseResult, FigureCase};

/// The case's path effect of the program's: the path again, offset by
/// `offset` points, in a pale colour, then the path.
#[derive(Debug)]
struct Ghost {
    offset: (f64, f64),
    color: Rgba,
}

impl Ghost {
    /// The effect offsetting by `offset` points, in `color`.
    fn effect(offset: (f64, f64), color: &str) -> Result<PathEffect, Box<dyn std::error::Error>> {
        let color = Color::from(color).resolve()?;
        Ok(PathEffect::custom(CustomEffect::new(Self {
            offset,
            color,
        })))
    }
}

impl CustomPathEffect for Ghost {
    fn draw_path(
        &self,
        renderer: &mut dyn Renderer,
        gc: &mut GraphicsContext,
        path: &Path,
        affine: &Affine2D,
        fill: Option<Rgba>,
    ) {
        let pale = GraphicsContext {
            foreground: self.color,
            ..gc.clone()
        };
        let offset = affine.translate(
            renderer.points_to_pixels(self.offset.0),
            renderer.points_to_pixels(self.offset.1),
        );
        renderer.draw_path(&pale, path, &offset, fill.map(|_| self.color));
        renderer.draw_path(gc, path, affine, fill);
    }
}

fn patheffect_demo_custom(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    let (ax1, ax2, ax3) = (&axs[(0, 0)], &axs[(0, 1)], &axs[(0, 2)]);
    ax1.plot_with(
        [0.0, 1.0, 2.0, 3.0],
        [0.0, 2.0, 1.0, 3.0],
        LineProps::fmt("o-")?
            .linewidth(3.0)
            .path_effects(vec![Ghost::effect((3.0, -3.0), "0.7")?]),
    )?;
    ax1.set_title("lines")
        .set_path_effects(vec![Ghost::effect((1.0, -1.0), "0.7")?]);
    let bars = ax2.bar(
        [0.0, 1.0, 2.0],
        [3.0, 1.0, 2.0],
        pltrs::axes::BarOptions {
            color: vec!["C1".into()],
            ..pltrs::axes::BarOptions::default()
        },
    )?;
    for bar in &bars.patches {
        bar.set_path_effects(vec![Ghost::effect((4.0, 4.0), "0.85")?]);
    }
    ax3.text(0.5, 0.5, "ghost")
        .set_fontsize(24.0)
        .set_halign(HAlign::Center)
        .set_path_effects(vec![Ghost::effect((2.0, -2.0), "pink")?]);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[FigureCase::new(
    "gallery_misc/patheffect_demo_custom",
    patheffect_demo_custom,
)
.sized(8.0, 3.0, 100.0)];
