//! The arrow of an annotation (matplotlib's `arrowprops`).

use plt_core::{Bbox, Path, Point};

use crate::artist::{ArtistBase, DrawContext, LayoutContext};
use crate::patches::{
    ArrowSpec, ArrowStyle, ClipRegion, ConnectionStyle, DisplayPath, PatchError, PatchProps,
    PatchStyle, paint_paths, round_ends,
};
use crate::rc::RcParams;

/// An annotation arrow with an explicit arrow style (matplotlib's
/// `arrowprops` containing `arrowstyle`).
#[derive(Clone, Debug, PartialEq)]
pub struct FancyArrowProps {
    /// Heads and body.
    pub arrowstyle: ArrowStyle,
    /// The path from the text to the point.
    pub connectionstyle: ConnectionStyle,
    /// Where the arrow leaves the text box, as fractions of its extent.
    pub relpos: (f64, f64),
    /// Shortening at the text end, in points.
    pub shrink_a: f64,
    /// Shortening at the annotated point, in points.
    pub shrink_b: f64,
    /// Scale of the arrow style in points; `None` is the font size.
    pub mutation_scale: Option<f64>,
    /// Vertical squeeze while the arrow is built.
    pub mutation_aspect: f64,
    /// Colours, widths... of the arrow patch.
    pub patch: PatchProps,
}

impl FancyArrowProps {
    /// An arrow of `arrowstyle` with matplotlib's defaults.
    pub fn new(arrowstyle: ArrowStyle) -> Self {
        Self {
            arrowstyle,
            connectionstyle: ConnectionStyle::default(),
            relpos: (0.5, 0.5),
            shrink_a: 2.0,
            shrink_b: 2.0,
            mutation_scale: None,
            mutation_aspect: 1.0,
            patch: PatchProps::default(),
        }
    }
}

/// matplotlib's emulation of its old `YAArrow` (`arrowprops` without
/// `arrowstyle`): a `simple` arrow sized in points, leaving the text box
/// from its corner or edge middle nearest to the point.
#[derive(Clone, Debug, PartialEq)]
pub struct LegacyArrowProps {
    /// Tail width in points.
    pub width: f64,
    /// Head width in points.
    pub headwidth: f64,
    /// Head length in points.
    pub headlength: f64,
    /// Fraction of the arrow's length cut off at both ends.
    pub shrink: f64,
    /// The path from the text to the point.
    pub connectionstyle: ConnectionStyle,
    /// Colours, widths... of the arrow patch.
    pub patch: PatchProps,
}

impl Default for LegacyArrowProps {
    fn default() -> Self {
        Self {
            width: 4.0,
            headwidth: 12.0,
            headlength: 12.0,
            shrink: 0.0,
            connectionstyle: ConnectionStyle::default(),
            patch: PatchProps::default(),
        }
    }
}

/// The arrow of an annotation.
#[derive(Clone, Debug, PartialEq)]
pub enum ArrowProps {
    /// A styled arrow.
    Fancy(FancyArrowProps),
    /// The legacy arrow.
    Legacy(LegacyArrowProps),
}

impl From<ArrowStyle> for ArrowProps {
    fn from(arrowstyle: ArrowStyle) -> Self {
        Self::Fancy(FancyArrowProps::new(arrowstyle))
    }
}

/// How the arrow's start and size are chosen.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Placement {
    /// From `relpos` of the text box, sized by the mutation scale.
    Fancy {
        relpos: (f64, f64),
        mutation_scale: Option<f64>,
    },
    /// From the nearest corner, sized in points.
    Legacy {
        width: f64,
        headwidth: f64,
        headlength: f64,
        shrink: f64,
    },
}

/// What an annotation needs to place its arrow.
#[derive(Clone, Debug)]
pub(crate) struct ArrowTarget {
    /// The text's extent in display pixels.
    pub(crate) text_bbox: Bbox,
    /// The annotated point in display pixels.
    pub(crate) point: Point,
    /// The text's font size in points.
    pub(crate) fontsize: f64,
    /// What one unit of mutation scale is worth, in points: 1 for
    /// annotations; annotation boxes give the pixels per point, as
    /// matplotlib scales their arrows by the resolution twice.
    pub(crate) mutation_unit: f64,
    /// The region the arrow starts from (`patchA`).
    pub(crate) start_region: Option<ClipRegion>,
}

/// An annotation's arrow patch.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AnnotationArrow {
    placement: Placement,
    spec: ArrowSpec,
    style: PatchStyle,
    alpha: Option<f64>,
}

impl AnnotationArrow {
    /// The arrow `props` describe, styled from `rc`.
    pub(crate) fn new(props: ArrowProps, rc: &RcParams) -> Self {
        let mut style = PatchStyle::from_rc(rc);
        round_ends(&mut style);
        let (placement, spec, patch) = match props {
            ArrowProps::Fancy(fancy) => (
                Placement::Fancy {
                    relpos: fancy.relpos,
                    mutation_scale: fancy.mutation_scale,
                },
                ArrowSpec {
                    arrowstyle: fancy.arrowstyle,
                    connectionstyle: fancy.connectionstyle,
                    shrink_a: fancy.shrink_a,
                    shrink_b: fancy.shrink_b,
                    mutation_aspect: fancy.mutation_aspect,
                    ..ArrowSpec::default()
                },
                fancy.patch,
            ),
            ArrowProps::Legacy(legacy) => (
                Placement::Legacy {
                    width: legacy.width,
                    headwidth: legacy.headwidth,
                    headlength: legacy.headlength,
                    shrink: legacy.shrink,
                },
                ArrowSpec {
                    connectionstyle: legacy.connectionstyle,
                    ..ArrowSpec::default()
                },
                legacy.patch,
            ),
        };
        let base = patch.apply_style(&mut style);
        Self {
            placement,
            spec,
            style,
            alpha: base.alpha,
        }
    }

    /// The arrow's pieces in display pixels (`Annotation.update_positions`
    /// followed by `FancyArrowPatch._get_path_in_displaycoord`).
    pub(crate) fn display_paths(
        &self,
        layout: LayoutContext<'_>,
        rc: &RcParams,
        target: ArrowTarget,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        let bbox = target.text_bbox;
        let end = target.point;
        let dpi_cor = layout.points_to_pixels(1.0);
        let mut spec = self.spec.clone();
        let relpos = match self.placement {
            Placement::Fancy {
                relpos,
                mutation_scale,
            } => {
                spec.mutation_scale =
                    mutation_scale.unwrap_or(target.fontsize) * target.mutation_unit;
                relpos
            }
            Placement::Legacy {
                width,
                headwidth,
                headlength,
                shrink,
            } => {
                let ms = target.fontsize;
                spec.mutation_scale = ms;
                spec.arrowstyle = ArrowStyle::Simple {
                    head_length: headlength / ms,
                    head_width: headwidth / ms,
                    tail_width: width / ms,
                };
                let (x, relx) = nearest(
                    end.x,
                    [bbox.x0(), f64::midpoint(bbox.x0(), bbox.x1()), bbox.x1()],
                );
                let (y, rely) = nearest(
                    end.y,
                    [bbox.y0(), f64::midpoint(bbox.y0(), bbox.y1()), bbox.y1()],
                );
                let shrink_points = shrink * (y - end.y).hypot(x - end.x) / dpi_cor;
                spec.shrink_a = shrink_points;
                spec.shrink_b = shrink_points;
                (relx, rely)
            }
        };
        let begin = Point::new(
            bbox.x0() + bbox.width() * relpos.0,
            bbox.y0() + bbox.height() * relpos.1,
        );
        spec.patch_a = target.start_region;
        let linewidth = self.style.linewidth.unwrap_or(rc.patch_linewidth);
        let path = spec.route(begin, end, dpi_cor)?;
        spec.display_paths(&path, dpi_cor, linewidth)
    }

    /// Whether display point `at` lies in a piece of the arrow, within the
    /// edge width when the edge is visible (`FancyArrowPatch.contains`).
    pub(crate) fn contains(
        &self,
        layout: LayoutContext<'_>,
        rc: &RcParams,
        target: ArrowTarget,
        at: Point,
    ) -> bool {
        let (Ok(paths), Ok(resolved)) = (
            self.display_paths(layout, rc, target),
            self.style.resolve(rc, self.alpha, true),
        ) else {
            return false;
        };
        let displayed: Vec<Path> = paths.iter().map(DisplayPath::to_display).collect();
        ClipRegion {
            path: Path::compound(&displayed),
            radius: resolved.hit_radius(),
        }
        .contains(at)
    }

    /// Draws the arrow (never clipped, like matplotlib's arrow patch,
    /// which has no clip box).
    pub(crate) fn draw(
        &self,
        ctx: &mut DrawContext<'_>,
        rc: &RcParams,
        target: ArrowTarget,
    ) -> Result<(), PatchError> {
        let paths = self.display_paths(*ctx.layout(), rc, target)?;
        let mut base = ArtistBase::default();
        base.alpha = self.alpha;
        paint_paths(ctx, &base, &self.style, rc, true, &paths)?;
        Ok(())
    }
}

/// The candidate nearest `value` with its relative position 0, 0.5 or 1
/// (the first on ties, as Python's `min`).
fn nearest(value: f64, candidates: [f64; 3]) -> (f64, f64) {
    let mut best = (candidates[0], 0.0);
    for (candidate, rel) in candidates.into_iter().zip([0.0, 0.5, 1.0]).skip(1) {
        if (candidate - value).abs() < (best.0 - value).abs() {
            best = (candidate, rel);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_prefers_the_first_on_ties() {
        assert_eq!(nearest(0.0, [0.0, 5.0, 10.0]), (0.0, 0.0));
        assert_eq!(nearest(6.0, [0.0, 5.0, 10.0]), (5.0, 0.5));
        assert_eq!(nearest(7.5, [0.0, 5.0, 10.0]), (5.0, 0.5));
        assert_eq!(nearest(20.0, [0.0, 5.0, 10.0]), (10.0, 1.0));
    }

    #[test]
    fn legacy_arrows_are_simple_arrows_sized_in_points() {
        let arrow = AnnotationArrow::new(
            ArrowProps::Legacy(LegacyArrowProps::default()),
            &RcParams::matplotlib(),
        );
        assert_eq!(arrow.style.capstyle, plt_render::CapStyle::Round);
        let Placement::Legacy { width, .. } = arrow.placement else {
            panic!("legacy props place the arrow the legacy way");
        };
        assert_eq!(width, 4.0);
    }
}
