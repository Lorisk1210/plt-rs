//! Axes sharing an axis (matplotlib's `sharex` and `sharey`).

use std::rc::Rc;

use super::{Axes, AxesState, AxisName, PlotError};
use crate::artist::{ArtistHandle, ArtistId, ArtistLookup, RawHandle};
use crate::mplot3d::SharedAxes;
use crate::projections::InitialShares;

impl Axes {
    /// Shares the x axis with `other`: both then show the same x range
    /// and ticks, autoscaled from the data of both, and changing the
    /// limits or scale of one changes all (matplotlib's `sharex`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let (top, bottom) = (fig.add_subplot(2, 1, 1)?, fig.add_subplot(2, 1, 2)?);
    /// bottom.share_x(&top)?;
    /// top.plot([0.0, 1.0], [0.0, 1.0])?;
    /// bottom.plot([3.0, 4.0], [0.0, 1.0])?;
    /// assert_eq!(bottom.xlim(), top.xlim());
    /// assert_eq!(top.xlim(), (-0.2, 4.2));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the x axis is already shared with a different axes, or this
    /// axes has a projection fixing its limits, such as a geographic one
    /// ([`PlotError::FixedLimits`]; matplotlib's geographic axes raise a
    /// `TypeError` when they share). `other` may be an axes of another
    /// figure: the two figures then follow each other's limits, scales and
    /// tick helpers, each laid out and drawn by itself
    /// ([`Axes::shared_across`]).
    pub fn share_x(&self, other: &Self) -> Result<&Self, PlotError> {
        self.share(AxisName::X, other)
    }

    /// Records the axes this axes is made sharing, `shares`, without
    /// joining their groups yet, as matplotlib's `__init__` keeps
    /// `sharex=` and `sharey=` until its first clear joins them: the
    /// [`clear`](Axes::clear) that follows joins them (axes made outside
    /// the figure for a class' constructor, which the class then clears).
    /// Partners of another figure are left to share later.
    pub fn give_shares(&self, shares: &InitialShares) {
        let figure = self.raw().figure_state();
        let partner = |other: &Option<SharedAxes>| {
            other
                .as_ref()
                .map(SharedAxes::raw)
                .filter(|raw| Rc::ptr_eq(figure, raw.figure_state()))
                .map(RawHandle::id)
        };
        let (x, y) = (partner(&shares.x), partner(&shares.y));
        self.raw.with_mut::<AxesState, _>(|state| {
            state.shares = (x.or(state.shares.0), y.or(state.shares.1));
        });
    }

    /// Shares the y axis with `other` (matplotlib's `sharey`); see
    /// [`Axes::share_x`].
    ///
    /// # Errors
    ///
    /// As [`Axes::share_x`].
    pub fn share_y(&self, other: &Self) -> Result<&Self, PlotError> {
        self.share(AxisName::Y, other)
    }

    /// Shares axis `name` with `other`, a 2-D or a 3-D axes (matplotlib's
    /// `sharex` and `sharey` take any `Axes`): a 3-D one gives its limits
    /// as they are, then follows those of the group as this axes does.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::ticker::AxisName;
    /// let fig = Figure::new();
    /// let deep = fig.add_subplot_3d(1, 2, 1, Default::default())?;
    /// let flat = fig.add_subplot(1, 2, 2)?;
    /// deep.set_ylim(-1.0, 1.0)?;
    /// flat.share_with(AxisName::Y, &deep.clone().into())?;
    /// assert_eq!(flat.ylim(), (-1.0, 1.0));
    /// assert_eq!(flat.sharing(AxisName::Y).len(), 2);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Axes::share_x`].
    pub fn share_with(&self, name: AxisName, other: &SharedAxes) -> Result<&Self, PlotError> {
        self.share_id(name, other.raw())
    }

    /// Puts this axes in the group of axes sharing axis `name` with
    /// `other`, an axes of either kind, without taking over its tick
    /// helpers, limits or scale (matplotlib's `_shared_axes[name].join`,
    /// which programs call to share limits alone): setting the limits of
    /// one sets those of the group, and autoscaling takes the data of all.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::ticker::AxisName;
    /// let fig = Figure::new();
    /// let (a, b) = (fig.add_subplot(2, 1, 1)?, fig.add_subplot(2, 1, 2)?);
    /// a.set_xlim(0.0, 5.0)?;
    /// b.join_sharing(AxisName::X, &a.clone().into())?;
    /// assert_eq!(b.xlim(), (0.0, 1.0), "the limits are not taken over");
    /// a.set_xlim(1.0, 2.0)?;
    /// assert_eq!(b.xlim(), (1.0, 2.0));
    /// assert_eq!(b.shared_from(AxisName::X), None);
    /// b.leave_sharing(AxisName::X);
    /// assert_eq!(b.sharing(AxisName::X).len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`PlotError::ForeignAxes`] when `other` belongs to another figure.
    pub fn join_sharing(&self, name: AxisName, other: &SharedAxes) -> Result<&Self, PlotError> {
        let figure = self.raw.figure_state();
        if !Rc::ptr_eq(figure, other.raw().figure_state()) {
            return Err(PlotError::ForeignAxes);
        }
        let mut state = figure.borrow_mut();
        state.shared.join(name, self.id(), other.raw().id());
        state.stale = true;
        Ok(self)
    }

    /// Takes this axes out of the group sharing axis `name`, which keeps
    /// the others (matplotlib's `_shared_axes[name].remove`).
    pub fn leave_sharing(&self, name: AxisName) -> &Self {
        let mut state = self.raw.figure_state().borrow_mut();
        state.shared.leave(name, self.id());
        state.stale = true;
        self
    }

    /// The axes of either kind sharing axis `name` with this one, this one
    /// included (matplotlib's `get_shared_x_axes().get_siblings(ax)`;
    /// [`Axes::shared_axes`] lists the 2-D ones).
    pub fn sharing(&self, name: AxisName) -> Vec<SharedAxes> {
        let figure = self.raw.figure_state();
        let ids = figure.borrow().siblings(name, self.id());
        ids.into_iter()
            .filter_map(|id| SharedAxes::of(figure, id))
            .collect()
    }

    /// Shares axis `name` with `other`.
    pub(crate) fn share(&self, name: AxisName, other: &Self) -> Result<&Self, PlotError> {
        self.share_id(name, other.raw())
    }

    /// Shares axis `name` with the axes, of either kind, `other` reaches.
    fn share_id(&self, name: AxisName, other: &RawHandle) -> Result<&Self, PlotError> {
        let figure = self.raw().figure_state();
        if self
            .raw
            .with::<AxesState, _>(|axes| axes.projection.fixed_limits())
        {
            return Err(PlotError::FixedLimits(name));
        }
        if !Rc::ptr_eq(figure, other.figure_state()) {
            // Axes of two figures share through links, 2-D ones only.
            let flat = other.downcast::<Self>().ok_or(PlotError::ForeignAxes)?;
            self.share_across(name, &flat)?;
            return Ok(self);
        }
        figure
            .borrow_mut()
            .share_axis(name, self.id(), other.id())?;
        Ok(self)
    }

    /// This axes and the 2-D axes sharing axis `name` with it
    /// (matplotlib's `get_shared_x_axes().get_siblings(ax)` without the
    /// 3-D axes; [`Axes::sharing`] lists them all).
    ///
    /// ```
    /// use pltrs::figure::{Figure, Share};
    /// use pltrs::ticker::AxisName;
    /// let fig = Figure::new();
    /// let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
    /// assert_eq!(axs[(0, 0)].shared_axes(AxisName::Y).len(), 2);
    /// assert_eq!(axs[(0, 0)].shared_axes(AxisName::X), [axs[(0, 0)].clone()]);
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    pub fn shared_axes(&self, name: AxisName) -> Vec<Self> {
        let ids: Vec<ArtistId> = {
            let state = self.raw.figure_state().borrow();
            (state.siblings(name, self.id()).into_iter())
                .filter(|&id| state.artist_as::<AxesState>(id).is_some() || id == self.id())
                .collect()
        };
        self.in_same_figure(ids)
    }

    /// The axes, of either kind, whose axis `name` this one shares
    /// (matplotlib's `_sharex` and `_sharey`): the one given to
    /// [`Axes::share_x`], [`Axes::share_y`] or [`Axes::share_with`], or as
    /// `sharex=` / `sharey=` when it was made; `None` for an axis it
    /// shares with none, or only lends to others.
    ///
    /// ```
    /// use pltrs::figure::{Figure, Share};
    /// use pltrs::mplot3d::SharedAxes;
    /// use pltrs::ticker::AxisName;
    /// let fig = Figure::new();
    /// let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
    /// let first = SharedAxes::Flat(axs[(0, 0)].clone());
    /// assert_eq!(axs[(0, 1)].shared_from(AxisName::Y), Some(first));
    /// assert_eq!(axs[(0, 0)].shared_from(AxisName::Y), None);
    /// assert_eq!(axs[(0, 1)].shared_from(AxisName::X), None);
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    pub fn shared_from(&self, name: AxisName) -> Option<SharedAxes> {
        let id = self.raw.with::<AxesState, _>(|axes| match name {
            AxisName::X => axes.shares.0,
            AxisName::Y => axes.shares.1,
        })?;
        SharedAxes::of(self.raw.figure_state(), id)
    }

    /// The axes whose axis `name` the locators of this one's are bound to
    /// (matplotlib's `locator.axis.axes`), whose length decides how many
    /// ticks fit: the axes they were set on, which may be another of the
    /// axes sharing the axis, or the last of those a clear reset.
    ///
    /// ```
    /// use pltrs::figure::{Figure, Share};
    /// use pltrs::ticker::AxisName;
    /// let fig = Figure::new();
    /// let axs = fig.subplots_shared(1, 2, Share::All, Share::None)?;
    /// let (left, right) = (&axs[(0, 0)], &axs[(0, 1)]);
    /// assert_eq!(&right.locator_owner(AxisName::X), left);
    /// left.clear();
    /// assert_eq!(&left.locator_owner(AxisName::X), right);
    /// assert_eq!(&left.locator_owner(AxisName::Y), left);
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    #[must_use]
    pub fn locator_owner(&self, name: AxisName) -> Self {
        // A 3-D axes the locators are bound to measures its axes its own
        // way: this axes stands for it.
        let owner = (self.raw)
            .with::<AxesState, _>(|axes| axes.axis(name).ticker_owner)
            .filter(|&owner| {
                (self.raw.figure_state().borrow())
                    .artist_as::<AxesState>(owner)
                    .is_some()
            })
            .unwrap_or_else(|| self.id());
        self.in_same_figure(vec![owner])
            .pop()
            .expect("one axes for one id")
    }

    /// The axes `ids` of this axes' figure.
    pub(crate) fn in_same_figure(&self, ids: Vec<ArtistId>) -> Vec<Self> {
        let figure = self.raw.figure_state();
        ids.into_iter()
            .map(|id| Self::from_raw(RawHandle::new(Rc::clone(figure), id)))
            .collect()
    }
}

/// Makes `axes`, new in the figure of `target` and of free limits, share
/// axis `name` with it, which always succeeds (the `sharex=` argument of
/// a new rectilinear or polar axes in matplotlib).
pub(crate) fn share_free(axes: &Axes, name: AxisName, target: &Axes) {
    axes.share(name, target)
        .expect("a new axes with free limits shares any axis of its figure");
}

#[cfg(test)]
mod tests {
    use crate::axes::AxisName;
    use crate::figure::{Figure, Share};

    #[test]
    fn shared_axes_list_the_group() {
        let fig = Figure::new();
        let axs = fig.subplots_shared(2, 2, Share::All, Share::Row).unwrap();
        assert_eq!(axs[(0, 0)].shared_axes(AxisName::X).len(), 4);
        assert_eq!(axs[(0, 0)].shared_axes(AxisName::Y).len(), 2);
        assert!(axs[(1, 1)].shared_axes(AxisName::Y).contains(&axs[(1, 0)]));
    }

    #[test]
    fn geographic_axes_share_nothing_as_in_matplotlib() {
        use crate::axes::{PlotError, ProjectionKind};
        use crate::figure::{Mosaic, MosaicError, MosaicOptions};
        use crate::projections::ProjectionError;

        let fig = Figure::new();
        let rect = fig.add_subplot(1, 2, 1).unwrap();
        let geo = fig
            .subplot_with_projection(ProjectionKind::Mollweide)
            .unwrap();
        assert_eq!(
            geo.share_x(&rect).err(),
            Some(PlotError::FixedLimits(AxisName::X))
        );
        assert_eq!(
            geo.shared_axes(AxisName::X),
            std::slice::from_ref(&geo),
            "not joined"
        );
        assert!(rect.share_y(&geo).is_ok(), "others may share theirs");

        let grid = Figure::new();
        assert_eq!(
            grid.subplots_with_projection(1, 2, Share::None, Share::All, ProjectionKind::Hammer),
            Err(ProjectionError::Share(PlotError::FixedLimits(AxisName::Y)))
        );
        assert_eq!(grid.axes().len(), 1, "matplotlib fails to make the second");

        let options = MosaicOptions {
            sharex: true,
            projection: ProjectionKind::Aitoff.into(),
            ..MosaicOptions::default()
        };
        let mosaic = "AB".parse::<Mosaic>().unwrap();
        assert!(matches!(
            Figure::new().subplot_mosaic(&mosaic, options),
            Err(MosaicError::Projection(ProjectionError::Share(_)))
        ));
    }
}
