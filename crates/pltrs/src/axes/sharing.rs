//! Axes sharing an axis (matplotlib's `sharex` and `sharey`).

use std::rc::Rc;

use super::{Axes, AxesState, AxisName, PlotError};
use crate::artist::{ArtistHandle, ArtistId, RawHandle};

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
    /// When the x axis is already shared with a different axes, `other`
    /// belongs to another figure, or this axes has a projection fixing its
    /// limits, such as a geographic one ([`PlotError::FixedLimits`];
    /// matplotlib's geographic axes raise a `TypeError` when they share).
    pub fn share_x(&self, other: &Self) -> Result<&Self, PlotError> {
        self.share(AxisName::X, other)
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

    /// Shares axis `name` with `other`.
    pub(crate) fn share(&self, name: AxisName, other: &Self) -> Result<&Self, PlotError> {
        let figure = self.raw().figure_state();
        if !Rc::ptr_eq(figure, other.raw().figure_state()) {
            return Err(PlotError::ForeignAxes);
        }
        if self
            .raw
            .with::<AxesState, _>(|axes| axes.projection.fixed_limits())
        {
            return Err(PlotError::FixedLimits(name));
        }
        figure
            .borrow_mut()
            .share_axis(name, self.id(), other.id())?;
        Ok(self)
    }

    /// This axes and the axes sharing axis `name` with it (matplotlib's
    /// `get_shared_x_axes().get_siblings(ax)`).
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
        let ids = self.raw.figure_state().borrow().siblings(name, self.id());
        self.in_same_figure(ids)
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
