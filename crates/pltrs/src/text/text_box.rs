//! The box drawn behind a text (matplotlib's `Text.set_bbox`).

use pltrs_core::{Affine2D, Point};

use crate::artist::{ArtistBase, DrawContext};
use crate::colors::Color;
use crate::patches::{
    BoxShape, BoxStyle, ClipRegion, DEFAULT_PAD, FancyBboxPatch, PatchError, PatchParts,
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

/// A text's box: a [`FancyBboxPatch`] fitted around the text at draw time
/// (matplotlib's `get_bbox_patch`), made from [`TextBoxProps`] with its
/// padding fixed for the text's font size at the time.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBoxPatch {
    boxstyle: BoxStyle,
    style: PatchStyle,
    alpha: Option<f64>,
    /// The box's own path effects (not the text's): `path.effects` when it
    /// was made, unless given.
    path_effects: Vec<PathEffect>,
}

pltrs_core::archive_struct!(TextBoxPatch {
    boxstyle,
    style,
    alpha,
    path_effects
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
            path_effects: base.path_effects.unwrap_or_else(|| rc.path_effects.clone()),
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
        self.boxstyle
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

    /// The effects the box is drawn through.
    pub fn path_effects(&self) -> &[PathEffect] {
        &self.path_effects
    }

    /// The box shape and its placement (`update_bbox_position_size`).
    fn shape(&self, placement: &BoxPlacement) -> (FancyBboxPatch, Affine2D) {
        let (width, height) = placement.size;
        let shape = FancyBboxPatch::new((0.0, 0.0), width, height, self.boxstyle)
            .with_mutation_scale(placement.fontsize_px);
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

    /// Draws the box behind the text.
    pub(crate) fn draw(
        &self,
        ctx: &mut DrawContext<'_>,
        text: &ArtistBase,
        rc: &RcParams,
        placement: &BoxPlacement,
    ) -> Result<(), PatchError> {
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
        Ok(ClipRegion {
            path: shape.path().transformed(&transform),
            radius: resolved.hit_radius(),
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
            TextBoxPatch::new(TextBoxProps::new(round), 10.0, &rc())
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
        let extents = region.path.extents();
        assert!((extents.x0() - 80.0).abs() < 1e-9 && (extents.y1() - 65.0).abs() < 1e-9);
        assert_eq!(region.radius, 1.0, "the visible edge widens the region");
    }
}
