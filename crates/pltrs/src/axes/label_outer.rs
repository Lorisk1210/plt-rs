//! Tick labels and axis labels kept on the outer edges of a grid of
//! subplots only (matplotlib's `label_outer`).

use super::{Axes, AxesState, AxisName};
use crate::artist::ArtistHandle;
use crate::gridspec::SubplotSpec;
use crate::spines::SpineSide;

impl Axes {
    /// Hides the tick labels, axis labels and offset texts that face
    /// neighbouring subplots, keeping only those on the outer edges of the
    /// grid (matplotlib's `label_outer`). Axes not placed on a grid are
    /// left alone.
    pub fn label_outer(&self) -> &Self {
        self.label_outer_with(false)
    }

    /// [`label_outer`](Self::label_outer), also removing the inner tick
    /// marks when `remove_inner_ticks` is set (matplotlib's
    /// `label_outer(remove_inner_ticks=True)`).
    pub fn label_outer_with(&self, remove_inner_ticks: bool) -> &Self {
        self.label_outer_axis(AxisName::X, remove_inner_ticks);
        self.label_outer_axis(AxisName::Y, remove_inner_ticks);
        self
    }

    /// What grids sharing axis `name` do to their axes (matplotlib's
    /// `_label_outer_xaxis(skip_non_rectangular_axes=True)`): the inner
    /// labels go, except on axes of curved projections, which have no
    /// inner sides.
    ///
    /// ```
    /// use pltrs::axes::AxisName;
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let top = fig.add_subplot(2, 1, 1)?;
    /// top.label_outer_shared(AxisName::X);
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    pub fn label_outer_shared(&self, name: AxisName) {
        let rectangular = self
            .raw
            .with::<AxesState, _>(|axes| axes.projection.is_rectangular());
        if rectangular {
            self.label_outer_axis(name, false);
        }
    }

    /// The grid cell the axes counts as in: its own, or, when a colorbar
    /// took a cell of a grid nested in it, the cell of that grid
    /// (matplotlib's `_get_subplotspec_with_optional_colorbar`).
    fn outer_spec(&self) -> Option<SubplotSpec> {
        let spec = self.subplot_spec()?;
        let figure = self.raw.figure_state().borrow();
        let beside_colorbar = figure.layout.colorbars.iter().any(|colorbar| {
            colorbar.parents.contains(&self.id())
                && figure
                    .store
                    .get_as::<AxesState>(colorbar.cax)
                    .is_some_and(|cax| cax.subplot_spec.is_some())
        });
        let enclosing = beside_colorbar
            .then(|| spec.gridspec().parent().cloned())
            .flatten();
        Some(enclosing.unwrap_or(spec))
    }

    /// matplotlib's `_label_outer_xaxis` / `_label_outer_yaxis`; with
    /// `remove_inner_ticks` the inner tick marks go too.
    fn label_outer_axis(&self, name: AxisName, remove_inner_ticks: bool) {
        let Some(spec) = self.outer_spec() else {
            return;
        };
        self.raw.with_mut::<AxesState, _>(|axes| {
            let (inner_low, inner_high) = match name {
                AxisName::X => (!spec.is_last_row(), !spec.is_first_row()),
                AxisName::Y => (!spec.is_first_col(), !spec.is_last_col()),
            };
            let axis = axes.axis_mut(name);
            for (inner, high) in [(inner_high, true), (inner_low, false)] {
                if !inner {
                    continue;
                }
                let side = SpineSide::of(name, high);
                if axis.label.side == side {
                    axis.label.set_text(String::new());
                }
                axis.restyle_ticks(|style| {
                    let (labels, ticks) = if high {
                        (&mut style.labels.second, &mut style.ticks.second)
                    } else {
                        (&mut style.labels.first, &mut style.ticks.first)
                    };
                    *labels = false;
                    if remove_inner_ticks {
                        *ticks = false;
                    }
                });
                if axis.offset_side == side {
                    axis.offset_visible = false;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle as _;
    use crate::axes::{AxesState, AxisName};
    use crate::figure::{Figure, Share};

    #[test]
    fn label_outer_keeps_only_the_grid_edges() {
        let fig = Figure::new();
        let axs = fig.subplots(2, 2).unwrap();
        for ax in axs.as_slice() {
            ax.set_xlabel("x").set_ylabel("y").label_outer();
        }
        let labels = |row: usize, col: usize| {
            let ax = &axs[(row, col)];
            let shown = ax.raw().with::<AxesState, _>(|state| {
                (
                    state.xaxis.major_style.labels.first,
                    state.yaxis.major_style.labels.first,
                )
            });
            (ax.label(AxisName::X), ax.label(AxisName::Y), shown)
        };
        assert_eq!(labels(1, 0), ("x".into(), "y".into(), (true, true)));
        assert_eq!(labels(0, 0), (String::new(), "y".into(), (false, true)));
        assert_eq!(labels(1, 1), ("x".into(), String::new(), (true, false)));
        assert_eq!(labels(0, 1), (String::new(), String::new(), (false, false)));
    }

    #[test]
    fn label_outer_judges_axes_beside_a_colorbar_by_their_cell() {
        use crate::colorbar::ColorbarOptions;
        use crate::colors::{ScalarMappable, colormaps};
        let fig = Figure::new();
        let axs = fig.subplots(2, 1).unwrap();
        let bottom = &axs[(1, 0)];
        let mapping = ScalarMappable::new(colormaps::get("viridis").unwrap());
        fig.colorbar(&mapping, ColorbarOptions::beside(bottom))
            .unwrap();
        bottom.set_xlabel("x").label_outer();
        assert_eq!(bottom.label(AxisName::X), "x", "still on the last row");
    }

    #[test]
    fn label_outer_can_remove_the_inner_ticks_too() {
        let fig = Figure::new();
        let axs = fig.subplots(2, 1).unwrap();
        let ticks = |row: usize| {
            axs[(row, 0)]
                .raw()
                .with::<AxesState, _>(|state| state.xaxis.major_style.ticks.first)
        };
        axs[(0, 0)].label_outer();
        assert!(ticks(0), "label_outer keeps the tick marks");
        axs[(0, 0)].label_outer_with(true);
        assert!(!ticks(0) && ticks(1));
    }

    #[test]
    fn shared_subplots_hide_inner_tick_labels() {
        let shown = |sharex: Share| {
            let axs = Figure::new()
                .subplots_shared(2, 1, sharex, Share::None)
                .unwrap();
            [0, 1].map(|row| {
                axs[(row, 0)]
                    .raw()
                    .with::<AxesState, _>(|state| state.xaxis.major_style.labels.first)
            })
        };
        assert_eq!(
            shown(Share::Row),
            [true, true],
            "rows sharing x keep every label"
        );
        assert_eq!(shown(Share::Col), [false, true]);
    }
}
