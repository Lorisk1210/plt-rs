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

use plt_core::{Affine2D, Bbox};

use super::mpl_axes::SimpleAxisArtist;
use crate::artist::{ArtistHandle, ArtistId, ArtistLookup, RawHandle};
use crate::axes::{Axes, AxesState, AxisName};
use crate::axisartist::Toggle;
use crate::figure::{AxesRectError, Figure, FigureState, insert_axes};
use crate::gridspec::GridSpecError;
use crate::spines::SpineSide;

/// How a parasite's coordinates relate to its host's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParasiteLink {
    /// Own data coordinates and view limits (matplotlib's
    /// `get_aux_axes(viewlim_mode=None)`).
    Independent,
    /// Own data coordinates, view limits equal to the host's
    /// (`viewlim_mode="equal"`, the default of `get_aux_axes`).
    Equal,
    /// Data coordinates mapped onto the host's by an affine transform, so
    /// the view limits are the host's mapped back
    /// (`get_aux_axes(tr, viewlim_mode="transform")`, `twin(aux_trans)`).
    /// The transform may scale and translate, not rotate or shear.
    Transform(Affine2D),
}

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
/// host draws it, at the host's position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Parasite {
    /// The host axes.
    host: ArtistId,
    /// How its coordinates relate to the host's.
    link: ParasiteLink,
    /// The map from the host's view limits to its own, when they follow.
    view_map: Option<Affine2D>,
    /// Whether it was made by `twin`, `twinx` or `twiny` (removing it then
    /// shows the host's far sides again).
    twin: bool,
}

/// An axes that can carry parasite axes (matplotlib's `HostAxes`, made by
/// `host_subplot` and `host_axes`). It dereferences to [`Axes`].
///
/// ```
/// use plt::axes_grid1::host_subplot;
/// use plt::prelude::*;
/// let fig = Figure::new();
/// let host = host_subplot(&fig, 1, 1, 1)?;
/// let par = host.twinx();
/// host.plot([0.0, 1.0, 2.0], [0.0, 1.0, 2.0])?.set_label("Density");
/// par.plot([0.0, 1.0, 2.0], [0.0, 3.0, 2.0])?.set_label("Temperature");
/// let legend = host.legend(plt::legend::LegendOptions::default())?;
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
        self.add_shared_twin(AxisName::X)
    }

    /// A parasite sharing the y axis, with its x axis on top, where the
    /// host's is hidden (matplotlib's `HostAxes.twiny`).
    pub fn twiny(&self) -> ParasiteAxes {
        self.add_shared_twin(AxisName::Y)
    }

    /// A parasite sharing no axis, whose data coordinates map onto the
    /// host's through `aux` (`None` = the identity) and whose axes are on
    /// the top and right, where the host's are hidden (matplotlib's
    /// `HostAxes.twin`). Its view limits are the host's mapped back.
    ///
    /// ```
    /// use plt::axes_grid1::host_axes;
    /// use plt::prelude::*;
    /// use plt::transforms::Affine2D;
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
        let link = ParasiteLink::Transform(aux.unwrap_or(Affine2D::IDENTITY));
        let view_map = link.view_map()?;
        let twin = self.add_parasite(link, view_map, true);
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
    fn add_shared_twin(&self, shared: AxisName) -> ParasiteAxes {
        let twin = self.add_parasite(ParasiteLink::Independent, None, true);
        twin.share(shared, &self.axes)
            .expect("a new axes of this figure shares nothing yet");
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
        let axes = insert_axes(figure, self.axes.id(), Bbox::UNIT, None);
        let mut state = figure.borrow_mut();
        state.current_axes = current;
        let decorations = state
            .artist_as_mut::<AxesState>(axes.id())
            .map(|parasite| {
                parasite.parasite = Some(Parasite {
                    host: self.axes.id(),
                    link,
                    view_map,
                    twin,
                });
                parasite.decorations.frame_on = false;
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
            if let Some(artist) = state.artist_mut(id) {
                artist.base_mut().visible = false;
            }
        }
        drop(state);
        ParasiteAxes { axes }
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

impl Axes {
    /// The axes whose line property cycle this one draws from: the host
    /// of a parasite (matplotlib's `_get_lines = parent._get_lines`),
    /// itself otherwise.
    pub(crate) fn line_cycle_owner(&self) -> Self {
        let host = self
            .raw()
            .with::<AxesState, _>(|state| state.parasite.map(|parasite| parasite.host));
        host.map_or_else(
            || self.clone(),
            |host| Self::from_raw(RawHandle::new(Rc::clone(self.raw().figure_state()), host)),
        )
    }
}

impl FigureState {
    /// Brings the view limits of a parasite that follows its host up to
    /// date: the host's, mapped back through the aux transform
    /// (matplotlib's `_sync_lims`, which runs when the host's limits
    /// change; here whenever the parasite's limits are needed, so a twin
    /// made after the host's limits were set follows them too). Does
    /// nothing for other axes.
    pub(crate) fn sync_parasite(&mut self, axes: ArtistId) {
        let Some((host, view_map)) = self
            .store
            .get_as::<AxesState>(axes)
            .and_then(|state| state.parasite)
            .and_then(|parasite| Some((parasite.host, parasite.view_map?)))
        else {
            return;
        };
        self.unstale_axes(host);
        let Some(view) = self
            .store
            .get_as::<AxesState>(host)
            .map(|state| state.view_lim)
        else {
            return;
        };
        let mapped = Bbox::new(
            view_map.transform_point(view.p0),
            view_map.transform_point(view.p1),
        );
        if let Some(state) = self.store.get_as_mut::<AxesState>(axes) {
            state.view_lim = mapped;
            state.view_mutated = (true, true);
            state.xaxis.autoscale.stale = false;
            state.yaxis.autoscale.stale = false;
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
    fn twin_follows_the_host_through_the_aux_transform() {
        let fig = Figure::new();
        let host = host_axes(&fig, [0.1, 0.1, 0.8, 0.8]).expect("a valid rectangle");
        let twin = host.twin(Some(Affine2D::scaling(2.0, 0.5))).unwrap();
        host.set_xlim(0.0, 4.0)
            .and_then(|ax| ax.set_ylim(-1.0, 1.0))
            .expect("finite limits");
        assert_eq!(twin.xlim(), (0.0, 2.0));
        assert_eq!(twin.ylim(), (-2.0, 2.0));
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
