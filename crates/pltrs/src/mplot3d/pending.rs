//! Autoscaling asked for later (matplotlib's `_request_autoscale_view`
//! and `_unstale_viewLim`): changing a margin of a 3-D axes or switching
//! its autoscaling on marks the axes concerned, whose limits are fitted
//! to the data when the axes is next drawn, or when a program asks; until
//! then the limits read as they were, as matplotlib's `get_xlim`,
//! `get_ylim` and `get_zlim` read them. Setting the limits of an axis,
//! autoscaling included, calls its autoscaling off.

use super::error::Axes3DError;
use super::handle::Axes3D;
use super::share::replay;
use super::state::Axes3DState;
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup};
use crate::axes::AxesState;
use crate::figure::FigureState;
use crate::ticker::AxisName;

impl Axes3DState {
    /// Marks the axes `selected` (x, y, z) for autoscaling later, `tight`
    /// replacing the tight setting when given.
    pub(crate) fn request_autoscale(&mut self, selected: [bool; 3], tight: Option<bool>) {
        for (pending, chosen) in self.limits.pending.iter_mut().zip(selected) {
            *pending |= chosen;
        }
        if let Some(tight) = tight {
            self.limits.tight = tight;
        }
    }

    /// Fits the axes marked for autoscaling to their data and unmarks
    /// them.
    fn unstale(&mut self) -> Result<(), Axes3DError> {
        let pending = std::mem::take(&mut self.limits.pending);
        self.autoscale_axes(pending)
    }
}

impl Axes3D {
    /// Marks the axes `selected` (x, y, z) for autoscaling when the axes
    /// is next drawn or [`Axes3D::unstale_view_lim`] is called, `tight`
    /// replacing the tight setting when given (matplotlib's
    /// `_request_autoscale_view`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.plot(&[0.0, 10.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// ax.set_xlim(2.0, 3.0)?;
    /// ax.request_autoscale([true, false, false], None);
    /// assert_eq!(ax.xlim(), (2.0, 3.0), "until the axes is drawn");
    /// ax.unstale_view_lim()?;
    /// assert_eq!(ax.xlim(), (2.0, 3.0), "x no longer autoscales");
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    pub fn request_autoscale(&self, selected: [bool; 3], tight: Option<bool>) -> &Self {
        self.with_mut(|axes| axes.request_autoscale(selected, tight));
        self
    }

    /// Fits the axes marked for autoscaling to their data now
    /// (matplotlib's `_unstale_viewLim`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.plot(&[0.0, 48.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// ax.set_margin(Axis3::X, 0.5)?;
    /// assert_eq!(ax.xlim(), (-3.5, 51.5), "the margin waits");
    /// ax.unstale_view_lim()?;
    /// assert_eq!(ax.xlim(), (-26.0, 74.0));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a locator fails while autoscaling.
    pub fn unstale_view_lim(&self) -> Result<&Self, Axes3DError> {
        self.raw()
            .figure_state()
            .borrow_mut()
            .gather_pending(self.id());
        self.with_mut(Axes3DState::unstale)?;
        Ok(self)
    }
}

/// Applies the autoscaling pending on the 3-D axes `id` before a draw lays
/// it out (matplotlib's draw starts with `_unstale_viewLim`, which a
/// figure's draws leave out for an animated or an invisible axes), the
/// axes sharing its limits following. A locator failing leaves the limits
/// as they are, its axis reporting the failure as it is drawn.
pub(crate) fn unstale_for_draw(figure: &mut FigureState, id: ArtistId) {
    let shown = figure
        .artist_as::<Axes3DState>(id)
        .is_some_and(|axes| !axes.base().animated && axes.base().visible);
    if shown {
        figure.gather_pending(id);
    }
    let changes = figure.with_others::<Axes3DState, _>(id, |axes, _| {
        if !shown {
            return Vec::new();
        }
        if axes.unstale().is_err() {
            axes.limits.pending = [false; 3];
        }
        axes.take_limit_changes()
    });
    if let Some(changes) = changes.filter(|changes| !changes.is_empty()) {
        replay(figure, id, &changes);
    }
}

impl FigureState {
    /// Marks the x and y axes of the 3-D axes `id` for autoscaling when an
    /// axes sharing them, of either kind, waits to be autoscaled, which
    /// then no longer waits (matplotlib's `_unstale_viewLim` asks every
    /// sibling, then rescales the axes it is called on).
    pub(crate) fn gather_pending(&mut self, id: ArtistId) {
        for (index, name) in [AxisName::X, AxisName::Y].into_iter().enumerate() {
            let siblings = self.siblings(name, id);
            let waits = siblings.iter().any(|&sibling| {
                self.deep_axis_pending(sibling, name).unwrap_or_else(|| {
                    self.artist_as::<AxesState>(sibling)
                        .is_some_and(|flat| flat.axis(name).autoscale.stale)
                })
            });
            if !waits {
                continue;
            }
            for sibling in siblings {
                self.settle_deep_axis(sibling, name);
                if let Some(flat) = self.artist_as_mut::<AxesState>(sibling) {
                    flat.axis_mut(name).autoscale.stale = false;
                }
            }
            if let Some(axes) = self.artist_as_mut::<Axes3DState>(id) {
                axes.limits.pending[index] = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;
    use crate::mplot3d::Axis3;

    #[test]
    fn a_draw_applies_the_autoscaling_pending() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        ax.plot([10.0, 30.0], [0.0, 1.0], [10.0, 30.0]).unwrap();
        let before = ax.zlim();
        ax.set_margin(Axis3::Z, 0.5).unwrap();
        assert_eq!(ax.zlim(), before);
        fig.canvas().draw().unwrap();
        assert!(
            ax.zlim().0 < before.0,
            "{:?} widened from {before:?}",
            ax.zlim()
        );
    }

    #[test]
    fn limits_set_call_the_pending_autoscaling_off() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        ax.plot([0.0, 10.0], [0.0, 1.0], [10.0, 30.0]).unwrap();
        // matplotlib's `margins` autoscales at once, which calls off what
        // setting each margin asked for.
        ax.set_margins([Some(0.2); 3], Some(true)).unwrap();
        ax.set_zbound(Some(-1.0), None).unwrap();
        ax.set_margin(Axis3::X, 0.0).unwrap();
        let z = ax.zlim();
        fig.canvas().draw().unwrap();
        assert_eq!(ax.zlim(), z, "z keeps the bound set");
        // matplotlib: (-0.20833333333333331, 10.208333333333334).
        let (lo, hi) = ax.xlim();
        assert!(
            (lo + 0.208_333_333_333_333_3).abs() < 1e-12
                && (hi - 10.208_333_333_333_334).abs() < 1e-12
        );
    }

    #[test]
    fn switching_autoscaling_on_waits_too() {
        let ax = Figure::new().subplot_3d();
        ax.plot([0.0, 10.0], [0.0, 1.0], [0.0, 1.0]).unwrap();
        ax.set_xlim(2.0, 3.0).unwrap();
        ax.autoscale(Some(true), [true, false, false], Some(true))
            .unwrap();
        assert_eq!(ax.xlim(), (2.0, 3.0));
        ax.unstale_view_lim().unwrap();
        // matplotlib: (-0.7291666666666666, 10.729166666666666).
        assert_eq!(
            ax.xlim(),
            (-0.729_166_666_666_666_6, 10.729_166_666_666_666)
        );
    }
}
