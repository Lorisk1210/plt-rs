//! The compressed layout: constrained layout that then closes the gaps
//! fixed-aspect axes leave in their cells (matplotlib's
//! `compress_fixed_aspect` and the suptitle it moves).

use pltrs_core::Bbox;

use super::super::grid::Margin;
use super::super::kiwi::SolverError;
use super::tree::{FIGURE, LayoutTree};
use crate::artist::Artist;
use crate::axes::AxesState;
use crate::figure::{FigureFrame, FigureState};
use crate::gridspec::GridSpec;
use crate::text::TextArtist;
use pltrs_text::VAlign;

impl LayoutTree {
    /// matplotlib's `compress_fixed_aspect`: applies every grid-placed
    /// axes' aspect, then widens the figure's margins by half the room
    /// the shrunk axes leave, summed over the columns and rows. Returns
    /// `false`, leaving the margins alone, when the axes do not all come
    /// from one grid specification or none does (where matplotlib raises
    /// a `ValueError` that ends the layout).
    pub(super) fn compress_fixed_aspect(
        &mut self,
        fig: &mut FigureState,
    ) -> Result<bool, SolverError> {
        let placed: Vec<_> = self
            .all_axes()
            .filter_map(|ax| Some((ax.id, ax.spec.clone()?)))
            .collect();
        let mut shared: Option<(GridSpec, Vec<f64>, Vec<f64>)> = None;
        for (id, spec) in placed {
            fig.unstale_axes(id);
            fig.apply_aspect(id);
            let Some(state) = fig.store.get_as::<AxesState>(id) else {
                continue;
            };
            let original = state.position;
            let actual = state.aspect.active_position(&original);
            let (_, extra_w, extra_h) = match &mut shared {
                Some(entry) if entry.0.is_same(spec.gridspec()) => entry,
                Some(_) => return Ok(false),
                None => {
                    let (nrows, ncols) = spec.gridspec().shape();
                    shared.insert((spec.gridspec().clone(), vec![0.0; ncols], vec![0.0; nrows]))
                }
            };
            widen(extra_w, spec.cols(), original.width() - actual.width());
            widen(extra_h, spec.rows(), original.height() - actual.height());
        }
        let Some((_, extra_w, extra_h)) = shared else {
            return Ok(false);
        };
        let w = extra_w.iter().sum::<f64>() / 2.0;
        let h = extra_h.iter().sum::<f64>() / 2.0;
        let mut figure = self.grids.get_mut(self.frames[FIGURE].grid);
        for (margin, size) in [
            (Margin::Left, w),
            (Margin::Right, w),
            (Margin::Top, h),
            (Margin::Bottom, h),
        ] {
            figure.edit_margin_min(margin, size, 0)?;
        }
        Ok(true)
    }

    /// Puts the figure's suptitle just above the compressed layout,
    /// bottom-aligned, as matplotlib does after compressing, unless it
    /// was placed by hand.
    pub(super) fn place_suptitle(&self, fig: &mut FigureState, h_pad: f64) {
        let inner = self
            .grids
            .get(self.frames[FIGURE].grid)
            .inner_bbox(&(0..1), &(0..1));
        let title = fig
            .store
            .get_as::<FigureFrame>(self.frames[FIGURE].id)
            .and_then(|frame| frame.super_texts.title)
            .filter(|title| title.auto_placed);
        if let Some(text) = title.and_then(|title| fig.store.get_as_mut::<TextArtist>(title.id))
            && text.base().in_layout
        {
            text.position.y = inner.y1() + h_pad;
            text.valign = VAlign::Bottom;
        }
    }
}

/// `extra[cells] = np.maximum(extra[cells], amount)` for a positive
/// `amount`.
fn widen(extra: &mut [f64], cells: std::ops::Range<usize>, amount: f64) {
    if amount > 0.0 {
        for slot in &mut extra[cells] {
            *slot = slot.max(amount);
        }
    }
}

/// The actual (aspect-applied) position of a colorbar's single parent in
/// figure fractions, which compressed layouts size the colorbar to
/// (matplotlib's `reposition_colorbar` with `compress=True`).
pub(super) fn actual_parent_position(
    fig: &mut FigureState,
    parent: crate::artist::ArtistId,
) -> Option<Bbox> {
    fig.apply_aspect(parent);
    let state = fig.store.get_as::<AxesState>(parent)?;
    let actual = state.aspect.active_position(&state.position);
    Some(super::frames::FrameMap::of_axes(fig, parent)?.to_figure(&actual))
}

#[cfg(test)]
mod tests {
    use pltrs_text::VAlign;

    use crate::artist::ArtistHandle;
    use crate::axes::{Axes, ImshowOptions};
    use crate::figure::Figure;
    use crate::layout::{ConstrainedLayout, LayoutEngine};
    use crate::numeric::Grid;

    /// Four square images without ticks on a 2 × 2 grid of a new figure
    /// laid out by constrained layout, compressed or not.
    fn square_images(compress: bool) -> (Figure, Grid<Axes>) {
        let fig = Figure::new();
        fig.set_layout_engine(Some(LayoutEngine::Constrained(ConstrainedLayout {
            compress,
            ..ConstrainedLayout::default()
        })))
        .unwrap();
        let axs = fig.subplots(2, 2).expect("a grid");
        for ax in axs.as_slice() {
            let data = Grid::from_fn(10, 10, |r, c| (r * 10 + c) as f64);
            ax.imshow(data, ImshowOptions::default()).expect("an image");
            ax.set_xticks(Vec::<f64>::new())
                .set_yticks(Vec::<f64>::new());
        }
        (fig, axs)
    }

    /// The positions of the top-left and bottom-right of four square
    /// images, and the suptitle's height, under constrained layout.
    fn images(compress: bool) -> ([f64; 4], [f64; 4], f64) {
        crate::rc::context(|| {
            crate::rc::set("text.hinting", "none").expect("valid setting");
            let (fig, axs) = square_images(compress);
            let title = fig.suptitle("t");
            fig.layout();
            let bounds = |row: usize, col: usize| {
                let (x, y, w, h) = axs[(row, col)].position().bounds();
                [x, y, w, h]
            };
            (bounds(0, 0), bounds(1, 1), title.position().y)
        })
    }

    fn close(actual: [f64; 4], expected: [f64; 4]) -> bool {
        actual
            .iter()
            .zip(expected)
            .all(|(a, e)| (a - e).abs() < 1e-6)
    }

    #[test]
    fn compression_closes_the_gaps_between_fixed_aspect_axes() {
        // matplotlib 3.11 under the spec's reference configuration.
        let (top_left, bottom_right, title) = images(false);
        assert!(close(
            top_left,
            [0.078_777, 0.482_639, 0.342_446, 0.456_595]
        ));
        assert!(close(
            bottom_right,
            [0.578_777, 0.008_681, 0.342_446, 0.456_595]
        ));
        assert!((title - 0.991_319).abs() < 1e-6);
        let (top_left, bottom_right, title) = images(true);
        assert!(close(
            top_left,
            [0.151_043, 0.482_639, 0.342_446, 0.456_595]
        ));
        assert!(close(
            bottom_right,
            [0.506_511, 0.008_681, 0.342_446, 0.456_595]
        ));
        assert!((title - 0.956_597).abs() < 1e-6, "the suptitle moves down");
    }

    #[test]
    fn a_suptitle_placed_by_hand_or_out_of_the_layout_stays() {
        // matplotlib's test_compressed_suptitle: each `suptitle` call
        // resets the title, which compressing then moves above the axes,
        // bottom-aligned, unless it is placed by hand or out of the layout.
        let (fig, _) = square_images(true);
        let title = fig.suptitle("t");
        fig.layout();
        assert!(title.position().y < 0.97, "{:?}", title.position());
        assert_eq!(title.valign(), VAlign::Bottom);
        fig.suptitle("t");
        fig.set_suptitle_y(0.98);
        fig.layout();
        assert_eq!((title.position().y, title.valign()), (0.98, VAlign::Top));
        fig.suptitle("t").set_in_layout(false);
        fig.layout();
        assert_eq!((title.position().y, title.valign()), (0.98, VAlign::Top));
    }

    #[test]
    fn axes_of_several_grids_stop_the_layout_after_one_placement() {
        let (line, image) = crate::rc::context(|| {
            crate::rc::set("text.hinting", "none").expect("valid setting");
            let fig = Figure::with_size(6.4, 9.6, 100.0);
            fig.set_layout_engine(Some(LayoutEngine::compressed()))
                .unwrap();
            let parts = fig
                .subfigures(3, 1, crate::figure::SubfigureOptions::default())
                .expect("subfigures");
            let mut first = None;
            for part in parts.as_slice() {
                let axs = part.subplots(1, 2).expect("a grid");
                axs[(0, 0)].plot([-10.0, 10.0], [0.0, 1.5]).expect("a line");
                axs[(0, 0)].set_xlim(-10.0, 10.0).expect("finite limits");
                let data = Grid::from_fn(2, 2, |r, c| (r * 2 + c) as f64);
                axs[(0, 1)]
                    .imshow(data, ImshowOptions::default())
                    .expect("an image");
                first.get_or_insert((axs[(0, 0)].clone(), axs[(0, 1)].clone()));
            }
            fig.layout();
            let (line, image) = first.expect("three subfigures");
            let bounds = |ax: &crate::axes::Axes| {
                let (x, y, w, h) = ax.position().bounds();
                [x, y, w, h]
            };
            (bounds(&line), bounds(&image))
        });
        // matplotlib 3.11 after one draw: compressing axes of three grids
        // raises, which its draw swallows, so the second round of margins
        // (which would move the line's axes to x = 0.056248) never runs.
        assert!(
            close(line, [0.070_066, 0.087_98, 0.398_783, 0.898_822]),
            "{line:?}"
        );
        assert!(
            close(image, [0.577_433, 0.133_219, 0.398_783, 0.808_344]),
            "{image:?}"
        );
    }
}
