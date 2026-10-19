//! Clearing a 3-D axes back to a fresh one at the same place
//! (matplotlib's `Axes3D.clear` and `cla`).

use super::axis3d::Axis3DState;
use super::camera::Camera;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::limits::Limits3D;
use super::state::Axes3DState;

impl Axes3DState {
    /// Forgets the artists and restarts the limits, axes, gridlines and
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
            fresh.major_style = axis.major_style.clone();
            fresh.minor_style = axis.minor_style.clone();
            *axis = fresh;
        }
        self.artists.clear();
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
    /// sharing and tick parameters are kept.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.plot(&[0.0, 5.0], &[0.0, 5.0], &[0.0, 5.0])?;
    /// ax.set_axis_off().set_zlabel("z");
    /// ax.clear()?;
    /// assert!(ax.axis_on());
    /// assert_eq!(ax.label(plt::mplot3d::Axis3::Z), "");
    /// assert_eq!(ax.zlim(), (-1.0 / 48.0, 49.0 / 48.0));
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When autoscaling the fresh limits fails.
    pub fn clear(&self) -> Result<&Self, Axes3DError> {
        let plane = self.plane();
        plane.clear();
        let (lo, hi) = Camera::view_window();
        plane.set_xlim(lo, hi).set_ylim(lo, hi);
        self.with_mut(|axes| {
            axes.clear();
            axes.autoscale_view()
        })?;
        Ok(self)
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
        let line = ax.plot(&[0.0, 5.0], &[0.0, 5.0], &[0.0, 5.0]).unwrap();
        let first_color = line.rgba().unwrap();
        ax.view_init(10.0, 20.0, 0.0);
        ax.set_title("before");
        ax.tick_params(&TickParams {
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
        let again = ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]).unwrap();
        assert_eq!(again.rgba().unwrap(), first_color);
    }
}
