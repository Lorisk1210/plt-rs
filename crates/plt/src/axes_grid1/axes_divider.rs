//! Appending axes beside an existing one, and room for decorations around
//! it (matplotlib's `make_axes_locatable`, `AxesDivider.append_axes` and
//! `make_axes_area_auto_adjustable`).

use std::rc::Rc;

use plt_core::Bbox;

use super::divider::{DividerAspect, DividerCell, DividerId, DividerRect, DividerState, signed};
use super::figure_divider::Divider;
use super::size::{Size, SizeSpec};
use crate::artist::{Artist, ArtistHandle, ArtistId};
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
    let mut divider = DividerState::new(
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

/// Makes `axes` divisible and pads it on each of `sides` so that the
/// decorations of `use_axes` fit inside its original position, `pad`
/// inches from its edges (matplotlib's `make_axes_area_auto_adjustable`,
/// which defaults to the axes itself, a pad of 0.1 and all four sides).
///
/// ```
/// use plt::axes_grid1::make_axes_area_auto_adjustable;
/// use plt::prelude::*;
/// use plt::spines::SpineSide;
/// let fig = Figure::new();
/// let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
/// make_axes_area_auto_adjustable(&ax, &[&ax], 0.1, &SpineSide::ALL);
/// let (x, y, _, _) = ax.position().bounds();
/// assert!(x > 0.0 && y > 0.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn make_axes_area_auto_adjustable(
    axes: &Axes,
    use_axes: &[&Axes],
    pad: f64,
    sides: &[SpineSide],
) -> AxesDivider {
    let divider = make_axes_locatable(axes);
    divider.add_auto_adjustable_area(use_axes, pad, sides);
    divider
}

impl AxesDivider {
    /// Pads the divided area on each of `sides` by how far the decorations
    /// of `use_axes` (tick labels, axis labels, titles) reach past that
    /// side, plus `pad` inches (matplotlib's `add_auto_adjustable_area`).
    pub fn add_auto_adjustable_area(&self, use_axes: &[&Axes], pad: f64, sides: &[SpineSide]) {
        let measured: Vec<ArtistId> = use_axes.iter().map(|axes| axes.id()).collect();
        let mut state = self.axes.raw().figure_state().borrow_mut();
        let divider = state.divider_mut(self.divider);
        for &side in sides {
            divider.append_size(side, Size::AxesDecorations(measured.clone(), side) + pad);
        }
    }

    /// The divided axes.
    pub fn axes(&self) -> &Axes {
        &self.axes
    }

    /// The divider itself, to read or change its sizes, aspect and
    /// anchor (matplotlib's `AxesDivider` is a `Divider`).
    ///
    /// ```
    /// use plt::axes_grid1::make_axes_locatable;
    /// use plt::prelude::*;
    /// use plt::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// let divider = make_axes_locatable(&ax);
    /// divider.append_axes(SpineSide::Right, 0.5, None);
    /// assert_eq!(divider.divider().horizontal().len(), 3, "axes, pad and appended axes");
    /// assert!(!divider.divider().aspect(), "the axes has no fixed aspect");
    /// ```
    pub fn divider(&self) -> Divider {
        Divider::from_id(self.axes.figure(), self.divider)
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
fn appended_cell(divider: &DividerState, id: DividerId, side: SpineSide) -> DividerCell {
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
        ax.set_xlim(-0.5, 19.5)
            .and_then(|ax| ax.set_ylim(19.5, -0.5))
            .expect("finite limits");
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

    #[test]
    fn auto_adjustable_areas_make_room_for_decorations_on_their_side() {
        let fig = Figure::with_size(5.0, 3.0, 100.0);
        let ax1 = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        let divider = make_axes_locatable(&ax1);
        let ax2 = divider.append_axes(SpineSide::Right, SizeSpec::Percent(100.0), Some(0.3.into()));
        ax2.share_y(&ax1).unwrap();
        ax2.tick_params(crate::axes::TickParams {
            labelleft: Some(false),
            ..Default::default()
        });
        divider.add_auto_adjustable_area(&[&ax1], 0.1, &[SpineSide::Left]);
        divider.add_auto_adjustable_area(&[&ax2], 0.1, &[SpineSide::Right]);
        divider.add_auto_adjustable_area(&[&ax1, &ax2], 0.1, &[SpineSide::Top, SpineSide::Bottom]);
        ax1.set_yticks(vec![0.5])
            .set_yticklabels(["very long label"])
            .unwrap();
        ax2.set_title("Title");
        ax2.set_xlabel("X - Label");
        // matplotlib 3.11 with its default rcParams,
        // `make_room_for_ylabel_using_axesgrid`: the positions the divider's
        // locators give.
        let expected = [
            (
                &ax1,
                [0.249_444_444, 0.178_344_184, 0.324_277_778, 0.717_211_372],
            ),
            (
                &ax2,
                [0.633_722_222, 0.178_344_184, 0.324_277_778, 0.717_211_372],
            ),
        ];
        for (axes, bounds) in expected {
            let (x, y, w, h) = axes.position().bounds();
            for (actual, expected) in [x, y, w, h].into_iter().zip(bounds) {
                assert!((actual - expected).abs() < 1e-9, "{actual} vs {expected}");
            }
        }
    }

    #[test]
    fn measured_axes_keep_their_placement() {
        let fig = Figure::with_size(5.0, 3.0, 100.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_ylabel("label");
        let divider = make_axes_area_auto_adjustable(&ax, &[&ax], 0.1, &SpineSide::ALL);
        let placed = ax.position();
        divider.add_auto_adjustable_area(&[&ax], 0.0, &[]);
        let mut state = fig.raw().figure_state().borrow_mut();
        let (extent, tight) = state.unlocated_extents(ax.id()).unwrap();
        assert_eq!(extent, Bbox::from_bounds(0.0, 0.0, 500.0, 300.0));
        assert!(tight.xmin() < 0.0 && tight.ymin() < 0.0);
        drop(state);
        assert_eq!(ax.position(), placed);
    }
}
