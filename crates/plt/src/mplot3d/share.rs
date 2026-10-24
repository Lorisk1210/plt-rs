//! 3-D axes sharing an axis or their view angles (matplotlib's
//! `_shared_axes["x"]`, `["y"]`, `["z"]` and `["view"]` groups, `sharex`,
//! `sharey`, `sharez` and `shareview`).
//!
//! Every member of a group records the whole group. Limits set with
//! `emit` (matplotlib's default) are queued by the axes and replayed on
//! its siblings once the change is done; the tick helpers and the scale of
//! a shared axis are shared objects, as in matplotlib.

use std::rc::Rc;

use super::camera::ViewAngles;
use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::state::Axes3DState;
use crate::artist::{ArtistHandle, ArtistId, ArtistLookup};
use crate::figure::FigureState;

/// What 3-D axes can share.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ShareKey {
    /// One data axis.
    Axis(Axis3),
    /// The view angles and vertical axis.
    View,
}

impl ShareKey {
    const fn index(self) -> usize {
        match self {
            Self::Axis(axis) => axis.index(),
            Self::View => 3,
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

/// The groups an axes belongs to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Shares {
    /// The members of each group (x, y, z, view), this axes included;
    /// empty while it shares nothing.
    groups: [Vec<ArtistId>; 4],
    /// The axes each was explicitly shared with (matplotlib's `_sharex`,
    /// `_sharey`, `_sharez` and `_shareview`).
    targets: [Option<ArtistId>; 4],
}

impl Shares {
    /// The members of the group of `key`, `me` included.
    fn members(&self, key: ShareKey, me: ArtistId) -> Vec<ArtistId> {
        let group = &self.groups[key.index()];
        if group.is_empty() {
            vec![me]
        } else {
            group.clone()
        }
    }

    /// The other members of the group of `key`.
    pub(crate) fn siblings(&self, key: ShareKey, me: ArtistId) -> Vec<ArtistId> {
        self.groups[key.index()]
            .iter()
            .copied()
            .filter(|&id| id != me)
            .collect()
    }
}

impl Axes3DState {
    /// The queued limit changes paired with the axes to replay them on.
    pub(crate) fn take_limit_changes(&mut self) -> Vec<(Vec<ArtistId>, LimitChange)> {
        let me = self.base_id();
        std::mem::take(&mut self.emitted)
            .into_iter()
            .map(|change| {
                (
                    self.shares.siblings(ShareKey::Axis(change.axis), me),
                    change,
                )
            })
            .filter(|(siblings, _)| !siblings.is_empty())
            .collect()
    }
}

/// Replays limit changes on the sibling axes, without emitting further
/// (matplotlib's `_set_lim(..., emit=False)` on the shared axes).
pub(crate) fn replay(figure: &mut FigureState, changes: &[(Vec<ArtistId>, LimitChange)]) {
    for (siblings, change) in changes {
        for &id in siblings {
            if let Some(axes) = figure.store.get_as_mut::<Axes3DState>(id) {
                let (v0, v1) = change.view;
                axes.set_lim(change.axis, v0, v1, change.auto, false);
            }
        }
    }
}

/// Takes the 3-D axes `id` out of the groups sharing a data axis, as
/// removing it from its figure does (matplotlib's
/// `Figure._remove_axes`): limits set on it no longer reach the others,
/// nor theirs it. Groups left with one member dissolve.
pub(crate) fn leave_shares(figure: &mut FigureState, id: ArtistId) {
    let Some(groups) = figure
        .artist_as::<Axes3DState>(id)
        .map(|axes| axes.shares.groups.clone())
    else {
        return;
    };
    for key in Axis3::ALL.map(|axis| ShareKey::Axis(axis).index()) {
        let mut rest: Vec<ArtistId> = groups[key].iter().copied().filter(|&m| m != id).collect();
        if rest.len() < 2 {
            rest.clear();
        }
        for &member in &groups[key] {
            if let Some(axes) = figure.artist_as_mut::<Axes3DState>(member) {
                axes.shares.groups[key] = if member == id {
                    Vec::new()
                } else {
                    rest.clone()
                };
            }
        }
    }
}

impl Axes3D {
    /// The axes of the group of `key`, this one included.
    fn group(&self, key: ShareKey) -> Vec<Axes3D> {
        let ids = self.with(|axes| axes.shares.members(key, axes.base_id()));
        let figure = self.raw().figure_state();
        let state = figure.borrow();
        ids.into_iter()
            .filter(|&id| state.artist_as::<Axes3DState>(id).is_some())
            .map(|id| Axes3D::from_raw(crate::artist::RawHandle::new(Rc::clone(figure), id)))
            .collect()
    }

    /// Applies `f` to this axes and every axes sharing `key` with it.
    pub(crate) fn update_group(&self, key: ShareKey, mut f: impl FnMut(&mut Axes3DState)) {
        for axes in self.group(key) {
            axes.with_mut(&mut f);
        }
    }

    /// Puts this axes and `other` (and everything already sharing `key`
    /// with either) in one group.
    fn join(&self, other: &Axes3D, key: ShareKey) -> Result<(), Axes3DError> {
        if !Rc::ptr_eq(self.raw().figure_state(), other.raw().figure_state()) {
            return Err(Axes3DError::OtherFigure);
        }
        let target = self.with(|axes| axes.shares.targets[key.index()]);
        if target.is_some_and(|id| id != other.id()) {
            return Err(Axes3DError::AlreadyShared(key.describe()));
        }
        let mut members = self.with(|axes| axes.shares.members(key, axes.base_id()));
        for id in other.with(|axes| axes.shares.members(key, axes.base_id())) {
            if !members.contains(&id) {
                members.push(id);
            }
        }
        let mut figure = self.raw().figure_state().borrow_mut();
        for &id in &members {
            if let Some(axes) = figure.artist_as_mut::<Axes3DState>(id) {
                axes.shares.groups[key.index()].clone_from(&members);
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
        self.join(other, ShareKey::Axis(axis))?;
        let (scale, tickers, (v0, v1), auto) = other.with(|axes| {
            let state = axes.axis(axis);
            (
                state.scale.clone(),
                state.tickers.clone(),
                axes.limits.view(axis),
                axes.limits.autoscale[axis.index()],
            )
        });
        self.with_mut(|axes| {
            let state = axes.axis_mut(axis);
            state.tickers = tickers;
            state.scale = scale;
            axes.set_lim3d_emitting(axis, (v0, v1), Some(auto), None, false)
        })?;
        Ok(self)
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
    /// use plt::figure::Figure;
    /// let fig = Figure::new();
    /// let (a, b) = (fig.add_subplot_3d(1, 2, 1, Default::default())?,
    ///               fig.add_subplot_3d(1, 2, 2, Default::default())?);
    /// b.share_z(&a)?;
    /// a.set_zlim(0.0, 5.0)?;
    /// assert_eq!(b.zlim(), (0.0, 5.0));
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
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
        self.join(other, ShareKey::View)?;
        let (angles, vertical) = other.with(|axes| (axes.camera.angles, axes.camera.vertical));
        self.set_view_shared(angles, vertical);
        Ok(self)
    }

    /// Joins the groups `options` asks for when the axes is created
    /// (matplotlib's `sharex`, `sharey`, `sharez` and `shareview`
    /// arguments of `Axes3D`).
    pub(crate) fn join_initial(
        &self,
        axes: [Option<&Axes3D>; 3],
        view: Option<&Axes3D>,
    ) -> Result<(), Axes3DError> {
        for (axis, other) in Axis3::ALL.into_iter().zip(axes) {
            if let Some(other) = other {
                self.share_axis(axis, other)?;
            }
        }
        if let Some(other) = view {
            self.join(other, ShareKey::View)?;
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

    /// The axes sharing one data axis with this one, this one included
    /// (matplotlib's `get_shared_x_axes().get_siblings(ax)`).
    pub fn shared_axes(&self, axis: Axis3) -> Vec<Axes3D> {
        self.group(ShareKey::Axis(axis))
    }

    /// The axes sharing the view angles with this one, this one included.
    pub fn shared_views(&self) -> Vec<Axes3D> {
        self.group(ShareKey::View)
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
        assert!(!c.autoscale(Axis3::Z), "autoscaling follows too");
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
