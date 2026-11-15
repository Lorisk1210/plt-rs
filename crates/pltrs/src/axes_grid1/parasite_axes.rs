//! Host axes and their parasites (matplotlib's `axes_grid1.parasite_axes`).
//!
//! A parasite axes sits exactly on its host and has a coordinate system
//! of its own, sharing one axis of the host ([`HostAxes::twinx`],
//! [`HostAxes::twiny`]), none ([`HostAxes::aux_axes`]), or mapped onto
//! the host's by a transform ([`HostAxes::twin`]). The host draws the
//! parasites' artists and visible decorations among its own, sorted by
//! z-order, and its legend lists them after its own.

use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

use pltrs_core::{Affine2D, Bbox};

use super::mpl_axes::SimpleAxisArtist;
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup, RawHandle};
use crate::axes::{Axes, AxesState, AxisName};
use crate::axisartist::Toggle;
use crate::figure::{AxesRectError, Figure, FigureState, insert_axes};
use crate::gridspec::GridSpecError;
use crate::spines::SpineSide;
use crate::transforms::{ResolvedTransform, TransformContext, unit_to_bbox};

/// How a parasite's coordinates relate to its host's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParasiteLink {
    /// Own data coordinates and view limits (matplotlib's
    /// `get_aux_axes(viewlim_mode=None)`).
    Independent,
    /// Own data coordinates, view limits equal to the host's
    /// (`viewlim_mode="equal"`, the default of `get_aux_axes`).
    Equal,
    /// Data coordinates mapped onto the host's by an affine transform: the
    /// parasite draws through it and then the host's data transform,
    /// whatever its own limits and scales (matplotlib's `transData =
    /// transAux + parent.transData`), and its view limits are the host's
    /// mapped back (`get_aux_axes(tr, viewlim_mode="transform")`,
    /// `twin(aux_trans)`). The transform may scale and translate, not
    /// rotate or shear.
    Transform(Affine2D),
}

pltrs_core::archive_enum!(ParasiteLink {
    Independent,
    Equal,
    Transform(v0),
});

impl ParasiteLink {
    /// The map from the host's view limits to the parasite's, if they
    /// follow the host.
    fn view_map(self) -> Result<Option<Affine2D>, ParasiteError> {
        match self {
            Self::Independent => Ok(None),
            Self::Equal => Ok(Some(Affine2D::IDENTITY)),
            Self::Transform(aux) if !aux.is_separable() => Err(ParasiteError::NotSeparable(aux)),
            Self::Transform(aux) => aux.inverted().map(Some).ok_or(ParasiteError::Singular(aux)),
        }
    }
}

/// Why a parasite axes cannot be made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParasiteError {
    /// The aux transform rotates or shears, which would turn the view
    /// limits of the parasite.
    NotSeparable(Affine2D),
    /// The aux transform cannot be inverted.
    Singular(Affine2D),
}

impl fmt::Display for ParasiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSeparable(aux) => {
                write!(
                    f,
                    "aux transform {aux:?} rotates or shears; only scalings and translations are supported"
                )
            }
            Self::Singular(aux) => write!(f, "aux transform {aux:?} cannot be inverted"),
        }
    }
}

impl std::error::Error for ParasiteError {}

/// What makes an axes a parasite (matplotlib's `ParasiteAxesBase`): its
/// host draws it, at the host's position, and one without an aux
/// transform has a box of its own between the host's draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Parasite {
    /// The host axes.
    pub(crate) host: ArtistId,
    /// How its coordinates relate to the host's.
    link: ParasiteLink,
    /// The map from the host's view limits to its own, when they follow
    /// (see [`FigureState::sync_parasites`]).
    view_map: Option<Affine2D>,
    /// Whether it was made by `twin`, `twinx` or `twiny` (removing it then
    /// shows the host's far sides again).
    twin: bool,
    /// The position matplotlib's parasite has of its own, in the fractions
    /// of its host's (sub)figure: its host's active position when it was
    /// made, until one is given.
    position: Bbox,
    /// Its active position, in the same fractions: the host's at the
    /// host's last draw (matplotlib's `HostAxes.draw` gives its parasites
    /// `apply_aspect(rect)`), `None` for its own `position`, to which
    /// `apply_aspect(None)` brings it back (its `get_tightbbox` and
    /// `get_position`).
    active: Option<Bbox>,
}

pltrs_core::archive_struct!(Parasite {
    host,
    link,
    view_map,
    twin,
    position,
    active
});

/// An axes that can carry parasite axes (matplotlib's `HostAxes`, made by
/// `host_subplot` and `host_axes`). It dereferences to [`Axes`].
///
/// ```
/// use pltrs::axes_grid1::host_subplot;
/// use pltrs::prelude::*;
/// let fig = Figure::new();
/// let host = host_subplot(&fig, 1, 1, 1)?;
/// let par = host.twinx();
/// host.plot([0.0, 1.0, 2.0], [0.0, 1.0, 2.0])?.set_label("Density");
/// par.plot([0.0, 1.0, 2.0], [0.0, 3.0, 2.0])?.set_label("Temperature");
/// let legend = host.legend(pltrs::legend::LegendOptions::default())?;
/// assert_eq!(legend.labels(), ["Density", "Temperature"]);
/// assert_eq!(fig.axes().len(), 1, "parasites are drawn by their host");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostAxes {
    axes: Axes,
}

impl Deref for HostAxes {
    type Target = Axes;

    fn deref(&self) -> &Axes {
        &self.axes
    }
}

impl From<Axes> for HostAxes {
    fn from(axes: Axes) -> Self {
        Self::new(axes)
    }
}

/// A host axes in cell `index` (1-based, row-major) of an `nrows × ncols`
/// grid of `fig` (matplotlib's `host_subplot`).
///
/// # Errors
///
/// When the grid or the index is invalid.
pub fn host_subplot(
    fig: &Figure,
    nrows: usize,
    ncols: usize,
    index: usize,
) -> Result<HostAxes, GridSpecError> {
    fig.add_subplot(nrows, ncols, index).map(HostAxes::new)
}

/// A host axes at `[left, bottom, width, height]` in figure fractions
/// (matplotlib's `host_axes`).
///
/// # Errors
///
/// As [`Figure::add_axes`].
pub fn host_axes(fig: &Figure, rect: [f64; 4]) -> Result<HostAxes, AxesRectError> {
    fig.add_axes(rect).map(HostAxes::new)
}

impl HostAxes {
    /// `axes` as a host: any axes can carry parasites.
    pub fn new(axes: Axes) -> Self {
        Self { axes }
    }

    /// The underlying axes.
    pub fn axes(&self) -> &Axes {
        &self.axes
    }

    /// One side of the host (matplotlib's `host.axis["left"]`).
    pub fn axis(&self, side: SpineSide) -> SimpleAxisArtist {
        SimpleAxisArtist::new(&self.axes, side)
    }

    /// The parasites, in creation order (matplotlib's `parasites`).
    pub fn parasites(&self) -> Vec<ParasiteAxes> {
        let figure = self.axes.raw().figure_state();
        let ids = parasites_of(&*figure.borrow(), self.axes.id());
        ids.into_iter()
            .map(|id| ParasiteAxes {
                axes: Axes::from_raw(RawHandle::new(Rc::clone(figure), id)),
            })
            .collect()
    }

    /// Adds a parasite whose coordinates relate to the host's as `link`
    /// says, with every decoration hidden (matplotlib's `get_aux_axes`).
    ///
    /// # Errors
    ///
    /// When the transform of a [`ParasiteLink::Transform`] rotates, shears
    /// or cannot be inverted.
    pub fn aux_axes(&self, link: ParasiteLink) -> Result<ParasiteAxes, ParasiteError> {
        let view_map = link.view_map()?;
        Ok(self.add_parasite(link, view_map, false))
    }

    /// A parasite sharing the x axis, with its y axis on the right, where
    /// the host's is hidden (matplotlib's `HostAxes.twinx`).
    pub fn twinx(&self) -> ParasiteAxes {
        self.show_shared_twin(AxisName::X)
    }

    /// A parasite sharing the y axis, with its x axis on top, where the
    /// host's is hidden (matplotlib's `HostAxes.twiny`).
    pub fn twiny(&self) -> ParasiteAxes {
        self.show_shared_twin(AxisName::Y)
    }

    /// A parasite sharing the axis `shared`, its sides and the host's as
    /// they were (matplotlib's `_add_twin_axes(sharex=...)`, which
    /// [`twinx`](Self::twinx) and [`twiny`](Self::twiny) follow by showing
    /// and hiding sides): for a host whose sides are drawn by axis artists
    /// of its own (axisartist's), which the program shows and hides.
    pub fn shared_twin(&self, shared: AxisName) -> ParasiteAxes {
        let twin = self.add_parasite(ParasiteLink::Independent, None, true);
        twin.share(shared, &self.axes)
            .expect("a new axes of this figure shares nothing yet");
        twin
    }

    /// A parasite sharing no axis, whose data coordinates map onto the
    /// host's through `aux` (`None` = the identity), its sides and the
    /// host's as they were (matplotlib's `_add_twin_axes(aux_transform=...)`,
    /// which [`twin`](Self::twin) follows by showing and hiding sides; see
    /// [`shared_twin`](Self::shared_twin)). Its view limits are the host's
    /// mapped back.
    ///
    /// # Errors
    ///
    /// When `aux` rotates, shears or cannot be inverted.
    pub fn linked_twin(&self, aux: Option<Affine2D>) -> Result<ParasiteAxes, ParasiteError> {
        let link = ParasiteLink::Transform(aux.unwrap_or(Affine2D::IDENTITY));
        let view_map = link.view_map()?;
        Ok(self.add_parasite(link, view_map, true))
    }

    /// A parasite sharing no axis, whose data coordinates map onto the
    /// host's through `aux` (`None` = the identity) and whose axes are on
    /// the top and right, where the host's are hidden (matplotlib's
    /// `HostAxes.twin`). Its view limits are the host's mapped back.
    ///
    /// ```
    /// use pltrs::axes_grid1::host_axes;
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Affine2D;
    /// let fig = Figure::new();
    /// let host = host_axes(&fig, [0.1, 0.1, 0.8, 0.8])?;
    /// let degrees = host.twin(Some(Affine2D::scaling(std::f64::consts::PI / 180.0, 1.0)))?;
    /// host.set_xlim(0.0, std::f64::consts::PI)?;
    /// fig.layout();
    /// let (low, high) = degrees.xlim();
    /// assert!(low.abs() < 1e-9 && (high - 180.0).abs() < 1e-9);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `aux` rotates, shears or cannot be inverted.
    pub fn twin(&self, aux: Option<Affine2D>) -> Result<ParasiteAxes, ParasiteError> {
        let twin = self.linked_twin(aux)?;
        for side in [SpineSide::Top, SpineSide::Right] {
            self.axis(side).set_visible(false);
        }
        for side in [SpineSide::Top, SpineSide::Right] {
            twin.axis(side).set_visible(true);
        }
        for side in [SpineSide::Left, SpineSide::Bottom] {
            twin.axis(side).set_visible(false);
        }
        Ok(twin)
    }

    /// `twinx` (sharing x) or `twiny` (sharing y).
    fn show_shared_twin(&self, shared: AxisName) -> ParasiteAxes {
        let twin = self.shared_twin(shared);
        let (own, others) = match shared {
            AxisName::X => (
                SpineSide::Right,
                [SpineSide::Left, SpineSide::Top, SpineSide::Bottom],
            ),
            AxisName::Y => (
                SpineSide::Top,
                [SpineSide::Left, SpineSide::Right, SpineSide::Bottom],
            ),
        };
        self.axis(own).set_visible(false);
        twin.axis(own).set_visible(true);
        for side in others {
            twin.axis(side).set_visible(false);
        }
        twin
    }

    /// Adds a parasite axes at the host's position, without frame,
    /// background, axes, spines or titles (matplotlib's
    /// `ParasiteAxesBase.__init__` and `clear`). The current axes stays
    /// the current axes.
    fn add_parasite(
        &self,
        link: ParasiteLink,
        view_map: Option<Affine2D>,
        twin: bool,
    ) -> ParasiteAxes {
        let figure = self.axes.raw().figure_state();
        let current = figure.borrow().current_axes;
        // matplotlib's parasite is made at its host's `_position`.
        let position = self
            .axes
            .raw()
            .with::<AxesState, _>(|host| host.aspect.active_position(&host.position));
        let axes = insert_axes(figure, self.axes.id(), Bbox::UNIT, None);
        let mut state = figure.borrow_mut();
        state.current_axes = current;
        if let Some(parasite) = state.artist_as_mut::<AxesState>(axes.id()) {
            parasite.parasite = Some(Parasite {
                host: self.axes.id(),
                link,
                view_map,
                twin,
                position,
                active: None,
            });
            if let ParasiteLink::Transform(aux) = link {
                parasite.aux_transform = Some(ResolvedTransform::affine(aux));
            }
            parasite.decorations.frame_on = false;
        }
        state.hide_parasite_decorations(axes.id());
        drop(state);
        ParasiteAxes { axes }
    }
}

impl FigureState {
    /// Hides the background, axes, spines and titles of the parasite
    /// `axes`, which its host never draws (matplotlib's
    /// `ParasiteAxesBase.clear` hides all its children).
    pub(crate) fn hide_parasite_decorations(&mut self, axes: ArtistId) {
        let decorations = self
            .artist_as_mut::<AxesState>(axes)
            .map(|parasite| {
                parasite.patch.visible = false;
                parasite.xaxis.visible = false;
                parasite.yaxis.visible = false;
                parasite
                    .spines
                    .iter()
                    .chain(&parasite.titles)
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for id in decorations {
            if let Some(artist) = self.artist_mut(id) {
                artist.base_mut().visible = false;
            }
        }
    }
}

/// An axes drawn by its host, at the host's position (matplotlib's
/// `ParasiteAxes`). It dereferences to [`Axes`]; its lines continue the
/// host's line property cycle. It is placed in its host like an inset
/// covering it: [`Axes::position`] is `[0, 0, 1, 1]` in the host's
/// fractions, and it is not among the figure's axes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParasiteAxes {
    axes: Axes,
}

impl Deref for ParasiteAxes {
    type Target = Axes;

    fn deref(&self) -> &Axes {
        &self.axes
    }
}

impl ParasiteAxes {
    /// The parasite `axes` is, if it is one.
    pub fn of(axes: &Axes) -> Option<Self> {
        let parasite = axes
            .raw()
            .with::<AxesState, _>(|state| state.parasite.is_some());
        parasite.then(|| Self { axes: axes.clone() })
    }

    /// The underlying axes.
    pub fn axes(&self) -> &Axes {
        &self.axes
    }

    /// One side of the parasite (matplotlib's `par.axis["right"]`).
    pub fn axis(&self, side: SpineSide) -> SimpleAxisArtist {
        SimpleAxisArtist::new(&self.axes, side)
    }

    /// How its coordinates relate to its host's.
    pub fn link(&self) -> ParasiteLink {
        self.parasite().link
    }

    /// The position the parasite has of its own, in figure fractions
    /// (matplotlib's `get_position(original=True)` of a parasite): its
    /// host's active position when it was made, until one is set.
    pub fn own_position(&self) -> Bbox {
        self.parasite().position
    }

    /// Gives the parasite a position of its own, in figure fractions
    /// (matplotlib's `set_position(pos, which="original")` of a parasite).
    pub fn set_own_position(&self, position: Bbox) {
        self.axes.raw().with_mut::<AxesState, _>(|state| {
            if let Some(parasite) = &mut state.parasite {
                parasite.position = position;
            }
        });
    }

    /// The position the parasite is at, in figure fractions (matplotlib's
    /// active position of a parasite): its host's at the host's last
    /// draw, else its own. A parasite without an aux transform is placed
    /// there between its host's draws; one with an aux transform follows
    /// its host (matplotlib's takes its host's `transAxes`).
    pub fn active_position(&self) -> Bbox {
        let parasite = self.parasite();
        parasite.active.unwrap_or(parasite.position)
    }

    /// Places the parasite at `position`, in figure fractions, until its
    /// host's next draw (matplotlib's `apply_aspect(position)` of a
    /// parasite), or at its own position for `None` (`apply_aspect()`).
    pub fn set_active_position(&self, position: Option<Bbox>) {
        self.axes.raw().with_mut::<AxesState, _>(|state| {
            if let Some(parasite) = &mut state.parasite {
                parasite.active = position;
            }
        });
    }

    /// The host.
    pub fn host(&self) -> HostAxes {
        let host = self.parasite().host;
        HostAxes::new(Axes::from_raw(RawHandle::new(
            Rc::clone(self.axes.raw().figure_state()),
            host,
        )))
    }

    /// Removes the parasite from its host; for a twin, the host's top and
    /// right sides it does not share come back with ticks but without
    /// tick labels or label (matplotlib's `_remove_any_twin`). Removing it
    /// again does nothing.
    pub fn remove(&self) {
        let figure = self.axes.raw().figure_state();
        if figure.borrow().is_removed(self.axes.id()) {
            return;
        }
        let Parasite { twin, .. } = self.parasite();
        if twin {
            let (shares_x, shares_y) = self
                .axes
                .raw()
                .with::<AxesState, _>(|state| (state.shares.0.is_some(), state.shares.1.is_some()));
            let host = self.host();
            let restored: Vec<SpineSide> =
                [(SpineSide::Top, shares_x), (SpineSide::Right, shares_y)]
                    .into_iter()
                    .filter_map(|(side, shared)| (!shared).then_some(side))
                    .collect();
            for &side in &restored {
                host.axis(side).set_visible(true);
            }
            for &side in &restored {
                host.axis(side).toggle(Toggle {
                    ticklabels: Some(false),
                    label: Some(false),
                    ..Toggle::default()
                });
            }
        }
        ArtistHandle::remove(&self.axes);
    }

    fn parasite(&self) -> Parasite {
        self.axes.raw().with::<AxesState, _>(|state| {
            state
                .parasite
                .expect("a parasite handle is only made for parasite axes")
        })
    }
}

/// Whether a locator places `axes` at each layout (matplotlib's
/// `get_axes_locator()` is not `None`): a function, a divider's cell, an
/// inset's or a colorbar's locator.
fn placed_by_locator(axes: &AxesState) -> bool {
    let aspect = &axes.aspect;
    axes.axes_locator.is_some()
        || aspect.divider.is_some()
        || aspect.inset.is_some()
        || aspect.locator.is_some()
}

/// Whether `axes` is the parasite of another axes.
pub(crate) fn is_parasite(store: &impl ArtistLookup, axes: ArtistId) -> bool {
    store
        .artist_as::<AxesState>(axes)
        .is_some_and(|state| state.parasite.is_some())
}

/// The parasites of `axes`, in creation order.
pub(crate) fn parasites_of(store: &impl ArtistLookup, axes: ArtistId) -> Vec<ArtistId> {
    store
        .artist_as::<AxesState>(axes)
        .map(|state| {
            state
                .children
                .iter()
                .copied()
                .filter(|&child| is_parasite(store, child))
                .collect()
        })
        .unwrap_or_default()
}

impl FigureState {
    /// The display box of the parasite `axes` at its active position
    /// (see [`ParasiteAxes::active_position`]); `None` for an axes that is
    /// no parasite or that follows its host's box: a parasite with an aux
    /// transform, and the parasite of a host placed by a locator, which
    /// matplotlib's parasite borrows to be placed where its host is.
    pub(crate) fn parasite_extent(&self, axes: ArtistId) -> Option<Bbox> {
        let parasite = self.store.get_as::<AxesState>(axes)?.parasite?;
        let host = self.store.get_as::<AxesState>(parasite.host)?;
        if matches!(parasite.link, ParasiteLink::Transform(_)) || placed_by_locator(host) {
            return None;
        }
        let frame = host.base().parent.or(host.home).unwrap_or(self.root);
        let frame = self.frame_bbox(frame)?;
        let position = parasite.active.unwrap_or(parasite.position);
        Some(position.transformed(&unit_to_bbox(&frame)))
    }

    /// Places the parasites of `host` at the host's active position, as
    /// matplotlib's `HostAxes.draw` does before it draws them.
    pub(crate) fn place_parasites_at_host(&mut self, host: ArtistId) {
        let Some(position) = self
            .store
            .get_as::<AxesState>(host)
            .map(|state| state.aspect.active_position(&state.position))
        else {
            return;
        };
        for parasite in parasites_of(&self.store, host) {
            if let Some(Some(state)) = self
                .store
                .get_as_mut::<AxesState>(parasite)
                .map(|state| state.parasite.as_mut())
            {
                state.active = Some(position);
            }
        }
    }

    /// Brings the parasite `axes` back to its own position (matplotlib's
    /// `apply_aspect(None)`, which its `get_tightbbox` calls); nothing for
    /// an axes that is no parasite.
    pub(crate) fn reset_parasite_position(&mut self, axes: ArtistId) {
        if let Some(Some(parasite)) = self
            .store
            .get_as_mut::<AxesState>(axes)
            .map(|state| state.parasite.as_mut())
        {
            parasite.active = None;
        }
    }

    /// Gives the parasites of `host` that follow its view limits the
    /// host's, mapped back through their aux transforms (matplotlib's
    /// `_sync_lims`, which the host's `xlim_changed` and `ylim_changed`
    /// callbacks run): between two changes of its host's limits, a
    /// parasite keeps limits of its own.
    pub(crate) fn sync_parasites(&mut self, host: ArtistId) {
        let Some(view) = self
            .store
            .get_as::<AxesState>(host)
            .map(|state| state.view_lim)
        else {
            return;
        };
        for parasite in parasites_of(&self.store, host) {
            if let Some(state) = self.store.get_as_mut::<AxesState>(parasite)
                && let Some(view_map) = state.parasite.and_then(|parasite| parasite.view_map)
            {
                state.view_lim = Bbox::new(
                    view_map.transform_point(view.p0),
                    view_map.transform_point(view.p1),
                );
                state.view_mutated = (true, true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legend::LegendOptions;
    use crate::testing::major_tick_sides;

    #[test]
    fn twinx_moves_the_y_axis_to_the_parasite() {
        let fig = Figure::new();
        let host = host_subplot(&fig, 1, 1, 1).unwrap();
        let par = host.twinx();
        assert_eq!(host.parasites(), std::slice::from_ref(&par));
        assert_eq!(par.host(), host);
        assert!(!host.spines().right.visible());
        assert!(par.spines().right.visible());
        assert!(!par.spines().left.visible() && !par.spines().bottom.visible());
        assert_eq!(
            major_tick_sides(&par, AxisName::Y),
            ((false, true), (false, true))
        );
        assert_eq!(
            major_tick_sides(&par, AxisName::X),
            ((false, false), (false, false))
        );
        assert_eq!(par.yaxis().label_position(), SpineSide::Right);
        assert!(par.xaxis().visible() && par.yaxis().visible());
        host.set_xlim(0.0, 5.0).expect("finite limits");
        assert_eq!(par.xlim(), (0.0, 5.0), "the x axis is shared");
        assert_eq!(fig.axes(), [host.axes().clone()]);
        assert_eq!(fig.gca(), *host.axes());
    }

    #[test]
    fn clearing_keeps_the_parasites_cleared() {
        // matplotlib's `HostAxesBase.clear` clears the parasites it keeps,
        // and a cleared parasite stays one, its decorations hidden.
        let fig = Figure::new();
        let host = host_subplot(&fig, 1, 1, 1).unwrap();
        let par = host.twinx();
        par.plot([0.0, 1.0], [2.0, 3.0]).expect("finite data");
        assert!(host.child_axes().is_empty(), "parasites are no child axes");
        host.cla();
        assert_eq!(host.parasites(), std::slice::from_ref(&par));
        assert_eq!(par.lines().len(), 0, "the parasite is cleared");
        assert!(!par.spines().right.visible() && !par.yaxis().visible());
        host.set_xlim(0.0, 5.0).expect("finite limits");
        assert_eq!(par.xlim(), (0.0, 5.0), "the x axis is still shared");
        let aux = host.aux_axes(ParasiteLink::Equal).unwrap();
        aux.cla();
        assert!(ParasiteAxes::of(&aux).is_some());
        assert_eq!(host.parasites().len(), 2);
        assert!(!aux.spines().left.visible() && !aux.xaxis().visible());
        host.set_ylim(-1.0, 1.0).expect("finite limits");
        fig.layout();
        assert_eq!(aux.ylim(), (-1.0, 1.0), "its limits still follow");
    }

    #[test]
    fn twin_follows_the_host_through_the_aux_transform() {
        let fig = Figure::new();
        let host = host_axes(&fig, [0.1, 0.1, 0.8, 0.8]).expect("a valid rectangle");
        let twin = host.twin(Some(Affine2D::scaling(2.0, 0.5))).unwrap();
        host.set_xlim(0.0, 4.0)
            .and_then(|ax| ax.set_ylim(-1.0, 1.0))
            .expect("finite limits");
        assert_eq!(twin.xlim(), (0.0, 2.0));
        assert_eq!(twin.ylim(), (-2.0, 2.0));
        twin.set_xlim(5.0, 6.0).expect("finite limits");
        assert_eq!(twin.xlim(), (5.0, 6.0), "its own until the host's change");
        host.set_ylim(-2.0, 2.0).expect("finite limits");
        assert_eq!((twin.xlim(), twin.ylim()), ((0.0, 2.0), (-4.0, 4.0)));
        assert_eq!(
            major_tick_sides(&twin, AxisName::X),
            ((false, true), (false, true))
        );
        assert_eq!(major_tick_sides(&host, AxisName::Y).0, (true, false));
        assert!(!host.spines().top.visible() && twin.spines().top.visible());
        let rotated = Affine2D::identity().rotate_deg(30.0);
        assert!(matches!(
            host.twin(Some(rotated)),
            Err(ParasiteError::NotSeparable(_))
        ));
        assert!(matches!(
            host.aux_axes(ParasiteLink::Transform(Affine2D::scaling(0.0, 1.0))),
            Err(ParasiteError::Singular(_))
        ));
        assert!(
            ParasiteError::Singular(Affine2D::IDENTITY)
                .to_string()
                .contains("inverted")
        );
    }

    #[test]
    fn a_parasite_is_measured_at_its_own_position() {
        let fig = Figure::new();
        fig.set_size_inches(4.0, 3.0).unwrap();
        let host = host_axes(&fig, [0.125, 0.11, 0.775, 0.77]).expect("a valid rectangle");
        let par = host.aux_axes(ParasiteLink::Equal).unwrap();
        assert_eq!(par.own_position(), host.original_position());
        assert!(ParasiteAxes::of(&host).is_none() && ParasiteAxes::of(&par).is_some());
        // Moving the host leaves the parasite where it was made, and its
        // tight box covers that place.
        host.set_position([0.5, 0.5, 0.3, 0.3]);
        let tight = host.tight_bbox(false).unwrap();
        assert!(tight.x0() <= 50.0 && tight.y0() <= 33.0 && tight.x1() >= 360.0);
        par.set_own_position(Bbox::from_bounds(0.6, 0.6, 0.1, 0.1));
        let tight = host.tight_bbox(false).unwrap();
        assert!(tight.x0() > 150.0 && tight.y0() > 100.0, "{tight:?}");
        // The parasite of a host placed by a locator is measured where the
        // locator places the host.
        par.set_own_position(Bbox::from_bounds(0.0, 0.0, 0.1, 0.1));
        host.set_axes_locator(Some(crate::axes::AxesLocatorFn::new(|_: Bbox| {
            Bbox::from_bounds(0.5, 0.5, 0.3, 0.3)
        })));
        let tight = host.tight_bbox(false).unwrap();
        assert!(tight.x0() > 150.0 && tight.y0() > 100.0, "{tight:?}");
    }

    #[test]
    fn parasites_continue_the_host_line_cycle_and_join_its_legend() {
        let fig = Figure::new();
        let host = host_subplot(&fig, 1, 1, 1).unwrap();
        let par = host.aux_axes(ParasiteLink::Equal).unwrap();
        let first = host.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        let second = par.plot([0.0, 1.0], [1.0, 0.0]).unwrap();
        first.set_label("host");
        second.set_label("parasite");
        assert_ne!(first.rgba().unwrap(), second.rgba().unwrap());
        let legend = host.legend(LegendOptions::default()).unwrap();
        assert_eq!(legend.labels(), ["host", "parasite"]);
        assert!(!par.spines().right.visible() && !par.xaxis().visible());
    }

    #[test]
    fn removing_a_twin_restores_the_host_side() {
        let fig = Figure::new();
        let host = host_subplot(&fig, 1, 1, 1).unwrap();
        let par = host.twinx();
        par.remove();
        assert_eq!(host.parasites(), Vec::<ParasiteAxes>::new());
        assert!(host.spines().right.visible());
        assert_eq!(
            major_tick_sides(&host, AxisName::Y),
            ((true, true), (true, false))
        );
        host.axis(SpineSide::Right).set_visible(false);
        par.remove();
        assert!(
            !host.spines().right.visible(),
            "removing the twin again leaves the host alone"
        );
    }
}
