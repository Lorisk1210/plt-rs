//! Archiving figures: the state behind a handle written to bytes and read
//! back, artist ids included, in this process or another (what pickling a
//! figure rests on; see [`pltrs_core::archive`]).
//!
//! Every artist type pltrs defines is archived under a tag of its own; an
//! artist type of the program's is archived by the program's
//! [`ProgramKinds`]. Artists refer to each other by id within their
//! figure; a handle held by a value archived on its own (a colorbar, a
//! container) is archived as its id and a [`FigureRef`] external.

use std::fmt;
use std::rc::Rc;

use pltrs_core::archive::{self, Archive, ArchiveError, Decoder, Encoder, External, ProgramKinds};

use super::{Artist, ArtistId, ArtistLookup, FilteredGroupArtist, RawHandle};
use crate::axes::AxesState;
use crate::axes_grid1::{BboxConnector, BboxConnectorPatch, BboxPatch};
use crate::axisartist::{AxisArtistState, AxisTextsState, TickMarksState};
use crate::collections::CollectionArtist;
use crate::colorbar::ColorbarArtist;
use crate::contour::ContourSetArtist;
use crate::figure::{FigureCell, FigureFrame, FigureState};
use crate::image::{
    AxesImageArtist, BboxImageArtist, FigureImageArtist, NonUniformImageArtist, PcolorImageArtist,
};
use crate::inset::IndicatorArtist;
use crate::legend::LegendArtist;
use crate::lines::LineArtist;
use crate::mplot3d::{Axes3DState, ProjectedOutline, ProjectedPath};
use crate::offsetbox::{
    AnchoredBoxArtist, AnnotationBboxArtist, PlacedBoxArtist, ProgramBoxArtist,
};
use crate::patches::{
    Annulus, Arc, Arrow, Circle, CirclePolygon, ConnectionPatch, Ellipse, FancyArrow,
    FancyArrowPatch, FancyBboxPatch, PatchArtist, PathShape, Polygon, Rectangle, RegularPolygon,
    Shadow, StepPatch, Wedge,
};
use crate::quiver::QuiverKeyArtist;
use crate::spines::SpineArtist;
use crate::table::{CellShape, TableArtist};
use crate::text::TextArtist;

pltrs_core::archive_kinds!(dyn Artist {
    "figure" => FigureFrame,
    "axes" => AxesState,
    "axes3d" => Axes3DState,
    "spine" => SpineArtist,
    "line" => LineArtist,
    "text" => TextArtist,
    "collection" => CollectionArtist,
    "image" => AxesImageArtist,
    "pcolor-image" => PcolorImageArtist,
    "nonuniform-image" => NonUniformImageArtist,
    "bbox-image" => BboxImageArtist,
    "figure-image" => FigureImageArtist,
    "contour-set" => ContourSetArtist,
    "legend" => LegendArtist,
    "colorbar" => ColorbarArtist,
    "quiver-key" => QuiverKeyArtist,
    "table" => TableArtist,
    "anchored-box" => AnchoredBoxArtist,
    "placed-box" => PlacedBoxArtist,
    "annotation-bbox" => AnnotationBboxArtist,
    "program-box" => ProgramBoxArtist,
    "axis-artist" => AxisArtistState,
    "tick-marks" => TickMarksState,
    "axis-texts" => AxisTextsState,
    "filtered-group" => FilteredGroupArtist,
    "inset-indicator" => IndicatorArtist,
    "rectangle" => PatchArtist<Rectangle>,
    "ellipse" => PatchArtist<Ellipse>,
    "circle" => PatchArtist<Circle>,
    "polygon" => PatchArtist<Polygon>,
    "wedge" => PatchArtist<Wedge>,
    "regular-polygon" => PatchArtist<RegularPolygon>,
    "circle-polygon" => PatchArtist<CirclePolygon>,
    "path-patch" => PatchArtist<PathShape>,
    "arc" => PatchArtist<Arc>,
    "annulus" => PatchArtist<Annulus>,
    "arrow" => PatchArtist<Arrow>,
    "fancy-arrow" => PatchArtist<FancyArrow>,
    "fancy-arrow-patch" => PatchArtist<FancyArrowPatch>,
    "connection-patch" => PatchArtist<ConnectionPatch>,
    "fancy-bbox" => PatchArtist<FancyBboxPatch>,
    "step-patch" => PatchArtist<StepPatch>,
    "bbox-patch" => PatchArtist<BboxPatch>,
    "bbox-connector" => PatchArtist<BboxConnector>,
    "bbox-connector-patch" => PatchArtist<BboxConnectorPatch>,
    "table-cell" => PatchArtist<CellShape>,
    "projected-outline" => PatchArtist<ProjectedOutline>,
    "projected-path" => PatchArtist<ProjectedPath>,
    "shadow:rectangle" => PatchArtist<Shadow<Rectangle>>,
    "shadow:ellipse" => PatchArtist<Shadow<Ellipse>>,
    "shadow:circle" => PatchArtist<Shadow<Circle>>,
    "shadow:polygon" => PatchArtist<Shadow<Polygon>>,
    "shadow:wedge" => PatchArtist<Shadow<Wedge>>,
    "shadow:regular-polygon" => PatchArtist<Shadow<RegularPolygon>>,
    "shadow:circle-polygon" => PatchArtist<Shadow<CirclePolygon>>,
    "shadow:path-patch" => PatchArtist<Shadow<PathShape>>,
    "shadow:arc" => PatchArtist<Shadow<Arc>>,
    "shadow:annulus" => PatchArtist<Shadow<Annulus>>,
    "shadow:arrow" => PatchArtist<Shadow<Arrow>>,
    "shadow:fancy-arrow" => PatchArtist<Shadow<FancyArrow>>,
    "shadow:fancy-arrow-patch" => PatchArtist<Shadow<FancyArrowPatch>>,
    "shadow:connection-patch" => PatchArtist<Shadow<ConnectionPatch>>,
    "shadow:fancy-bbox" => PatchArtist<Shadow<FancyBboxPatch>>,
    "shadow:step-patch" => PatchArtist<Shadow<StepPatch>>,
    "shadow:bbox-patch" => PatchArtist<Shadow<BboxPatch>>,
    "shadow:bbox-connector" => PatchArtist<Shadow<BboxConnector>>,
    "shadow:bbox-connector-patch" => PatchArtist<Shadow<BboxConnectorPatch>>,
    "shadow:table-cell" => PatchArtist<Shadow<CellShape>>,
    "shadow:projected-outline" => PatchArtist<Shadow<ProjectedOutline>>,
    "shadow:projected-path" => PatchArtist<Shadow<ProjectedPath>>,
});

/// The figure a handle reaches, as the external an archived handle refers
/// to (see [`RawHandle`]'s [`Archive`]): the program keeps it as it keeps
/// the figure, and gives it back to read the handle.
#[derive(Clone)]
pub struct FigureRef(Rc<FigureCell>);

impl FigureRef {
    /// The figure `handle` reaches.
    pub fn of(handle: &RawHandle) -> Self {
        Self(Rc::clone(handle.figure_state()))
    }

    /// The handle of artist `id` in the figure.
    pub fn handle(&self, id: ArtistId) -> RawHandle {
        RawHandle::new(Rc::clone(&self.0), id)
    }
}

impl fmt::Debug for FigureRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FigureRef(..)")
    }
}

/// A handle is archived as the external [`FigureRef`] of its figure and
/// the id of its artist there.
impl Archive for RawHandle {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_external(&External::new(FigureRef::of(self)));
        self.id().encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let external = decoder.read_external()?;
        let figure = external
            .downcast_ref::<FigureRef>()
            .ok_or_else(|| ArchiveError::invalid("a handle without its figure"))?;
        Ok(figure.handle(ArtistId::decode(decoder)?))
    }
}

impl RawHandle {
    /// A number telling the artist's figure apart from every other figure
    /// alive (bindings key what they keep per figure by it).
    pub fn figure_key(&self) -> usize {
        Rc::as_ptr(self.figure_state()).addr()
    }

    /// Archives the figure the artist is in, removed artists included (or
    /// the holder of a loose artist, see [`RawHandle::is_loose`]): the
    /// bytes, and the externals they refer to, which `program` made of
    /// what it archives itself.
    ///
    /// ```
    /// use pltrs::artist::RawHandle;
    /// use pltrs::prelude::*;
    /// use pltrs_core::archive::NoProgram;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 3.0])?;
    /// let (bytes, externals) = line.raw().archive_figure(&NoProgram)?;
    /// let back = RawHandle::from_figure_archive(&bytes, &externals, &NoProgram, line.id())?;
    /// assert_eq!(back.downcast::<pltrs::lines::Line2D>().map(|l| l.ydata()), Some(vec![1.0, 3.0]));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn archive_figure(
        &self,
        program: &dyn ProgramKinds,
    ) -> Result<(Vec<u8>, Vec<External>), ArchiveError> {
        archive::to_bytes(&*self.figure_state().borrow(), program)
    }

    /// The handle of artist `id` in a figure read back from an archive
    /// written by [`RawHandle::archive_figure`].
    pub fn from_figure_archive(
        bytes: &[u8],
        externals: &[External],
        program: &dyn ProgramKinds,
        id: ArtistId,
    ) -> Result<Self, ArchiveError> {
        let state: FigureState = archive::from_bytes(bytes, externals, program)?;
        let handle = Self::new(Rc::new(FigureCell::new(state)), id);
        reconnect_drags(&handle);
        Ok(handle)
    }

    /// Replaces the whole state of the artist's figure with a figure read
    /// back from an archive written by [`RawHandle::archive_figure`]:
    /// every handle of this figure, made before or after, then reaches the
    /// artist of its id in the figure read back. This is how a figure is
    /// unpickled, its handles being made before its state is read.
    pub fn restore_figure(
        &self,
        bytes: &[u8],
        externals: &[External],
        program: &dyn ProgramKinds,
    ) -> Result<(), ArchiveError> {
        let state: FigureState = archive::from_bytes(bytes, externals, program)?;
        *self.figure_state().borrow_mut() = state;
        reconnect_drags(self);
        Ok(())
    }

    /// The handle of artist `id` in the same figure as this one, whether
    /// or not there is such an artist yet (a figure being restored).
    #[must_use]
    pub fn sibling(&self, id: ArtistId) -> Self {
        Self::new(Rc::clone(self.figure_state()), id)
    }

    /// Whether the figure holds the artist (in it or removed from it):
    /// not yet for the handle of a figure being restored, made before the
    /// figure is read back ([`RawHandle::sibling`]), whose methods must
    /// wait for [`RawHandle::restore_figure`]. A figure that code further
    /// up the call stack is changing holds its artists.
    ///
    /// ```
    /// use pltrs::artist::ArtistId;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.raw().exists());
    /// assert!(!ax.raw().sibling(ArtistId::from_index(9999)).exists());
    /// ```
    pub fn exists(&self) -> bool {
        self.figure_state()
            .try_borrow()
            .is_none_or(|state| state.artist(self.id()).is_some())
    }
}

/// Connects the drags of the draggable artists of the figure `handle` is
/// in, read back from an archive, to its callbacks, which archives leave
/// out.
pub(crate) fn reconnect_drags(handle: &RawHandle) {
    let ids: Vec<ArtistId> = handle.figure_state().borrow().store.ids().collect();
    for id in ids {
        let artist = handle.sibling(id);
        crate::legend::reconnect_drag(&artist);
        crate::text::reconnect_drag(&artist);
        crate::offsetbox::reconnect_drag(&artist);
    }
}
