//! Where matplotlib updates the ticks of the axes of a 3-D axes
//! (`Axis._update_ticks`, and `get_minorticklocs`, which numpy's `isclose`
//! has warn when the tolerance it drops the minor ticks on major ones with
//! is not finite: a view outside the domain of the axis' scale).
//!
//! pltrs computes the ticks of a 3-D axis once per layout; the warnings
//! matplotlib's sites issue are issued at the same sites, as often:
//!
//! * a draw of the axes updates the ticks of each axis where it draws its
//!   gridlines (`draw_grid`, with the grid on) and its ticks (`draw`, once
//!   per edge the ticks are on), when the axes are shown, and where it
//!   places its titles (`_update_title_position` measures the y axis
//!   once per title with a text, once per edge);
//! * measuring the axes for a tight box places its titles too, then
//!   measures each shown axis (once per edge);
//! * measuring one axis for a tight box (`Axis3D.get_tightbbox`), reading
//!   its tick labels (`_update_ticks`) and reading its minor tick
//!   locations (`get_minorticklocs`) each count once (per edge for the
//!   first).

use super::axis3d::{Axis3DState, AxisSpan};
use super::coords::Axis3;
use super::state::Axes3DState;
use crate::artist::ArtistStore;
use crate::axes::AxesState;
use crate::text::TextArtist;
use crate::ticker::OverlapTolerance;

/// Which of matplotlib's computations of the minor ticks a site runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MinorTicks {
    /// `_update_ticks`, which asks for the minor ticks only when some of
    /// them are shown and placed by a locator.
    Updated,
    /// `get_minorticklocs` called on its own.
    Located,
}

impl Axis3DState {
    /// Warns as numpy does where matplotlib's `get_minorticklocs` drops
    /// the minor ticks on major ones with a tolerance that is not finite,
    /// when the site runs it (`minor`) and the ticks could be computed.
    pub(crate) fn warn_minor_overlaps(&self, span: &AxisSpan, minor: MinorTicks) {
        let computed = match minor {
            MinorTicks::Updated => {
                Self::level_computed(&self.minor_style, self.tickers.minor_locator.as_ref())
            }
            MinorTicks::Located => true,
        };
        if self.remove_overlapping && computed && self.ticks.is_ok() {
            OverlapTolerance::of_view(span.view, |value| self.scale.forward(value))
                .warn_if_invalid();
        }
    }

    /// One of matplotlib's tick updates of the axis (`_update_ticks`): it
    /// places the axis' ticks (see [`TickSet::labelled`]), and warns as
    /// [`warn_minor_overlaps`](Self::warn_minor_overlaps) does.
    ///
    /// [`TickSet::labelled`]: crate::axes::TickSet::labelled
    pub(crate) fn update_at_site(&self, span: &AxisSpan) {
        if let Ok(ticks) = &self.ticks {
            ticks.place(false);
        }
        self.warn_minor_overlaps(span, MinorTicks::Updated);
    }

    /// How many edges the ticks are drawn on (matplotlib's
    /// `_update_all_positions` updates the ticks once per edge).
    pub(crate) fn tick_edges(&self) -> usize {
        self.tick_position.edges().len()
    }
}

impl Axes3DState {
    /// Issues the warnings of the tick updates of a draw of the axes.
    pub(super) fn warn_drawn_tick_sites(&self, store: &ArtistStore) {
        let titles = self.titles_with_text(store);
        for dir in Axis3::ALL {
            let axis = self.axis(dir);
            let mut sites = self.title_sites(dir, titles);
            if self.axis_on {
                sites += usize::from(self.grid_on) + axis.tick_edges();
            }
            self.warn_sites(dir, sites);
        }
    }

    /// Issues the warnings of the tick updates of a measure of the axes
    /// for a tight box.
    pub(super) fn warn_measured_tick_sites(&self, store: &ArtistStore) {
        let titles = self.titles_with_text(store);
        for dir in Axis3::ALL {
            let axis = self.axis(dir);
            let mut sites = self.title_sites(dir, titles);
            if self.axis_on && axis.visible {
                sites += axis.tick_edges();
            }
            self.warn_sites(dir, sites);
        }
    }

    /// The tick updates of axis `dir` where `titles` titles with a text
    /// are placed: the y axis measured for each, when it is shown.
    fn title_sites(&self, dir: Axis3, titles: usize) -> usize {
        let axis = self.axis(dir);
        if dir == Axis3::Y && axis.visible {
            titles * axis.tick_edges()
        } else {
            0
        }
    }

    /// Issues the warnings of `sites` tick updates of axis `dir`.
    fn warn_sites(&self, dir: Axis3, sites: usize) {
        let span = self.limits.span(dir);
        for _ in 0..sites {
            self.axis(dir).update_at_site(&span);
        }
    }

    /// How many of the titles of the axes have a text.
    fn titles_with_text(&self, store: &ArtistStore) -> usize {
        store.get_as::<AxesState>(self.plane).map_or(0, |plane| {
            (plane.titles.iter())
                .filter(|&&id| {
                    store
                        .get_as::<TextArtist>(id)
                        .is_some_and(|t| !t.text.is_empty())
                })
                .count()
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;
    use crate::scale::ScaleName;
    use crate::testing::RecordingRenderer;

    /// How many tolerance warnings `work` issues.
    fn warned(work: impl FnOnce()) -> usize {
        let ((), warnings) = crate::warnings::capturing_runtime(work);
        warnings
            .iter()
            .filter(|w| w.starts_with("One of rtol or atol"))
            .count()
    }

    #[test]
    fn invalid_tolerances_warn_at_matplotlibs_sites() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let x: Vec<f64> = (0..7).map(f64::from).collect();
        let y: Vec<f64> = x.iter().map(|x| (x - 3.0) / 3.0).collect();
        ax.scatter(&x, &y, &x).expect("matching coordinates");
        ax.set_ylim(-1.0, 1.0).expect("finite limits");
        ax.set_margin(crate::mplot3d::Axis3::Y, 0.0)
            .expect("a valid margin");
        ax.set_yscale(ScaleName::Logit).expect("a known scale");
        ax.set_title("t");
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        // The grid, the ticks of one edge, and the title placement.
        assert_eq!(warned(|| drop(fig.draw(&mut renderer))), 3);
        // The title placement, then the axis.
        assert_eq!(warned(|| assert!(ax.tight_bbox(false).is_some())), 2);
        assert_eq!(warned(|| drop(ax.yaxis().update_ticks())), 1);
        assert_eq!(warned(|| drop(ax.xaxis().update_ticks())), 0);
    }
}
