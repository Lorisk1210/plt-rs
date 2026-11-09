//! What an axis is as an artist of its own, apart from its ticks: whether
//! it is animated and the image filter it is drawn through (matplotlib's
//! `Artist.set_animated` and `set_agg_filter` on an `Axis`, which its
//! `Axes.draw` and `allow_rasterization` honour).

use super::axis_handle::Axis;
use crate::artist::AggFilter;

impl Axis {
    /// Animates the axis, or not: its axes leaves an animated axis out of
    /// the draws of its figure, but for printing, as matplotlib's
    /// `Axes.draw` leaves out its animated children.
    ///
    /// ```
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.xaxis().set_animated(true);
    /// assert!(ax.xaxis().animated() && !ax.yaxis().animated());
    /// ```
    pub fn set_animated(&self, animated: bool) -> &Self {
        self.update(|axis| axis.animated = animated);
        self
    }

    /// Whether the axis is animated (see [`Axis::set_animated`]).
    pub fn animated(&self) -> bool {
        self.read(|axis| axis.animated)
    }

    /// Draws the axis (its ticks, labels and offset text) through `filter`,
    /// or plainly for `None` (matplotlib's `set_agg_filter` on an axis).
    pub fn set_agg_filter(&self, filter: Option<AggFilter>) -> &Self {
        self.update(|axis| axis.agg_filter = filter);
        self
    }

    /// The image filter the axis is drawn through, if any.
    pub fn agg_filter(&self) -> Option<AggFilter> {
        self.read(|axis| axis.agg_filter.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;
    use crate::testing::RecordingRenderer;

    /// How many calls a draw of `fig` (not a print) makes.
    fn drawn_calls(fig: &Figure) -> usize {
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        renderer.calls.len()
    }

    #[test]
    fn an_animated_axis_is_left_out_of_draws_but_printing() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let shown = drawn_calls(&fig);
        ax.xaxis().set_animated(true);
        assert!(drawn_calls(&fig) < shown, "the x axis is left out");
        let svg = fig.to_svg().expect("printed");
        assert!(svg.contains("id=\"xtick_1\""), "printing draws it");
        ax.xaxis().set_animated(false);
        assert_eq!(drawn_calls(&fig), shown);
    }

    #[test]
    fn the_axis_keeps_its_drawing_through_a_clear() {
        let ax = Figure::new().subplot();
        ax.xaxis().set_animated(true);
        ax.clear();
        assert!(ax.xaxis().animated());
        ax.xaxis().clear();
        assert!(ax.xaxis().animated());
        assert!(ax.xaxis().agg_filter().is_none());
    }
}
