//! Single ticks as artists of their own (matplotlib's `Tick.set_visible`
//! and `Tick.set_agg_filter` on one tick of an axis): whether a tick is
//! drawn and the image filter it is drawn through, kept by tick index.
//! Its labels count in the axis' extent either way, as matplotlib measures
//! the labels of a hidden tick.

use crate::artist::AggFilter;
use crate::axes::Axis;
use crate::axes::axis::AxisState;

/// The entry of `list` at `index`, the list grown with defaults to reach
/// it.
fn entry<T: Clone + Default>(list: &mut Vec<T>, index: usize) -> &mut T {
    if list.len() <= index {
        list.resize(index + 1, T::default());
    }
    &mut list[index]
}

impl Axis {
    /// Draws the major (or `minor`) tick at `index` among those the
    /// locator places, or hides it (matplotlib's
    /// `axis.get_major_ticks()[index].set_visible(visible)`): a hidden
    /// tick draws neither its gridline, nor its marks, nor its labels.
    /// Ticks made later take the visibility of the first tick, as
    /// matplotlib's copy it (`Axis._copy_tick_props`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot();
    /// ax.xaxis().set_tick_visible(false, 1, false);
    /// assert!(!ax.xaxis().tick_visible(false, 1));
    /// assert!(ax.xaxis().tick_visible(false, 0));
    /// ```
    pub fn set_tick_visible(&self, minor: bool, index: usize, visible: bool) -> &Self {
        self.update(|axis| *entry(&mut axis.overrides_mut(minor).visible, index) = Some(visible));
        self
    }

    /// Whether the major (or `minor`) tick at `index` is drawn (see
    /// [`set_tick_visible`](Self::set_tick_visible)).
    pub fn tick_visible(&self, minor: bool, index: usize) -> bool {
        self.read(|axis| axis.tick_visible(minor, index))
    }

    /// Draws the major (or `minor`) tick at `index` through `filter`, or
    /// plainly for `None` (matplotlib's `Tick.set_agg_filter` on one tick
    /// of an axis). Ticks made later are drawn plainly.
    pub fn set_tick_agg_filter(
        &self,
        minor: bool,
        index: usize,
        filter: Option<AggFilter>,
    ) -> &Self {
        self.update(|axis| *entry(&mut axis.overrides_mut(minor).agg_filters, index) = filter);
        self
    }

    /// The image filter the major (or `minor`) tick at `index` is drawn
    /// through, if any (see [`set_tick_agg_filter`](Self::set_tick_agg_filter)).
    pub fn tick_agg_filter(&self, minor: bool, index: usize) -> Option<AggFilter> {
        self.read(|axis| axis.tick_agg_filter(minor, index).cloned())
    }
}

impl AxisState {
    /// Whether the major (or `minor`) tick at `index` is drawn.
    pub(crate) fn tick_visible(&self, minor: bool, index: usize) -> bool {
        let visible = self.overrides(minor).visible.get(index).copied().flatten();
        visible.unwrap_or(true)
    }

    /// The image filter the major (or `minor`) tick at `index` is drawn
    /// through, if any.
    pub(crate) fn tick_agg_filter(&self, minor: bool, index: usize) -> Option<&AggFilter> {
        self.overrides(minor).agg_filters.get(index)?.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::{AggFilter, Filtered};
    use crate::axes::{TickLevel, TickParams};
    use crate::figure::Figure;
    use crate::testing::RecordingRenderer;

    /// How many calls a draw of `fig` makes.
    fn drawn_calls(fig: &Figure) -> usize {
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        renderer.calls.len()
    }

    #[test]
    fn a_hidden_tick_draws_nothing_and_keeps_the_axis_extent() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xticks([0.0, 0.5, 1.0]);
        let shown = drawn_calls(&fig);
        let extent = ax.xaxis().tight_bbox();
        ax.xaxis().set_tick_visible(false, 1, false);
        assert!(drawn_calls(&fig) < shown, "the tick is left out");
        assert_eq!(ax.xaxis().tight_bbox(), extent, "its label still counts");
        ax.xaxis().set_tick_visible(false, 1, true);
        assert_eq!(drawn_calls(&fig), shown);
    }

    #[test]
    fn ticks_made_later_take_the_first_ticks_visibility_only() {
        let ax = Figure::new().subplot();
        let axis = ax.xaxis();
        let unchanged = AggFilter::new(|image, _| Filtered {
            image: image.clone(),
            offset: (0.0, 0.0),
        });
        axis.set_tick_visible(false, 0, false)
            .set_tick_agg_filter(false, 0, Some(unchanged))
            .make_ticks(false, 4);
        assert!(!axis.tick_visible(false, 3), "copied from the first tick");
        assert!(axis.tick_agg_filter(false, 3).is_none(), "not copied");
        assert!(axis.tick_agg_filter(false, 0).is_some());
    }

    #[test]
    fn a_reset_of_the_tick_parameters_makes_the_ticks_anew() {
        let ax = Figure::new().subplot();
        ax.yaxis().set_tick_visible(true, 2, false);
        ax.tick_params(TickParams {
            reset: true,
            which: TickLevel::Both,
            ..TickParams::default()
        });
        assert!(ax.yaxis().tick_visible(true, 2));
    }
}
