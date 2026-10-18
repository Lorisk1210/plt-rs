//! Offset boxes anchored at one of nine positions of their axes or figure
//! (matplotlib's `AnchoredOffsetbox` and `AnchoredText`).

use std::rc::Rc;

use plt_core::Bbox;

use super::pending::{Parts, adopt_box, frame_patch};
use super::{
    AnchorLoc, BboxToAnchor, BoxItem, OffsetBox, OffsetBoxError, Placement, TextArea,
    anchored_bbox, transformed_bbox,
};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, LayoutContext, RawHandle,
};
use crate::axes::Axes;
use crate::figure::Figure;
use crate::fonts::FontProperties;
use crate::patches::{BoxShape, BoxStyle, FancyBboxPatch, Patch, PatchProps};
use crate::text::{Text, TextArtist, TextProps};
use crate::transforms::Transform;

/// The z-order of anchored boxes (matplotlib's `AnchoredOffsetbox.zorder`,
/// that of legends).
const ANCHORED_ZORDER: f64 = 5.0;

/// The settings of an anchored box (matplotlib's `AnchoredOffsetbox`
/// keyword arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct AnchoredOptions {
    /// Where the box goes within `bbox_to_anchor`.
    pub loc: AnchorLoc,
    /// Padding around the child, as a fraction of the font size.
    pub pad: f64,
    /// Padding between the frame and `bbox_to_anchor`, horizontally and
    /// vertically, as fractions of the font size.
    pub borderpad: (f64, f64),
    /// The font whose size the paddings refer to (`None` =
    /// `legend.fontsize`).
    pub prop: Option<FontProperties>,
    /// Whether the frame is drawn.
    pub frameon: bool,
    /// The box the anchored box is placed in (`None` = its axes, or the
    /// whole figure for a box added to a figure).
    pub bbox_to_anchor: Option<BboxToAnchor>,
    /// The coordinates of `bbox_to_anchor` (`None` = display pixels).
    pub bbox_transform: Option<Transform>,
}

impl AnchoredOptions {
    /// matplotlib's defaults for a box at `loc`: a framed box padded by 0.4
    /// and 0.5 font sizes, anchored in its axes.
    pub fn new(loc: AnchorLoc) -> Self {
        Self {
            loc,
            pad: 0.4,
            borderpad: (0.5, 0.5),
            prop: None,
            frameon: true,
            bbox_to_anchor: None,
            bbox_transform: None,
        }
    }
}

/// An anchored box before it joins an axes: its child tree and settings
/// (matplotlib's `AnchoredOffsetbox` before `add_artist`).
#[derive(Debug)]
pub struct AnchoredBox {
    child: OffsetBox<BoxItem>,
    options: AnchoredOptions,
    /// Whether the paddings follow the font of the child text (anchored
    /// texts) rather than `options.prop`.
    follows_text: bool,
}

impl AnchoredBox {
    /// `child` anchored as `options` say (matplotlib's
    /// `AnchoredOffsetbox(loc, child=child, ...)`).
    pub fn new(child: impl Into<OffsetBox<BoxItem>>, options: AnchoredOptions) -> Self {
        Self {
            child: child.into(),
            options,
            follows_text: false,
        }
    }

    /// A text styled from `props`, anchored as `options` say, whose
    /// paddings follow the text's font size rather than `options.prop`
    /// (matplotlib's `AnchoredText`).
    ///
    /// # Errors
    ///
    /// [`OffsetBoxError::VerticalAlignment`] when `props` set a vertical
    /// alignment.
    pub fn text(
        text: impl Into<String>,
        props: TextProps,
        options: AnchoredOptions,
    ) -> Result<Self, OffsetBoxError> {
        if props.valign.is_some() {
            return Err(OffsetBoxError::VerticalAlignment);
        }
        Ok(Self {
            child: TextArea::from_text(text, props).into(),
            options,
            follows_text: true,
        })
    }
}

/// What the paddings of an anchored box are fractions of.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PadFont {
    /// A fixed size in points.
    Points(f64),
    /// The current size of a text of the box (anchored texts).
    Text(ArtistId),
}

/// Where an owner box and its parts go.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BoxLayout {
    /// The framed box, in display pixels.
    pub(crate) extent: Bbox,
    /// Where the frame, the texts, the artists and the images go.
    pub(crate) placements: Vec<Placement<ArtistId>>,
}

/// State of an anchored box: its frame and child tree are artists of the
/// figure owned by the box; the layout pass places them.
#[derive(Debug)]
pub(crate) struct AnchoredBoxArtist {
    base: ArtistBase,
    /// The child tree (matplotlib's `get_child()`).
    pub(crate) child: OffsetBox<ArtistId>,
    loc: AnchorLoc,
    pad: f64,
    borderpad: (f64, f64),
    font: PadFont,
    /// `bbox_to_anchor` with its coordinates; `None` anchors in `owner`.
    anchor: Option<(Bbox, Transform)>,
    /// The axes or (sub)figure the box was added to.
    owner: ArtistId,
    /// The frame (matplotlib's `patch`).
    frame: ArtistId,
    /// Every part, for removal with the box.
    parts: Vec<ArtistId>,
}

impl AnchoredBoxArtist {
    /// The font size the paddings are fractions of, in points.
    fn font_size(&self, ctx: LayoutContext<'_>) -> Result<f64, OffsetBoxError> {
        match self.font {
            PadFont::Points(points) => Ok(points),
            PadFont::Text(id) => ctx
                .store()
                .get_as::<TextArtist>(id)
                .map(|text| text.font.size)
                .ok_or(OffsetBoxError::MissingText(id)),
        }
    }

    /// The display box the box is anchored in (matplotlib's
    /// `get_bbox_to_anchor`): `bbox_to_anchor`, else the box of its axes or
    /// figure (where matplotlib requires `bbox_to_anchor`).
    fn anchor_box(&self, ctx: LayoutContext<'_>) -> Option<Bbox> {
        match &self.anchor {
            None => ctx
                .axes_extent(self.owner)
                .or_else(|| ctx.frame_extent(self.owner)),
            Some((bbox, transform)) => Some(transformed_bbox(bbox, &ctx.resolve(transform)?)),
        }
    }

    /// Lays the box out for the current figure geometry (matplotlib's
    /// `AnchoredOffsetbox.draw`).
    pub(crate) fn layout(&self, ctx: LayoutContext<'_>) -> Result<BoxLayout, OffsetBoxError> {
        let fontsize = ctx.points_to_pixels(self.font_size(ctx)?);
        let pad = self.pad * fontsize;
        let bbox = self.child.bbox(ctx)?.padded(pad, pad);
        let anchor = self.anchor_box(ctx).ok_or(OffsetBoxError::Detached)?;
        let corner = anchored_bbox(
            self.loc,
            &Bbox::from_bounds(0.0, 0.0, bbox.width(), bbox.height()),
            &anchor,
            self.borderpad.0 * fontsize,
            self.borderpad.1 * fontsize,
        );
        let offset = corner - bbox.p0;
        let extent = bbox.translated(offset.x, offset.y);
        let mut placements = vec![Placement::Frame {
            frame: self.frame,
            bbox: extent,
            mutation_scale: Some(fontsize),
        }];
        placements.extend(self.child.place(ctx, offset)?);
        Ok(BoxLayout { extent, placements })
    }
}

impl Artist for AnchoredBoxArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    /// Draws the frame, then the child's artists in box order.
    fn draw(&self, ctx: &mut DrawContext<'_>) {
        ctx.draw_child(self.frame);
        for &id in self.child.artists() {
            ctx.draw_child(id);
        }
    }

    fn children(&self) -> Vec<ArtistId> {
        self.parts.clone()
    }

    fn remove_child(&mut self, child: ArtistId) {
        self.parts.retain(|&id| id != child);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.layout(*ctx).ok().map(|layout| layout.extent)
    }
}

/// A handle to a box anchored in an axes (matplotlib's
/// `AnchoredOffsetbox`, `AnchoredText` and the `axes_grid1` anchored
/// artists).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchoredOffsetbox {
    raw: RawHandle,
}

impl ArtistHandle for AnchoredOffsetbox {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl AnchoredOffsetbox {
    /// Where the box goes within its anchor.
    pub fn loc(&self) -> AnchorLoc {
        self.raw
            .with::<AnchoredBoxArtist, _>(|anchored| anchored.loc)
    }

    /// Moves the box to another position of its anchor.
    pub fn set_loc(&self, loc: AnchorLoc) -> &Self {
        self.raw
            .with_mut::<AnchoredBoxArtist, _>(|anchored| anchored.loc = loc);
        self
    }

    /// The frame (matplotlib's `patch`), whose box style and colours can
    /// be changed.
    pub fn patch(&self) -> Patch<FancyBboxPatch> {
        let frame = self
            .raw
            .with::<AnchoredBoxArtist, _>(|anchored| anchored.frame);
        Patch::from_raw(part(&self.raw, frame))
    }

    /// The texts of the box's text areas, in box order.
    pub fn texts(&self) -> Vec<Text> {
        let artists = self.raw.with::<AnchoredBoxArtist, _>(|anchored| {
            anchored.child.artists().into_iter().copied().collect()
        });
        texts_of(&self.raw, artists)
    }
}

/// A handle to the part `id` of the box behind `raw`.
pub(super) fn part(raw: &RawHandle, id: ArtistId) -> RawHandle {
    RawHandle::new(Rc::clone(raw.figure_state()), id)
}

/// Handles to those of `artists` that are texts.
pub(super) fn texts_of(raw: &RawHandle, artists: Vec<ArtistId>) -> Vec<Text> {
    let texts: Vec<ArtistId> = {
        let state = raw.figure_state().borrow();
        artists
            .into_iter()
            .filter(|&id| state.store.get_as::<TextArtist>(id).is_some())
            .collect()
    };
    texts
        .into_iter()
        .map(|id| Text::from_raw(part(raw, id)))
        .collect()
}

impl Axes {
    /// Adds a box anchored in the axes (matplotlib's `add_artist` of an
    /// `AnchoredOffsetbox`, an `AnchoredText` or an `axes_grid1` anchored
    /// artist). The box is placed at every draw.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions};
    /// use plt::text::TextProps;
    /// let ax = Figure::new().subplot();
    /// let label = AnchoredBox::text("Figure 1a", TextProps::default(), AnchoredOptions::new(AnchorLoc::UpperLeft))?;
    /// let anchored = ax.add_anchored_box(label);
    /// assert_eq!(anchored.texts()[0].text(), "Figure 1a");
    /// # Ok::<(), plt::offsetbox::OffsetBoxError>(())
    /// ```
    pub fn add_anchored_box(&self, spec: impl Into<AnchoredBox>) -> AnchoredOffsetbox {
        add_anchored(self.raw(), spec.into())
    }
}

impl Figure {
    /// Adds a box anchored in the figure (matplotlib's `fig.add_artist` of
    /// an `AnchoredOffsetbox` or `AnchoredText`): at `bbox_to_anchor`, for
    /// example a point in figure fractions
    /// ([`Transform::Figure`]), or
    /// else in the whole (sub)figure. The box is placed at every draw.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions};
    /// use plt::text::TextProps;
    /// let fig = Figure::new();
    /// let note = AnchoredBox::text("draft", TextProps::default(), AnchoredOptions::new(AnchorLoc::LowerRight))?;
    /// fig.add_anchored_box(note);
    /// fig.to_png()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn add_anchored_box(&self, spec: impl Into<AnchoredBox>) -> AnchoredOffsetbox {
        add_anchored(self.raw(), spec.into())
    }
}

/// Adds the box `spec` to `owner`, an axes or a (sub)figure.
fn add_anchored(owner: &RawHandle, spec: AnchoredBox) -> AnchoredOffsetbox {
    let AnchoredBox {
        child,
        options,
        follows_text,
    } = spec;
    let rc = crate::rc::params();
    let points = options.prop.as_ref().map_or_else(
        || rc.legend_fontsize.to_points(rc.font_size),
        |prop| prop.size,
    );
    let square = BoxStyle::new(BoxShape::Square).with_pad(0.0);
    let figure = owner.figure_state();
    let mut state = figure.borrow_mut();
    let mut parts = Parts::new(&mut state.store);
    let frame = parts.insert(frame_patch(square, options.frameon, PatchProps::default()));
    let child = parts.insert_tree(child);
    let parts = parts.into_ids();
    let font = match &child {
        OffsetBox::TextArea(area) if follows_text => PadFont::Text(area.text),
        _ => PadFont::Points(points),
    };
    let anchor = options.bbox_to_anchor.map(|anchor| {
        let transform = options.bbox_transform.clone().unwrap_or(Transform::Display);
        (anchor.bbox(), transform)
    });
    let artist = AnchoredBoxArtist {
        base: ArtistBase::with_zorder(ANCHORED_ZORDER),
        child,
        loc: options.loc,
        pad: options.pad,
        borderpad: options.borderpad,
        font,
        anchor,
        owner: owner.id(),
        frame,
        parts: parts.clone(),
    };
    let id = adopt_box(&mut state, owner.id(), Box::new(artist), &parts);
    drop(state);
    AnchoredOffsetbox {
        raw: RawHandle::new(Rc::clone(figure), id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::image::BboxImageOptions;
    use crate::offsetbox::{AuxTransformBox, DrawingArea, OffsetImage};
    use crate::patches::{PatchShape, Rectangle};
    use crate::testing::RecordingRenderer;

    /// A 640 × 480 px figure with one axes over `(0, 10)²` at
    /// `(80, 52.8)`, 496 × 369.6 px.
    fn axes() -> (Figure, Axes) {
        let fig = Figure::new();
        let ax = fig.add_axes([0.125, 0.11, 0.775, 0.77]);
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        (fig, ax)
    }

    fn draw(fig: &Figure) {
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// The display extent of the first artist of the box's child.
    fn first_artist_extent<S: PatchShape>(anchored: &AnchoredOffsetbox) -> Bbox {
        let id = anchored
            .raw()
            .with::<AnchoredBoxArtist, _>(|a| *a.child.artists()[0]);
        Patch::<S>::from_raw(part(anchored.raw(), id))
            .window_extent()
            .expect("patches have an extent")
    }

    /// Paddings and borders of zero, in the lower-left corner.
    fn tight(loc: AnchorLoc) -> AnchoredOptions {
        AnchoredOptions {
            pad: 0.0,
            borderpad: (0.0, 0.0),
            ..AnchoredOptions::new(loc)
        }
    }

    #[test]
    fn texts_sit_inside_their_frame_in_the_corner() {
        let (fig, ax) = axes();
        let options = AnchoredOptions::new(AnchorLoc::UpperLeft);
        let label = AnchoredBox::text("label", TextProps::default(), options).expect("valid");
        let anchored = ax.add_anchored_box(label);
        draw(&fig);
        // legend.fontsize is 10 pt (13.9 px); the border pad is half of it.
        let pad = 0.5 * 10.0 * 100.0 / 72.0;
        let frame = anchored.patch().shape();
        assert!(close(frame.xy.x, 80.0 + pad), "{frame:?}");
        assert!(close(frame.xy.y + frame.height, 52.8 + 369.6 - pad));
        assert!(close(frame.mutation_scale, 2.0 * pad));
        let window = anchored.window_extent().expect("an extent");
        assert!(close(window.x0(), frame.xy.x) && close(window.width(), frame.width));
        assert_eq!(anchored.texts()[0].text(), "label");
        assert_eq!(anchored.loc(), AnchorLoc::UpperLeft);
        anchored.set_loc(AnchorLoc::LowerRight);
        draw(&fig);
        let moved = anchored.patch().shape();
        assert!(close(moved.xy.x + moved.width, 80.0 + 496.0 - pad));
    }

    #[test]
    fn anchored_texts_refuse_vertical_alignments() {
        let props = TextProps::default().valign(plt_text::VAlign::Top);
        let options = AnchoredOptions::new(AnchorLoc::Center);
        assert_eq!(
            AnchoredBox::text("t", props, options).err(),
            Some(OffsetBoxError::VerticalAlignment)
        );
    }

    #[test]
    fn drawing_area_artists_are_placed_in_points() {
        let (fig, ax) = axes();
        let square = BoxItem::patch(
            Rectangle::new((0.0, 0.0), 10.0, 10.0),
            PatchProps::default(),
        );
        let area = DrawingArea::new(10.0, 10.0, 0.0, 0.0).with_artist(square);
        let options = AnchoredOptions {
            frameon: false,
            ..tight(AnchorLoc::LowerLeft)
        };
        let anchored = ax.add_anchored_box(AnchoredBox::new(area, options));
        draw(&fig);
        let placed = first_artist_extent::<Rectangle>(&anchored);
        let side = 10.0 * 100.0 / 72.0;
        assert!(
            close(placed.x0(), 80.0) && close(placed.y0(), 52.8),
            "{placed:?}"
        );
        assert!(close(placed.width(), side) && close(placed.height(), side));
        assert!(!anchored.patch().visible(), "frameon=False hides the frame");
    }

    #[test]
    fn images_fill_their_zoomed_box() {
        let (fig, ax) = axes();
        let data = crate::numeric::Grid::from_fn(3, 4, |r, c| (r * 4 + c) as f64);
        // 7.2 points are 10 pixels at 100 dpi.
        let image = OffsetImage::new(data, 7.2, BboxImageOptions::default()).expect("valid");
        ax.add_anchored_box(AnchoredBox::new(image, tight(AnchorLoc::LowerLeft)));
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        let drawn = renderer.calls.iter().find_map(|call| match call {
            crate::testing::Call::Image { origin, size } => Some((*origin, *size)),
            _ => None,
        });
        assert_eq!(drawn, Some((plt_core::Point::new(80.0, 53.0), (40, 30))));
        let bad = BboxImageOptions {
            filterrad: Some(-1.0),
            ..BboxImageOptions::default()
        };
        let grid = crate::numeric::Grid::filled(1, 1, 0.0);
        assert_eq!(
            OffsetImage::new(grid, 1.0, bad).err(),
            Some(crate::image::ImageError::FilterRadius(-1.0))
        );
    }

    #[test]
    fn aux_boxes_are_as_large_as_their_artists() {
        let (fig, ax) = axes();
        let bar = BoxItem::patch(Rectangle::new((3.0, 4.0), 2.0, 1.0), PatchProps::default());
        let aux = AuxTransformBox::new(ax.trans_data()).with_artist(bar);
        let anchored = ax.add_anchored_box(AnchoredBox::new(aux, tight(AnchorLoc::UpperRight)));
        draw(&fig);
        let frame = anchored.patch().shape();
        assert!(
            close(frame.width, 2.0 * 49.6) && close(frame.height, 36.96),
            "{frame:?}"
        );
        assert!(close(frame.xy.x + frame.width, 576.0));
        let bar = first_artist_extent::<Rectangle>(&anchored);
        assert!(
            close(bar.x0(), frame.xy.x) && close(bar.y0(), frame.xy.y),
            "{bar:?}"
        );
    }
}
