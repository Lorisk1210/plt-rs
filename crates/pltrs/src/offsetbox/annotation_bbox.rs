//! Offset boxes placed at a point, optionally with an arrow to the point
//! they annotate (matplotlib's `AnnotationBbox`).

use std::rc::Rc;

use pltrs_core::{Bbox, Point};

use super::anchored_box::{part, texts_of};
use super::pending::{Parts, adopt_box, frame_patch};
use super::{BoxItem, OffsetBox, OffsetBoxError, Placement};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, LayoutContext, RawHandle, TypedHandle,
};
use crate::axes::Axes;
use crate::coords::{CoordError, CoordFrame, Coords, annotation_shown};
use crate::events::Dragger;
use crate::figure::Figure;
use crate::fonts::FontSize;
use crate::patches::{
    BoxShape, BoxStyle, ClipRegion, DisplayPath, FancyBboxPatch, Patch, PatchArtist, PatchProps,
    PatchShape,
};
use crate::rc::RcParams;
use crate::text::{AnnotationArrow, ArrowProps, ArrowTarget, FancyArrowProps, Text};

/// The z-order of annotation boxes (matplotlib's `AnnotationBbox.zorder`).
const ANNOTATION_BBOX_ZORDER: f64 = 3.0;

/// The options of an annotation box (matplotlib's `AnnotationBbox`
/// keyword arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationBboxOptions {
    /// Where the box goes; `None` puts it at the annotated point.
    pub xybox: Option<Point>,
    /// The coordinates of the annotated point (not an offset).
    pub xycoords: Coords,
    /// The coordinates of `xybox`; `None` uses `xycoords`.
    pub boxcoords: Option<Coords>,
    /// Whether the frame is drawn.
    pub frameon: bool,
    /// Padding of the frame's square box style, as a fraction of
    /// `fontsize`.
    pub pad: f64,
    /// Whether to hide the box when the point leaves the axes: always
    /// (`Some(true)`), never (`Some(false)`), or for points in data
    /// coordinates (`None`).
    pub annotation_clip: Option<bool>,
    /// Which point of the box goes at `xybox`, as fractions of its width
    /// and height from the lower-left corner.
    pub box_alignment: (f64, f64),
    /// A box style replacing the padded square (matplotlib's `boxstyle` in
    /// `bboxprops`, which brings its own padding).
    pub boxstyle: Option<BoxStyle>,
    /// Colours, widths... of the frame.
    pub bboxprops: PatchProps,
    /// The arrow from the box to the annotated point, if any.
    pub arrowprops: Option<FancyArrowProps>,
    /// The size the frame's box style and the arrow are scaled by (`None`
    /// = `legend.fontsize`).
    pub fontsize: Option<FontSize>,
}

impl Default for AnnotationBboxOptions {
    /// matplotlib's defaults: a framed box centred on the annotated point.
    fn default() -> Self {
        Self {
            xybox: None,
            xycoords: Coords::Data,
            boxcoords: None,
            frameon: true,
            pad: 0.4,
            annotation_clip: None,
            box_alignment: (0.5, 0.5),
            boxstyle: None,
            bboxprops: PatchProps::default(),
            arrowprops: None,
            fontsize: None,
        }
    }
}

/// Where the annotated point and the box are, in display pixels.
struct Geometry {
    point: Point,
    /// Where `xybox` is.
    anchor: Point,
    offset: Point,
    extent: Bbox,
}

/// State of an annotation box: its frame and child tree are artists of the
/// figure owned by the box; the layout pass places them.
#[derive(Debug)]
pub(crate) struct AnnotationBboxArtist {
    base: ArtistBase,
    /// The child tree (matplotlib's `offsetbox`).
    pub(crate) child: OffsetBox<ArtistId>,
    xy: Point,
    xybox: Point,
    xycoords: Coords,
    boxcoords: Coords,
    box_alignment: (f64, f64),
    annotation_clip: Option<bool>,
    /// The frame's mutation scale in points.
    fontsize: f64,
    arrow: Option<AnnotationArrow>,
    /// The axes of data coordinates and of the clipping check; `None` for
    /// a box of a (sub)figure.
    axes: Option<ArtistId>,
    /// The frame (matplotlib's `patch`).
    frame: ArtistId,
    /// Every part, for removal with the box.
    parts: Vec<ArtistId>,
    rc: Rc<RcParams>,
    /// The callbacks dragging the box, while it is draggable.
    pub(crate) dragger: Option<Dragger>,
}

pltrs_core::archive_struct!(AnnotationBboxArtist {
    base,
    child,
    xy,
    xybox,
    xycoords,
    boxcoords,
    box_alignment,
    annotation_clip,
    fontsize,
    arrow,
    axes,
    frame,
    parts,
    rc,
    dragger
});

impl AnnotationBboxArtist {
    fn coord_frame<'a>(
        &self,
        layout: LayoutContext<'a>,
        offset_origin: Option<Point>,
    ) -> CoordFrame<'a> {
        CoordFrame {
            layout,
            axes: self.axes,
            owner: self.base.parent,
            offset_origin,
            fontsize: self.fontsize,
        }
    }

    /// The artists the box is placed against, through its coordinates.
    pub(crate) fn placing_artists(&self) -> Vec<ArtistId> {
        [self.xycoords.artists(), self.boxcoords.artists()].concat()
    }

    /// The display position of the box anchor (matplotlib's `xybox` in
    /// `boxcoords`).
    pub(crate) fn box_display(&self, layout: LayoutContext<'_>) -> Option<Point> {
        Some(self.geometry(layout).ok()?.anchor)
    }

    /// The `xybox`, in `boxcoords`, that puts the box anchor at display
    /// position `at`; `None` when the coordinates cannot be inverted.
    pub(crate) fn xybox_at(&self, layout: LayoutContext<'_>, at: Point) -> Option<Point> {
        let point = self.geometry(layout).ok()?.point;
        let transform = self
            .boxcoords
            .resolve(&self.coord_frame(layout, Some(point)))
            .ok()?;
        Some(transform.inverted()?.transform_point(at))
    }

    /// Moves the box anchor (matplotlib's `xybox`).
    pub(crate) fn set_xybox(&mut self, xybox: Point) {
        self.xybox = xybox;
    }

    /// The annotated point and the box (matplotlib's `update_positions`).
    fn geometry(&self, layout: LayoutContext<'_>) -> Result<Geometry, OffsetBoxError> {
        let point = self
            .xycoords
            .resolve(&self.coord_frame(layout, None))?
            .transform_point(self.xy);
        let anchor = self
            .boxcoords
            .resolve(&self.coord_frame(layout, Some(point)))?
            .transform_point(self.xybox);
        let bbox = self.child.bbox(layout)?;
        let (fw, fh) = self.box_alignment;
        let offset = Point::new(
            anchor.x - fw * bbox.width() - bbox.x0(),
            anchor.y - fh * bbox.height() - bbox.y0(),
        );
        Ok(Geometry {
            point,
            anchor,
            offset,
            extent: bbox.translated(offset.x, offset.y),
        })
    }

    /// Where the frame and the child's parts go.
    pub(crate) fn placements(
        &self,
        layout: LayoutContext<'_>,
    ) -> Result<Vec<Placement<ArtistId>>, OffsetBoxError> {
        let geometry = self.geometry(layout)?;
        let mut placements = vec![Placement::Frame {
            frame: self.frame,
            bbox: geometry.extent,
            mutation_scale: Some(layout.points_to_pixels(self.fontsize)),
        }];
        placements.extend(self.child.place(layout, geometry.offset)?);
        Ok(placements)
    }

    /// The frame as the region the arrow starts from (`patchA`): its
    /// outline, with the edge width as tolerance when the edge shows.
    fn frame_region(&self, layout: LayoutContext<'_>) -> Result<Option<ClipRegion>, String> {
        let Some(frame) = layout
            .store()
            .get_as::<PatchArtist<FancyBboxPatch>>(self.frame)
        else {
            return Ok(None);
        };
        let resolved = frame
            .style
            .resolve(&self.rc, frame.base().alpha, FancyBboxPatch::EDGE_DEFAULT)
            .map_err(|err| err.to_string())?;
        Ok(Some(ClipRegion {
            path: frame.shape.path(),
            radius: resolved.hit_radius(),
        }))
    }

    /// What the arrow is aimed at, from the box at `geometry`.
    fn arrow_target(
        &self,
        layout: LayoutContext<'_>,
        geometry: &Geometry,
    ) -> Result<ArrowTarget, String> {
        Ok(ArrowTarget {
            owner: self.base.id(),
            text_bbox: geometry.extent,
            point: geometry.point,
            fontsize: self.fontsize,
            mutation_unit: layout.points_to_pixels(1.0),
            start_region: self.frame_region(layout)?,
        })
    }

    /// The box and its arrow, if the annotated point is shown.
    fn draw_parts(&self, ctx: &mut DrawContext<'_>) -> Result<(), String> {
        let layout = *ctx.layout();
        let geometry = self.geometry(layout).map_err(|err| err.to_string())?;
        if !annotation_shown(
            layout,
            self.annotation_clip,
            &self.xycoords,
            self.axes,
            geometry.point,
        ) {
            return Ok(());
        }
        ctx.renderer()
            .open_group("AnnotationBbox", self.base.gid.as_deref());
        if let Some(arrow) = &self.arrow {
            let target = self.arrow_target(layout, &geometry)?;
            arrow
                .draw(ctx, &self.rc, target)
                .map_err(|err| err.to_string())?;
        }
        ctx.draw_child(self.frame);
        for &id in self.child.artists() {
            ctx.draw_child(id);
        }
        ctx.renderer().close_group("AnnotationBbox");
        Ok(())
    }

    /// The extent of the box, its frame and its arrow (matplotlib's
    /// `get_window_extent`).
    fn extent(&self, layout: LayoutContext<'_>) -> Option<Bbox> {
        let geometry = self.geometry(layout).ok()?;
        let mut extent = geometry.extent;
        let framed = layout
            .store()
            .get(self.frame)
            .and_then(|frame| frame.window_extent(&layout));
        if let Some(framed) = framed {
            extent = extent.union(&framed);
        }
        if let Some(arrow) = &self.arrow {
            let target = self.arrow_target(layout, &geometry).ok()?;
            let paths = arrow.measured_paths(layout, &self.rc, target).ok()?;
            let displayed: Vec<pltrs_core::Path> =
                paths.iter().map(DisplayPath::to_display).collect();
            extent = extent.union(&pltrs_core::Path::compound(&displayed).extents());
        }
        Some(extent)
    }
}

impl Artist for AnnotationBboxArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    /// Draws the arrow, the frame, then the child's artists (matplotlib's
    /// `AnnotationBbox.draw`).
    fn draw(&self, ctx: &mut DrawContext<'_>) {
        if let Err(problem) = self.draw_parts(ctx) {
            ctx.report(self.base.id(), problem);
        }
    }

    fn children(&self) -> Vec<ArtistId> {
        self.parts.clone()
    }

    fn remove_child(&mut self, child: ArtistId) {
        self.parts.retain(|&id| id != child);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.extent(*ctx)
    }

    /// matplotlib's `AnnotationBbox.contains`: on an artist of the child
    /// box while the box is shown; the arrow does not count.
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        let Ok(geometry) = self.geometry(*ctx) else {
            return false;
        };
        let shown = annotation_shown(
            *ctx,
            self.annotation_clip,
            &self.xycoords,
            self.axes,
            geometry.point,
        );
        shown
            && self
                .child
                .artists()
                .into_iter()
                .any(|&id| ctx.store().get(id).is_some_and(|a| a.contains(ctx, point)))
    }
}

/// A handle to an annotation box (matplotlib's `AnnotationBbox`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotationBbox {
    raw: RawHandle,
}

impl TypedHandle for AnnotationBbox {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<AnnotationBboxArtist>()
            .then(|| Self { raw: raw.clone() })
    }
}

impl ArtistHandle for AnnotationBbox {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl AnnotationBbox {
    /// The annotated point.
    pub fn xy(&self) -> Point {
        self.raw.with::<AnnotationBboxArtist, _>(|ab| ab.xy)
    }

    /// Moves the annotated point.
    pub fn set_xy(&self, xy: impl Into<Point>) -> &Self {
        let xy = xy.into();
        self.raw
            .with_mut::<AnnotationBboxArtist, _>(|ab| ab.xy = xy);
        self
    }

    /// Where the box goes (matplotlib's `xybox`).
    pub fn xybox(&self) -> Point {
        self.raw.with::<AnnotationBboxArtist, _>(|ab| ab.xybox)
    }

    /// The coordinates of [`xybox`](Self::xybox): those given, else those
    /// of the annotated point (matplotlib's `anncoords`).
    pub fn boxcoords(&self) -> Coords {
        self.raw
            .with::<AnnotationBboxArtist, _>(|ab| ab.boxcoords.clone())
    }

    /// Moves the box.
    pub fn set_xybox(&self, xybox: impl Into<Point>) -> &Self {
        let xybox = xybox.into();
        self.raw
            .with_mut::<AnnotationBboxArtist, _>(|ab| ab.xybox = xybox);
        self
    }

    /// Places the frame and the parts of the child where the figure draws
    /// them in its current layout, as the figure lays out its boxes before
    /// a draw (matplotlib's `update_positions`): the frame's box and
    /// mutation scale ([`AnnotationBbox::patch`]) are then those drawn.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::offsetbox::{AnnotationBboxOptions, TextArea};
    /// use pltrs::text::TextProps;
    /// let ax = Figure::new().subplot();
    /// let note = ax.add_annotation_bbox(
    ///     TextArea::from_text("peak", TextProps::default()),
    ///     (0.5, 0.7),
    ///     AnnotationBboxOptions::default(),
    /// )?;
    /// note.update_positions();
    /// assert!(note.patch().shape().width > 0.0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn update_positions(&self) {
        self.raw.figure_state().borrow_mut().prepare();
    }

    /// The frame (matplotlib's `patch`).
    pub fn patch(&self) -> Patch<FancyBboxPatch> {
        let frame = self.raw.with::<AnnotationBboxArtist, _>(|ab| ab.frame);
        Patch::from_raw(part(&self.raw, frame))
    }

    /// The texts of the box's text areas, in box order.
    pub fn texts(&self) -> Vec<Text> {
        texts_of(&self.raw, self.artist_ids())
    }

    /// The artists the box draws (the texts of text areas, the artists of
    /// drawing areas, images, ...) in drawing order, frames before what
    /// they frame: what other annotations can be placed relative to, as
    /// [`Coords::Artist`] (matplotlib passes the artist itself, as in
    /// `boxcoords=("data", annulus)`).
    pub fn artist_ids(&self) -> Vec<ArtistId> {
        self.raw
            .with::<AnnotationBboxArtist, _>(|ab| ab.child.artists().into_iter().copied().collect())
    }

    /// Handles to the artists of [`artist_ids`](Self::artist_ids), in the
    /// same order.
    pub fn artists(&self) -> Vec<RawHandle> {
        self.artist_ids()
            .into_iter()
            .map(|id| part(&self.raw, id))
            .collect()
    }

    /// Clips the artists of the drawing area at `area` among those of the
    /// box to its bounds, or stops clipping them, as
    /// [`AnchoredOffsetbox::set_clip_children`] does; returns whether the
    /// box has such an area.
    ///
    /// [`AnchoredOffsetbox::set_clip_children`]: super::AnchoredOffsetbox::set_clip_children
    pub fn set_clip_children(&self, area: usize, clip: bool) -> bool {
        self.raw.with_mut::<AnnotationBboxArtist, _>(|ab| {
            ab.child
                .drawing_area_mut(area)
                .map(|found| found.clip = clip)
                .is_some()
        })
    }
}

impl Axes {
    /// Adds `child` as a box at `options.xybox` that annotates the point
    /// `xy`, optionally with an arrow to it (matplotlib's `add_artist` of
    /// an `AnnotationBbox`). Fails when `options.xycoords` is an offset.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::offsetbox::{AnnotationBboxOptions, TextArea};
    /// use pltrs::patches::ArrowStyle;
    /// use pltrs::text::{FancyArrowProps, TextProps};
    /// let ax = Figure::new().subplot();
    /// let note = ax.add_annotation_bbox(
    ///     TextArea::from_text("peak", TextProps::default()),
    ///     (0.5, 0.7),
    ///     AnnotationBboxOptions {
    ///         xybox: Some((-20.0, 40.0).into()),
    ///         boxcoords: Some("offset points".parse()?),
    ///         arrowprops: Some(FancyArrowProps::new("->".parse::<ArrowStyle>()?)),
    ///         ..AnnotationBboxOptions::default()
    ///     },
    /// )?;
    /// assert_eq!(note.xy(), (0.5, 0.7).into());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn add_annotation_bbox(
        &self,
        child: impl Into<OffsetBox<BoxItem>>,
        xy: impl Into<Point>,
        options: AnnotationBboxOptions,
    ) -> Result<AnnotationBbox, CoordError> {
        add_annotation_bbox(
            self.raw(),
            Some(self.id()),
            child.into(),
            xy.into(),
            options,
        )
    }
}

impl Figure {
    /// Adds `child` as a box of the figure at `options.xybox` that
    /// annotates the point `xy`, optionally with an arrow to it
    /// (matplotlib's `fig.add_artist` of an `AnnotationBbox`). Without an
    /// axes, the point is given in figure coordinates or relative to
    /// another artist, such as a [`ConnectionPatch`] joining two axes.
    /// Fails when `options.xycoords` is an offset.
    ///
    /// [`ConnectionPatch`]: crate::patches::ConnectionPatch
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::offsetbox::{AnnotationBboxOptions, TextArea};
    /// use pltrs::text::{FancyArrowProps, TextProps};
    /// let fig = Figure::new();
    /// let note = fig.add_annotation_bbox(
    ///     TextArea::from_text("corner", TextProps::default()),
    ///     (0.1, 0.1),
    ///     AnnotationBboxOptions {
    ///         xycoords: "figure fraction".parse()?,
    ///         xybox: Some((30.0, 30.0).into()),
    ///         boxcoords: Some("offset points".parse()?),
    ///         arrowprops: Some("->".parse::<FancyArrowProps>()?),
    ///         ..AnnotationBboxOptions::default()
    ///     },
    /// )?;
    /// assert_eq!(note.xybox(), (30.0, 30.0).into());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn add_annotation_bbox(
        &self,
        child: impl Into<OffsetBox<BoxItem>>,
        xy: impl Into<Point>,
        options: AnnotationBboxOptions,
    ) -> Result<AnnotationBbox, CoordError> {
        add_annotation_bbox(self.raw(), None, child.into(), xy.into(), options)
    }
}

/// Adds the box `child` annotating `xy` to `owner`, an axes (also given as
/// `axes`) or a (sub)figure.
fn add_annotation_bbox(
    owner: &RawHandle,
    axes: Option<ArtistId>,
    child: OffsetBox<BoxItem>,
    xy: Point,
    options: AnnotationBboxOptions,
) -> Result<AnnotationBbox, CoordError> {
    if options.xycoords.is_offset() {
        return Err(CoordError::NoOffset);
    }
    let rc = crate::rc::params();
    let fontsize = options
        .fontsize
        .unwrap_or(rc.legend_fontsize)
        .to_points(rc.font_size);
    let boxstyle = options
        .boxstyle
        .unwrap_or(BoxStyle::new(BoxShape::Square).with_pad(options.pad));
    let arrow = options
        .arrowprops
        .map(|props| AnnotationArrow::new(ArrowProps::Fancy(props), &rc));
    let figure = owner.figure_state();
    let mut state = figure.borrow_mut();
    let mut parts = Parts::new(&mut state.store);
    let frame = parts.insert(frame_patch(boxstyle, options.frameon, options.bboxprops));
    let child = parts.insert_tree(child, owner.id());
    let parts = parts.into_ids();
    let artist = AnnotationBboxArtist {
        base: ArtistBase::with_zorder(ANNOTATION_BBOX_ZORDER),
        child,
        xy,
        xybox: options.xybox.unwrap_or(xy),
        boxcoords: options
            .boxcoords
            .unwrap_or_else(|| options.xycoords.clone()),
        xycoords: options.xycoords,
        box_alignment: options.box_alignment,
        annotation_clip: options.annotation_clip,
        fontsize,
        arrow,
        axes,
        frame,
        parts: parts.clone(),
        rc,
        dragger: None,
    };
    let id = adopt_box(&mut state, owner.id(), Box::new(artist), &parts);
    drop(state);
    Ok(AnnotationBbox {
        raw: RawHandle::new(Rc::clone(figure), id),
    })
}

#[cfg(test)]
mod tests;
