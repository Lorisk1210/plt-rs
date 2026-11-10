//! The arrow of an annotation (matplotlib's `arrowprops`).

use std::cell::RefCell;
use std::str::FromStr;

use pltrs_core::{Bbox, Path, Point};
use pltrs_render::Sketch;

use crate::artist::{
    ArtistBase, ArtistHandle, ArtistId, DrawContext, DrawnDpiCor, LayoutContext, ProgramShaping,
    rc_sketch,
};
use crate::patches::{
    ArrowSpec, ArrowStyle, ClipRegion, ConnectionStyle, DisplayPath, FancyArrowPatch, Patch,
    PatchError, PatchProps, PatchStyle, StyleError, paint_paths, round_ends,
};
use crate::patheffects::PathEffect;
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
    /// What clips the arrow at the text (matplotlib's `patchA`).
    pub patch_a: ArrowStart,
    /// The artist whose outline clips the arrow at the annotated point
    /// (matplotlib's `patchB`): a patch, or a text whose box then clips
    /// (as `get_bbox_patch()` does).
    pub patch_b: Option<ArtistId>,
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
            patch_a: ArrowStart::default(),
            patch_b: None,
            patch: PatchProps::default(),
        }
    }
}

/// What clips an annotation arrow where it leaves the text (matplotlib's
/// `patchA`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArrowStart {
    /// The text's box, or else its extent padded by 2 points unless the
    /// text is empty (matplotlib's default, without `patchA`).
    #[default]
    TextBox,
    /// Nothing: the arrow starts right at `relpos` (`patchA=None`).
    Unclipped,
    /// The outline of another artist: a patch, or a text whose box then
    /// clips (`patchA=artist`, `patchA=text.get_bbox_patch()`).
    Artist(ArtistId),
}

pltrs_core::archive_enum!(ArrowStart {
    TextBox,
    Unclipped,
    Artist(v0),
});

impl FromStr for FancyArrowProps {
    type Err = StyleError;

    /// An arrow of the arrow style `description` (as [`ArrowStyle`]
    /// parses it) with matplotlib's defaults: `dict(arrowstyle=...)`.
    ///
    /// ```
    /// use pltrs::text::FancyArrowProps;
    /// let arrow: FancyArrowProps = "->".parse()?;
    /// assert_eq!(arrow.shrink_a, 2.0);
    /// # Ok::<(), pltrs::patches::StyleError>(())
    /// ```
    fn from_str(description: &str) -> Result<Self, Self::Err> {
        description.parse().map(Self::new)
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

impl From<FancyArrowProps> for ArrowProps {
    fn from(arrow: FancyArrowProps) -> Self {
        Self::Fancy(arrow)
    }
}

impl From<LegacyArrowProps> for ArrowProps {
    fn from(arrow: LegacyArrowProps) -> Self {
        Self::Legacy(arrow)
    }
}

impl FromStr for ArrowProps {
    type Err = StyleError;

    /// A styled arrow of the arrow style `description` with matplotlib's
    /// defaults (`arrowprops=dict(arrowstyle=...)`; see
    /// [`FancyArrowProps`]).
    ///
    /// ```
    /// use pltrs::text::{AnnotateOptions, ArrowProps};
    /// let options = AnnotateOptions {
    ///     arrowprops: Some("->".parse()?),
    ///     ..AnnotateOptions::default()
    /// };
    /// assert!(matches!(options.arrowprops, Some(ArrowProps::Fancy(_))));
    /// assert!("-~>".parse::<ArrowProps>().is_err());
    /// # Ok::<(), pltrs::patches::StyleError>(())
    /// ```
    fn from_str(description: &str) -> Result<Self, Self::Err> {
        description.parse().map(Self::Fancy)
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

pltrs_core::archive_enum!(Placement {
    Fancy { relpos, mutation_scale },
    Legacy { width, headwidth, headlength, shrink },
});

/// What an annotation needs to place its arrow.
#[derive(Clone, Debug)]
pub(crate) struct ArrowTarget {
    /// The artist the arrow belongs to: an annotation or an annotation
    /// box.
    pub(crate) owner: ArtistId,
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
    pub(crate) start_region: StartRegion,
}

/// The region clipping an arrow where it leaves its text (`patchA`).
#[derive(Clone, Debug)]
pub(crate) enum StartRegion {
    /// A region made anew whenever the arrow is placed: the text's extent
    /// padded, an annotation box's frame, or none.
    Fresh(Option<ClipRegion>),
    /// The box of the text, `now` where drawing the text puts it,
    /// `initial` where it stands before the text is first drawn
    /// (matplotlib's `FancyBboxPatch` at `(0, 0)`, 1 × 1): matplotlib only
    /// moves and sizes the box when it draws the text, so an arrow
    /// measured between draws starts from the box of the last draw.
    TextBox {
        now: ClipRegion,
        initial: ClipRegion,
    },
}

/// The box an arrow started from at its last draw (see
/// [`StartRegion::TextBox`]).
#[derive(Clone, Debug, Default, PartialEq)]
struct DrawnBox(RefCell<Option<ClipRegion>>);

pltrs_core::archive_struct!(DrawnBox(v0));

/// An annotation's arrow patch.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AnnotationArrow {
    placement: Placement,
    spec: ArrowSpec,
    /// What clips the arrow at the text.
    patch_a: ArrowStart,
    /// The artist clipping the arrow at the annotated point.
    patch_b: Option<ArtistId>,
    style: PatchStyle,
    alpha: Option<f64>,
    /// The arrow's own path effects (not the text's): `path.effects` when
    /// it was made, unless given.
    path_effects: Vec<PathEffect>,
    /// The arrow's own sketch: `path.sketch` when it was made (matplotlib's
    /// arrow patch takes it when the annotation makes it).
    sketch: Option<Sketch>,
    /// The resolution of the arrow's last draw.
    dpi_cor: DrawnDpiCor,
    /// The box of the text at the arrow's last draw.
    drawn_box: DrawnBox,
}

pltrs_core::archive_struct!(AnnotationArrow {
    placement,
    spec,
    patch_a,
    patch_b,
    style,
    alpha,
    path_effects,
    sketch,
    dpi_cor,
    drawn_box
});

impl AnnotationArrow {
    /// A loose fancy arrow patch with the arrow's look (its arrow and
    /// connection styles, shrinking, face, edge, alpha and path effects)
    /// from `(0, 0)` to `(1, 1)` (see `Annotation::arrow_patch`).
    pub(crate) fn to_patch(&self) -> Patch<FancyArrowPatch> {
        let mut shape = FancyArrowPatch::between((0.0, 0.0), (1.0, 1.0));
        shape.spec = self.spec.clone();
        let patch = Patch::loose(shape, PatchProps::default());
        patch.update_style(|style| style.clone_from(&self.style));
        patch
            .raw()
            .set_alpha(self.alpha)
            .set_path_effects(self.path_effects.clone())
            .set_sketch_params(self.sketch);
        patch
    }

    /// Takes the look of `patch` (made by [`AnnotationArrow::to_patch`]):
    /// face, edge, alpha and path effects, and the arrow's build but for
    /// what the annotation sets anew at every draw, as matplotlib's
    /// `update_positions` does (the whole simple arrow of the legacy
    /// `arrowprops`, but its connection style).
    pub(crate) fn restyle_from(&mut self, patch: &Patch<FancyArrowPatch>) {
        let spec = patch.shape().spec;
        match self.placement {
            Placement::Fancy { .. } => self.spec = spec,
            Placement::Legacy { .. } => self.spec.connectionstyle = spec.connectionstyle,
        }
        self.style = patch.style();
        self.alpha = patch.raw().alpha();
        self.path_effects = patch.raw().path_effects();
        self.sketch = patch.raw().sketch_params();
    }

    /// The arrow `props` describe, styled from `rc`.
    pub(crate) fn new(props: ArrowProps, rc: &RcParams) -> Self {
        let mut style = PatchStyle::from_rc(rc);
        round_ends(&mut style);
        let (patch_a, patch_b) = match &props {
            ArrowProps::Fancy(fancy) => (fancy.patch_a, fancy.patch_b),
            ArrowProps::Legacy(_) => (ArrowStart::TextBox, None),
        };
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
            patch_a,
            patch_b,
            style,
            alpha: base.alpha,
            path_effects: base.path_effects.unwrap_or_else(|| rc.path_effects.clone()),
            sketch: rc_sketch(rc),
            dpi_cor: DrawnDpiCor::default(),
            drawn_box: DrawnBox::default(),
        }
    }

    /// The arrow's pieces in display pixels as measured, between draws
    /// (see [`DrawnDpiCor`]).
    pub(crate) fn measured_paths(
        &self,
        layout: LayoutContext<'_>,
        rc: &RcParams,
        target: ArrowTarget,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        let dpi_cor = self.dpi_cor.measured(layout, target.owner);
        let start = match &target.start_region {
            StartRegion::Fresh(region) => region.clone(),
            StartRegion::TextBox { now, .. } if layout.counts_as_drawn(target.owner) => {
                Some(now.clone())
            }
            StartRegion::TextBox { initial, .. } => Some(
                self.drawn_box
                    .0
                    .borrow()
                    .clone()
                    .unwrap_or_else(|| initial.clone()),
            ),
        };
        self.display_paths(layout, rc, &target, dpi_cor, start)
    }

    /// The arrow's pieces in display pixels (`Annotation.update_positions`
    /// followed by `FancyArrowPatch._get_path_in_displaycoord`), its sizes
    /// scaled by `dpi_cor` pixels per point, `start` clipping it at the
    /// text.
    fn display_paths(
        &self,
        layout: LayoutContext<'_>,
        rc: &RcParams,
        target: &ArrowTarget,
        dpi_cor: f64,
        start: Option<ClipRegion>,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        self.routed(layout, rc, target, dpi_cor, start).paths()
    }

    /// The calls of the program's connection and arrow styles the arrow's
    /// draw makes (see `Annotation::program_drawing`).
    pub(crate) fn program_drawing(
        &self,
        layout: LayoutContext<'_>,
        rc: &RcParams,
        target: &ArrowTarget,
    ) -> Option<ProgramShaping> {
        if !self.calls_program() {
            return None;
        }
        let start = match &target.start_region {
            StartRegion::Fresh(region) => region.clone(),
            StartRegion::TextBox { now, .. } => Some(now.clone()),
        };
        let routed = self.routed(layout, rc, target, layout.points_to_pixels(1.0), start);
        Some(Box::new(move || drop(routed.paths())))
    }

    /// Whether drawing the arrow calls a connection or arrow style of the
    /// program's.
    pub(crate) fn calls_program(&self) -> bool {
        let fancy = matches!(self.placement, Placement::Fancy { .. });
        self.spec.connectionstyle.calls_program() || (fancy && self.spec.arrowstyle.calls_program())
    }

    /// The arrow as [`display_paths`](Self::display_paths) makes it, up to
    /// its route.
    fn routed(
        &self,
        layout: LayoutContext<'_>,
        rc: &RcParams,
        target: &ArrowTarget,
        dpi_cor: f64,
        start: Option<ClipRegion>,
    ) -> Routed {
        let bbox = target.text_bbox;
        let end = target.point;
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
                // In points at the canvas resolution, whatever `dpi_cor` is.
                let distance = (y - end.y).hypot(x - end.x);
                let shrink_points = shrink * distance / layout.points_to_pixels(1.0);
                spec.shrink_a = shrink_points;
                spec.shrink_b = shrink_points;
                (relx, rely)
            }
        };
        let begin = Point::new(
            bbox.x0() + bbox.width() * relpos.0,
            bbox.y0() + bbox.height() * relpos.1,
        );
        let region = |id: ArtistId| layout.store().get(id)?.connection_region(&layout);
        spec.patch_a = match self.patch_a {
            ArrowStart::TextBox => start,
            ArrowStart::Unclipped => None,
            ArrowStart::Artist(id) => region(id),
        };
        spec.patch_b = self.patch_b.and_then(region);
        Routed {
            spec,
            ends: (begin, end),
            dpi_cor,
            linewidth: self.style.linewidth.unwrap_or(rc.patch_linewidth),
        }
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
            self.measured_paths(layout, rc, target),
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
        let layout = ctx.layout();
        let dpi_cor = layout.points_to_pixels(1.0);
        self.dpi_cor.record(dpi_cor);
        let start = match &target.start_region {
            StartRegion::Fresh(region) => region.clone(),
            StartRegion::TextBox { now, .. } => {
                self.drawn_box.0.replace(Some(now.clone()));
                Some(now.clone())
            }
        };
        let paths = self.display_paths(layout, rc, &target, dpi_cor, start)?;
        let mut base = ArtistBase::default();
        base.alpha = self.alpha;
        base.path_effects.clone_from(&self.path_effects);
        base.sketch = self.sketch;
        paint_paths(ctx, &base, &self.style, rc, true, &paths)?;
        Ok(())
    }
}

/// An annotation's arrow ready to be routed and made: its spec, the ends
/// of its route in display pixels, the pixels per point and the edge
/// width (points).
struct Routed {
    spec: ArrowSpec,
    ends: (Point, Point),
    dpi_cor: f64,
    linewidth: f64,
}

impl Routed {
    /// The arrow's pieces in display pixels.
    fn paths(&self) -> Result<Vec<DisplayPath>, PatchError> {
        let path = self.spec.route(self.ends.0, self.ends.1, self.dpi_cor)?;
        self.spec.display_paths(&path, self.dpi_cor, self.linewidth)
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
        assert_eq!(arrow.style.capstyle, pltrs_render::CapStyle::Round);
        let Placement::Legacy { width, .. } = arrow.placement else {
            panic!("legacy props place the arrow the legacy way");
        };
        assert_eq!(width, 4.0);
    }
}
