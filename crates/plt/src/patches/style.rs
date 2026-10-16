//! Edge, face and hatch properties shared by every patch.

use std::rc::Rc;

use plt_core::{Affine2D, Path, Rgba};
use plt_render::{CapStyle, GraphicsContext, Hatch, JoinStyle};

use crate::artist::{ArtistBase, DrawContext};
use crate::colors::{Color, ColorError};
use crate::lines::{LineStyle, inverse_dashes};
use crate::rc::{ColorOrEdge, RcParams};

/// How a patch's outline, face and hatch are painted (matplotlib's `Patch`
/// properties).
#[derive(Clone, Debug, PartialEq)]
pub struct PatchStyle {
    /// Edge colour; `None` applies matplotlib's default edge rule.
    pub edgecolor: Option<Color>,
    /// Face colour; `None` uses `patch.facecolor`.
    pub facecolor: Option<Color>,
    /// Whether the face is filled.
    pub fill: bool,
    /// Edge width in points; `None` uses `patch.linewidth`.
    pub linewidth: Option<f64>,
    /// Edge line style.
    pub linestyle: LineStyle,
    /// Colour drawn in the gaps of a dashed edge (`"none"` leaves them empty).
    pub edgegapcolor: Color,
    /// Anti-aliasing; `None` uses `patch.antialiased`.
    pub antialiased: Option<bool>,
    /// Hatch pattern such as `"//"`, `"x"`, `"o."`.
    pub hatch: Option<String>,
    /// Hatch colour, or the edge colour.
    pub hatchcolor: ColorOrEdge,
    /// Hatch line width in points.
    pub hatch_linewidth: f64,
    /// Edge end caps.
    pub capstyle: CapStyle,
    /// Edge corner joins.
    pub joinstyle: JoinStyle,
}

impl PatchStyle {
    /// matplotlib's patch defaults under `rc`.
    pub fn from_rc(rc: &RcParams) -> Self {
        Self {
            edgecolor: None,
            facecolor: None,
            fill: true,
            linewidth: None,
            linestyle: LineStyle::Solid,
            edgegapcolor: Color::none(),
            antialiased: None,
            hatch: None,
            hatchcolor: rc.hatch_color.clone(),
            hatch_linewidth: rc.hatch_linewidth,
            capstyle: CapStyle::Butt,
            joinstyle: JoinStyle::Miter,
        }
    }

    /// Colours and widths resolved against `rc` and the artist's alpha.
    pub(crate) fn resolve(
        &self,
        rc: &RcParams,
        alpha: Option<f64>,
        edge_default: bool,
    ) -> Result<ResolvedStyle, ColorError> {
        let edge_spec = match &self.edgecolor {
            Some(color) => color.clone(),
            None if rc.patch_force_edgecolor || !self.fill || edge_default => {
                rc.patch_edgecolor.clone()
            }
            None => Color::none(),
        };
        let edge = edge_spec.resolve_alpha(alpha)?;
        let face_spec = self
            .facecolor
            .clone()
            .unwrap_or_else(|| rc.patch_facecolor.clone());
        let face = face_spec.resolve_alpha(if self.fill { alpha } else { Some(0.0) })?;
        let hatch_color = match &self.hatchcolor {
            ColorOrEdge::Edge if edge.is_transparent() => rc.patch_edgecolor.resolve()?,
            ColorOrEdge::Edge => edge,
            ColorOrEdge::Color(color) => color.resolve_alpha(alpha)?,
        };
        let gap = if self.edgegapcolor.is_none() {
            None
        } else {
            Some(self.edgegapcolor.resolve_alpha(alpha)?)
        };
        Ok(ResolvedStyle {
            edge,
            face,
            hatch_color,
            gap,
            linewidth: self.linewidth.unwrap_or(rc.patch_linewidth),
            antialiased: self.antialiased.unwrap_or(rc.patch_antialiased),
        })
    }
}

/// A [`PatchStyle`] with every colour and default resolved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedStyle {
    pub(crate) edge: Rgba,
    pub(crate) face: Rgba,
    pub(crate) hatch_color: Rgba,
    pub(crate) gap: Option<Rgba>,
    pub(crate) linewidth: f64,
    pub(crate) antialiased: bool,
}

/// Paints `paths` (each already split into a non-affine part and its
/// display affine) with the patch's properties, exactly as matplotlib's
/// `Patch._draw_paths_with_artist_properties` does.
pub(crate) fn draw_patch_paths(
    ctx: &mut DrawContext<'_>,
    base: &ArtistBase,
    style: &PatchStyle,
    rc: &Rc<RcParams>,
    edge_default: bool,
    paths: &[(Path, Affine2D)],
) -> Result<(), ColorError> {
    let resolved = style.resolve(rc, base.alpha, edge_default)?;
    let mut gc: GraphicsContext = ctx.base_gc(base);
    let linewidth = if resolved.edge.is_transparent() || !style.linestyle.is_visible() {
        0.0
    } else {
        resolved.linewidth
    };
    gc.linewidth = linewidth;
    gc.cap = style.capstyle;
    gc.join = style.joinstyle;
    gc.antialiased = resolved.antialiased;
    if let Some(pattern) = &style.hatch {
        gc.hatch = Some(Hatch {
            pattern: pattern.clone(),
            color: resolved.hatch_color,
            linewidth: style.hatch_linewidth,
        });
    }
    let fill = (!resolved.face.is_transparent()).then_some(resolved.face);
    let dashes = style.linestyle.scaled_dashes(resolved.linewidth, rc);

    ctx.renderer().open_group("patch", base.gid.as_deref());
    if let (true, true, Some(gap)) = (linewidth > 0.0, style.linestyle.is_dashed(), resolved.gap) {
        gc.foreground = gap;
        gc.dashes = inverse_dashes(&dashes);
        for (path, affine) in paths {
            ctx.renderer().draw_path(&gc, path, affine, fill);
        }
    }
    gc.foreground = resolved.edge;
    gc.dashes = if linewidth > 0.0 {
        dashes
    } else {
        plt_render::Dashes::solid()
    };
    for (path, affine) in paths {
        ctx.renderer().draw_path(&gc, path, affine, fill);
    }
    ctx.renderer().close_group("patch");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_edges_follow_matplotlib() {
        let rc = RcParams::matplotlib();
        let style = PatchStyle::from_rc(&rc);
        assert!(
            style
                .resolve(&rc, None, false)
                .expect("valid")
                .edge
                .is_transparent()
        );
        assert!(
            !style
                .resolve(&rc, None, true)
                .expect("valid")
                .edge
                .is_transparent(),
            "edge-default classes"
        );
        let unfilled = PatchStyle {
            fill: false,
            ..style.clone()
        };
        let resolved = unfilled.resolve(&rc, None, false).expect("valid");
        assert!(
            !resolved.edge.is_transparent(),
            "unfilled patches get an edge"
        );
        assert_eq!(resolved.face.a, 0.0);
    }

    #[test]
    fn alpha_and_hatch_colours() {
        let rc = RcParams::matplotlib();
        let style = PatchStyle {
            facecolor: Some("red".into()),
            edgecolor: Some("blue".into()),
            ..PatchStyle::from_rc(&rc)
        };
        let resolved = style.resolve(&rc, Some(0.5), false).expect("valid");
        assert_eq!((resolved.face.a, resolved.edge.a), (0.5, 0.5));
        assert_eq!(
            resolved.hatch_color, resolved.edge,
            "hatches follow the edge by default"
        );
    }
}
