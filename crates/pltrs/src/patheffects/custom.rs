//! Path effects of the program's (matplotlib's `AbstractPathEffect`
//! subclasses): any type implementing [`CustomPathEffect`], in a
//! [`PathEffect::custom`](super::PathEffect::custom).

use std::any::Any;
use std::fmt;
use std::rc::Rc;

use pltrs_core::archive::{Archive, ArchiveError, Decoder, Encoder};
use pltrs_core::{Affine2D, Path, Rgba};
use pltrs_render::{GraphicsContext, Renderer};

/// What a path effect of the program's draws for every path of the
/// artists it is set on.
///
/// ```
/// use pltrs::patheffects::{CustomEffect, CustomPathEffect, PathEffect};
/// use pltrs::prelude::*;
/// use pltrs_core::{Affine2D, Path, Rgba};
/// use pltrs_render::{GraphicsContext, Renderer};
///
/// /// Draws every path twice, the copy 3 pixels to the right.
/// #[derive(Debug)]
/// struct Doubled;
///
/// impl CustomPathEffect for Doubled {
///     fn draw_path(
///         &self,
///         renderer: &mut dyn Renderer,
///         gc: &mut GraphicsContext,
///         path: &Path,
///         affine: &Affine2D,
///         fill: Option<Rgba>,
///     ) {
///         renderer.draw_path(gc, path, &affine.translate(3.0, 0.0), fill);
///         renderer.draw_path(gc, path, affine, fill);
///     }
/// }
///
/// let ax = Figure::new().subplot();
/// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
/// line.set_path_effects(vec![PathEffect::custom(CustomEffect::new(Doubled))]);
/// ax.figure().to_png()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait CustomPathEffect: Any + fmt::Debug {
    /// Draws `path`, mapped into display pixels by `affine`, stroked as
    /// `gc` says and filled with `fill`, with the effect onto `renderer`,
    /// the renderer the artist draws onto (matplotlib's
    /// `draw_path(renderer, gc, tpath, affine, rgbFace)`). The effects of
    /// an artist share `gc` in turn, as matplotlib hands them all the same
    /// graphics context: what the effect changes in it, the effects after
    /// it see.
    fn draw_path(
        &self,
        renderer: &mut dyn Renderer,
        gc: &mut GraphicsContext,
        path: &Path,
        affine: &Affine2D,
        fill: Option<Rgba>,
    );
}

pltrs_core::archive_kinds!(dyn CustomPathEffect {});

/// A path effect of the program's, shared by its clones. A figure with
/// one is archived only if the program archives its type (its
/// [`ProgramKinds`](pltrs_core::archive::ProgramKinds)).
#[derive(Clone, Debug)]
pub struct CustomEffect(Rc<dyn CustomPathEffect>);

impl CustomEffect {
    /// The effect `effect`.
    pub fn new(effect: impl CustomPathEffect) -> Self {
        Self(Rc::new(effect))
    }

    /// What draws.
    pub fn get(&self) -> &dyn CustomPathEffect {
        &*self.0
    }
}

impl PartialEq for CustomEffect {
    /// Effects are equal when they are the same effect.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Archive for CustomEffect {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        self.0.encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        Archive::decode(decoder).map(Self)
    }
}
