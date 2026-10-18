//! The compressed layout: constrained layout that then closes the gaps
//! fixed-aspect axes leave in their cells (matplotlib's
//! `compress_fixed_aspect` and the suptitle it moves).

use plt_core::Bbox;

use super::super::grid::Margin;
use super::tree::{FIGURE, LayoutTree};
use crate::artist::Artist;
use crate::axes::AxesState;
use crate::figure::{FigureFrame, FigureState};
use crate::gridspec::GridSpec;
use crate::text::TextArtist;
use plt_text::VAlign;

impl LayoutTree {
    /// matplotlib's `compress_fixed_aspect`: applies every grid-placed
    /// axes' aspect, then widens the figure's margins by half the room
    /// the shrunk axes leave, summed over the columns and rows. Returns
    /// `false`, changing nothing, when the axes do not all come from one
    /// grid specification or none does (where matplotlib raises).
    pub(super) fn compress_fixed_aspect(&mut self, fig: &mut FigureState) -> bool {
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
                Some(_) => return false,
                None => {
                    let (nrows, ncols) = spec.gridspec().shape();
                    shared.insert((spec.gridspec().clone(), vec![0.0; ncols], vec![0.0; nrows]))
                }
            };
            widen(extra_w, spec.cols(), original.width() - actual.width());
            widen(extra_h, spec.rows(), original.height() - actual.height());
        }
        let Some((_, extra_w, extra_h)) = shared else {
            return false;
        };
        let w = extra_w.iter().sum::<f64>() / 2.0;
        let h = extra_h.iter().sum::<f64>() / 2.0;
        let figure = self.grids.get_mut(self.frames[FIGURE].grid);
        for (margin, size) in [
            (Margin::Left, w),
            (Margin::Right, w),
            (Margin::Top, h),
            (Margin::Bottom, h),
        ] {
            figure.edit_margin_min(margin, size, 0);
        }
        true
    }

    /// Puts the figure's suptitle just above the compressed layout,
    /// bottom-aligned, as matplotlib does after compressing.
    pub(super) fn place_suptitle(&self, fig: &mut FigureState, h_pad: f64) {
        let inner = self
            .grids
            .get(self.frames[FIGURE].grid)
            .inner_bbox(&(0..1), &(0..1));
        let title = fig
            .store
            .get_as::<FigureFrame>(self.frames[FIGURE].id)
            .and_then(|frame| frame.super_texts.title);
        if let Some(text) = title.and_then(|id| fig.store.get_as_mut::<TextArtist>(id))
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
    let frame = state.base().parent?;
    Some(super::tree::from_frame(
        &actual,
        &super::tree::frame_extent(fig, frame)?,
    ))
}

#[cfg(test)]
mod tests {
    use crate::axes::ImshowOptions;
    use crate::figure::Figure;
    use crate::layout::{ConstrainedLayout, LayoutEngine};
    use crate::numeric::Grid;

    /// The positions of the top-left and bottom-right of four square
    /// images, and the suptitle's height, under constrained layout.
    fn images(compress: bool) -> ([f64; 4], [f64; 4], f64) {
        crate::rc::context(|| {
            crate::rc::set("text.hinting", "none").expect("valid setting");
            let fig = Figure::new();
            fig.set_layout_engine(Some(LayoutEngine::Constrained(ConstrainedLayout {
                compress,
                ..ConstrainedLayout::default()
            })));
            let axs = fig.subplots(2, 2).expect("a grid");
            for ax in axs.as_slice() {
                let data = Grid::from_fn(10, 10, |r, c| (r * 10 + c) as f64);
                ax.imshow(data, ImshowOptions::default()).expect("an image");
                ax.set_xticks(Vec::<f64>::new())
                    .set_yticks(Vec::<f64>::new());
            }
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
}
