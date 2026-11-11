//! The box drawn behind a text (matplotlib's `Text.set_bbox`).

use pltrs_core::{Affine2D, Point};

use super::{Text, TextArtist};
use crate::artist::{
    ArtistBase, ArtistHandle, ArtistLookup, DrawContext, LayoutContext, ProgramShaping,
};
use crate::colors::Color;
use crate::patches::{
    BoxShape, BoxStyle, ClipRegion, DEFAULT_PAD, FancyBboxPatch, Patch, PatchError, PatchParts,
    PatchProps, PatchShape, PatchStyle,
};
use crate::patheffects::PathEffect;
use crate::rc::RcParams;
use crate::ticker::pyfmt;
use crate::transforms::Transform;

/// The box of a text (matplotlib's `bbox` dictionary).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextBoxProps {
    /// The outline; `None` is a square.
    pub boxstyle: Option<BoxStyle>,
    /// Padding used when the box style sets none: in points for the
    /// default square (4 points when unset), as a fraction of the font
    /// size otherwise ([`DEFAULT_PAD`] when unset).
    pub pad: Option<f64>,
    /// Face, edge, alpha... of the box.
    pub patch: PatchProps,
    /// The height the box style shapes the box at, as a fraction of its
    /// real height (matplotlib's `mutation_aspect`); 1 when unset.
    pub mutation_aspect: Option<f64>,
}

impl TextBoxProps {
    /// A box of `boxstyle`.
    pub fn new(boxstyle: BoxStyle) -> Self {
        Self {
            boxstyle: Some(boxstyle),
            ..Self::default()
        }
    }

    /// The same box with other patch properties.
    #[must_use]
    pub fn with_patch(self, patch: PatchProps) -> Self {
        Self { patch, ..self }
    }
}

impl TextArtist {
    /// The box where the text draws it: its shape and the transform
    /// placing it in display pixels; `None` without a box or an anchor.
    fn box_frame(&self, ctx: LayoutContext<'_>) -> Option<(FancyBboxPatch, Affine2D)> {
        let (bbox, placement) = self.box_placement(ctx, self.box_anchor(ctx)?).ok()??;
        Some(bbox.shape(&placement))
    }

    /// Whether the text has a visible box of a style of the program's.
    pub(super) fn box_shaped_by_program(&self) -> bool {
        self.bbox
            .as_ref()
            .is_some_and(|bbox| bbox.visible && matches!(bbox.boxstyle.shape, BoxShape::Program(_)))
    }

    /// The call of the program's box style shaping the text's box, as
    /// the text's draw makes it (matplotlib's `_bbox_patch.draw`, for a
    /// text with text and a box drawn); the text is measured for it
    /// without noting its fonts, which pltrs notes for every draw a
    /// print stands for (see [`font_notes`](crate::text::font_notes)).
    pub(super) fn program_box_shaping(&self, ctx: LayoutContext<'_>) -> Option<ProgramShaping> {
        if !self.box_shaped_by_program() || self.text.is_empty() {
            return None;
        }
        let (shape, _) = crate::text::font_notes::unnoted(|| self.box_frame(ctx))?;
        Some(Box::new(move || drop(shape.path())))
    }
}

impl Text {
    /// The box behind the text where it is drawn in the figure's current
    /// layout (laid out first): the [`FancyBboxPatch`] at the origin with
    /// the size of the text's frame and a mutation scale of its font size
    /// in pixels, and the transform rotating it and placing it in display
    /// pixels (matplotlib's `update_bbox_position_size`, which moves the
    /// patch of `get_bbox_patch` there): also of a text the figure keeps
    /// apart, given it without adding it (cartopy's gridliner labels).
    /// `None` without a box, or for a text outside any figure.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::text::TextBoxProps;
    /// let fig = Figure::with_size(4.0, 3.0, 100.0);
    /// let text = fig.text(0.5, 0.5, "boxed");
    /// assert!(text.bbox_frame().is_none());
    /// text.set_bbox(Some(TextBoxProps::default()));
    /// let (shape, transform) = text.bbox_frame().expect("a box");
    /// assert_eq!(shape.mutation_scale, text.font().size * 100.0 / 72.0);
    /// // The frame of the left-aligned text, from its anchor's x and below
    /// // its baseline; the box style pads it when it is drawn.
    /// let corner = transform.transform_point((0.0, 0.0).into());
    /// assert!((corner.x - 200.0).abs() < 1e-9 && corner.y < 150.0);
    /// ```
    pub fn bbox_frame(&self) -> Option<(FancyBboxPatch, Affine2D)> {
        let raw = self.raw();
        let state = raw.figure_state().prepared();
        let layout = LayoutContext::new(&state);
        state.artist_as::<TextArtist>(raw.id())?.box_frame(layout)
    }
}

/// Where a text's box goes: the text's frame and anchor in display pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoxPlacement {
    /// The text anchor.
    pub(crate) anchor: Point,
    /// Lower-left corner of the rotated frame, relative to the anchor.
    pub(crate) corner: Point,
    /// Unrotated size of the frame.
    pub(crate) size: (f64, f64),
    /// Rotation in degrees.
    pub(crate) rotation: f64,
    /// The font size in pixels (the box's mutation scale).
    pub(crate) fontsize_px: f64,
}

impl BoxPlacement {
    /// Where a text's box stands before the text is first drawn
    /// (matplotlib's `FancyBboxPatch((0, 0), 1, 1)` in display pixels, at
    /// a mutation scale of 1).
    pub(crate) fn before_drawn() -> Self {
        Self {
            anchor: Point::new(0.0, 0.0),
            corner: Point::new(0.0, 0.0),
            size: (1.0, 1.0),
            rotation: 0.0,
            fontsize_px: 1.0,
        }
    }
}

/// A text's box: a [`FancyBboxPatch`] fitted around the text at draw time
/// (matplotlib's `get_bbox_patch`), made from [`TextBoxProps`] with its
/// padding fixed for the text's font size at the time.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBoxPatch {
    boxstyle: BoxStyle,
    style: PatchStyle,
    alpha: Option<f64>,
    /// Whether the box is drawn (the text is drawn either way).
    visible: bool,
    /// The box's own path effects (not the text's): `path.effects` when it
    /// was made, unless given.
    path_effects: Vec<PathEffect>,
    /// The aspect the box style shapes the box at (matplotlib's
    /// `mutation_aspect`).
    mutation_aspect: f64,
}

pltrs_core::archive_struct!(TextBoxPatch {
    boxstyle,
    style,
    alpha,
    visible,
    path_effects,
    mutation_aspect
});

impl TextBoxPatch {
    /// The box `props` describe for text of `fontsize` points: the padding
    /// is fixed now, rounded to two decimals as matplotlib's `"%0.2f"`
    /// appends it to the style name.
    pub(crate) fn new(props: TextBoxProps, fontsize: f64, rc: &RcParams) -> Self {
        let (boxstyle, pad) = match props.boxstyle {
            None => (
                BoxStyle::new(BoxShape::Square),
                props.pad.unwrap_or(4.0) / fontsize,
            ),
            Some(style) => (style, props.pad.unwrap_or(DEFAULT_PAD)),
        };
        let boxstyle = if boxstyle.pad.is_some() {
            boxstyle
        } else {
            boxstyle.with_pad(pyfmt::fixed(pad, 2).parse().unwrap_or(pad))
        };
        let mut style = PatchStyle::from_rc(rc);
        let base = props.patch.apply_style(&mut style);
        Self {
            boxstyle,
            style,
            alpha: base.alpha,
            visible: base.visible.unwrap_or(true),
            path_effects: base.path_effects.unwrap_or_else(|| rc.path_effects.clone()),
            mutation_aspect: props.mutation_aspect.unwrap_or(1.0),
        }
    }

    /// The box of matplotlib's `set_backgroundcolor`: face and edge in
    /// `color`.
    pub(crate) fn background(color: Color, fontsize: f64, rc: &RcParams) -> Self {
        let patch = PatchProps::default()
            .facecolor(color.clone())
            .edgecolor(color);
        Self::new(
            TextBoxProps {
                patch,
                ..TextBoxProps::default()
            },
            fontsize,
            rc,
        )
    }

    /// Changes the face colour (`set_backgroundcolor` on an existing box).
    pub(crate) fn set_facecolor(&mut self, color: Color) {
        self.style.facecolor = Some(color);
    }

    /// The outline, with the padding in font sizes (matplotlib's
    /// `get_boxstyle`).
    pub fn boxstyle(&self) -> BoxStyle {
        self.boxstyle.clone()
    }

    /// The face, edge and hatch of the box (matplotlib's `get_facecolor`,
    /// `get_edgecolor`, ... of the patch).
    pub fn style(&self) -> &PatchStyle {
        &self.style
    }

    /// The opacity of the box, if set.
    pub fn alpha(&self) -> Option<f64> {
        self.alpha
    }

    /// Whether the box is drawn behind its text.
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// The effects the box is drawn through.
    pub fn path_effects(&self) -> &[PathEffect] {
        &self.path_effects
    }

    /// A loose fancy box patch with the box's look (box style, face,
    /// edge, hatch, alpha, visibility and path effects) on the unit square: what a
    /// program reads and changes of the box as the patch matplotlib's
    /// `get_bbox_patch` returns, the text placing it when drawn. Changes
    /// come back with [`TextBoxPatch::restyle_from`].
    pub fn to_patch(&self) -> Patch<FancyBboxPatch> {
        let patch = Patch::loose(
            FancyBboxPatch::new((0.0, 0.0), 1.0, 1.0, self.boxstyle.clone())
                .with_mutation_aspect(self.mutation_aspect),
            PatchProps::default(),
        );
        patch.update_style(|style| style.clone_from(&self.style));
        patch
            .raw()
            .set_alpha(self.alpha)
            .set_visible(self.visible)
            .set_path_effects(self.path_effects.clone());
        patch
    }

    /// Takes the look of `patch` (made by [`TextBoxPatch::to_patch`]): its
    /// box style and aspect, face, edge, hatch, alpha, visibility and path
    /// effects.
    pub fn restyle_from(&mut self, patch: &Patch<FancyBboxPatch>) {
        let shape = patch.shape();
        self.boxstyle = shape.boxstyle;
        self.mutation_aspect = shape.mutation_aspect;
        self.style = patch.style();
        self.alpha = patch.raw().alpha();
        self.visible = patch.raw().visible();
        self.path_effects = patch.raw().path_effects();
    }

    /// The box shape and its placement (`update_bbox_position_size`).
    fn shape(&self, placement: &BoxPlacement) -> (FancyBboxPatch, Affine2D) {
        let (width, height) = placement.size;
        let shape = FancyBboxPatch::new((0.0, 0.0), width, height, self.boxstyle.clone())
            .with_mutation_scale(placement.fontsize_px)
            .with_mutation_aspect(self.mutation_aspect);
        let origin = placement.anchor + placement.corner;
        let transform = Affine2D::identity()
            .rotate_deg(placement.rotation)
            .translate(origin.x, origin.y);
        (shape, transform)
    }

    /// The artist properties of the box: the text's clipping, the box's
    /// own alpha and path effects.
    fn base(&self, text: &ArtistBase, transform: Affine2D) -> ArtistBase {
        let mut base = ArtistBase::default();
        base.alpha = self.alpha;
        base.path_effects.clone_from(&self.path_effects);
        base.clip_on = text.clip_on;
        base.clip_box = text.clip_box;
        base.clip_path.clone_from(&text.clip_path);
        base.parent = text.parent;
        base.transform = Some(Transform::Affine(transform));
        base
    }

    /// Draws the box behind the text, unless it is hidden.
    pub(crate) fn draw(
        &self,
        ctx: &mut DrawContext<'_>,
        text: &ArtistBase,
        rc: &RcParams,
        placement: &BoxPlacement,
    ) -> Result<(), PatchError> {
        if !self.visible {
            return Ok(());
        }
        let (shape, transform) = self.shape(placement);
        let base = self.base(text, transform);
        PatchParts {
            base: &base,
            style: &self.style,
            rc,
            shape: &shape,
            interpolation_steps: 1,
            drawn: None,
        }
        .paint(ctx)
    }

    /// The box as a region an annotation arrow starts from: its outline in
    /// display pixels, with the edge width as tolerance when the edge is
    /// visible (`Patch.contains`).
    pub(crate) fn clip_region(
        &self,
        rc: &RcParams,
        placement: &BoxPlacement,
    ) -> Result<ClipRegion, PatchError> {
        let (shape, transform) = self.shape(placement);
        let resolved = self
            .style
            .resolve(rc, self.alpha, FancyBboxPatch::EDGE_DEFAULT)?;
        let radius = resolved.hit_radius();
        Ok(if shape.runs_program() {
            ClipRegion::remade(move || shape.path().transformed(&transform), radius)
        } else {
            ClipRegion::new(shape.path().transformed(&transform), radius)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patches::BoxShape;

    fn rc() -> RcParams {
        RcParams::matplotlib()
    }

    #[test]
    fn the_box_keeps_its_mutation_aspect_through_its_patch() {
        let props = TextBoxProps {
            mutation_aspect: Some(0.75),
            ..TextBoxProps::new(BoxStyle::new(BoxShape::Round {
                rounding_size: None,
            }))
        };
        let mut bbox = TextBoxPatch::new(props, 10.0, &rc());
        let patch = bbox.to_patch();
        assert_eq!(patch.shape().mutation_aspect, 0.75);
        let placement = BoxPlacement {
            anchor: Point::new(0.0, 0.0),
            corner: Point::new(0.0, 0.0),
            size: (40.0, 20.0),
            rotation: 0.0,
            fontsize_px: 10.0,
        };
        assert_eq!(bbox.shape(&placement).0.mutation_aspect, 0.75);
        bbox.restyle_from(&Patch::loose(
            FancyBboxPatch::new((0.0, 0.0), 1.0, 1.0, bbox.boxstyle()).with_mutation_aspect(2.0),
            PatchProps::default(),
        ));
        assert_eq!(bbox.shape(&placement).0.mutation_aspect, 2.0);
        let plain = TextBoxPatch::new(TextBoxProps::default(), 10.0, &rc());
        assert_eq!(plain.shape(&placement).0.mutation_aspect, 1.0);
    }

    #[test]
    fn the_default_square_pads_four_points_rounded_like_matplotlib() {
        let square = TextBoxPatch::new(TextBoxProps::default(), 11.0, &rc());
        assert_eq!(square.boxstyle.shape, BoxShape::Square);
        assert_eq!(
            square.boxstyle.pad,
            Some(0.36),
            "4 / 11 = 0.3636... -> \"0.36\""
        );
        let padded = TextBoxPatch::new(
            TextBoxProps {
                pad: Some(8.0),
                ..TextBoxProps::default()
            },
            10.0,
            &rc(),
        );
        assert_eq!(padded.boxstyle.pad, Some(0.8));
    }

    #[test]
    fn styles_keep_their_own_padding() {
        let round: BoxStyle = "round".parse().expect("valid style");
        assert_eq!(
            TextBoxPatch::new(TextBoxProps::new(round.clone()), 10.0, &rc())
                .boxstyle
                .pad,
            Some(0.3)
        );
        let explicit: BoxStyle = "round,pad=0.123".parse().expect("valid style");
        assert_eq!(
            TextBoxPatch::new(TextBoxProps::new(explicit), 10.0, &rc())
                .boxstyle
                .pad,
            Some(0.123)
        );
    }

    #[test]
    fn boxes_come_back_from_their_patches() {
        let round: BoxStyle = "round".parse().expect("valid style");
        let mut text_box = TextBoxPatch::new(TextBoxProps::new(round.clone()), 10.0, &rc());
        let patch = text_box.to_patch();
        assert_eq!(patch.shape().boxstyle, text_box.boxstyle);
        patch.set_alpha(Some(0.25));
        patch.update_style(|style| style.facecolor = Some(Color::from("red")));
        text_box.restyle_from(&patch);
        assert_eq!(text_box.alpha(), Some(0.25));
        assert_eq!(text_box.style().facecolor, Some(Color::from("red")));
        assert_eq!(text_box.boxstyle(), round.with_pad(0.3));
    }

    #[test]
    fn hidden_boxes_stay_hidden_through_their_patches() {
        // cartopy's gridliner labels: `bbox=dict(pad=0, visible=False)`.
        let mut props = TextBoxProps {
            pad: Some(0.0),
            ..TextBoxProps::default()
        };
        props.patch.base.visible = Some(false);
        let mut hidden = TextBoxPatch::new(props, 10.0, &rc());
        assert!(!hidden.visible());
        let patch = hidden.to_patch();
        assert!(!patch.visible());
        patch.set_visible(true);
        hidden.restyle_from(&patch);
        assert!(hidden.visible());
        assert!(TextBoxPatch::new(TextBoxProps::default(), 10.0, &rc()).visible());
    }

    #[test]
    fn backgrounds_colour_face_and_edge() {
        let mut background = TextBoxPatch::background(Color::from("yellow"), 10.0, &rc());
        assert_eq!(background.style.facecolor, Some(Color::from("yellow")));
        assert_eq!(background.style.edgecolor, Some(Color::from("yellow")));
        background.set_facecolor(Color::from("red"));
        assert_eq!(background.style.facecolor, Some(Color::from("red")));
    }

    #[test]
    fn boxes_are_placed_on_the_rotated_frame() {
        let square = TextBoxPatch::new(
            TextBoxProps::new("square,pad=0".parse().expect("valid style")),
            10.0,
            &rc(),
        );
        let placement = BoxPlacement {
            anchor: Point::new(100.0, 50.0),
            corner: Point::new(-10.0, -5.0),
            size: (20.0, 10.0),
            rotation: 90.0,
            fontsize_px: 10.0,
        };
        let region = square
            .clip_region(&rc(), &placement)
            .expect("valid colours");
        let extents = region.outline().extents();
        assert!((extents.x0() - 80.0).abs() < 1e-9 && (extents.y1() - 65.0).abs() < 1e-9);
        assert_eq!(region.radius(), 1.0, "the visible edge widens the region");
    }
}
