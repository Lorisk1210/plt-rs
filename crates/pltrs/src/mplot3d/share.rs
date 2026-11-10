//! 3-D axes sharing an axis or their view angles (matplotlib's
//! `_shared_axes["x"]`, `["y"]`, `["z"]` and `["view"]` groups, `sharex`,
//! `sharey`, `sharez` and `shareview`).
//!
//! The x and y groups are the figure's, which 2-D axes are in too (see
//! [`super::flat_share`]); every member of a z or view group records the
//! whole group. Limits set with `emit` (matplotlib's default) are queued
//! by the axes and replayed on its siblings once the change is done; the
//! tick helpers and the scale of a shared axis are shared objects, as in
//! matplotlib.

use std::rc::Rc;

use super::camera::ViewAngles;
use super::coords::Axis3;
use super::error::Axes3DError;
use super::flat_share::SharedAxes;
use super::handle::Axes3D;
use super::state::Axes3DState;
use crate::artist::{ArtistHandle, ArtistId, ArtistLookup};
use crate::axes::AxesState;
use crate::figure::FigureState;
use crate::ticker::AxisName;

/// What 3-D axes can share.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ShareKey {
    /// One data axis.
    Axis(Axis3),
    /// The view angles and vertical axis.
    View,
}

impl ShareKey {
    /// The slot of the share in [`Shares::targets`].
    const fn index(self) -> usize {
        match self {
            Self::Axis(axis) => axis.index(),
            Self::View => 3,
        }
    }

    /// The figure's group of axes of either kind this share joins, for x
    /// and y; `None` for the groups of 3-D axes only (z and the view).
    const fn flat(self) -> Option<AxisName> {
        match self {
            Self::Axis(Axis3::X) => Some(AxisName::X),
            Self::Axis(Axis3::Y) => Some(AxisName::Y),
            Self::Axis(Axis3::Z) | Self::View => None,
        }
    }

    /// The slot of a group of 3-D axes only in [`Shares::groups`].
    const fn deep_slot(self) -> usize {
        match self {
            Self::View => 1,
            Self::Axis(_) => 0,
        }
    }

    /// What matplotlib's error message calls the shared thing.
    const fn describe(self) -> &'static str {
        match self {
            Self::Axis(Axis3::X) => "x-axis",
            Self::Axis(Axis3::Y) => "y-axis",
            Self::Axis(Axis3::Z) => "z-axis",
            Self::View => "view angles",
        }
    }
}

/// New view limits of one axis, to be replayed on the axes sharing it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LimitChange {
    pub(crate) axis: Axis3,
    /// The view interval set (decreasing when inverted).
    pub(crate) view: (f64, f64),
    /// The autoscaling change that came with it.
    pub(crate) auto: Option<bool>,
}

pltrs_core::archive_struct!(LimitChange { axis, view, auto });

/// The groups of 3-D axes only an axes belongs to (z and the view), and
/// the axes it shares each thing with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Shares {
    /// The members of the z group and of the view group, this axes
    /// included; empty while it shares nothing.
    groups: [Vec<ArtistId>; 2],
    /// The axes each was explicitly shared with (matplotlib's `_sharex`,
    /// `_sharey`, `_sharez` and `_shareview`): x and y with an axes of
    /// either kind.
    targets: [Option<ArtistId>; 4],
}

pltrs_core::archive_struct!(Shares { groups, targets });

impl Shares {
    /// The members of the z or view group of `key`, `me` included.
    fn members(&self, key: ShareKey, me: ArtistId) -> Vec<ArtistId> {
        let group = &self.groups[key.deep_slot()];
        if group.is_empty() {
            vec![me]
        } else {
            group.clone()
        }
    }
}

impl FigureState {
    /// The axes, of either kind, sharing `key` with the 3-D axes `me`, `me`
    /// included.
    pub(crate) fn deep_siblings(&self, key: ShareKey, me: ArtistId) -> Vec<ArtistId> {
        match key.flat() {
            Some(name) => self.siblings(name, me),
            None => self
                .artist_as::<Axes3DState>(me)
                .map_or_else(|| vec![me], |axes| axes.shares.members(key, me)),
        }
    }
}

/// Replays the limit changes of the 3-D axes `source` on the axes sharing
/// them, without emitting further (matplotlib's `_set_lim(...,
/// emit=False)` on the shared axes): the 3-D ones take them as they are,
/// the 2-D ones as their own limits, their callbacks queued.
pub(crate) fn replay(figure: &mut FigureState, source: ArtistId, changes: &[LimitChange]) {
    for change in changes {
        let key = ShareKey::Axis(change.axis);
        for id in figure.deep_siblings(key, source) {
            if id == source {
                continue;
            }
            let (v0, v1) = change.view;
            if let Some(axes) = figure.store.get_as_mut::<Axes3DState>(id) {
                axes.set_lim(change.axis, v0, v1, change.auto, false);
            } else if let Some(name) = key.flat()
                && figure.artist_as::<AxesState>(id).is_some()
            {
                figure.set_axis_view(id, name, (v0, v1), change.auto, false);
                figure.limit_callbacks.notify(id, name);
            }
        }
    }
}

/// Takes the 3-D axes `id` out of the groups of 3-D axes sharing their z
/// axis, as removing it from its figure does (matplotlib's
/// `Figure._remove_axes`; the x and y groups are the figure's, which it
/// leaves as 2-D axes do): limits set on it no longer reach the others,
/// nor theirs it. Groups left with one member dissolve.
pub(crate) fn leave_shares(figure: &mut FigureState, id: ArtistId) {
    let key = ShareKey::Axis(Axis3::Z);
    let Some(group) = figure
        .artist_as::<Axes3DState>(id)
        .map(|axes| axes.shares.groups[key.deep_slot()].clone())
    else {
        return;
    };
    let mut rest: Vec<ArtistId> = group.iter().copied().filter(|&m| m != id).collect();
    if rest.len() < 2 {
        rest.clear();
    }
    for &member in &group {
        if let Some(axes) = figure.artist_as_mut::<Axes3DState>(member) {
            axes.shares.groups[key.deep_slot()] = if member == id {
                Vec::new()
            } else {
                rest.clone()
            };
        }
    }
}

impl Axes3D {
    /// The 3-D axes of the group of `key`, this one included.
    fn group(&self, key: ShareKey) -> Vec<Axes3D> {
        let figure = self.raw().figure_state();
        let state = figure.borrow();
        state
            .deep_siblings(key, self.id())
            .into_iter()
            .filter(|&id| state.artist_as::<Axes3DState>(id).is_some())
            .map(|id| Axes3D::from_raw(crate::artist::RawHandle::new(Rc::clone(figure), id)))
            .collect()
    }

    /// Applies `f` to this axes and every 3-D axes sharing `key` with it;
    /// the 2-D axes sharing an x or y axis take its tick helpers and
    /// scale as they are then (matplotlib's shared axes hold one `Ticker`
    /// and one scale).
    pub(crate) fn update_group(&self, key: ShareKey, mut f: impl FnMut(&mut Axes3DState)) {
        for axes in self.group(key) {
            axes.with_mut(&mut f);
        }
        if let ShareKey::Axis(axis) = key
            && key.flat().is_some()
        {
            self.raw()
                .figure_state()
                .borrow_mut()
                .lend_deep_tickers(self.id(), axis);
        }
    }

    /// Puts this axes and `other`, of either kind (and everything already
    /// sharing `key` with either), in one group.
    fn join(&self, other: &SharedAxes, key: ShareKey) -> Result<(), Axes3DError> {
        if !Rc::ptr_eq(self.raw().figure_state(), other.raw().figure_state()) {
            return Err(Axes3DError::OtherFigure);
        }
        let target = self.with(|axes| axes.shares.targets[key.index()]);
        if target.is_some_and(|id| id != other.id()) {
            return Err(Axes3DError::AlreadyShared(key.describe()));
        }
        let mut figure = self.raw().figure_state().borrow_mut();
        if let Some(name) = key.flat() {
            figure.shared.join(name, self.id(), other.id());
        } else {
            let mut members = figure.deep_siblings(key, self.id());
            for id in figure.deep_siblings(key, other.id()) {
                if !members.contains(&id) {
                    members.push(id);
                }
            }
            for &id in &members {
                if let Some(axes) = figure.artist_as_mut::<Axes3DState>(id) {
                    axes.shares.groups[key.deep_slot()].clone_from(&members);
                }
            }
        }
        if let Some(axes) = figure.artist_as_mut::<Axes3DState>(self.id()) {
            axes.shares.targets[key.index()] = Some(other.id());
        }
        Ok(())
    }

    /// Shares one data axis with `other`: the tick helpers, the scale and
    /// the limits, which `other` gives first (matplotlib's `sharex`,
    /// `sharey` and `sharez`).
    ///
    /// # Errors
    ///
    /// When the axis is already shared with another axes, or `other`
    /// belongs to another figure.
    pub fn share_axis(&self, axis: Axis3, other: &Axes3D) -> Result<&Self, Axes3DError> {
        self.join(&SharedAxes::Deep(other.clone()), ShareKey::Axis(axis))?;
        let (scale, tickers, owner, (v0, v1), auto) = other.with(|axes| {
            let state = axes.axis(axis);
            (
                state.scale.clone(),
                state.tickers.clone(),
                state.locator_owner.unwrap_or_else(|| other.id()),
                axes.limits.view(axis),
                axes.limits.autoscale[axis.index()],
            )
        });
        self.with_mut(|axes| {
            let state = axes.axis_mut(axis);
            state.tickers = tickers;
            state.locator_owner = Some(owner);
            state.scale = scale;
            axes.set_lim3d_emitting(axis, (v0, v1), Some(auto), None, false)
        })?;
        Ok(self)
    }

    /// Shares one data axis with `other`, a 2-D or a 3-D axes (z only with
    /// a 3-D one), as [`Axes3D::share_axis`] does: matplotlib's `sharex`
    /// and `sharey` take any `Axes`.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let fig = Figure::new();
    /// let flat = fig.add_subplot(1, 2, 1)?;
    /// let deep = fig.add_subplot_3d(1, 2, 2, Default::default())?;
    /// flat.set_xlim(2.0, 3.0)?;
    /// deep.share_axis_with(Axis3::X, &flat.clone().into())?;
    /// assert_eq!(deep.xlim(), (2.0, 3.0));
    /// deep.set_xlim(0.0, 5.0)?;
    /// assert_eq!(flat.xlim(), (0.0, 5.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Axes3D::share_axis`], and when a 2-D axes is given for z
    /// ([`Axes3DError::NoFlatAxis`]).
    pub fn share_axis_with(&self, axis: Axis3, other: &SharedAxes) -> Result<&Self, Axes3DError> {
        match other {
            SharedAxes::Deep(other) => self.share_axis(axis, other),
            SharedAxes::Flat(other) => {
                let key = ShareKey::Axis(axis);
                let name = key.flat().ok_or(Axes3DError::NoFlatAxis(axis))?;
                self.join(&SharedAxes::Flat(other.clone()), key)?;
                self.take_flat_axis(axis, name, other);
                Ok(self)
            }
        }
    }

    /// Shares the x axis with `other` (matplotlib's `sharex`).
    ///
    /// # Errors
    ///
    /// As [`Axes3D::share_axis`].
    pub fn share_x(&self, other: &Axes3D) -> Result<&Self, Axes3DError> {
        self.share_axis(Axis3::X, other)
    }

    /// Shares the y axis with `other` (matplotlib's `sharey`).
    ///
    /// # Errors
    ///
    /// As [`Axes3D::share_axis`].
    pub fn share_y(&self, other: &Axes3D) -> Result<&Self, Axes3DError> {
        self.share_axis(Axis3::Y, other)
    }

    /// Shares the z axis with `other` (matplotlib's `sharez`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let (a, b) = (fig.add_subplot_3d(1, 2, 1, Default::default())?,
    ///               fig.add_subplot_3d(1, 2, 2, Default::default())?);
    /// b.share_z(&a)?;
    /// a.set_zlim(0.0, 5.0)?;
    /// assert_eq!(b.zlim(), (0.0, 5.0));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Axes3D::share_axis`].
    pub fn share_z(&self, other: &Axes3D) -> Result<&Self, Axes3DError> {
        self.share_axis(Axis3::Z, other)
    }

    /// Shares the view angles with `other`, taking its angles and vertical
    /// axis (matplotlib's `shareview`); rotating either with the mouse
    /// turns both.
    ///
    /// # Errors
    ///
    /// When the view is already shared with another axes, or `other`
    /// belongs to another figure.
    pub fn share_view(&self, other: &Axes3D) -> Result<&Self, Axes3DError> {
        self.join(&SharedAxes::Deep(other.clone()), ShareKey::View)?;
        let (angles, vertical) = other.with(|axes| (axes.camera.angles, axes.camera.vertical));
        self.set_view_shared(angles, vertical);
        Ok(self)
    }

    /// Joins the groups `options` asks for when the axes is created
    /// (matplotlib's `sharex`, `sharey`, `sharez` and `shareview`
    /// arguments of `Axes3D`).
    pub(crate) fn join_initial(
        &self,
        axes: [Option<&SharedAxes>; 3],
        view: Option<&Axes3D>,
    ) -> Result<(), Axes3DError> {
        for (axis, other) in Axis3::ALL.into_iter().zip(axes) {
            if let Some(other) = other {
                self.share_axis_with(axis, other)?;
            }
        }
        if let Some(other) = view {
            self.join(&SharedAxes::Deep(other.clone()), ShareKey::View)?;
        }
        Ok(())
    }

    /// Sets the view angles of every axes sharing the view with this one
    /// and gives them this axes' vertical axis (matplotlib's
    /// `view_init(..., share=True)`).
    pub fn view_init_shared(&self, elev: f64, azim: f64, roll: f64) -> &Self {
        let vertical = self.with(|axes| axes.camera.vertical);
        self.set_view_shared(ViewAngles { elev, azim, roll }, vertical);
        self
    }

    fn set_view_shared(&self, angles: ViewAngles, vertical: Axis3) {
        self.update_group(ShareKey::View, |axes| {
            axes.camera.angles = angles;
            axes.camera.vertical = vertical;
        });
    }

    /// The axes `axis` was explicitly shared with, of either kind, if it
    /// still is in the figure (matplotlib's `_sharex`, `_sharey` and
    /// `_sharez`: the axes given to [`Axes3D::share_axis_with`] or when
    /// this one was made).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::{Axis3, SharedAxes};
    /// let fig = Figure::new();
    /// let (a, b) = (fig.add_subplot_3d(1, 2, 1, Default::default())?,
    ///               fig.add_subplot_3d(1, 2, 2, Default::default())?);
    /// b.share_z(&a)?;
    /// assert_eq!(b.share_target(Axis3::Z), Some(SharedAxes::Deep(a.clone())));
    /// assert_eq!(a.share_target(Axis3::Z), None, "a shares nothing itself");
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    pub fn share_target(&self, axis: Axis3) -> Option<SharedAxes> {
        let id = self.with(|axes| axes.shares.targets[ShareKey::Axis(axis).index()])?;
        SharedAxes::of(self.raw().figure_state(), id)
    }

    /// The axes the view angles were explicitly shared with, if it still
    /// is in the figure (matplotlib's `_shareview`).
    pub fn view_share_target(&self) -> Option<Axes3D> {
        let id = self.with(|axes| axes.shares.targets[ShareKey::View.index()])?;
        match SharedAxes::of(self.raw().figure_state(), id)? {
            SharedAxes::Deep(axes) => Some(axes),
            SharedAxes::Flat(_) => None,
        }
    }

    /// The 3-D axes sharing one data axis with this one, this one
    /// included (matplotlib's `get_shared_x_axes().get_siblings(ax)`
    /// without the 2-D axes; [`Axes3D::sharing`] lists them all).
    pub fn shared_axes(&self, axis: Axis3) -> Vec<Axes3D> {
        self.group(ShareKey::Axis(axis))
    }

    /// The axes of either kind sharing one data axis with this one, this
    /// one included, in the order they joined (matplotlib's
    /// `get_shared_x_axes().get_siblings(ax)`).
    pub fn sharing(&self, axis: Axis3) -> Vec<SharedAxes> {
        let figure = self.raw().figure_state();
        let ids = figure
            .borrow()
            .deep_siblings(ShareKey::Axis(axis), self.id());
        ids.into_iter()
            .filter_map(|id| SharedAxes::of(figure, id))
            .collect()
    }

    /// The axes sharing the view angles with this one, this one included.
    pub fn shared_views(&self) -> Vec<Axes3D> {
        self.group(ShareKey::View)
    }
}

impl Axes3DState {
    /// The limit changes queued for the axes sharing them (see
    /// [`replay`]).
    pub(crate) fn take_limit_changes(&mut self) -> Vec<LimitChange> {
        std::mem::take(&mut self.emitted)
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;
    use crate::mplot3d::{Axes3DError, Axis3};

    #[test]
    fn shared_limits_follow_and_groups_merge() {
        let fig = Figure::new();
        let a = fig.subplot_3d();
        let b = fig.subplot_3d();
        let c = fig.subplot_3d();
        b.share_z(&a).unwrap();
        c.share_z(&b).unwrap();
        a.set_zlim(-3.0, 4.0).unwrap();
        assert_eq!(c.zlim(), (-3.0, 4.0));
        assert!(!c.autoscale_on(Axis3::Z), "autoscaling follows too");
        assert_eq!(a.shared_axes(Axis3::Z).len(), 3);
        assert_eq!(a.shared_axes(Axis3::X).len(), 1);
        let d = fig.subplot_3d();
        assert_eq!(
            b.share_z(&d).unwrap_err(),
            Axes3DError::AlreadyShared("z-axis")
        );
        assert!(
            b.share_z(&a).is_ok(),
            "sharing again with the same axes is fine"
        );
        let other = Figure::new().subplot_3d();
        assert_eq!(a.share_x(&other).unwrap_err(), Axes3DError::OtherFigure);
        let flat = fig.subplot();
        assert_eq!(
            a.share_axis_with(Axis3::Z, &flat.into()).unwrap_err(),
            Axes3DError::NoFlatAxis(Axis3::Z)
        );
    }

    #[test]
    fn shared_views_turn_together() {
        let fig = Figure::new();
        let a = fig.subplot_3d();
        let b = fig.subplot_3d();
        a.view_init(10.0, 20.0, 30.0);
        b.share_view(&a).unwrap();
        assert_eq!(b.view(), a.view());
        b.view_init_shared(45.0, -10.0, 0.0);
        assert_eq!(a.view().elev, 45.0);
        b.view_init(5.0, 5.0, 5.0);
        assert_eq!(a.view().elev, 45.0, "a plain view_init only turns one axes");
    }
}
