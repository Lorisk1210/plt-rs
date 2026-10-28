//! The boxes of an offset-box tree and their layout (matplotlib's
//! `HPacker`, `VPacker` and the `OffsetBox` protocol).
//!
//! Layout follows matplotlib's `_get_bbox_and_child_offsets`: every box
//! reports its extent relative to its own origin, and a packer reports the
//! offset of each visible child's origin from its own. Pads and
//! separations are in points and drawing-area sizes too; both are scaled
//! by the canvas resolution, while explicit packer widths and heights are
//! in pixels.

use plt_core::{Bbox, Path, Point};

use super::leaves::{AuxTransformBox, DrawingArea, OffsetImage, PaddedBox, TextArea};
use super::{ArtistRef, OffsetBoxError, PackAlign, PackMode, aligned_offsets, packed_offsets};
use crate::artist::LayoutContext;
use crate::transforms::Transform;

/// A box of an offset-box tree; its leaves refer to artists through `A`.
#[derive(Clone, Debug, PartialEq)]
pub enum OffsetBox<A> {
    /// Children side by side, left to right (matplotlib's `HPacker`).
    HPacker(Packer<A>),
    /// Children stacked top to bottom (matplotlib's `VPacker`).
    VPacker(Packer<A>),
    /// A fixed-size area for artists (matplotlib's `DrawingArea`).
    DrawingArea(DrawingArea<A>),
    /// A single text (matplotlib's `TextArea`).
    TextArea(TextArea<A>),
    /// Artists in their own coordinates (matplotlib's `AuxTransformBox`).
    AuxTransformBox(AuxTransformBox<A>),
    /// An image (matplotlib's `OffsetImage`).
    Image(OffsetImage<A>),
    /// A child with padding and a frame (matplotlib's `PaddedBox`).
    PaddedBox(PaddedBox<A>),
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

    fn map<B>(self, f: &mut impl FnMut(A) -> B) -> Packer<B> {
        Packer {
            pad: self.pad,
            sep: self.sep,
            width: self.width,
            height: self.height,
            align: self.align,
            mode: self.mode,
            children: self.children.into_iter().map(|c| c.map(f)).collect(),
            visible: self.visible,
        }
    }
}

/// Where the artists of a leaf go once the tree is laid out.
#[derive(Clone, Debug, PartialEq)]
pub enum Placement<A> {
    /// A text to be drawn with its anchor (the start of its last baseline)
    /// at `anchor`, in display pixels.
    Text {
        /// The text artist.
        text: A,
        /// The anchor in display pixels.
        anchor: Point,
    },
    /// Artists of a drawing area or an aux-transform box, whose
    /// coordinates map to display pixels through `transform`.
    Artists {
        /// The artists.
        artists: Vec<A>,
        /// Their coordinates to display pixels.
        transform: Transform,
        /// A clip outline in the artists' coordinates, if they are clipped.
        clip: Option<Path>,
    },
    /// An image drawn into `bbox`, in display pixels.
    Image {
        /// The image artist.
        image: A,
        /// Where the image goes.
        bbox: Bbox,
    },
    /// A frame outlining `bbox`, in display pixels.
    Frame {
        /// The frame patch (a `FancyBboxPatch`).
        frame: A,
        /// The framed box.
        bbox: Bbox,
        /// The new mutation scale of the frame's box style, if it changes.
        mutation_scale: Option<f64>,
    },
}

impl<A> OffsetBox<A> {
    /// Whether the box takes part in its parent's layout.
    pub fn is_visible(&self) -> bool {
        match self {
            Self::HPacker(packer) | Self::VPacker(packer) => packer.visible,
            Self::DrawingArea(area) => area.visible,
            Self::TextArea(area) => area.visible,
            Self::AuxTransformBox(aux) => aux.visible,
            Self::Image(image) => image.visible,
            Self::PaddedBox(padded) => padded.visible,
        }
    }

    /// Shows or hides the box.
    pub fn set_visible(&mut self, visible: bool) -> &mut Self {
        match self {
            Self::HPacker(packer) | Self::VPacker(packer) => packer.visible = visible,
            Self::DrawingArea(area) => area.visible = visible,
            Self::TextArea(area) => area.visible = visible,
            Self::AuxTransformBox(aux) => aux.visible = visible,
            Self::Image(image) => image.visible = visible,
            Self::PaddedBox(padded) => padded.visible = visible,
        }
        self
    }

    /// Fixes the width of a packer, in pixels (matplotlib's `set_width`);
    /// other boxes keep their size.
    pub fn set_width(&mut self, width: Option<f64>) -> &mut Self {
        if let Self::HPacker(packer) | Self::VPacker(packer) = self {
            packer.width = width;
        }
        self
    }

    /// The artists of the visible boxes in drawing order: frames before
    /// what they frame.
    pub fn artists(&self) -> Vec<&A> {
        let mut artists = Vec::new();
        self.collect_artists(&mut artists);
        artists
    }

    fn collect_artists<'a>(&'a self, artists: &mut Vec<&'a A>) {
        match self {
            Self::HPacker(packer) | Self::VPacker(packer) => {
                for child in packer.children.iter().filter(|c| c.is_visible()) {
                    child.collect_artists(artists);
                }
            }
            Self::DrawingArea(area) => artists.extend(&area.artists),
            Self::TextArea(area) => artists.push(&area.text),
            Self::AuxTransformBox(aux) => artists.extend(&aux.artists),
            Self::Image(image) => artists.push(&image.image),
            Self::PaddedBox(padded) => {
                artists.push(&padded.frame);
                if padded.child.is_visible() {
                    padded.child.collect_artists(artists);
                }
            }
        }
    }

    /// The same tree with every artist reference replaced by `f` of it,
    /// in drawing order (how a box's artists join a figure).
    pub fn map<B>(self, f: &mut impl FnMut(A) -> B) -> OffsetBox<B> {
        match self {
            Self::HPacker(packer) => OffsetBox::HPacker(packer.map(f)),
            Self::VPacker(packer) => OffsetBox::VPacker(packer.map(f)),
            Self::DrawingArea(area) => OffsetBox::DrawingArea(DrawingArea {
                width: area.width,
                height: area.height,
                xdescent: area.xdescent,
                ydescent: area.ydescent,
                clip: area.clip,
                artists: area.artists.into_iter().map(&mut *f).collect(),
                visible: area.visible,
            }),
            Self::TextArea(area) => OffsetBox::TextArea(TextArea {
                text: f(area.text),
                multilinebaseline: area.multilinebaseline,
                visible: area.visible,
            }),
            Self::AuxTransformBox(aux) => OffsetBox::AuxTransformBox(AuxTransformBox {
                aux_transform: aux.aux_transform,
                artists: aux.artists.into_iter().map(&mut *f).collect(),
                visible: aux.visible,
                extent: aux.extent,
            }),
            Self::Image(image) => OffsetBox::Image(OffsetImage {
                image: f(image.image),
                zoom: image.zoom,
                dpi_cor: image.dpi_cor,
                visible: image.visible,
            }),
            Self::PaddedBox(padded) => {
                let frame = f(padded.frame);
                OffsetBox::PaddedBox(PaddedBox {
                    pad: padded.pad,
                    child: Box::new(padded.child.map(f)),
                    frame,
                    visible: padded.visible,
                })
            }
        }
    }

    /// Calls `f` on every aux-transform box of the tree.
    pub(crate) fn for_each_aux_mut(&mut self, f: &mut impl FnMut(&mut AuxTransformBox<A>)) {
        match self {
            Self::HPacker(packer) | Self::VPacker(packer) => {
                for child in &mut packer.children {
                    child.for_each_aux_mut(f);
                }
            }
            Self::AuxTransformBox(aux) => f(aux),
            Self::PaddedBox(padded) => padded.child.for_each_aux_mut(f),
            Self::DrawingArea(_) | Self::TextArea(_) | Self::Image(_) => {}
        }
    }

    /// Whether the box is a packer in [`PackMode::Expand`], which takes the
    /// width of a vertical packer containing it.
    fn expands(&self) -> bool {
        matches!(self, Self::HPacker(p) | Self::VPacker(p) if p.mode == PackMode::Expand)
    }
}

impl<A: ArtistRef> OffsetBox<A> {
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
            Self::AuxTransformBox(aux) => Ok(aux.bbox()),
            Self::Image(image) => image.bbox(ctx),
            Self::PaddedBox(padded) => {
                let pad = padded.pad * ctx.points_to_pixels(1.0);
                Ok(padded.child.bbox(ctx)?.padded(pad, pad))
            }
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
            Self::DrawingArea(area) => placements.push(area.placement(ctx, offset)),
            Self::TextArea(area) => {
                let shift = area.extent(ctx)?.baseline_shift;
                placements.push(Placement::Text {
                    text: area.text.clone(),
                    anchor: Point::new(offset.x, offset.y + shift),
                });
            }
            Self::AuxTransformBox(aux) => placements.push(aux.placement(offset)),
            Self::Image(image) => placements.push(Placement::Image {
                image: image.image.clone(),
                bbox: image.bbox(ctx)?.translated(offset.x, offset.y),
            }),
            Self::PaddedBox(padded) => {
                placements.push(Placement::Frame {
                    frame: padded.frame.clone(),
                    bbox: self.window_extent(ctx, offset)?,
                    mutation_scale: None,
                });
                if padded.child.is_visible() {
                    padded.child.place_into(ctx, offset, None, placements)?;
                }
            }
        }
        Ok(())
    }
}

impl<A: ArtistRef> Packer<A> {
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

macro_rules! box_from {
    ($($leaf:ident => $variant:ident),* $(,)?) => {
        $(
            impl<A> From<$leaf<A>> for OffsetBox<A> {
                fn from(leaf: $leaf<A>) -> Self {
                    Self::$variant(leaf)
                }
            }
        )*
    };
}

box_from! {
    DrawingArea => DrawingArea,
    TextArea => TextArea,
    AuxTransformBox => AuxTransformBox,
    OffsetImage => Image,
    PaddedBox => PaddedBox,
}
