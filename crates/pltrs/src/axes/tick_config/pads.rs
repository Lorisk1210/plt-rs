//! The pads of single ticks (matplotlib's `Tick.set_pad` on one tick of
//! an axis): the gap between a tick's mark and its labels, kept by tick
//! index on top of the pad of its level.

use crate::axes::Axis;

impl Axis {
    /// Sets the gap between the mark and the labels of the major (or
    /// `minor`) tick at `index` among those the locator places, in points
    /// (matplotlib's `axis.get_major_ticks()[index].set_pad(pad)`). It
    /// applies to that tick only, until tick parameters set the pad of its
    /// level.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot();
    /// ax.set_xticks([0.0, 0.5, 1.0]);
    /// ax.xaxis().set_tick_pad(false, 1, 12.0);
    /// assert_eq!(ax.xaxis().tick_style_at(false, 1).pad, 12.0);
    /// assert_eq!(ax.xaxis().tick_pad(false, 1), Some(12.0));
    /// assert_eq!(ax.xaxis().tick_pad(false, 0), None);
    /// ```
    pub fn set_tick_pad(&self, minor: bool, index: usize, pad: f64) -> &Self {
        self.update_labels(|axis| {
            let pads = &mut axis.overrides_mut(minor).pads;
            if pads.len() <= index {
                pads.resize(index + 1, None);
            }
            pads[index] = Some(pad);
        });
        self
    }

    /// The pad the major (or `minor`) tick at `index` has of its own, in
    /// points (see [`set_tick_pad`](Self::set_tick_pad)); `None` while it
    /// keeps the pad of its level.
    pub fn tick_pad(&self, minor: bool, index: usize) -> Option<f64> {
        self.read(|axis| axis.overrides(minor).pads.get(index).copied().flatten())
    }
}

#[cfg(test)]
mod tests {
    use crate::axes::{TickLevel, TickParams};
    use crate::figure::Figure;
    use crate::ticker::AxisName;

    #[test]
    fn a_tick_pad_moves_that_label_only() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot();
        ax.set_xticks([0.0, 0.5, 1.0]);
        let bottom = |index| {
            ax.xaxis()
                .tick_label_extent(false, index, true)
                .expect("a drawn label")
                .y1()
        };
        let (first, second) = (bottom(0), bottom(1));
        assert_eq!(first, second);
        ax.xaxis().set_tick_pad(false, 1, 3.5 + 10.0);
        assert!(
            (bottom(1) - (second - 10.0 / 72.0 * 100.0)).abs() < 1e-9,
            "10 points further down"
        );
        assert_eq!(bottom(0), first, "the other labels stay");
        assert_eq!(ax.yaxis().tick_pad(false, 1), None);
    }

    #[test]
    fn tick_parameters_setting_the_pad_drop_single_pads() {
        let ax = Figure::new().subplot();
        ax.xaxis()
            .set_tick_pad(false, 0, 9.0)
            .set_tick_pad(true, 2, 4.0);
        ax.tick_params(TickParams {
            axis: Some(AxisName::X),
            which: TickLevel::Major,
            labelsize: Some(8.0.into()),
            ..TickParams::default()
        });
        assert_eq!(ax.xaxis().tick_pad(false, 0), Some(9.0), "kept");
        ax.tick_params(TickParams {
            axis: Some(AxisName::X),
            which: TickLevel::Major,
            pad: Some(2.0),
            ..TickParams::default()
        });
        assert_eq!(ax.xaxis().tick_pad(false, 0), None);
        assert_eq!(ax.xaxis().tick_style_at(false, 0).pad, 2.0);
        assert_eq!(ax.xaxis().tick_pad(true, 2), Some(4.0), "minor kept");
    }
}
