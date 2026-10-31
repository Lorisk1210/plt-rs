//! Inset indicators (matplotlib's `inset` module): a rectangle marking an
//! area of an axes, optionally joined by connectors to the inset axes that
//! shows the area enlarged ([`Axes::indicate_inset`] and
//! [`Axes::indicate_inset_zoom`]).
//!
//! As in matplotlib, an indicator owns a [`Rectangle`] patch and, when it
//! indicates an inset, four [`ConnectionPatch`]es to the inset's corners,
//! each with its own style and visibility ([`InsetIndicator::rectangle`],
//! [`InsetIndicator::connectors`]). The rectangle of an indicator made
//! without bounds follows the view limits of the inset, and the
//! connectors follow both. Two connectors, those that do not cross the
//! rectangle, are visible: which is chosen when the figure is first drawn
//! or the connectors are first asked for. Connectors in the rectangle's
//! style are drawn with it as one outline.

mod artist;
mod geometry;

use std::rc::Rc;

use pltrs_core::Point;

pub(crate) use artist::IndicatorArtist;
pub(crate) use geometry::{choose_connectors, place_indicators};

use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, AttachError, RawHandle, TypedHandle, downcast_ref,
    take_in,
};
use crate::axes::Axes;
use crate::colors::Color;
use crate::coords::Coords;
use crate::patches::{ConnectionEnd, ConnectionPatch, Patch, PatchArtist, PatchProps, Rectangle};
use crate::transforms::Transform;

/// The z-order of indicators, just below the default of inset axes.
const INDICATOR_ZORDER: f64 = 4.99;

/// A handle to an inset indicator (matplotlib's `InsetIndicator`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsetIndicator {
    raw: RawHandle,
}

impl ArtistHandle for InsetIndicator {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for InsetIndicator {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<IndicatorArtist>()
            .then(|| Self { raw: raw.clone() })
    }
}

impl InsetIndicator {
    /// The indicator's artist state.
    fn with<R>(&self, f: impl FnOnce(&IndicatorArtist) -> R) -> R {
        self.raw.with::<IndicatorArtist, _>(f)
    }

    /// The frame (matplotlib's `rectangle`).
    pub fn rectangle(&self) -> Patch<Rectangle> {
        let id = self.with(|indicator| indicator.rectangle);
        Patch::from_raw(RawHandle::new(Rc::clone(self.raw.figure_state()), id))
    }

    /// The connectors to the lower-left, upper-left, lower-right and
    /// upper-right corners of the inset (matplotlib's `connectors`), placed
    /// as the figure is laid out now; `None` without an inset. As in
    /// matplotlib, asking for them chooses which are visible if the figure
    /// has not been drawn since the indicator was made.
    pub fn connectors(&self) -> Option<Vec<Patch<ConnectionPatch>>> {
        self.with(|indicator| indicator.inset)?;
        {
            let mut state = self.raw.figure_state().borrow_mut();
            state.prepare();
            choose_connectors(&mut state);
        }
        Some(self.connector_patches())
    }

    /// The connectors as they are, without choosing which are visible:
    /// the four connectors of an indicator of an inset, else none.
    pub fn connector_patches(&self) -> Vec<Patch<ConnectionPatch>> {
        let figure = self.raw.figure_state();
        self.with(|indicator| indicator.connectors.clone())
            .into_iter()
            .map(|id| Patch::from_raw(RawHandle::new(Rc::clone(figure), id)))
            .collect()
    }

    /// Styles the rectangle and the connectors with `props`, the indicator
    /// taking their alpha (matplotlib's `set_alpha`, `set_edgecolor`,
    /// `set_color`, `set_linewidth` and `set_linestyle` of the indicator).
    pub fn update_props(&self, props: PatchProps) -> &Self {
        if let Some(alpha) = props.base.alpha {
            self.set_alpha(Some(alpha));
        }
        self.rectangle().update_props(props.clone());
        for connector in self.connector_patches() {
            connector.update_props(props.clone());
        }
        self
    }
}

/// The rectangle of an indicator made by [`Axes::indicate_inset`]: matplotlib's
/// unfilled, grey, half transparent and unclipped frame, styled by `props`.
fn frame(bounds: [f64; 4], props: PatchProps) -> PatchArtist<Rectangle> {
    let [x, y, width, height] = bounds;
    let mut rectangle =
        PatchArtist::new(Rectangle::new((x, y), width, height), crate::rc::params());
    rectangle.style.facecolor = Some(Color::none());
    rectangle.style.edgecolor = Some(Color::from("0.5"));
    let base = rectangle.base_mut();
    base.alpha = Some(0.5);
    base.clip_on = false;
    base.label = "_indicate_inset".into();
    props.apply(&mut rectangle);
    rectangle
}

/// A connector of `rectangle`, styled after it as matplotlib's
/// `_update_connectors` makes them: a plain line in its edge colour,
/// alpha, line style and width.
fn connector(rectangle: &PatchArtist<Rectangle>) -> PatchArtist<ConnectionPatch> {
    let origin = ConnectionEnd::new(Point::ORIGIN, Coords::Data, None);
    let mut connector = PatchArtist::new(
        ConnectionPatch::new(origin.clone(), origin),
        crate::rc::params(),
    );
    connector
        .style
        .edgecolor
        .clone_from(&rectangle.style.edgecolor);
    connector.style.linestyle = rectangle.style.linestyle.clone();
    connector.style.linewidth = rectangle.style.linewidth;
    connector.base_mut().alpha = rectangle.base().alpha;
    connector
}

impl Axes {
    /// Marks the area `[x0, y0, width, height]` (data coordinates) with a
    /// rectangle, joined to the corners of `inset` if given (matplotlib's
    /// `indicate_inset`). The rectangle is unfilled, grey and half
    /// transparent, drawn just below inset axes and not clipped, unless
    /// `props` say otherwise.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let marker = ax.indicate_inset([1.0, 1.0, 2.0, 2.0], None, PatchProps::default());
    /// assert_eq!((marker.zorder(), marker.alpha()), (4.99, Some(0.5)));
    /// assert!(marker.connectors().is_none());
    /// ```
    pub fn indicate_inset(
        &self,
        bounds: [f64; 4],
        inset: Option<&Axes>,
        props: PatchProps,
    ) -> InsetIndicator {
        self.add_indicator(bounds, props, false, inset)
    }

    /// Marks the area shown by `inset` (its view limits, followed when
    /// they change) and joins it to the inset's corners (matplotlib's
    /// `indicate_inset_zoom`); styled as [`Axes::indicate_inset`].
    pub fn indicate_inset_zoom(&self, inset: &Axes, props: PatchProps) -> InsetIndicator {
        let ((x0, x1), (y0, y1)) = (inset.xlim(), inset.ylim());
        self.add_indicator([x0, y0, x1 - x0, y1 - y0], props, true, Some(inset))
    }

    /// Adds an indicator made outside any figure (matplotlib's
    /// `add_artist(InsetIndicator(...))`): `artist`, made outside any
    /// figure, gives it its common properties and `rectangle`, a
    /// rectangle made outside any figure, becomes its frame, in this axes'
    /// data coordinates unless it has coordinates of its own. With
    /// `follows_inset`, the frame follows the view limits of `inset`.
    ///
    /// # Errors
    ///
    /// [`AttachError::OtherFigure`] when `artist` or `rectangle` belongs
    /// to a figure, [`AttachError::Misplaced`] when `rectangle` is no
    /// [`Rectangle`].
    pub fn adopt_indicator(
        &self,
        artist: &RawHandle,
        rectangle: &RawHandle,
        follows_inset: bool,
        inset: Option<&Axes>,
    ) -> Result<InsetIndicator, AttachError> {
        if !artist.is_loose() || !rectangle.is_loose() {
            return Err(AttachError::OtherFigure);
        }
        let mut base = artist.with_base(Clone::clone);
        base.parent = None;
        let frame = take_in(self.raw().figure_state(), rectangle)?.id();
        {
            let mut state = self.raw().figure_state().borrow_mut();
            let patch = state.detached.take(frame).ok_or(AttachError::OtherFigure)?;
            let is_rectangle = downcast_ref::<PatchArtist<Rectangle>>(patch.as_ref()).is_some();
            state.store.put_back(frame, patch);
            if !is_rectangle {
                state.remove_artist(frame);
                return Err(AttachError::Misplaced);
            }
        }
        Ok(self.insert_indicator(base, frame, follows_inset, inset))
    }

    /// Adds an indicator of `bounds` styled by `props` over matplotlib's
    /// defaults.
    fn add_indicator(
        &self,
        bounds: [f64; 4],
        props: PatchProps,
        follows_inset: bool,
        inset: Option<&Axes>,
    ) -> InsetIndicator {
        let mut base = ArtistBase::with_zorder(props.base.zorder.unwrap_or(INDICATOR_ZORDER));
        let rectangle = frame(bounds, props);
        base.alpha = rectangle.base().alpha;
        let frame = self
            .raw()
            .figure_state()
            .borrow_mut()
            .store
            .insert(Box::new(rectangle));
        self.insert_indicator(base, frame, follows_inset, inset)
    }

    /// Puts the indicator of common properties `base` framed by the
    /// rectangle `frame`, which is in the figure's store, into the axes,
    /// with connectors to `inset` if given.
    fn insert_indicator(
        &self,
        base: ArtistBase,
        frame: ArtistId,
        follows_inset: bool,
        inset: Option<&Axes>,
    ) -> InsetIndicator {
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let rectangle = state
            .store
            .get_as_mut::<PatchArtist<Rectangle>>(frame)
            .expect("the frame is a rectangle in the figure");
        rectangle
            .base_mut()
            .transform
            .get_or_insert(Transform::Data(self.id()));
        let connector = connector(rectangle);
        let connectors: Vec<ArtistId> = match inset {
            Some(_) => (0..4)
                .map(|_| state.store.insert(Box::new(connector.clone())))
                .collect(),
            None => Vec::new(),
        };
        let indicator = IndicatorArtist {
            base,
            rectangle: frame,
            connectors: connectors.clone(),
            inset: inset.map(ArtistHandle::id),
            follows_inset,
            chosen: false,
        };
        let id = state.adopt(self.id(), Box::new(indicator));
        for part in std::iter::once(frame).chain(connectors) {
            if let Some(artist) = state.artist_mut(part) {
                artist.base_mut().parent = Some(id);
            }
        }
        drop(state);
        InsetIndicator {
            raw: RawHandle::new(Rc::clone(figure), id),
        }
    }
}

#[cfg(test)]
mod tests;
