//! Edge, face and hatch properties shared by every patch.

use std::rc::Rc;

use pltrs_core::{Affine2D, Path, Rgba};
use pltrs_render::{CapStyle, Dashes, GraphicsContext, Hatch, JoinStyle};

use super::DisplayPath;
use crate::artist::{ArtistBase, DrawContext};
use crate::colors::{Color, ColorError};
use crate::lines::{LineStyle, inverse_dashes};
use crate::patheffects::with_path_effects;
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

pltrs_core::archive_struct!(PatchStyle {
    edgecolor,
    facecolor,
    fill,
    linewidth,
    linestyle,
    edgegapcolor,
    antialiased,
    hatch,
    hatchcolor,
    hatch_linewidth,
    capstyle,
    joinstyle
});

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

impl ResolvedStyle {
    /// How far outside its outline a point still hits the patch when it
    /// has no picker: the edge width when the edge is visible
    /// (matplotlib's `Patch._process_radius`).
    pub(crate) fn hit_radius(&self) -> f64 {
        if self.edge.a == 0.0 {
            0.0
        } else {
            self.linewidth
        }
    }
}

/// Paints `paths` (each already split into a non-affine part and its
/// display affine) with the patch's properties, filling every face; see
/// [`paint_paths`].
pub(crate) fn draw_patch_paths(
    ctx: &mut DrawContext<'_>,
    base: &ArtistBase,
    style: &PatchStyle,
    rc: &Rc<RcParams>,
    edge_default: bool,
    paths: &[(Path, Affine2D)],
) -> Result<(), ColorError> {
    let paths: Vec<DisplayPath> = paths
        .iter()
        .map(|(path, affine)| DisplayPath::filled(path.clone(), *affine))
        .collect();
    paint_paths(ctx, base, style, rc, edge_default, &paths)
}

/// Paints `paths` with the patch's properties through the artist's path
/// effects, exactly as matplotlib's
/// `Patch._draw_paths_with_artist_properties` does: the gaps of a dashed
/// edge first (when they have a colour), then the faces of the filled
/// paths with their hatch, and the edges.
pub(crate) fn paint_paths(
    ctx: &mut DrawContext<'_>,
    base: &ArtistBase,
    style: &PatchStyle,
    rc: &RcParams,
    edge_default: bool,
    paths: &[DisplayPath],
) -> Result<(), ColorError> {
    let paint = PatchPaint::new(style, rc, base.alpha, edge_default)?;
    let gc = ctx.base_gc(base);
    ctx.renderer().open_group("patch", base.gid.as_deref());
    let painted = with_path_effects(ctx, &base.path_effects, |ctx| {
        paint.paint(gc, |gc, face| {
            for path in paths {
                let fill = face.filter(|_| path.filled);
                ctx.renderer().draw_path(gc, &path.path, &path.affine, fill);
            }
        });
    });
    ctx.renderer().close_group("patch");
    painted
}

/// What matplotlib's `Patch._draw_paths_with_artist_properties` puts in
/// the graphics context, resolved from a [`PatchStyle`]: colours, edge
/// width and dashes, caps, joins and hatch.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PatchPaint {
    /// The face, `None` when transparent.
    face: Option<Rgba>,
    edge: Rgba,
    /// The colour of the gaps of a dashed edge (`None` = empty gaps).
    gap: Option<Rgba>,
    /// The edge width, 0 when the edge does not show.
    linewidth: f64,
    dashes: Dashes,
    dashed: bool,
    cap: CapStyle,
    join: JoinStyle,
    antialiased: bool,
    hatch: Option<Hatch>,
}

pltrs_core::archive_struct!(PatchPaint {
    face,
    edge,
    gap,
    linewidth,
    dashes,
    dashed,
    cap,
    join,
    antialiased,
    hatch
});

impl PatchPaint {
    /// `style` resolved against `rc` and the artist's `alpha`.
    pub(crate) fn new(
        style: &PatchStyle,
        rc: &RcParams,
        alpha: Option<f64>,
        edge_default: bool,
    ) -> Result<Self, ColorError> {
        let resolved = style.resolve(rc, alpha, edge_default)?;
        let edge_shows = !resolved.edge.is_transparent() && style.linestyle.is_visible();
        // matplotlib only hands the renderer non-empty hatches.
        let hatch = style
            .hatch
            .as_ref()
            .filter(|pattern| !pattern.is_empty())
            .map(|pattern| Hatch {
                pattern: pattern.clone(),
                color: resolved.hatch_color,
                linewidth: style.hatch_linewidth,
            });
        Ok(Self {
            face: (!resolved.face.is_transparent()).then_some(resolved.face),
            edge: resolved.edge,
            gap: resolved.gap,
            linewidth: if edge_shows { resolved.linewidth } else { 0.0 },
            dashes: style.linestyle.scaled_dashes(resolved.linewidth, rc),
            dashed: style.linestyle.is_dashed(),
            cap: style.capstyle,
            join: style.joinstyle,
            antialiased: resolved.antialiased,
            hatch,
        })
    }

    /// Completes `gc` with the patch's properties and calls `draw` with it
    /// and the face colour: first for the gaps of a dashed edge when they
    /// have a colour, then for the face, its hatch and the edge.
    pub(crate) fn paint(
        &self,
        mut gc: GraphicsContext,
        mut draw: impl FnMut(&GraphicsContext, Option<Rgba>),
    ) {
        gc.linewidth = self.linewidth;
        gc.cap = self.cap;
        gc.join = self.join;
        gc.antialiased = self.antialiased;
        gc.hatch.clone_from(&self.hatch);
        let edged = self.linewidth > 0.0;
        if let (true, true, Some(gap)) = (edged, self.dashed, self.gap) {
            gc.foreground = gap;
            gc.dashes = inverse_dashes(&self.dashes);
            draw(&gc, self.face);
        }
        gc.foreground = self.edge;
        gc.dashes = if edged {
            self.dashes.clone()
        } else {
            Dashes::solid()
        };
        draw(&gc, self.face);
    }
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
