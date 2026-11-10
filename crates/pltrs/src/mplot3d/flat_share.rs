//! 3-D axes sharing their x or y axis with 2-D axes: matplotlib keeps one
//! `_shared_axes["x"]` and one `["y"]` grouper for every axes, whatever
//! its kind, so a 3-D axes made with `sharex=ax` of a 2-D `ax` (or the
//! other way round) joins the figure's group of that axis, and the
//! members of either kind follow each other's limits, autoscaling, tick
//! helpers and scale. A 3-D axes is an `Axes` in matplotlib, whose 2-D
//! data limits and children the axes of its group autoscale from: in
//! pltrs they are its projection plane's.

use std::rc::Rc;

use super::axis3d::Tickers3D;
use super::coords::Axis3;
use super::handle::Axes3D;
use super::scaling::AxisScale;
use super::state::Axes3DState;
use crate::artist::{ArtistHandle, ArtistId, ArtistLookup, RawHandle};
use crate::axes::{Axes, AxesState, Tickers};
use crate::figure::{FigureCell, FigureState};
use crate::scale::Scale;
use crate::ticker::AxisName;

/// An axes of either kind another axes shares an axis with (matplotlib's
/// `sharex` and `sharey` take any `Axes`, 3-D ones included).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SharedAxes {
    /// A 2-D axes.
    Flat(Axes),
    /// A 3-D axes.
    Deep(Axes3D),
}

impl SharedAxes {
    /// The axes `id` of the figure in `figure`, of the kind it is, while
    /// it is an axes of it.
    pub(crate) fn of(figure: &Rc<FigureCell>, id: ArtistId) -> Option<Self> {
        let state = figure.borrow();
        let raw = RawHandle::new(Rc::clone(figure), id);
        if state.artist_as::<Axes3DState>(id).is_some() {
            Some(Self::Deep(Axes3D::from_raw(raw)))
        } else {
            state
                .artist_as::<AxesState>(id)
                .map(|_| Self::Flat(Axes::from_raw(raw)))
        }
    }
}

impl ArtistHandle for SharedAxes {
    fn raw(&self) -> &RawHandle {
        match self {
            Self::Flat(axes) => axes.raw(),
            Self::Deep(axes) => axes.raw(),
        }
    }
}

impl From<Axes> for SharedAxes {
    fn from(axes: Axes) -> Self {
        Self::Flat(axes)
    }
}

impl From<Axes3D> for SharedAxes {
    fn from(axes: Axes3D) -> Self {
        Self::Deep(axes)
    }
}

/// What an axes sharing an axis takes of the axes it shares it with (see
/// [`FigureState::shared_axis_of`]).
pub(crate) type SharedAxisState = (Tickers, ArtistId, Rc<dyn Scale>, (f64, f64), bool);

/// The 3-D axis matching the 2-D axis `name`.
const fn deep_axis(name: AxisName) -> Axis3 {
    match name {
        AxisName::X => Axis3::X,
        AxisName::Y => Axis3::Y,
    }
}

impl FigureState {
    /// The 2-D state autoscaling reads of axes `id` of either kind: its
    /// own, or the projection plane's of a 3-D axes (matplotlib's
    /// `dataLim` and children of an `Axes3D`).
    pub(crate) fn flat_part(&self, id: ArtistId) -> Option<&AxesState> {
        match self.store.get_as::<Axes3DState>(id) {
            Some(deep) => self.store.get_as::<AxesState>(deep.plane),
            None => self.store.get_as::<AxesState>(id),
        }
    }

    /// Whether axis `name` of the 3-D axes `id` waits to be autoscaled
    /// (matplotlib's `_stale_viewlims`); `None` for an axes of no 3-D
    /// kind.
    pub(crate) fn deep_axis_pending(&self, id: ArtistId, name: AxisName) -> Option<bool> {
        let deep = self.store.get_as::<Axes3DState>(id)?;
        Some(deep.limits.pending[deep_axis(name).index()])
    }

    /// Calls off the autoscaling axis `name` of the 3-D axes `id` waits
    /// for, if it is one.
    pub(crate) fn settle_deep_axis(&mut self, id: ArtistId, name: AxisName) {
        if let Some(deep) = self.artist_as_mut::<Axes3DState>(id) {
            deep.limits.pending[deep_axis(name).index()] = false;
        }
    }

    /// Sets the view of axis `name` of the 3-D axes `id` to `view`, as a
    /// 2-D axes sharing it makes the others follow (matplotlib's
    /// `Axis._set_lim(..., emit=False)`); whether `id` is a 3-D axes.
    pub(crate) fn set_deep_view(
        &mut self,
        id: ArtistId,
        name: AxisName,
        (v0, v1): (f64, f64),
        auto: Option<bool>,
    ) -> bool {
        let Some(deep) = self.artist_as_mut::<Axes3DState>(id) else {
            return false;
        };
        deep.set_lim(deep_axis(name), v0, v1, auto, false);
        true
    }

    /// What an axes sharing axis `name` of the axes `id`, of either kind,
    /// takes of it (matplotlib's `sharex`): its tick helpers, the axes
    /// they are bound to, its scale, its view limits and whether it
    /// autoscales.
    pub(crate) fn shared_axis_of(&self, id: ArtistId, name: AxisName) -> Option<SharedAxisState> {
        if let Some(deep) = self.store.get_as::<Axes3DState>(id) {
            let axis = deep_axis(name);
            let state = deep.axis(axis);
            return Some((
                state.tickers.shared(),
                state.locator_owner.unwrap_or(id),
                Rc::clone(state.scale.scale()),
                deep.limits.view(axis),
                deep.limits.autoscale[axis.index()],
            ));
        }
        let source = self.artist_as::<AxesState>(id)?;
        let axis = source.axis(name);
        Some((
            axis.tickers(),
            axis.ticker_owner.unwrap_or(id),
            axis.scale_rc(),
            source.view_interval(name),
            axis.autoscale.enabled,
        ))
    }

    /// Gives the 3-D axes sharing axis `name` with the 2-D axes `from` its
    /// tick helpers and scale as they are (matplotlib's shared axes hold
    /// one `Ticker` and one scale).
    pub(crate) fn lend_flat_tickers(&mut self, from: ArtistId, name: AxisName) {
        let Some(source) = self.store.get_as::<AxesState>(from) else {
            return;
        };
        let axis = source.axis(name);
        let (tickers, scale) = (axis.tickers(), axis.scale_rc());
        let owner = axis.ticker_owner.unwrap_or(from);
        for id in self.siblings(name, from) {
            if let Some(deep) = self.artist_as_mut::<Axes3DState>(id) {
                let state = deep.axis_mut(deep_axis(name));
                state.tickers.take_shared(&tickers);
                state.scale = AxisScale::new(Rc::clone(&scale));
                state.locator_owner = Some(owner);
            }
        }
    }

    /// Gives the 2-D axes sharing `axis` (x or y) with the 3-D axes `from`
    /// its tick helpers and scale as they are.
    pub(crate) fn lend_deep_tickers(&mut self, from: ArtistId, axis: Axis3) {
        let name = match axis {
            Axis3::X => AxisName::X,
            Axis3::Y => AxisName::Y,
            Axis3::Z => return,
        };
        let Some(source) = self.store.get_as::<Axes3DState>(from) else {
            return;
        };
        let state = source.axis(axis);
        let tickers = state.tickers.shared();
        let scale = Rc::clone(state.scale.scale());
        let owner = state.locator_owner.unwrap_or(from);
        for id in self.siblings(name, from) {
            if let Some(flat) = self.artist_as_mut::<AxesState>(id) {
                let axis = flat.axis_mut(name);
                axis.use_tickers(tickers.clone());
                axis.adopt_scale(Rc::clone(&scale));
                axis.ticker_owner = Some(owner);
            }
        }
    }
}

impl Axes3D {
    /// Takes axis `axis` of the 2-D axes `other`, whose group (`name`)
    /// this axes joined: its tick helpers, its limits, which `other` gives
    /// once its pending autoscaling is done, its autoscaling and its scale
    /// (matplotlib's `sharex` of an `Axes3D`, through `set_xlim(...,
    /// emit=False)`).
    pub(crate) fn take_flat_axis(&self, axis: Axis3, name: AxisName, other: &Axes) {
        let figure = Rc::clone(self.raw().figure_state());
        figure.work_on(|state| state.unstale_axes(other.id()));
        let taken = {
            let state = figure.borrow();
            state.artist_as::<AxesState>(other.id()).map(|source| {
                let flat = source.axis(name);
                (
                    flat.tickers(),
                    flat.scale_rc(),
                    flat.ticker_owner.unwrap_or_else(|| other.id()),
                    source.view_interval(name),
                    flat.autoscale.enabled,
                )
            })
        };
        let Some((tickers, scale, owner, view, auto)) = taken else {
            return;
        };
        self.with_mut(|axes| {
            let state = axes.axis_mut(axis);
            state.tickers.take_shared(&tickers);
            state.locator_owner = Some(owner);
            axes.set_lim(axis, view.0, view.1, Some(auto), false);
            axes.axis_mut(axis).scale = AxisScale::new(scale);
        });
    }
}

impl Tickers3D {
    /// Uses the tick helpers of a 2-D axis sharing this one (matplotlib's
    /// `sharex` assigning the `major` and `minor` tickers).
    fn take_shared(&mut self, tickers: &Tickers) {
        self.major_locator = Rc::clone(&tickers.major_locator);
        self.major_formatter = Rc::clone(&tickers.major_formatter);
        self.minor_locator = Rc::clone(&tickers.minor_locator);
        self.minor_formatter = Rc::clone(&tickers.minor_formatter);
    }

    /// The tick helpers, for a 2-D axis sharing this one.
    fn shared(&self) -> Tickers {
        Tickers {
            major_locator: Rc::clone(&self.major_locator),
            major_formatter: Rc::clone(&self.major_formatter),
            minor_locator: Rc::clone(&self.minor_locator),
            minor_formatter: Rc::clone(&self.minor_formatter),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;
    use crate::mplot3d::{Axes3DOptions, Axis3, SharedAxes};
    use crate::ticker::{AxisName, MultipleLocator};

    #[test]
    fn three_d_axes_share_with_two_d_ones_both_ways() {
        let fig = Figure::new();
        let flat = fig.add_subplot(1, 3, 1).unwrap();
        flat.plot([0.0, 10.0], [0.0, 4.0]).unwrap();
        let options = Axes3DOptions {
            sharex: Some(flat.clone().into()),
            sharey: Some(flat.clone().into()),
            ..Axes3DOptions::default()
        };
        let deep = fig.add_subplot_3d(1, 3, 2, options).unwrap();
        // matplotlib 3.11: the 3-D axes autoscales its unit data box when
        // it is made, which the 2-D axes follows.
        let unit = (-0.020_833_333_333_333_34, 1.020_833_333_333_333_3);
        assert_eq!(flat.xlim(), unit);
        assert_eq!(deep.xlim(), unit);
        deep.plot([0.0, 48.0], [0.0, 48.0], [0.0, 1.0]).unwrap();
        assert_eq!(flat.ylim(), deep.ylim(), "{:?}", deep.ylim());
        flat.set_xlim(2.0, 3.0).unwrap();
        assert_eq!(deep.xlim(), (2.0, 3.0));
        assert_eq!(deep.sharing(Axis3::X).len(), 2);
        assert_eq!(
            deep.share_target(Axis3::Y),
            Some(SharedAxes::Flat(flat.clone()))
        );
        let other = fig.add_subplot(1, 3, 3).unwrap();
        other.share_with(AxisName::X, &deep.clone().into()).unwrap();
        assert_eq!(other.xlim(), (2.0, 3.0));
        deep.set_xlim(0.0, 6.0).unwrap();
        assert_eq!((flat.xlim(), other.xlim()), ((0.0, 6.0), (0.0, 6.0)));
        other
            .xaxis()
            .set_major_locator(MultipleLocator::new(3.0).unwrap());
        let locs: Vec<f64> = (deep.axis(Axis3::X).ticks().unwrap().major.iter())
            .map(|t| t.loc)
            .collect();
        assert_eq!(locs, [0.0, 3.0, 6.0], "the locator is shared");
    }
}
