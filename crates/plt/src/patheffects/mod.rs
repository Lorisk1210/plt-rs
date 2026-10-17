//! Path effects (matplotlib.patheffects): extra strokes, shadows and ticks
//! drawn around the paths of lines, patches and texts.
//!
//! An artist with effects ([`ArtistHandle::set_path_effects`]) draws through
//! a [`PathEffectRenderer`] wrapping the real renderer: every path the
//! artist draws is handed to each effect in turn, and text is converted to
//! glyph outlines first so that it can be stroked and shadowed too.
//!
//! ```
//! use plt::prelude::*;
//! use plt::patheffects::{GcUpdate, PathEffect, Stroke};
//! let ax = Figure::new().subplot();
//! let halo = Stroke::new(GcUpdate::default().linewidth(4.0).foreground("k")?);
//! ax.text(0.5, 0.5, "outlined").set_path_effects(vec![PathEffect::with_stroke(halo)]);
//! # Ok::<(), plt::colors::ColorError>(())
//! ```
//!
//! [`ArtistHandle::set_path_effects`]: crate::artist::ArtistHandle::set_path_effects

mod renderer;
mod ticked;

use plt_core::{Affine2D, Path, Rgba};
use plt_render::{CapStyle, GraphicsContext, JoinStyle, Renderer};

pub use renderer::PathEffectRenderer;
pub use ticked::TickedStroke;

use crate::artist::DrawContext;
use crate::colors::{Color, ColorError};

/// Changes an effect makes to the graphics state of the paths it draws
/// (the keyword arguments matplotlib forwards to the graphics context).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GcUpdate {
    /// Line width in points.
    pub linewidth: Option<f64>,
    /// Stroke colour.
    pub foreground: Option<Rgba>,
    /// Opacity override.
    pub alpha: Option<f64>,
    /// Line end style.
    pub capstyle: Option<CapStyle>,
    /// Line join style.
    pub joinstyle: Option<JoinStyle>,
}

impl GcUpdate {
    /// Sets the line width in points.
    #[must_use]
    pub fn linewidth(self, linewidth: f64) -> Self {
        Self {
            linewidth: Some(linewidth),
            ..self
        }
    }

    /// Sets the stroke colour.
    pub fn foreground(self, color: impl Into<Color>) -> Result<Self, ColorError> {
        Ok(Self {
            foreground: Some(color.into().resolve()?),
            ..self
        })
    }

    /// Sets the opacity override.
    #[must_use]
    pub fn alpha(self, alpha: f64) -> Self {
        Self {
            alpha: Some(alpha),
            ..self
        }
    }

    /// Sets the line end style.
    #[must_use]
    pub fn capstyle(self, capstyle: CapStyle) -> Self {
        Self {
            capstyle: Some(capstyle),
            ..self
        }
    }

    /// Sets the line join style.
    #[must_use]
    pub fn joinstyle(self, joinstyle: JoinStyle) -> Self {
        Self {
            joinstyle: Some(joinstyle),
            ..self
        }
    }

    /// Writes the set properties into `gc` (`AbstractPathEffect._update_gc`).
    fn apply(&self, gc: &mut GraphicsContext) {
        if let Some(linewidth) = self.linewidth {
            gc.linewidth = linewidth;
        }
        if let Some(foreground) = self.foreground {
            gc.foreground = foreground;
        }
        if let Some(alpha) = self.alpha {
            gc.alpha = Some(alpha);
        }
        if let Some(cap) = self.capstyle {
            gc.cap = cap;
        }
        if let Some(join) = self.joinstyle {
            gc.join = join;
        }
    }
}

/// Redraws the path stroked with an updated graphics state (matplotlib's
/// `Stroke`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stroke {
    /// Offset of the stroke in points.
    pub offset: (f64, f64),
    /// The graphics state changes.
    pub gc: GcUpdate,
}

impl Stroke {
    /// A stroke with the graphics state changes `gc`, not offset.
    pub fn new(gc: GcUpdate) -> Self {
        Self {
            offset: (0.0, 0.0),
            gc,
        }
    }
}

/// A shadow filled below the path (matplotlib's `SimplePatchShadow`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatchShadow {
    /// Offset of the shadow in points.
    pub offset: (f64, f64),
    /// Shadow colour; `None` darkens the face colour by `rho`.
    pub color: Option<Rgba>,
    /// Shadow opacity.
    pub alpha: f64,
    /// Darkening factor of the face colour.
    pub rho: f64,
    /// Further graphics state changes.
    pub gc: GcUpdate,
}

impl Default for PatchShadow {
    fn default() -> Self {
        Self {
            offset: (2.0, -2.0),
            color: None,
            alpha: 0.3,
            rho: 0.3,
            gc: GcUpdate::default(),
        }
    }
}

impl PatchShadow {
    /// The same shadow in an explicit colour.
    pub fn with_color(self, color: impl Into<Color>) -> Result<Self, ColorError> {
        Ok(Self {
            color: Some(color.into().resolve()?),
            ..self
        })
    }
}

/// A shadow stroked below the path (matplotlib's `SimpleLineShadow`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineShadow {
    /// Offset of the shadow in points.
    pub offset: (f64, f64),
    /// Shadow colour; `None` darkens the stroke colour by `rho`.
    pub color: Option<Rgba>,
    /// Shadow opacity.
    pub alpha: f64,
    /// Darkening factor of the stroke colour.
    pub rho: f64,
    /// Further graphics state changes.
    pub gc: GcUpdate,
}

impl Default for LineShadow {
    fn default() -> Self {
        Self {
            offset: (2.0, -2.0),
            color: Some(Rgba::BLACK),
            alpha: 0.3,
            rho: 0.3,
            gc: GcUpdate::default(),
        }
    }
}

/// What an effect draws.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// The path as it is (matplotlib's `Normal`).
    Normal,
    /// An extra stroke.
    Stroke(Stroke),
    /// A filled shadow.
    PatchShadow(PatchShadow),
    /// A stroked shadow.
    LineShadow(LineShadow),
    /// Ticks along the path.
    TickedStroke(TickedStroke),
}

/// One path effect: an [`Effect`], optionally followed by the unchanged
/// path (matplotlib's `with...` variants such as `withStroke`).
#[derive(Clone, Debug, PartialEq)]
pub struct PathEffect {
    /// What is drawn.
    pub effect: Effect,
    /// Whether the original path is drawn afterwards.
    pub then_normal: bool,
}

impl PathEffect {
    const fn of(effect: Effect, then_normal: bool) -> Self {
        Self {
            effect,
            then_normal,
        }
    }

    /// The path as it is (`Normal()`).
    pub const fn normal() -> Self {
        Self::of(Effect::Normal, false)
    }

    /// An extra stroke (`Stroke(...)`).
    pub const fn stroke(stroke: Stroke) -> Self {
        Self::of(Effect::Stroke(stroke), false)
    }

    /// An extra stroke below the path (`withStroke(...)`).
    pub const fn with_stroke(stroke: Stroke) -> Self {
        Self::of(Effect::Stroke(stroke), true)
    }

    /// A filled shadow (`SimplePatchShadow(...)`).
    pub const fn simple_patch_shadow(shadow: PatchShadow) -> Self {
        Self::of(Effect::PatchShadow(shadow), false)
    }

    /// A filled shadow below the path (`withSimplePatchShadow(...)`).
    pub const fn with_simple_patch_shadow(shadow: PatchShadow) -> Self {
        Self::of(Effect::PatchShadow(shadow), true)
    }

    /// A stroked shadow (`SimpleLineShadow(...)`).
    pub const fn simple_line_shadow(shadow: LineShadow) -> Self {
        Self::of(Effect::LineShadow(shadow), false)
    }

    /// Ticks along the path (`TickedStroke(...)`).
    pub const fn ticked_stroke(ticks: TickedStroke) -> Self {
        Self::of(Effect::TickedStroke(ticks), false)
    }

    /// Ticks along the path, which is drawn too (`withTickedStroke(...)`).
    pub const fn with_ticked_stroke(ticks: TickedStroke) -> Self {
        Self::of(Effect::TickedStroke(ticks), true)
    }

    /// Draws `path` (mapped by `affine`) with the effect on `renderer`.
    pub fn draw_path(
        &self,
        renderer: &mut dyn Renderer,
        gc: &GraphicsContext,
        path: &Path,
        affine: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.effect.draw_path(renderer, gc, path, affine, fill);
        if self.then_normal {
            renderer.draw_path(gc, path, affine, fill);
        }
    }
}

/// `affine` followed by a translation of `offset` points.
fn offset_affine(renderer: &dyn Renderer, affine: &Affine2D, offset: (f64, f64)) -> Affine2D {
    affine.translate(
        renderer.points_to_pixels(offset.0),
        renderer.points_to_pixels(offset.1),
    )
}

/// The RGB part of `color` darkened by `rho`, opaque.
fn darkened(color: Rgba, rho: f64) -> Rgba {
    Rgba::new(color.r * rho, color.g * rho, color.b * rho, 1.0)
}

impl Effect {
    fn draw_path(
        &self,
        renderer: &mut dyn Renderer,
        gc: &GraphicsContext,
        path: &Path,
        affine: &Affine2D,
        fill: Option<Rgba>,
    ) {
        match self {
            Self::Normal => renderer.draw_path(gc, path, affine, fill),
            Self::Stroke(stroke) => {
                let mut gc = gc.clone();
                stroke.gc.apply(&mut gc);
                let placed = offset_affine(renderer, affine, stroke.offset);
                renderer.draw_path(&gc, path, &placed, fill);
            }
            Self::PatchShadow(shadow) => {
                let face = shadow
                    .color
                    .unwrap_or_else(|| darkened(fill.unwrap_or(Rgba::WHITE), shadow.rho));
                let mut gc = GraphicsContext {
                    foreground: Rgba::TRANSPARENT,
                    alpha: Some(shadow.alpha),
                    linewidth: 0.0,
                    ..gc.clone()
                };
                shadow.gc.apply(&mut gc);
                let placed = offset_affine(renderer, affine, shadow.offset);
                renderer.draw_path(&gc, path, &placed, Some(face));
            }
            Self::LineShadow(shadow) => {
                let color = shadow
                    .color
                    .unwrap_or_else(|| darkened(gc.foreground, shadow.rho));
                let mut gc = GraphicsContext {
                    foreground: color,
                    alpha: Some(shadow.alpha),
                    ..gc.clone()
                };
                shadow.gc.apply(&mut gc);
                let placed = offset_affine(renderer, affine, shadow.offset);
                renderer.draw_path(&gc, path, &placed, None);
            }
            Self::TickedStroke(ticks) => ticks.draw_path(renderer, gc, path, affine, fill),
        }
    }
}

/// Runs `f` drawing through `effects`, or directly when there are none.
pub(crate) fn with_path_effects<R>(
    ctx: &mut DrawContext<'_>,
    effects: &[PathEffect],
    f: impl FnOnce(&mut DrawContext<'_>) -> R,
) -> R {
    if effects.is_empty() {
        return f(ctx);
    }
    let effects = effects.to_vec();
    ctx.with_wrapped_renderer(
        move |renderer| Box::new(PathEffectRenderer::new(effects, renderer)),
        f,
    )
}

#[cfg(test)]
mod tests;
