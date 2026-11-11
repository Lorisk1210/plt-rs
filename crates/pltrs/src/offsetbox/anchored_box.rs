//! Offset boxes anchored at one of nine positions of their axes or figure
//! (matplotlib's `AnchoredOffsetbox` and `AnchoredText`).

use std::rc::Rc;

use pltrs_core::{Bbox, Point};

use super::anchored_node::{AnchoredLayout, Anchoring, Framed, pad_font_points};
use super::owners::BoxOwner;
use super::pending::{Parts, adopt_box, frame_patch};
use super::{AnchorLoc, BboxToAnchor, BoxItem, OffsetBox, OffsetBoxError, TextArea, TreeLayout};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ArtistLookup, DrawContext, KeptArtist,
    LayoutContext, RawHandle, TypedHandle, downcast_ref,
};
use crate::axes::Axes;
use crate::events::Dragger;
use crate::figure::Figure;
use crate::fonts::FontProperties;
use crate::patches::{BoxShape, BoxStyle, FancyBboxPatch, Patch, PatchArtist, PatchProps};
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
    /// The frame the program made, if any (see [`AnchoredBox::with_frame`]).
    frame: Option<BoxItem>,
}

/// The parts of an [`AnchoredBox`] joining a figure.
pub(super) struct AnchoredParts {
    /// The child tree.
    pub(super) child: OffsetBox<BoxItem>,
    /// The frame: the program's, or a new square one shown when the
    /// options say `frameon`.
    pub(super) frame: BoxItem,
    /// The settings.
    pub(super) options: AnchoredOptions,
    /// Whether the paddings follow the font of the child text.
    pub(super) follows_text: bool,
}

/// The frame an anchored box makes itself: a snapped square fancy box
/// without padding, in the `pltrs.offsetbox.*` colours, shown when
/// `frameon` (matplotlib's `AnchoredOffsetbox.patch`).
fn default_frame(frameon: bool) -> BoxItem {
    frame_patch(
        BoxStyle::new(BoxShape::Square).with_pad(0.0),
        frameon,
        PatchProps::default(),
    )
}

impl AnchoredBox {
    /// The parts of the box, its frame made if the program gave none.
    pub(super) fn into_parts(self) -> AnchoredParts {
        let frameon = self.options.frameon;
        AnchoredParts {
            child: self.child,
            frame: self.frame.unwrap_or_else(|| default_frame(frameon)),
            options: self.options,
            follows_text: self.follows_text,
        }
    }

    /// A loose frame as an anchored box makes its own, shown when
    /// `frameon`: a snapped square fancy box without padding, in the
    /// `pltrs.offsetbox.*` colours (matplotlib's `AnchoredOffsetbox.patch`,
    /// made with the box), for a program to style before it frames a box
    /// with it ([`AnchoredBox::with_frame`]).
    pub fn new_frame(frameon: bool) -> Patch<FancyBboxPatch> {
        Patch::from_raw(RawHandle::loose(default_frame(frameon).artist))
    }
}

impl AnchoredBox {
    /// `child` anchored as `options` say (matplotlib's
    /// `AnchoredOffsetbox(loc, child=child, ...)`).
    pub fn new(child: impl Into<OffsetBox<BoxItem>>, options: AnchoredOptions) -> Self {
        Self {
            child: child.into(),
            options,
            follows_text: false,
            frame: None,
        }
    }

    /// The same box framed by `frame`, a fancy box patch made before the
    /// box (matplotlib's `AnchoredOffsetbox.patch`, which a program styles
    /// before adding the box), in place of a new square frame: the frame
    /// keeps its style and visibility (`frameon` no longer decides it),
    /// and the box fits its geometry and mutation scale whenever it is
    /// laid out.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, BoxItem};
    /// use pltrs::patches::{BoxShape, BoxStyle, FancyBboxPatch, Patch, PatchProps};
    /// use pltrs::text::TextProps;
    /// use pltrs::prelude::*;
    /// let circle = BoxStyle::new(BoxShape::Circle).with_pad(0.0);
    /// let shape = FancyBboxPatch::new((0.0, 0.0), 1.0, 1.0, circle.clone());
    /// let frame = Patch::loose(shape, PatchProps::default().facecolor("y"));
    /// let options = AnchoredOptions::new(AnchorLoc::UpperLeft);
    /// let label = AnchoredBox::text("note", TextProps::default(), options)?
    ///     .with_frame(BoxItem::from_loose(frame.raw())?)?;
    /// let ax = Figure::new().subplot();
    /// let added = ax.add_anchored_box(label);
    /// assert_eq!(added.patch().shape().boxstyle, circle);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`OffsetBoxError::NotAFrame`] when `frame` is not a fancy box patch.
    pub fn with_frame(self, frame: BoxItem) -> Result<Self, OffsetBoxError> {
        if downcast_ref::<PatchArtist<FancyBboxPatch>>(frame.artist.get()).is_none() {
            return Err(OffsetBoxError::NotAFrame);
        }
        Ok(Self {
            frame: Some(frame),
            ..self
        })
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
            frame: None,
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

pltrs_core::archive_enum!(PadFont {
    Points(v0),
    Text(v0),
});

/// State of an anchored box: its frame and child tree are artists of the
/// figure owned by the box; the layout pass places them.
#[derive(Clone, Debug)]
pub(crate) struct AnchoredBoxArtist {
    base: ArtistBase,
    /// The child tree (matplotlib's `get_child()`).
    pub(crate) child: OffsetBox<ArtistId>,
    /// Where the box goes in its anchor.
    anchoring: Anchoring,
    font: PadFont,
    /// The axes or (sub)figure the box was added to.
    owner: ArtistId,
    /// The frame (matplotlib's `patch`).
    frame: ArtistId,
    /// Every part, for removal with the box.
    parts: Vec<ArtistId>,
    /// Where the child box's origin is placed instead of at the anchor,
    /// in display pixels: pinned by dragging (matplotlib's `set_offset`
    /// from `DraggableOffsetBox`), or where the `get_offset` of a box of
    /// the program's puts it ([`AnchoredOffsetbox::set_child_offset`]).
    pub(crate) pinned: Option<Point>,
    /// The callbacks dragging the box, while it is draggable.
    pub(crate) dragger: Option<Dragger>,
}

pltrs_core::archive_struct!(AnchoredBoxArtist {
    base,
    child,
    anchoring,
    font,
    owner,
    frame,
    parts,
    pinned,
    dragger
});

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

    /// Lays the box out for the current figure geometry (matplotlib's
    /// `AnchoredOffsetbox.draw`).
    pub(crate) fn layout(
        &self,
        ctx: LayoutContext<'_>,
    ) -> Result<AnchoredLayout<ArtistId>, OffsetBoxError> {
        let fontsize = ctx.measuring_points_to_pixels(self.font_size(ctx)?);
        let mut tree = TreeLayout::of(&self.child);
        let framed = Framed {
            child: &self.child,
            frame: &self.frame,
            fontsize,
            pinned: self.pinned,
        };
        let extent = self
            .anchoring
            .lay_out_into(ctx, self.owner, framed, &mut tree, 0)?;
        Ok(AnchoredLayout { extent, tree })
    }
}

impl BoxOwner for AnchoredBoxArtist {
    fn tree(&self) -> &OffsetBox<ArtistId> {
        &self.child
    }

    fn tree_mut(&mut self) -> &mut OffsetBox<ArtistId> {
        &mut self.child
    }

    fn parts_mut(&mut self) -> &mut Vec<ArtistId> {
        &mut self.parts
    }

    fn lay_out_tree(&self, ctx: LayoutContext<'_>) -> Result<TreeLayout<ArtistId>, OffsetBoxError> {
        self.layout(ctx).map(|layout| layout.tree)
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

    /// matplotlib's `OffsetBox.contains`: on an artist of the child box.
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        self.child
            .artists()
            .into_iter()
            .any(|&id| ctx.store().get(id).is_some_and(|a| a.contains(ctx, point)))
    }
}

/// A handle to a box anchored in an axes (matplotlib's
/// `AnchoredOffsetbox`, `AnchoredText` and the `axes_grid1` anchored
/// artists).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchoredOffsetbox {
    raw: RawHandle,
}

impl TypedHandle for AnchoredOffsetbox {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<AnchoredBoxArtist>()
            .then(|| Self { raw: raw.clone() })
    }
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
            .with::<AnchoredBoxArtist, _>(|anchored| anchored.anchoring.loc)
    }

    /// Moves the box to another position of its anchor.
    pub fn set_loc(&self, loc: AnchorLoc) -> &Self {
        self.raw
            .with_mut::<AnchoredBoxArtist, _>(|anchored| anchored.anchoring.loc = loc);
        self
    }

    /// Anchors the box as `options` say from now on (matplotlib reads an
    /// anchored box's settings again whenever it draws it): its location,
    /// paddings, font and `bbox_to_anchor`; the frame keeps its look, and
    /// an anchored text keeps following its text's size.
    pub fn set_options(&self, options: &AnchoredOptions) -> &Self {
        self.raw.with_mut::<AnchoredBoxArtist, _>(|anchored| {
            anchored.anchoring = Anchoring::from(options);
            if let PadFont::Points(_) = anchored.font {
                anchored.font = PadFont::Points(pad_font_points(options.prop.as_ref()));
            }
        });
        self
    }

    /// Places the child box's origin at the display point `offset`
    /// instead of where the anchor puts it, or back at the anchor
    /// (`None`): where a box of the program's overriding matplotlib's
    /// `AnchoredOffsetbox.get_offset` puts its child when it is drawn
    /// (plotnine's legends). The frame goes around the child there.
    pub fn set_child_offset(&self, offset: Option<Point>) -> &Self {
        self.raw
            .with_mut::<AnchoredBoxArtist, _>(|anchored| anchored.pinned = offset);
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
        texts_of(&self.raw, self.artist_ids())
    }

    /// The artists the box draws (the texts of text areas, the artists of
    /// drawing areas, images, the frames of padded boxes) in drawing
    /// order, frames before what they frame.
    pub fn artists(&self) -> Vec<RawHandle> {
        self.artist_ids()
            .into_iter()
            .map(|id| part(&self.raw, id))
            .collect()
    }

    fn artist_ids(&self) -> Vec<ArtistId> {
        self.raw.with::<AnchoredBoxArtist, _>(|anchored| {
            anchored.child.artists().into_iter().copied().collect()
        })
    }

    /// Clips the artists of the drawing area at `area` among those of the
    /// box (see [`OffsetBox::drawing_area_mut`]) to its bounds, or stops
    /// clipping them (matplotlib's `DrawingArea.clip_children`); the box
    /// and its figure become stale.
    ///
    /// ```
    /// use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, DrawingArea};
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let area = DrawingArea::new(10.0, 10.0, 0.0, 0.0);
    /// let options = AnchoredOptions::new(AnchorLoc::Center);
    /// let anchored = fig.subplot().add_anchored_box(AnchoredBox::new(area, options));
    /// fig.draw_without_rendering()?;
    /// assert!(anchored.set_clip_children(0, true));
    /// assert!(fig.is_stale() && !anchored.set_clip_children(1, true));
    /// # Ok::<(), pltrs::figure::DrawErrors>(())
    /// ```
    ///
    /// Returns whether the box has such an area.
    pub fn set_clip_children(&self, area: usize, clip: bool) -> bool {
        self.raw.with_mut::<AnchoredBoxArtist, _>(|anchored| {
            anchored
                .child
                .drawing_area_mut(area)
                .map(|found| found.clip = clip)
                .is_some()
        })
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
            .filter(|&id| state.artist_as::<TextArtist>(id).is_some())
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
    /// use pltrs::prelude::*;
    /// use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions};
    /// use pltrs::text::TextProps;
    /// let ax = Figure::new().subplot();
    /// let label = AnchoredBox::text("Figure 1a", TextProps::default(), AnchoredOptions::new(AnchorLoc::UpperLeft))?;
    /// let anchored = ax.add_anchored_box(label);
    /// assert_eq!(anchored.texts()[0].text(), "Figure 1a");
    /// # Ok::<(), pltrs::offsetbox::OffsetBoxError>(())
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
    /// use pltrs::prelude::*;
    /// use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions};
    /// use pltrs::text::TextProps;
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
    let AnchoredParts {
        child,
        frame,
        options,
        follows_text,
    } = spec.into_parts();
    let points = pad_font_points(options.prop.as_ref());
    let figure = owner.figure_state();
    let mut state = figure.borrow_mut();
    let mut parts = Parts::new(&mut state.store);
    let frame = parts.insert(frame);
    let child = parts.insert_tree(child, owner.id());
    let parts = parts.into_ids();
    let font = match &child {
        OffsetBox::TextArea(area) if follows_text => PadFont::Text(area.text),
        _ => PadFont::Points(points),
    };
    let artist = AnchoredBoxArtist {
        base: ArtistBase::with_zorder(ANCHORED_ZORDER),
        child,
        anchoring: Anchoring::from(&options),
        font,
        owner: owner.id(),
        frame,
        parts: parts.clone(),
        pinned: None,
        dragger: None,
    };
    let id = adopt_box(&mut state, owner.id(), KeptArtist::new(artist), &parts);
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
    use crate::testing::{RecordingRenderer, close, data_axes};

    fn draw(fig: &Figure) {
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
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
        let (fig, ax) = data_axes();
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
        let props = TextProps::default().valign(pltrs_text::VAlign::Top);
        let options = AnchoredOptions::new(AnchorLoc::Center);
        assert_eq!(
            AnchoredBox::text("t", props, options).err(),
            Some(OffsetBoxError::VerticalAlignment)
        );
    }

    #[test]
    fn drawing_area_artists_are_placed_in_points() {
        let (fig, ax) = data_axes();
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
    fn drawn_children_keep_the_outline_of_the_area_they_were_first_drawn_in() {
        use crate::artist::ClipPathSpec;
        let (fig, ax) = data_axes();
        let square = BoxItem::patch(
            Rectangle::new((0.0, 0.0), 50.0, 50.0),
            PatchProps::default(),
        );
        let mut area = DrawingArea::new(10.0, 10.0, 0.0, 0.0).with_artist(square);
        area.clip = true;
        let anchored = ax.add_anchored_box(AnchoredBox::new(area, tight(AnchorLoc::LowerLeft)));
        let outline = || {
            let id = anchored.artist_ids()[0];
            let state = fig.state();
            match &state.store.get(id).expect("the square").base().clip_path {
                Some(ClipPathSpec::Path { path, .. }) => path.extents().width(),
                other => panic!("clipped to the area: {other:?}"),
            }
        };
        let resize = |side: f64| {
            anchored.raw.with_mut::<AnchoredBoxArtist, _>(|anchored| {
                let area = anchored.child.drawing_area_mut(0).expect("the area");
                (area.width, area.height) = (side, side);
            });
        };
        // Resized before its first draw, the area clips to its size then.
        resize(30.0);
        fig.layout();
        assert!(close(outline(), 30.0));
        draw(&fig);
        // matplotlib's area gives its outline only to children without one.
        resize(60.0);
        draw(&fig);
        assert!(close(outline(), 30.0));
    }

    #[test]
    fn images_fill_their_zoomed_box() {
        let (fig, ax) = data_axes();
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
        assert_eq!(drawn, Some((pltrs_core::Point::new(80.0, 53.0), (40, 30))));
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
        let (fig, ax) = data_axes();
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
