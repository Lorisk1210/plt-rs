//! Clearing a 3-D axes back to a fresh one at the same place
//! (matplotlib's `Axes3D.clear` and `cla`), or one of its axes
//! (`Axis.clear`).

use super::axis_handle::Axis3D;
use super::axis3d::Axis3DState;
use super::coords::Axis3;
use super::create::show_view_window;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::limits::Limits3D;
use super::scaling::AxisScale;
use super::share::ShareKey;
use super::state::Axes3DState;
use crate::artist::ArtistHandle;
use crate::axes::TickStyle;
use crate::rc::RcParams;

impl Axis3DState {
    /// Takes the tick parameters of `old` but whether gridlines are on,
    /// which restart from the rc (matplotlib's `Axis.clear` keeps the
    /// tick parameters and resets `gridOn`).
    fn keep_tick_styles(&mut self, old: &Self) {
        for (kept, restarted) in [
            (&old.major_style, &mut self.major_style),
            (&old.minor_style, &mut self.minor_style),
        ] {
            *restarted = TickStyle {
                grid_on: restarted.grid_on,
                ..kept.clone()
            };
        }
    }

    /// The axis a cleared one restarts from under `rc` (matplotlib's
    /// `Axis.clear`): the label, labelpad, scale, tickers, ticks, units
    /// and whether gridlines are on start over; the other tick
    /// parameters, the label's rotation, the pane and the line (the
    /// axis' `_axinfo`), the sides of the ticks and of the label, the
    /// offset text's visibility, the overlapping minor ticks, the
    /// visibility and the gids stay.
    fn cleared(&self, rc: &RcParams) -> Self {
        let mut fresh = Self::new(self.dir, rc);
        fresh.keep_tick_styles(self);
        fresh.label.rotate = self.label.rotate;
        fresh.pane_color = self.pane_color.clone();
        fresh.pane_style.clone_from(&self.pane_style);
        fresh.style = self.style.clone();
        fresh.line_visible = self.line_visible;
        fresh.tick_position = self.tick_position;
        fresh.label_position = self.label_position;
        fresh.offset_shown = self.offset_shown;
        fresh.remove_overlapping = self.remove_overlapping;
        fresh.visible = self.visible;
        fresh.gids = self.gids.clone();
        fresh
    }
}

impl Axes3DState {
    /// Sets the artists aside and restarts the limits, axes, gridlines and
    /// decorations from the current rcParams, keeping the tick parameters
    /// (matplotlib's `Axes3D.clear`, which turns the axes back on).
    fn clear(&mut self) {
        let rc = crate::rc::params();
        let zmargin = if self.camera.is_ortho() {
            rc.axes_zmargin
        } else {
            0.0
        };
        self.limits = Limits3D::new(zmargin, &rc);
        for axis in &mut self.axes {
            let mut fresh = Axis3DState::new(axis.dir, &rc);
            fresh.keep_tick_styles(axis);
            // matplotlib builds an axis' `_axinfo` once, with the axis.
            fresh.juggled = axis.juggled;
            *axis = fresh;
        }
        // Set aside, as their artists keep their 3-D data (see `readd`).
        let cleared = std::mem::take(&mut self.artists);
        self.removed.extend(cleared);
        self.grid_on = rc.axes3d_grid;
        self.axis_on = true;
    }
}

impl Axes3D {
    /// Removes everything drawn in the axes and resets it under the
    /// current rcParams (matplotlib's `Axes3D.clear`): titles, labels,
    /// limits (autoscaled from the default box), scales, tickers,
    /// gridlines and the property cycle start over, and panes and axes
    /// are shown again; the position, view, projection, box aspect,
    /// sharing and tick parameters are kept. The axes sharing an axis
    /// take its fresh tick helpers too (matplotlib's shared axes hold one
    /// `Ticker`), and an axis shared explicitly with another axes
    /// ([`Axes3D::share_axis`]) takes its limits, scale and tick helpers
    /// again, as matplotlib's clear shares it again.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// ax.plot(&[0.0, 5.0], &[0.0, 5.0], &[0.0, 5.0])?;
    /// ax.set_axis_off().set_zlabel("z");
    /// ax.clear()?;
    /// assert!(ax.axis_on());
    /// assert_eq!(ax.label(pltrs::mplot3d::Axis3::Z), "");
    /// assert_eq!(ax.zlim(), (-1.0 / 48.0, 49.0 / 48.0));
    /// let other = fig.add_subplot_3d(1, 2, 2, Default::default())?;
    /// other.set_zlim(0.0, 2.0)?;
    /// ax.share_z(&other)?;
    /// ax.clear()?;
    /// assert_eq!(ax.zlim(), (0.0, 2.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When autoscaling the fresh limits fails.
    pub fn clear(&self) -> Result<&Self, Axes3DError> {
        // matplotlib's clear makes the axes a new callback registry first.
        let disconnected = (self.raw().figure_state().borrow_mut())
            .limit_callbacks
            .take_axes(self.id());
        // Dropped once the figure is free: a callback may own handles.
        drop(disconnected);
        let plane = self.plane();
        plane.clear();
        show_view_window(&plane);
        self.with_mut(Axes3DState::clear);
        for axis in Axis3::ALL {
            self.reset_shared(axis);
            if let Some(target) = self.share_target(axis) {
                self.share_axis_with(axis, &target)?;
            } else {
                // matplotlib's clear sets the limits of an axis it does not
                // share again to (0, 1) (`_set_lim(0, 1, auto=True)`), which
                // the axes sharing it follow, their limit callbacks running.
                self.with_mut(|axes| axes.set_lim(axis, 0.0, 1.0, Some(true), true));
            }
        }
        self.with_mut(Axes3DState::autoscale_view)?;
        Ok(self)
    }

    /// Resets axis `axis` of the other axes sharing it as matplotlib's
    /// clear does: each takes the linear scale and its default tick
    /// helpers in turn, on the `Ticker` they all share with this one, so
    /// that every axes of the group ends with the last one's, which owns
    /// the locator.
    fn reset_shared(&self, axis: Axis3) {
        let others: Vec<Axes3D> = (self.shared_axes(axis).into_iter())
            .filter(|other| other != self)
            .collect();
        for other in &others {
            other.with_mut(|axes| axes.axis_mut(axis).set_scale(AxisScale::default()));
        }
        let Some(last) = others.last() else {
            return;
        };
        let tickers = last.with(|axes| axes.axis(axis).tickers.clone());
        self.update_group(ShareKey::Axis(axis), |axes| {
            axes.axis_mut(axis).tickers = tickers.clone();
        });
        last.axis(axis).own_major_locator();
    }

    /// Clears the axes (matplotlib's `cla`, an alias of
    /// [`Axes3D::clear`]).
    ///
    /// # Errors
    ///
    /// As [`Axes3D::clear`].
    pub fn cla(&self) -> Result<&Self, Axes3DError> {
        self.clear()
    }
}

impl Axis3D {
    /// Resets the axis under the current rcParams (matplotlib's
    /// `Axis.clear`): its label, scale (linear), tickers, units and
    /// whether gridlines are on start over; the other tick parameters,
    /// the limits of the axes, the pane and the sides of the ticks and
    /// of the label stay.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_zlabel("height").set_zlim(0.0, 3.0)?;
    /// ax.axis(Axis3::Z).clear();
    /// assert_eq!(ax.label(Axis3::Z), "");
    /// assert_eq!(ax.zlim(), (0.0, 3.0));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    pub fn clear(&self) -> &Self {
        let rc = crate::rc::params();
        let dir = self.dir;
        self.axes.with_mut(|axes| {
            let axis = axes.axis_mut(dir);
            *axis = axis.cleared(&rc);
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::axes::TickParams;
    use crate::colors::Color;
    use crate::figure::Figure;
    use crate::mplot3d::Axis3;

    #[test]
    fn clearing_forgets_the_artists_but_keeps_the_view_and_tick_parameters() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let line = ax.plot([0.0, 5.0], [0.0, 5.0], [0.0, 5.0]).unwrap();
        let first_color = line.rgba().unwrap();
        ax.view_init(10.0, 20.0, 0.0);
        ax.set_title("before");
        ax.tick_params(TickParams {
            color: Some(Color::from("red")),
            ..TickParams::default()
        });
        ax.cla().unwrap();
        assert!(!ax.has_data());
        assert!(!fig.raw().figure_state().borrow().store.contains(line.id()));
        assert_eq!(ax.title(), "");
        assert_eq!(ax.view().elev, 10.0);
        let tick_color = ax.with(|axes| axes.axis(Axis3::X).major_style.color.clone());
        assert_eq!(tick_color, Color::from("red"));
        // The property cycle starts over.
        let again = ax.plot([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).unwrap();
        assert_eq!(again.rgba().unwrap(), first_color);
    }
}
