//! Offset boxes before they join a figure: their artists are owned by the
//! tree as [`BoxItem`]s and move into the figure when the box is added
//! (matplotlib creates them with the box and attaches them on
//! `add_artist`).

use std::rc::Rc;

use pltrs_core::Point;

use super::{OffsetBox, OffsetImage, PaddedBox, TextArea};
use crate::artist::{Artist, ArtistId, ArtistStore, AttachError, RawHandle};
use crate::axes::PlotError;
use crate::figure::FigureState;
use crate::image::{BboxImageArtist, BboxImageOptions, ImageData, ImageError};
use crate::lines::{LineArtist, LineProps};
use crate::patches::{BoxShape, BoxStyle, FancyBboxPatch, PatchArtist, PatchProps, PatchShape};
use crate::text::{TextArtist, TextProps};

/// An artist of an offset box that has not joined a figure yet: a patch or
/// a line of a drawing area or an aux-transform box, the text of a text
/// area, the image of an offset image or the frame of a padded box.
#[derive(Debug)]
pub struct BoxItem {
    pub(crate) artist: Box<dyn Artist>,
}

impl BoxItem {
    /// A patch of `shape` styled from the rc parameters and `props`.
    pub fn patch<S: PatchShape>(shape: S, props: PatchProps) -> Self {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        props.apply(&mut patch);
        Self {
            artist: Box::new(patch),
        }
    }

    /// A line through `(x, y)` styled from the rc parameters and `props`
    /// (without the property cycle).
    ///
    /// # Errors
    ///
    /// When `x` and `y` differ in length.
    pub fn line(x: Vec<f64>, y: Vec<f64>, props: LineProps) -> Result<Self, PlotError> {
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let mut line = LineArtist::new(x, y, crate::rc::params());
        props.apply(&mut line);
        Ok(Self {
            artist: Box::new(line),
        })
    }

    /// The artist of the loose handle `artist` (made outside any figure,
    /// see [`RawHandle::is_loose`]) as an item of a box, moved out of its
    /// holder, which is empty afterwards: matplotlib makes the artists of
    /// a box before the box joins a figure.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::lines::{Line2D, LineProps};
    /// use pltrs::offsetbox::{BoxItem, DrawingArea};
    /// let line = Line2D::loose(vec![0.0, 10.0], vec![5.0, 5.0], LineProps::default());
    /// let area = DrawingArea::new(10.0, 10.0, 0.0, 0.0).with_artist(BoxItem::from_loose(line.raw())?);
    /// assert_eq!(area.artists.len(), 1);
    /// assert!(BoxItem::from_loose(line.raw()).is_err(), "moved once");
    /// # Ok::<(), pltrs::artist::AttachError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AttachError::OtherFigure`] when the artist is not loose: it is in
    /// a figure, or was moved out of its holder before.
    pub fn from_loose(artist: &RawHandle) -> Result<Self, AttachError> {
        if !artist.is_loose() {
            return Err(AttachError::OtherFigure);
        }
        let artist = artist
            .figure_state()
            .borrow_mut()
            .detached
            .take(artist.id())
            .ok_or(AttachError::OtherFigure)?;
        Ok(Self { artist })
    }

    /// Moves the artist into `store` (unclipped, its transform to be set
    /// by the box) and returns its id.
    pub(crate) fn insert(self, store: &mut ArtistStore) -> ArtistId {
        let mut artist = self.artist;
        let base = artist.base_mut();
        base.clip_box = None;
        base.clip_path = None;
        store.insert(artist)
    }
}

impl TextArea<BoxItem> {
    /// An area for a new text of `text` styled from the rc parameters and
    /// `props` (matplotlib's `TextArea(s, textprops=...)`).
    pub fn from_text(text: impl Into<String>, props: TextProps) -> Self {
        let mut artist = TextArtist::new(Point::ORIGIN, text, crate::rc::params());
        props.apply(&mut artist);
        Self::new(
            BoxItem {
                artist: Box::new(artist),
            },
            false,
        )
    }
}

impl OffsetImage<BoxItem> {
    /// An image of `data` drawn with each array cell `zoom` points wide
    /// (matplotlib's `OffsetImage(arr, zoom=...)`).
    ///
    /// # Errors
    ///
    /// When the colormap, norm or filter radius of `options` is invalid.
    pub fn new(
        data: impl Into<ImageData>,
        zoom: f64,
        options: BboxImageOptions,
    ) -> Result<Self, ImageError> {
        let image = BboxImageArtist::new(data, options, &crate::rc::params())?;
        Ok(Self {
            image: BoxItem {
                artist: Box::new(image),
            },
            zoom,
            dpi_cor: true,
            visible: true,
        })
    }
}

impl PaddedBox<BoxItem> {
    /// `child` padded by `pad` points, framed by a square box styled from
    /// `frame_props` (white face, black edge by default) when `draw_frame`
    /// is set (matplotlib's `PaddedBox`).
    pub fn new(
        child: impl Into<OffsetBox<BoxItem>>,
        pad: f64,
        draw_frame: bool,
        frame_props: PatchProps,
    ) -> Self {
        Self {
            pad,
            child: Box::new(child.into()),
            frame: frame_patch(
                BoxStyle::new(BoxShape::Square).with_pad(0.0),
                draw_frame,
                frame_props,
            ),
            visible: true,
        }
    }
}

/// The parts of a box joining a figure, gathered so that they can be
/// parented to the box once it is added.
pub(crate) struct Parts<'a> {
    store: &'a mut ArtistStore,
    ids: Vec<ArtistId>,
}

impl<'a> Parts<'a> {
    /// Starts gathering parts into `store`.
    pub(crate) fn new(store: &'a mut ArtistStore) -> Self {
        Self {
            store,
            ids: Vec::new(),
        }
    }

    /// Moves `item` into the figure.
    pub(crate) fn insert(&mut self, item: BoxItem) -> ArtistId {
        let id = item.insert(self.store);
        self.ids.push(id);
        id
    }

    /// Moves every artist of `tree` into the figure.
    pub(crate) fn insert_tree(&mut self, tree: OffsetBox<BoxItem>) -> OffsetBox<ArtistId> {
        tree.map(&mut |item| self.insert(item))
    }

    /// The ids of the parts, in insertion order.
    pub(crate) fn into_ids(self) -> Vec<ArtistId> {
        self.ids
    }
}

/// Adds the box artist `owner` to `parent`, an axes or a (sub)figure, and
/// makes it the parent of its `parts`.
pub(crate) fn adopt_box(
    state: &mut FigureState,
    parent: ArtistId,
    owner: Box<dyn Artist>,
    parts: &[ArtistId],
) -> ArtistId {
    let id = state.adopt(parent, owner);
    for &part in parts {
        if let Some(artist) = state.artist_mut(part) {
            artist.base_mut().parent = Some(id);
        }
    }
    id
}

/// The frame of a box: a snapped `FancyBboxPatch` of `boxstyle` in the
/// `pltrs.offsetbox.*` colours (white with a black edge) unless `props` say
/// otherwise, shown when `visible`.
pub(crate) fn frame_patch(boxstyle: BoxStyle, visible: bool, props: PatchProps) -> BoxItem {
    let rc = crate::rc::params();
    let mut frame = PatchArtist::new(
        FancyBboxPatch::new((0.0, 0.0), 1.0, 1.0, boxstyle),
        Rc::clone(&rc),
    );
    // Like matplotlib's `patch.update(props)` after construction, a colour
    // in `props` replaces both defaults.
    PatchProps::default()
        .facecolor(rc.pltrs_offsetbox_facecolor.clone())
        .edgecolor(rc.pltrs_offsetbox_edgecolor.clone())
        .apply(&mut frame);
    props.apply(&mut frame);
    let base = frame.base_mut();
    base.snap = Some(true);
    base.visible = visible;
    BoxItem {
        artist: Box::new(frame),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Color;

    #[test]
    fn lines_need_matching_coordinates() {
        assert!(BoxItem::line(vec![0.0, 1.0], vec![0.0, 1.0], LineProps::default()).is_ok());
        assert_eq!(
            BoxItem::line(vec![0.0], Vec::new(), LineProps::default()).err(),
            Some(PlotError::LengthMismatch { x: 1, y: 0 })
        );
    }

    #[test]
    fn frame_colours_replace_both_defaults() {
        let square = BoxStyle::new(BoxShape::Square);
        let frame = frame_patch(square, false, PatchProps::default().color("r"));
        let style = frame.artist.as_patch().expect("a patch").style;
        assert_eq!(style.facecolor, Some(Color::from("r")));
        assert_eq!(style.edgecolor, Some(Color::from("r")));
        let base = frame.artist.base();
        assert_eq!((base.snap, base.visible), (Some(true), false));
        let plain = frame_patch(square, true, PatchProps::default());
        let style = plain.artist.as_patch().expect("a patch").style;
        assert_eq!(style.facecolor, Some(Color::from("w")));
    }
}
