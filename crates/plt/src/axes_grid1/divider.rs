//! Dividers (matplotlib's `axes_grid1.axes_divider.Divider`): a rectangle
//! split into columns and rows by [`Size`]s, recomputed at every layout
//! pass, so that axes placed in its cells keep fixed paddings in inches
//! and the proportions of their data.

use plt_core::{Anchor, Bbox};

use super::size::Size;
use crate::artist::ArtistId;
use crate::axes::AxesState;
use crate::figure::{FigureFrame, FigureState};
use crate::gridspec::SubplotSpec;
use crate::spines::SpineSide;
use crate::transforms::TransformContext;

/// A divider of a figure, by its index among the figure's dividers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DividerId(pub(crate) usize);

/// The rectangle a divider splits, in fractions of its frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DividerRect {
    /// A fixed rectangle `[left, bottom, width, height]` (`Divider`).
    Fixed([f64; 4]),
    /// The cells of a subplot grid, under the frame's current subplot
    /// parameters (`SubplotDivider`).
    Subplot(SubplotSpec),
    /// The original position of an axes (`AxesDivider`).
    Axes(ArtistId),
}

/// Whether the cells keep the proportions of their relative sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DividerAspect {
    /// Always, or never.
    Fixed(bool),
    /// When the axes has a fixed aspect (`AxesDivider`'s default).
    OfAxes(ArtistId),
}

/// A rectangle split by sizes (matplotlib's `Divider` and subclasses).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Divider {
    /// The frame whose fractions the rectangle is in.
    pub(crate) frame: ArtistId,
    pub(crate) rect: DividerRect,
    /// Column widths, left to right.
    pub(crate) horizontal: Vec<Size>,
    /// Row heights, bottom to top.
    pub(crate) vertical: Vec<Size>,
    pub(crate) aspect: DividerAspect,
    /// Where the rectangle shrunk to the aspect sits.
    pub(crate) anchor: Anchor,
    /// Columns inserted at the start since cells were handed out.
    pub(crate) x_ref: usize,
    /// Rows inserted at the bottom since cells were handed out.
    pub(crate) y_ref: usize,
}

/// Where a divided axes sits: a block of cells of a divider, given as
/// offset indices that may count from the end like Python's (matplotlib's
/// `new_locator`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DividerCell {
    pub(crate) divider: DividerId,
    /// First column, relative to the divider's `x_ref` when created.
    pub(crate) nx: isize,
    /// Column after the last.
    pub(crate) nx1: isize,
    /// First row, relative to the divider's `y_ref` when created.
    pub(crate) ny: isize,
    /// Row after the last.
    pub(crate) ny1: isize,
}

impl Divider {
    /// A divider of `rect` in `frame`, without sizes yet.
    pub(crate) fn new(frame: ArtistId, rect: DividerRect, aspect: DividerAspect) -> Self {
        Self {
            frame,
            rect,
            horizontal: Vec::new(),
            vertical: Vec::new(),
            aspect,
            anchor: Anchor::C,
            x_ref: 0,
            y_ref: 0,
        }
    }

    /// The block of columns `nx..nx1` and rows `ny..ny1` (counted from the
    /// current reference cell; negative indices count from the end).
    pub(crate) fn cell(
        &self,
        id: DividerId,
        (nx, nx1): (isize, isize),
        (ny, ny1): (isize, isize),
    ) -> DividerCell {
        let (x_ref, y_ref) = (signed(self.x_ref), signed(self.y_ref));
        DividerCell {
            divider: id,
            nx: nx - x_ref,
            nx1: nx1 - x_ref,
            ny: ny - y_ref,
            ny1: ny1 - y_ref,
        }
    }

    /// Adds a size on `side` (matplotlib's `append_size`), shifting the
    /// reference cell when it goes before it.
    pub(crate) fn append_size(&mut self, side: SpineSide, size: Size) {
        match side {
            SpineSide::Left => {
                self.horizontal.insert(0, size);
                self.x_ref += 1;
            }
            SpineSide::Right => self.horizontal.push(size),
            SpineSide::Bottom => {
                self.vertical.insert(0, size);
                self.y_ref += 1;
            }
            SpineSide::Top => self.vertical.push(size),
        }
    }
}

/// `value` as a signed index (dividers never hold that many sizes).
pub(crate) fn signed(value: usize) -> isize {
    isize::try_from(value).unwrap_or(isize::MAX)
}

/// The prefix sums of `sizes` with the relative parts scaled by `k`
/// (matplotlib's `_calc_offsets`).
fn offsets(sizes: &[(f64, f64)], k: f64) -> Vec<f64> {
    let mut sums = Vec::with_capacity(sizes.len() + 1);
    let mut total = 0.0;
    sums.push(total);
    for (relative, absolute) in sizes {
        total += relative * k + absolute;
        sums.push(total);
    }
    sums
}

/// The scale of relative sizes that makes `sizes` fill `total` inches
/// (matplotlib's `_calc_k`).
fn scale(sizes: &[(f64, f64)], total: f64) -> f64 {
    let (relative, absolute) = sizes
        .iter()
        .fold((0.0, 0.0), |(r, a), (sr, sa)| (r + sr, a + sa));
    if relative == 0.0 {
        0.0
    } else {
        (total - absolute) / relative
    }
}

/// The element at Python index `index` (negative counts from the end).
fn at(values: &[f64], index: isize) -> Option<f64> {
    let len = signed(values.len());
    let index = if index < 0 { len + index } else { index };
    usize::try_from(index)
        .ok()
        .and_then(|i| values.get(i).copied())
}

impl FigureState {
    /// The position of a divided axes in its frame's fractions
    /// (matplotlib's `Divider._locate`), or `None` if the divider or its
    /// rectangle is gone.
    pub(crate) fn locate_in_divider(&mut self, cell: DividerCell) -> Option<Bbox> {
        let divider = self.layout.dividers.get(cell.divider.0)?.clone();
        let frame = self.frame_bbox(divider.frame)?;
        let (fig_w, fig_h) = (frame.width() / self.dpi(), frame.height() / self.dpi());
        let (x, y, w, h) = self.divider_rect(&divider)?.bounds();
        let horizontal: Vec<(f64, f64)> = divider
            .horizontal
            .iter()
            .map(|s| s.evaluate(self))
            .collect();
        let vertical: Vec<(f64, f64)> = divider.vertical.iter().map(|s| s.evaluate(self)).collect();
        let k_h = scale(&horizontal, fig_w * w);
        let k_v = scale(&vertical, fig_h * h);
        let keep_aspect = match divider.aspect {
            DividerAspect::Fixed(keep) => keep,
            DividerAspect::OfAxes(axes) => self
                .store
                .get_as::<AxesState>(axes)
                .is_some_and(|state| state.aspect.ratio().is_some()),
        };
        let (ox, oy, x0, y0) = if keep_aspect {
            let k = k_h.min(k_v);
            let (ox, oy) = (offsets(&horizontal, k), offsets(&vertical, k));
            let span = |o: &[f64]| o.last().copied().unwrap_or(0.0) - o[0];
            let pb = Bbox::from_bounds(x, y, w, h);
            let shrunk = Bbox::from_bounds(x, y, span(&ox) / fig_w, span(&oy) / fig_h);
            let p0 = shrunk.anchored(divider.anchor, &pb).p0;
            (ox, oy, p0.x, p0.y)
        } else {
            (offsets(&horizontal, k_h), offsets(&vertical, k_v), x, y)
        };
        let (x_ref, y_ref) = (signed(divider.x_ref), signed(divider.y_ref));
        let (left, right) = (at(&ox, cell.nx + x_ref)?, at(&ox, cell.nx1 + x_ref)?);
        let (bottom, top) = (at(&oy, cell.ny + y_ref)?, at(&oy, cell.ny1 + y_ref)?);
        Some(Bbox::from_bounds(
            x0 + left / fig_w,
            y0 + bottom / fig_h,
            (right - left) / fig_w,
            (top - bottom) / fig_h,
        ))
    }

    /// The rectangle the divider `id` splits, in its frame's fractions
    /// (matplotlib's `Divider.get_position`).
    pub(crate) fn divider_position(&self, id: DividerId) -> Option<Bbox> {
        self.divider_rect(self.layout.dividers.get(id.0)?)
    }

    /// The rectangle a divider splits, in its frame's fractions.
    fn divider_rect(&self, divider: &Divider) -> Option<Bbox> {
        match &divider.rect {
            DividerRect::Fixed([left, bottom, width, height]) => {
                Some(Bbox::from_bounds(*left, *bottom, *width, *height))
            }
            DividerRect::Subplot(spec) => {
                let params = self
                    .store
                    .get_as::<FigureFrame>(divider.frame)?
                    .subplot_params;
                spec.position(&params).ok()
            }
            DividerRect::Axes(axes) => Some(self.store.get_as::<AxesState>(*axes)?.position),
        }
    }

    /// Registers `divider` with the figure.
    pub(crate) fn add_divider(&mut self, divider: Divider) -> DividerId {
        self.layout.dividers.push(divider);
        self.stale = true;
        DividerId(self.layout.dividers.len() - 1)
    }

    /// The divider `id`, mutably.
    pub(crate) fn divider_mut(&mut self, id: DividerId) -> &mut Divider {
        self.stale = true;
        self.layout
            .dividers
            .get_mut(id.0)
            .expect("dividers are never removed from their figure")
    }

    /// Places `axes` in `cell` of its divider from the next layout pass
    /// on (matplotlib's `set_axes_locator`).
    pub(crate) fn set_divider_cell(&mut self, axes: ArtistId, cell: DividerCell) {
        if let Some(state) = self.store.get_as_mut::<AxesState>(axes) {
            state.aspect.divider = Some(cell);
        }
        self.stale = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_and_scales_follow_matplotlib() {
        let sizes = [(20.0, 0.0), (0.0, 0.05), (1.0, 0.0)];
        let k = scale(&sizes, 3.875);
        assert!((k - 3.825 / 21.0).abs() < 1e-15);
        let ox = offsets(&sizes, 0.119);
        assert_eq!(ox.len(), 4);
        assert!((ox[3] - (2.38 + 0.05 + 0.119)).abs() < 1e-12);
        assert_eq!(scale(&[(0.0, 1.0)], 2.0), 0.0);
        assert_eq!(at(&ox, -1), Some(ox[3]));
        assert_eq!(at(&ox, -2), Some(ox[2]));
        assert_eq!(at(&ox, 4), None);
    }

    use crate::artist::ArtistHandle;

    #[test]
    fn cells_survive_sizes_inserted_before_them() {
        let frame = crate::figure::Figure::new().raw().id();
        let mut divider = Divider::new(
            frame,
            DividerRect::Fixed([0.0, 0.0, 1.0, 1.0]),
            DividerAspect::Fixed(false),
        );
        divider.append_size(SpineSide::Right, Size::Scaled(1.0));
        let cell = divider.cell(DividerId(0), (0, 1), (0, 1));
        divider.append_size(SpineSide::Left, Size::Fixed(1.0));
        divider.append_size(SpineSide::Bottom, Size::Fixed(1.0));
        assert_eq!((divider.x_ref, divider.y_ref), (1, 1));
        assert_eq!((cell.nx + 1, cell.ny + 1), (1, 1));
    }
}
