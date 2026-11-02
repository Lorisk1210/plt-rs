//! Clearing one axis of an axes back to a fresh one (matplotlib's
//! `Axis.clear`).

use super::axis::AxisState;
use super::{Axis, TickStyle};
use crate::rc::RcParams;

impl AxisState {
    /// The axis a cleared one restarts from under `rc`: a fresh axis that
    /// keeps the part `self` plays in its projection, its tick parameters
    /// but whether gridlines are on (which restarts from the rc, as
    /// matplotlib's `Axis.clear` resets `gridOn`), its tick label size,
    /// its picking, visibility and place in layouts, and how it autoscales.
    pub(super) fn cleared(&self, rc: &RcParams) -> Self {
        let mut fresh = Self::new(self.name, rc);
        fresh.role = self.role;
        fresh.keep_tick_styles(self);
        fresh.visible = self.visible;
        fresh.in_layout = self.in_layout;
        fresh.autoscale = self.autoscale;
        fresh.autolimit_mode = self.autolimit_mode;
        fresh.ticker_owner = self.ticker_owner;
        fresh.gids = self.gids.clone();
        fresh
    }

    /// Takes the tick parameters of `old` but whether gridlines are on,
    /// the tick label size and the picking, which an axis keeps when it or
    /// its axes is cleared.
    pub(super) fn keep_tick_styles(&mut self, old: &Self) {
        for (kept, restarted) in [
            (&old.major_style, &mut self.major_style),
            (&old.minor_style, &mut self.minor_style),
        ] {
            *restarted = TickStyle {
                grid_on: restarted.grid_on,
                ..kept.clone()
            };
        }
        self.label_size = old.label_size;
        self.picking = old.picking.clone();
    }
}

impl Axis {
    /// Resets the axis to a fresh one under the current rcParams
    /// (matplotlib's `Axis.clear`): the label, the scale (linear), the
    /// locators and formatters, the ticks, the units and whether gridlines
    /// are on (as `axes.grid` says) start over; the other tick parameters,
    /// the visibility and the view limits stay.
    ///
    /// ```
    /// use pltrs::axes::AxisName;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.set_xlabel("time").set_xscale("log")?;
    /// ax.set_xlim(1.0, 100.0)?;
    /// ax.xaxis().clear();
    /// assert_eq!(ax.label(AxisName::X), "");
    /// assert_eq!(ax.xscale(), pltrs::scale::ScaleName::Linear);
    /// assert_eq!(ax.xlim(), (1.0, 100.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn clear(&self) -> &Self {
        let rc = crate::rc::params();
        self.update(|axis| *axis = axis.cleared(&rc));
        self
    }
}
