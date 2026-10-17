//! The boxes of an offset-box tree and their layout (matplotlib's
//! `HPacker`, `VPacker`, `DrawingArea` and `TextArea`).
//!
//! Layout follows matplotlib's `_get_bbox_and_child_offsets`: every box
//! reports its extent relative to its own origin, and a packer reports the
//! offset of each visible child's origin from its own. Pads and
//! separations are in points and drawing-area sizes too; both are scaled
//! by the canvas resolution, while explicit packer widths and heights are
//! in pixels.

use plt_core::{Affine2D, Bbox, Point};
use plt_text::HAlign;

use super::{OffsetBoxError, PackAlign, PackMode, aligned_offsets, packed_offsets};
use crate::artist::{ArtistId, LayoutContext};
use crate::text::{TextArtist, measure_text};

/// A box of an offset-box tree; drawing areas hold artists of type `A`.
#[derive(Clone, Debug, PartialEq)]
pub enum OffsetBox<A> {
    /// Children side by side, left to right (matplotlib's `HPacker`).
    HPacker(Packer<A>),
    /// Children stacked top to bottom (matplotlib's `VPacker`).
    VPacker(Packer<A>),
    /// A fixed-size area for artists (matplotlib's `DrawingArea`).
    DrawingArea(DrawingArea<A>),
    /// A single text (matplotlib's `TextArea`).
    TextArea(TextArea),
}

/// The settings and children of an [`OffsetBox::HPacker`] or
/// [`OffsetBox::VPacker`] (matplotlib's `PackerBase`).
#[derive(Clone, Debug, PartialEq)]
pub struct Packer<A> {
    /// Padding around the children, in points.
    pub pad: f64,
    /// Space between children, in points.
    pub sep: f64,
    /// Width in pixels; `None` fits the children.
    pub width: Option<f64>,
    /// Height in pixels; `None` fits the children.
    pub height: Option<f64>,
    /// Alignment across the packing direction.
    pub align: PackAlign,
    /// Distribution along the packing direction.
    pub mode: PackMode,
    /// The packed boxes.
    pub children: Vec<OffsetBox<A>>,
    /// Whether the packer takes part in its parent's layout.
    pub visible: bool,
}

impl<A> Packer<A> {
    /// A visible packer fitting `children` with the given pad, separation
    /// and alignment, packed tight.
    pub fn new(pad: f64, sep: f64, align: PackAlign, children: Vec<OffsetBox<A>>) -> Self {
        Self {
            pad,
            sep,
            width: None,
            height: None,
            align,
            mode: PackMode::Fixed,
            children,
            visible: true,
        }
    }
}

/// An area of fixed size whose artists are positioned relative to its
/// origin, in points (matplotlib's `DrawingArea`).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingArea<A> {
    /// Width in points.
    pub width: f64,
    /// Height in points.
    pub height: f64,
    /// How far the area extends left of its origin, in points.
    pub xdescent: f64,
    /// How far the area extends below its origin, in points.
    pub ydescent: f64,
    /// The artists drawn in the area, in drawing order.
    pub artists: Vec<A>,
    /// Whether the area takes part in its parent's layout.
    pub visible: bool,
}

/// A text whose origin is the start of its (last) baseline (matplotlib's
/// `TextArea`). The text artist must be left, centre or right aligned
/// horizontally and baseline aligned vertically.
#[derive(Clone, Debug, PartialEq)]
pub struct TextArea {
    /// The text artist.
    pub text: ArtistId,
    /// Shift multi-line text so that it is roughly centred on the
    /// baseline of single-line text (matplotlib's `multilinebaseline`).
    pub multilinebaseline: bool,
    /// Whether the area takes part in its parent's layout.
    pub visible: bool,
}

/// Where a leaf of the tree ends up once laid out.
#[derive(Clone, Debug, PartialEq)]
pub enum Placement<A> {
    /// A text to be drawn with its anchor (the start of its last baseline)
    /// at `anchor`, in display pixels.
    Text {
        /// The text artist.
        text: ArtistId,
        /// The anchor in display pixels.
        anchor: Point,
    },
    /// A drawing area: its artists' coordinates (points from the area's
    /// origin) map to display pixels through `transform`.
    Area {
        /// The artists of the area.
        artists: Vec<A>,
        /// Points from the area's origin to display pixels.
        transform: Affine2D,
    },
}

/// A visible leaf of the tree, in drawing order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Leaf<'a, A> {
    /// A text artist.
    Text(ArtistId),
    /// The artists of a drawing area.
    Area(&'a [A]),
}

/// The extent of a text area and how far its text is shifted up from the
/// area's origin.
struct TextExtent {
    bbox: Bbox,
    baseline_shift: f64,
}

impl<A: Clone> OffsetBox<A> {
    /// Whether the box takes part in its parent's layout.
    pub fn is_visible(&self) -> bool {
        match self {
            Self::HPacker(packer) | Self::VPacker(packer) => packer.visible,
            Self::DrawingArea(area) => area.visible,
            Self::TextArea(area) => area.visible,
        }
    }

    /// Shows or hides the box.
    pub fn set_visible(&mut self, visible: bool) {
        match self {
            Self::HPacker(packer) | Self::VPacker(packer) => packer.visible = visible,
            Self::DrawingArea(area) => area.visible = visible,
            Self::TextArea(area) => area.visible = visible,
        }
    }

    /// Fixes the width of a packer, in pixels (matplotlib's `set_width`);
    /// other boxes keep their size.
    pub fn set_width(&mut self, width: Option<f64>) {
        if let Self::HPacker(packer) | Self::VPacker(packer) = self {
            packer.width = width;
        }
    }

    /// The extent relative to the box's origin, in pixels (matplotlib's
    /// `get_bbox`).
    pub fn bbox(&self, ctx: LayoutContext<'_>) -> Result<Bbox, OffsetBoxError> {
        self.bbox_sized(ctx, None)
    }

    /// The extent of the box once its origin is at `offset` (matplotlib's
    /// `get_window_extent`).
    pub fn window_extent(
        &self,
        ctx: LayoutContext<'_>,
        offset: Point,
    ) -> Result<Bbox, OffsetBoxError> {
        Ok(self.bbox(ctx)?.translated(offset.x, offset.y))
    }

    /// Where every visible leaf goes when the box's origin is at `offset`,
    /// in drawing order (the offsets matplotlib's `OffsetBox.draw` sets).
    pub fn place(
        &self,
        ctx: LayoutContext<'_>,
        offset: Point,
    ) -> Result<Vec<Placement<A>>, OffsetBoxError> {
        let mut placements = Vec::new();
        self.place_into(ctx, offset, None, &mut placements)?;
        Ok(placements)
    }

    /// The visible leaves in drawing order.
    pub fn leaves(&self) -> Vec<Leaf<'_, A>> {
        let mut leaves = Vec::new();
        self.collect_leaves(&mut leaves);
        leaves
    }

    fn collect_leaves<'a>(&'a self, leaves: &mut Vec<Leaf<'a, A>>) {
        match self {
            Self::HPacker(packer) | Self::VPacker(packer) => {
                for child in packer.children.iter().filter(|c| c.is_visible()) {
                    child.collect_leaves(leaves);
                }
            }
            Self::DrawingArea(area) => leaves.push(Leaf::Area(&area.artists)),
            Self::TextArea(area) => leaves.push(Leaf::Text(area.text)),
        }
    }

    /// The extent, with the width a parent imposes on an expanding packer.
    fn bbox_sized(
        &self,
        ctx: LayoutContext<'_>,
        imposed_width: Option<f64>,
    ) -> Result<Bbox, OffsetBoxError> {
        match self {
            Self::HPacker(packer) => Ok(packer.horizontal(ctx, imposed_width)?.0),
            Self::VPacker(packer) => Ok(packer.vertical(ctx, imposed_width)?.0),
            Self::DrawingArea(area) => Ok(area.bbox(ctx)),
            Self::TextArea(area) => Ok(area.extent(ctx)?.bbox),
        }
    }

    fn place_into(
        &self,
        ctx: LayoutContext<'_>,
        offset: Point,
        imposed_width: Option<f64>,
        placements: &mut Vec<Placement<A>>,
    ) -> Result<(), OffsetBoxError> {
        match self {
            Self::HPacker(packer) => {
                let (_, offsets) = packer.horizontal(ctx, imposed_width)?;
                packer.place_children(ctx, offset, &offsets, None, placements)?;
            }
            Self::VPacker(packer) => {
                let (_, offsets) = packer.vertical(ctx, imposed_width)?;
                let width = imposed_width.or(packer.width);
                packer.place_children(ctx, offset, &offsets, width, placements)?;
            }
            Self::DrawingArea(area) => {
                let scale = ctx.points_to_pixels(1.0);
                placements.push(Placement::Area {
                    artists: area.artists.clone(),
                    transform: Affine2D::scaling(scale, scale).translate(offset.x, offset.y),
                });
            }
            Self::TextArea(area) => {
                let shift = area.extent(ctx)?.baseline_shift;
                placements.push(Placement::Text {
                    text: area.text,
                    anchor: Point::new(offset.x, offset.y + shift),
                });
            }
        }
        Ok(())
    }

    /// Whether the box is a packer in [`PackMode::Expand`], which takes the
    /// width of a vertical packer containing it.
    fn expands(&self) -> bool {
        matches!(self, Self::HPacker(p) | Self::VPacker(p) if p.mode == PackMode::Expand)
    }
}

impl<A: Clone> Packer<A> {
    fn visible_children(&self) -> impl Iterator<Item = &OffsetBox<A>> {
        self.children.iter().filter(|c| c.is_visible())
    }

    /// Places the visible children at `offsets` from the packer's origin
    /// at `origin`; expanding children take `width` when it is set.
    fn place_children(
        &self,
        ctx: LayoutContext<'_>,
        origin: Point,
        offsets: &[Point],
        width: Option<f64>,
        placements: &mut Vec<Placement<A>>,
    ) -> Result<(), OffsetBoxError> {
        for (child, &offset) in self.visible_children().zip(offsets) {
            let imposed = width.filter(|_| child.expands());
            child.place_into(ctx, origin + offset, imposed, placements)?;
        }
        Ok(())
    }

    /// matplotlib's `HPacker._get_bbox_and_child_offsets`.
    fn horizontal(
        &self,
        ctx: LayoutContext<'_>,
        imposed_width: Option<f64>,
    ) -> Result<(Bbox, Vec<Point>), OffsetBoxError> {
        let scale = ctx.points_to_pixels(1.0);
        let (pad, sep) = (self.pad * scale, self.sep * scale);
        let bboxes = self
            .visible_children()
            .map(|child| child.bbox(ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let Some(first) = bboxes.first() else {
            return Ok((
                Bbox::from_bounds(0.0, 0.0, 0.0, 0.0).padded(pad, pad),
                Vec::new(),
            ));
        };
        let spans: Vec<(f64, f64)> = bboxes.iter().map(|b| (b.y0(), b.y1())).collect();
        let ((y0, y1), yoffsets) = aligned_offsets(&spans, self.height, self.align);
        let widths: Vec<f64> = bboxes.iter().map(Bbox::width).collect();
        let width = imposed_width.or(self.width);
        let (total, xoffsets) = packed_offsets(&widths, width, sep, self.mode);
        let x0 = first.x0();
        let offsets = xoffsets
            .iter()
            .zip(&bboxes)
            .zip(yoffsets)
            .map(|((x, bbox), y)| Point::new(x - (bbox.x0() - x0), y))
            .collect();
        Ok((
            Bbox::from_bounds(x0, y0, total, y1 - y0).padded(pad, pad),
            offsets,
        ))
    }

    /// matplotlib's `VPacker._get_bbox_and_child_offsets`; a set width is
    /// passed on to expanding children.
    fn vertical(
        &self,
        ctx: LayoutContext<'_>,
        imposed_width: Option<f64>,
    ) -> Result<(Bbox, Vec<Point>), OffsetBoxError> {
        let scale = ctx.points_to_pixels(1.0);
        let (pad, sep) = (self.pad * scale, self.sep * scale);
        let width = imposed_width.or(self.width);
        let bboxes = self
            .visible_children()
            .map(|child| child.bbox_sized(ctx, width.filter(|_| child.expands())))
            .collect::<Result<Vec<_>, _>>()?;
        if bboxes.is_empty() {
            return Ok((
                Bbox::from_bounds(0.0, 0.0, 0.0, 0.0).padded(pad, pad),
                Vec::new(),
            ));
        }
        let spans: Vec<(f64, f64)> = bboxes.iter().map(|b| (b.x0(), b.x1())).collect();
        let ((x0, x1), xoffsets) = aligned_offsets(&spans, width, self.align);
        let heights: Vec<f64> = bboxes.iter().map(Bbox::height).collect();
        let (height, packed) = packed_offsets(&heights, self.height, sep, self.mode);
        let yoffsets: Vec<f64> = packed
            .iter()
            .zip(&bboxes)
            .map(|(y, bbox)| height - (y + bbox.y1()))
            .collect();
        let ydescent = yoffsets[0];
        let offsets = xoffsets
            .into_iter()
            .zip(yoffsets)
            .map(|(x, y)| Point::new(x, y - ydescent))
            .collect();
        Ok((
            Bbox::from_bounds(x0, -ydescent, x1 - x0, height).padded(pad, pad),
            offsets,
        ))
    }
}

impl<A> DrawingArea<A> {
    /// A visible area of the given size in points, with no artists yet.
    pub fn new(width: f64, height: f64, xdescent: f64, ydescent: f64) -> Self {
        Self {
            width,
            height,
            xdescent,
            ydescent,
            artists: Vec::new(),
            visible: true,
        }
    }

    /// matplotlib's `DrawingArea.get_bbox`.
    fn bbox(&self, ctx: LayoutContext<'_>) -> Bbox {
        let scale = ctx.points_to_pixels(1.0);
        Bbox::from_bounds(
            -self.xdescent * scale,
            -self.ydescent * scale,
            self.width * scale,
            self.height * scale,
        )
    }
}

impl TextArea {
    /// A visible area for the text artist `text`.
    pub fn new(text: ArtistId, multilinebaseline: bool) -> Self {
        Self {
            text,
            multilinebaseline,
            visible: true,
        }
    }

    /// matplotlib's `TextArea.get_bbox`: the text's extent, at least as tall
    /// as `"lp"` above the baseline, with multi-line text optionally moved
    /// to centre it on the baseline.
    fn extent(&self, ctx: LayoutContext<'_>) -> Result<TextExtent, OffsetBoxError> {
        let text = ctx
            .store()
            .get_as::<TextArtist>(self.text)
            .ok_or(OffsetBoxError::MissingText(self.text))?;
        let reference = measure_text("lp", &text.font, false, ctx.dpi(), &text.rc)?;
        let (h_ref, d_ref) = (reference.height, reference.descent);
        let laid_out = text.layout(ctx)?;
        let (width, mut height) = (laid_out.block.bbox.width(), laid_out.block.bbox.height());
        let mut descent = laid_out.block.lines.last().map_or(0.0, |line| line.descent);
        let mut baseline_shift = 0.0;
        if laid_out.block.lines.len() > 1 && self.multilinebaseline {
            let centred = 0.5 * height - 0.5 * (h_ref - d_ref);
            baseline_shift = descent - centred;
            descent = centred;
        } else {
            height = (h_ref - d_ref).max(height - descent) + descent;
        }
        let x0 = match text.halign {
            HAlign::Left => 0.0,
            HAlign::Center => -width / 2.0,
            HAlign::Right => -width,
        };
        Ok(TextExtent {
            bbox: Bbox::from_bounds(x0, -descent, width, height),
            baseline_shift,
        })
    }
}
