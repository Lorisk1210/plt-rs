//! Appending axes beside an existing one (matplotlib's
//! `make_axes_locatable` and `AxesDivider.append_axes`).

use std::rc::Rc;

use plt_core::Bbox;

use super::divider::{Divider, DividerAspect, DividerCell, DividerId, DividerRect, signed};
use super::size::{Size, SizeSpec};
use crate::artist::{Artist, ArtistHandle};
use crate::axes::{Axes, AxesState};
use crate::figure::insert_axes;
use crate::spines::SpineSide;

/// Divides the area of an axes so that axes can be appended beside it,
/// with paddings and widths in inches or as percentages of the axes
/// (matplotlib's `AxesDivider`). The axes and everything appended are
/// placed anew at every layout pass, keeping the axes' aspect.
#[derive(Clone, Debug)]
pub struct AxesDivider {
    axes: Axes,
    divider: DividerId,
}

/// Makes `axes` divisible: from now on it is placed by a divider of its
/// original position, to which axes can be appended (matplotlib's
/// `make_axes_locatable`).
///
/// ```
/// use plt::axes_grid1::{SizeSpec, make_axes_locatable};
/// use plt::prelude::*;
/// use plt::spines::SpineSide;
/// let fig = Figure::new();
/// let ax = fig.subplot();
/// let divider = make_axes_locatable(&ax);
/// let side = divider.append_axes(SpineSide::Right, SizeSpec::Percent(5.0), Some(0.05.into()));
/// let (x, _, w, _) = ax.position().bounds();
/// assert!(side.position().bounds().0 > x + w);
/// ```
pub fn make_axes_locatable(axes: &Axes) -> AxesDivider {
    let mut state = axes.raw().figure_state().borrow_mut();
    let frame = state
        .store
        .get_as::<AxesState>(axes.id())
        .and_then(|a| a.base().parent)
        .unwrap_or(state.root);
    let mut divider = Divider::new(
        frame,
        DividerRect::Axes(axes.id()),
        DividerAspect::OfAxes(axes.id()),
    );
    divider.horizontal.push(Size::axes_x(axes.id()));
    divider.vertical.push(Size::axes_y(axes.id()));
    let id = DividerId(state.layout.dividers.len());
    let cell = divider.cell(id, (0, 1), (0, 1));
    state.add_divider(divider);
    state.set_divider_cell(axes.id(), cell);
    AxesDivider {
        axes: axes.clone(),
        divider: id,
    }
}

impl AxesDivider {
    /// The divided axes.
    pub fn axes(&self) -> &Axes {
        &self.axes
    }

    /// Adds an axes on `side` of the divided axes, `size` wide (left,
    /// right) or tall (bottom, top) and `pad` away from it (matplotlib's
    /// `append_axes`). Percentages are of the divided axes' data width or
    /// height; `pad` defaults to `figure.subplot.wspace` (`hspace`) times
    /// the axes, and a zero pad in inches adds no padding.
    pub fn append_axes(
        &self,
        side: SpineSide,
        size: impl Into<SizeSpec>,
        pad: Option<SizeSpec>,
    ) -> Axes {
        let main = self.axes.id();
        let rc = crate::rc::params();
        let (reference, default_pad) = match side {
            SpineSide::Left | SpineSide::Right => (Size::axes_x(main), rc.figure_subplot_wspace),
            SpineSide::Bottom | SpineSide::Top => (Size::axes_y(main), rc.figure_subplot_hspace),
        };
        let pad = match pad {
            None => Some(reference.clone().times(default_pad)),
            Some(SizeSpec::Inches(0.0)) => None,
            Some(spec) => Some(spec.resolve(&reference)),
        };
        let size = size.into().resolve(&reference);
        let figure = Rc::clone(self.axes.raw().figure_state());
        let (frame, position, cell) = {
            let mut state = figure.borrow_mut();
            let divider = state.divider_mut(self.divider);
            if let Some(pad) = pad {
                divider.append_size(side, pad);
            }
            divider.append_size(side, size);
            let cell = appended_cell(divider, self.divider, side);
            let frame = divider.frame;
            let position = state
                .store
                .get_as::<AxesState>(main)
                .map_or(Bbox::UNIT, |a| a.position);
            (frame, position, cell)
        };
        let appended = insert_axes(&figure, frame, position, None);
        figure.borrow_mut().set_divider_cell(appended.id(), cell);
        appended
    }
}

/// The cell of an axes just appended on `side`: the outermost column or
/// row, level with the divided axes (matplotlib's `new_horizontal` and
/// `new_vertical` locators).
fn appended_cell(divider: &Divider, id: DividerId, side: SpineSide) -> DividerCell {
    let last = |sizes: &[Size]| signed(sizes.len()) - 1;
    let (x_ref, y_ref) = (signed(divider.x_ref), signed(divider.y_ref));
    let (nx, ny) = match side {
        SpineSide::Left => (0, y_ref),
        SpineSide::Right => (last(&divider.horizontal), y_ref),
        SpineSide::Bottom => (x_ref, 0),
        SpineSide::Top => (x_ref, last(&divider.vertical)),
    };
    divider.cell(id, (nx, nx + 1), (ny, ny + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::rc::Aspect;

    #[test]
    fn appended_axes_keep_their_size_in_inches() {
        let fig = Figure::with_size(5.0, 4.0, 100.0);
        let ax = fig.subplot();
        ax.set_xlim(-0.5, 19.5).set_ylim(19.5, -0.5);
        ax.set_aspect(Aspect::Equal).unwrap();
        let divider = make_axes_locatable(&ax);
        let right =
            divider.append_axes(SpineSide::Right, SizeSpec::Percent(5.0), Some(0.05.into()));
        let top = divider.append_axes(SpineSide::Top, 0.6, Some(0.1.into()));
        // matplotlib 3.11: `misc/axes_grid1_append_axes`.
        let expected = [
            (&ax, [0.2576, 0.11, 0.476, 0.595]),
            (&right, [0.7436, 0.11, 0.0238, 0.595]),
            (&top, [0.2576, 0.73, 0.476, 0.15]),
        ];
        for (axes, bounds) in expected {
            let (x, y, w, h) = axes.position().bounds();
            for (actual, expected) in [x, y, w, h].into_iter().zip(bounds) {
                assert!((actual - expected).abs() < 1e-9, "{actual} vs {expected}");
            }
        }
    }
}
