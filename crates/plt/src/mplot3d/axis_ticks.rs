//! The ticks of a 3-D axis: those drawn, every location with its label,
//! and ticking an axis of dates (matplotlib's `get_ticklocs`,
//! `get_ticklabels` and `axis_date`, which `Axes3D` wraps as
//! `get_zticks`, `get_zticklabels` and `zaxis_date`).

use std::rc::Rc;

use super::axis_handle::Axis3D;
use crate::artist::ArtistHandle;
use crate::axes::TickSet;
use crate::dates::TimeZone;
use crate::units::date_tickers;

impl Axis3D {
    /// The ticks the axis draws in the current view, after a layout pass
    /// (matplotlib's visible ticks of `_update_ticks`).
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails, with its message.
    pub fn ticks(&self) -> Result<TickSet, String> {
        self.axes.raw().figure_state().borrow_mut().prepare();
        let dir = self.dir;
        self.axes.with(|axes| axes.axis(dir).ticks.clone())
    }

    /// Every major (or minor) tick location of the current view, inside
    /// it or not; minor locations on major ticks are left out
    /// (matplotlib's `Axis.get_ticklocs`, `Axes3D.get_zticks` for the z
    /// axis). A level whose marks, labels and gridlines are all hidden
    /// has no locations.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_zlim(0.0, 1.0)?;
    /// assert_eq!(ax.zaxis().ticklocs(false)?, [0.0, 0.2, 0.4, 0.6000000000000001, 0.8, 1.0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails, with its message.
    pub fn ticklocs(&self, minor: bool) -> Result<Vec<f64>, String> {
        Ok(self.ticks()?.unclipped_locs(minor))
    }

    /// The labels of the [`ticklocs`](Self::ticklocs) on each side that
    /// shows them, those on the first side first (matplotlib's
    /// `Axis.get_ticklabels`; `get_zticklabels`, `get_zmajorticklabels`
    /// and `get_zminorticklabels` for the z axis).
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails, with its message.
    pub fn ticklabels(&self, minor: bool) -> Result<Vec<String>, String> {
        let ticks = self.ticks()?;
        let dir = self.dir;
        let sides = self.axes.with(|axes| {
            let axis = axes.axis(dir);
            if minor {
                axis.minor_style.labels
            } else {
                axis.major_style.labels
            }
        });
        Ok(ticks.unclipped_labels(minor, sides))
    }

    /// Ticks the axis as an axis of dates in `tz`: its values are date
    /// numbers (days since the [epoch](crate::dates::get_epoch), see
    /// [`date2num`](crate::dates::date2num)), placed by an
    /// `AutoDateLocator` and labelled by the formatter `date.converter`
    /// chooses (matplotlib's `Axis.axis_date`, `Axes3D.zaxis_date` for the
    /// z axis). Like matplotlib, it replaces only the major locator and
    /// formatter that were not set explicitly, and leaves the limits as
    /// they are; the axes sharing the axis follow.
    ///
    /// ```
    /// use plt::dates::TimeZone;
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[19_000.0, 19_010.0])?;
    /// ax.zaxis().axis_date(&TimeZone::Utc);
    /// assert_eq!(ax.zaxis().ticklabels(false)?[0], "2022-01-08");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn axis_date(&self, tz: &TimeZone) -> &Self {
        let (locator, formatter) = date_tickers(tz);
        self.set_tickers(|t| {
            if t.default_major.0 {
                t.major_locator = Rc::clone(&locator);
            }
            if t.default_major.1 {
                t.major_formatter = Rc::clone(&formatter);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::dates::TimeZone;
    use crate::figure::Figure;
    use crate::mplot3d::Axis3;
    use crate::ticker::{MultipleLocator, NullFormatter};

    #[test]
    fn locations_and_labels_include_ticks_outside_the_view() {
        let ax = Figure::new().subplot_3d();
        ax.set_zlim(0.05, 0.95).unwrap();
        let axis = ax.zaxis();
        let locs = axis.ticklocs(false).unwrap();
        assert_eq!(locs.first(), Some(&0.0), "outside the view");
        assert_eq!(axis.ticks().unwrap().major.len(), locs.len() - 2);
        let labels = axis.ticklabels(false).unwrap();
        assert_eq!(labels.len(), locs.len());
        assert_eq!(labels[1], "0.2");
        assert_eq!(
            axis.ticklocs(true).unwrap(),
            Vec::<f64>::new(),
            "no minor ticks"
        );
    }

    #[test]
    fn minor_locations_skip_the_major_ones_and_labels_follow_the_sides() {
        let ax = Figure::new().subplot_3d();
        ax.set_xlim(0.0, 1.0).unwrap();
        let axis = ax.xaxis();
        axis.set_minor_locator(MultipleLocator::new(0.1).unwrap());
        let minor = axis.ticklocs(true).unwrap();
        // matplotlib: -0.1, 0.1, 0.3, ..., 1.1 without 0.0, 0.2, ...
        assert_eq!(minor.len(), 7);
        assert!((minor[0] + 0.1).abs() < 1e-12 && (minor[6] - 1.1).abs() < 1e-12);
        ax.with_mut(|axes| axes.axis_mut(Axis3::X).major_style.labels.second = true);
        let labels = axis.ticklabels(false).unwrap();
        assert_eq!(labels.len(), 2 * axis.ticklocs(false).unwrap().len());
    }

    #[test]
    fn date_axes_keep_explicit_tick_helpers() {
        let ax = Figure::new().subplot_3d();
        ax.set_zlim(19_000.0, 19_003.0).unwrap();
        ax.zaxis().axis_date(&TimeZone::Utc);
        assert_eq!(ax.zlim(), (19_000.0, 19_003.0), "limits stay");
        let labels = ax.zaxis().ticklabels(false).unwrap();
        assert_eq!(labels[..2], ["01-08 00", "01-08 12"], "twice a day");
        let ax = Figure::new().subplot_3d();
        ax.zaxis().set_major_formatter(NullFormatter);
        ax.zaxis().axis_date(&TimeZone::Utc);
        let labels = ax.zaxis().ticklabels(false).unwrap();
        assert!(labels.iter().all(String::is_empty), "{labels:?}");
        assert_eq!(ax.zaxis().ticklocs(false).unwrap()[1], 0.125, "date ticks");
    }
}
