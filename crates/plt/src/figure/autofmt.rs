//! Rotated date labels (matplotlib's `Figure.autofmt_xdate`).

use plt_text::HAlign;

use super::Figure;
use crate::axes::{Axes, TickLabelProps, TickLevel, TickParams};
use crate::colorbar::is_colorbar_axes;
use crate::gridspec::{GridSpecError, SubplotOverrides};

impl Figure {
    /// Makes room for long date tick labels (matplotlib's `autofmt_xdate`):
    /// the x tick labels of level `which` are rotated by `rotation`
    /// degrees and aligned `halign`. Colorbars aside, a figure of one axes
    /// has its labels turned; of several subplots, the bottom row keeps
    /// its labels turned and the others lose theirs and their x labels.
    /// Figures of subplots alone get `bottom` as their bottom margin,
    /// unless their layout engine places the axes itself.
    ///
    /// ```
    /// use plt::axes::TickLevel;
    /// use plt::figure::Figure;
    /// use plt::text::HAlign;
    /// let fig = Figure::new();
    /// let axs = fig.subplots(2, 1)?;
    /// fig.autofmt_xdate(0.2, 30.0, HAlign::Right, TickLevel::Major)?;
    /// let (top, bottom) = (axs[(0, 0)].xaxis(), axs[(1, 0)].xaxis());
    /// assert_eq!(bottom.tick_style(false).label_rotation, 30.0);
    /// assert!(!top.tick_style(false).labels.first, "only the bottom row keeps its labels");
    /// # Ok::<(), plt::gridspec::GridSpecError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `bottom` leaves no room for the subplots.
    pub fn autofmt_xdate(
        &self,
        bottom: f64,
        rotation: f64,
        halign: HAlign,
        which: TickLevel,
    ) -> Result<&Self, GridSpecError> {
        let all = self.axes();
        let axes: Vec<&Axes> = all.iter().filter(|ax| !is_colorbar_axes(ax)).collect();
        let all_subplots = axes.iter().all(|ax| ax.subplot_spec().is_some());
        let rotated = TickLabelProps {
            rotation: Some(rotation),
            halign: Some(halign),
            ..TickLabelProps::default()
        };
        if let ([_], Some(first)) = (axes.as_slice(), all.first()) {
            first.xaxis().set_ticklabel_props(which, rotated);
        } else if all_subplots {
            for ax in axes {
                if ax.subplot_spec().is_some_and(|spec| spec.is_last_row()) {
                    ax.xaxis().set_ticklabel_props(which, rotated.clone());
                } else {
                    ax.xaxis().set_tick_params(TickParams {
                        which,
                        labelbottom: Some(false),
                        labeltop: Some(false),
                        ..TickParams::default()
                    });
                    ax.set_xlabel("");
                }
            }
        }
        let adjustable = self
            .layout_engine()
            .is_none_or(|engine| engine.adjust_compatible());
        if all_subplots && adjustable {
            self.subplots_adjust(SubplotOverrides {
                bottom: Some(bottom),
                ..SubplotOverrides::default()
            })?;
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colorbar::ColorbarOptions;
    use crate::layout::LayoutEngine;
    use crate::numeric::Grid;

    #[test]
    fn colorbars_do_not_count_as_axes() {
        // matplotlib's test_autofmt_xdate_colorbar_constrained.
        let fig = Figure::with_size(6.4, 4.8, 100.0);
        fig.set_layout_engine(LayoutEngine::constrained()).unwrap();
        let ax = fig.subplot();
        let data = Grid::from_vec(2, 3, vec![1.0, 4.0, 6.0, 2.0, 3.0, 5.0]).unwrap();
        let image = ax
            .imshow(data, crate::axes::ImshowOptions::default())
            .unwrap();
        let cbar = fig.colorbar(&image, ColorbarOptions::default()).unwrap();
        let before = fig.subplot_params();
        fig.autofmt_xdate(0.2, 30.0, HAlign::Right, TickLevel::Major)
            .unwrap();
        assert_eq!(ax.xaxis().tick_style(false).label_rotation, 30.0);
        assert_eq!(cbar.ax().xaxis().tick_style(false).label_rotation, 0.0);
        assert_eq!(
            fig.subplot_params(),
            before,
            "constrained layout places the axes"
        );
    }

    #[test]
    fn only_the_chosen_level_turns() {
        let fig = Figure::with_size(6.4, 4.8, 100.0);
        let ax = fig.subplot();
        fig.autofmt_xdate(0.2, 60.0, HAlign::Right, TickLevel::Minor)
            .unwrap();
        assert_eq!(ax.xaxis().tick_style(false).label_rotation, 0.0);
        assert_eq!(ax.xaxis().tick_style(true).label_rotation, 60.0);
    }
}
