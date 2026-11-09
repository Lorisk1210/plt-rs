//! The legend box pinned at a display point (matplotlib's
//! `_legend_box.set_offset((x, y))`, which `DraggableLegend` uses while a
//! legend is dragged), and where a legend's location puts its box
//! (`_findoffset`).

use pltrs_core::Point;

use super::Legend;
use super::artist::LegendArtist;
use crate::artist::{ArtistHandle, LayoutContext};

impl Legend {
    /// Pins the origin of the legend box (the point its extent is offset
    /// from, see [`Legend::located_box_offset`]) at the display point
    /// `offset`, where the legend stays whatever its location says until
    /// the location is set again (matplotlib's
    /// `_legend_box.set_offset((x, y))`); `None` places the legend at its
    /// location again (`_legend_box.set_offset(legend._findoffset)`).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::lines::LineProps;
    /// use pltrs::offsetbox::AnchorLoc;
    /// use pltrs::prelude::*;
    /// use pltrs_core::Point;
    /// let ax = Figure::new().subplot();
    /// ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("line"))?;
    /// let legend = ax.legend(Default::default())?;
    /// let at = legend.located_box_offset().expect("laid out");
    /// legend.set_box_offset(Some(Point::new(at.x + 10.0, at.y - 5.0)));
    /// let moved = legend.raw().window_extent().expect("laid out");
    /// legend.set_loc(AnchorLoc::LowerLeft)?;
    /// assert_eq!(legend.box_offset(), None, "setting the location unpins it");
    /// legend.set_box_offset(Some(Point::new(at.x + 10.0, at.y - 5.0)));
    /// assert_eq!(legend.raw().window_extent(), Some(moved));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_box_offset(&self, offset: Option<Point>) -> &Self {
        self.raw
            .with_mut::<LegendArtist, _>(|legend| legend.box_offset = offset);
        self.raw.figure_state().borrow_mut().stale = true;
        self
    }

    /// The display point the origin of the legend box is pinned at, if it
    /// is (see [`Legend::set_box_offset`]).
    pub fn box_offset(&self) -> Option<Point> {
        self.read(|legend| legend.box_offset)
    }

    /// Where the location of the legend puts the origin of its box in the
    /// figure's current layout, pinned or not (matplotlib's
    /// `_findoffset`): the legend's extent is the box's, which starts at
    /// its descents below and left of the origin, moved by it. `None` for
    /// a legend its figure does not lay out (one removed from it).
    pub fn located_box_offset(&self) -> Option<Point> {
        let state = self.raw().figure_state().prepared();
        let ctx = LayoutContext::new(&state);
        let legend = ctx.store().get_as::<LegendArtist>(self.id())?;
        let fontsize_px = ctx.points_to_pixels(legend.settings.fontsize);
        let bbox = legend.legend_box.bbox(ctx).ok()?;
        legend.located_offset(ctx, &bbox, fontsize_px).ok()
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Point;

    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::legend::LegendOptions;
    use crate::lines::LineProps;

    #[test]
    fn a_pinned_box_moves_the_legend_by_the_offset() {
        let ax = Figure::new().subplot();
        ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("a"))
            .unwrap();
        let legend = ax.legend(LegendOptions::default()).unwrap();
        let before = legend.raw().window_extent().unwrap();
        let at = legend.located_box_offset().unwrap();
        legend.set_box_offset(Some(Point::new(at.x + 10.0, at.y - 5.0)));
        let after = legend.raw().window_extent().unwrap();
        assert!((after.x0() - before.x0() - 10.0).abs() < 1e-9);
        assert!((after.y0() - before.y0() + 5.0).abs() < 1e-9);
        assert_eq!(
            legend.located_box_offset(),
            Some(at),
            "the location is kept"
        );
        legend.set_box_offset(None);
        assert_eq!(legend.raw().window_extent(), Some(before));
    }

    #[test]
    fn a_removed_legend_has_no_located_offset() {
        let ax = Figure::new().subplot();
        ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("a"))
            .unwrap();
        let legend = ax.legend(LegendOptions::default()).unwrap();
        assert!(legend.located_box_offset().is_some());
        legend.raw().remove();
        assert_eq!(legend.located_box_offset(), None);
    }
}
