//! A patch drawn along the path (matplotlib's `PathPatchEffect`).

use pltrs_core::{Affine2D, Path};
use pltrs_render::{GraphicsContext, Renderer, Sketch, SnapMode};

use super::offset_affine;
use crate::artist::rc_sketch;
use crate::colors::ColorError;
use crate::patches::{PatchPaint, PatchProps, PatchShape, PatchStyle, PathShape};

/// The path drawn as a patch of its own, with its own face, edge and
/// hatch, offset by a few points (matplotlib's `PathPatchEffect`): only
/// the clipping comes from the artist drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct PatchEffect {
    /// Offset of the patch in points.
    pub offset: (f64, f64),
    paint: PatchPaint,
    snap: SnapMode,
    sketch: Option<Sketch>,
}

impl PatchEffect {
    /// A patch styled by `props` over matplotlib's patch defaults under
    /// the current rcParams, `offset` points away (`PathPatchEffect(offset,
    /// **kwargs)`, which makes its `PathPatch` at once).
    ///
    /// # Errors
    ///
    /// When a colour of `props` is invalid.
    pub fn new(offset: (f64, f64), props: PatchProps) -> Result<Self, ColorError> {
        let rc = crate::rc::params();
        let mut style = PatchStyle::from_rc(&rc);
        let base = props.apply_style(&mut style);
        Ok(Self {
            offset,
            paint: PatchPaint::new(&style, &rc, base.alpha, PathShape::EDGE_DEFAULT)?,
            snap: if rc.path_snap {
                SnapMode::Auto
            } else {
                SnapMode::Never
            },
            sketch: rc_sketch(&rc),
        })
    }

    /// Draws `path` mapped by `affine` as the patch, clipped like `gc`.
    pub(super) fn draw_path(
        &self,
        renderer: &mut dyn Renderer,
        gc: &GraphicsContext,
        path: &Path,
        affine: &Affine2D,
    ) {
        let placed = offset_affine(renderer, affine, self.offset);
        let patch_gc = GraphicsContext {
            clip_rect: gc.clip_rect,
            clip_path: gc.clip_path.clone(),
            snap: self.snap,
            sketch: self.sketch,
            ..GraphicsContext::default()
        };
        self.paint.paint(patch_gc, |gc, face| {
            renderer.draw_path(gc, path, &placed, face);
        });
    }
}
