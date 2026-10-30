//! Dividers (matplotlib's `axes_grid1.axes_divider.Divider`): a rectangle
//! split into columns and rows by [`Size`]s, recomputed at every layout
//! pass, so that axes placed in its cells keep fixed paddings in inches
//! and the proportions of their data.

use pltrs_core::{Anchor, Bbox, Point};

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

pltrs_core::archive_struct!(DividerId(v0));

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

pltrs_core::archive_enum!(DividerRect {
    Fixed(v0),
    Subplot(v0),
    Axes(v0),
});

/// Whether the cells keep the proportions of their relative sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DividerAspect {
    /// Always, or never.
    Fixed(bool),
    /// When the axes has a fixed aspect (`AxesDivider`'s default).
    OfAxes(ArtistId),
}

pltrs_core::archive_enum!(DividerAspect {
    Fixed(v0),
    OfAxes(v0),
});

/// How a divider sizes its cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum DividerKind {
    /// Columns and rows scaled independently (`Divider`).
    #[default]
    Grid,
    /// One row of cells of equal height, the n-th vertical size giving
    /// the height of the n-th column (`HBoxDivider`).
    HBox,
    /// One column of cells of equal width, the n-th horizontal size
    /// giving the width of the n-th row (`VBoxDivider`).
    VBox,
}

pltrs_core::archive_enum!(DividerKind { Grid, HBox, VBox });

/// A rectangle split by sizes (matplotlib's `Divider` and subclasses).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DividerState {
    /// The frame whose fractions the rectangle is in.
    pub(crate) frame: ArtistId,
    pub(crate) rect: DividerRect,
    /// Column widths, left to right.
    pub(crate) horizontal: Vec<Size>,
    /// Row heights, bottom to top.
    pub(crate) vertical: Vec<Size>,
    pub(crate) aspect: DividerAspect,
    pub(crate) kind: DividerKind,
    /// Where the rectangle shrunk to the aspect sits.
    pub(crate) anchor: Anchor,
    /// Columns inserted at the start since cells were handed out.
    pub(crate) x_ref: usize,
    /// Rows inserted at the bottom since cells were handed out.
    pub(crate) y_ref: usize,
}

pltrs_core::archive_struct!(DividerState {
    frame,
    rect,
    horizontal,
    vertical,
    aspect,
    kind,
    anchor,
    x_ref,
    y_ref
});

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

pltrs_core::archive_struct!(DividerCell {
    divider,
    nx,
    nx1,
    ny,
    ny1
});

impl DividerState {
    /// A grid divider of `rect` in `frame`, without sizes yet.
    pub(crate) fn new(frame: ArtistId, rect: DividerRect, aspect: DividerAspect) -> Self {
        Self {
            frame,
            rect,
            horizontal: Vec::new(),
            vertical: Vec::new(),
            aspect,
            kind: DividerKind::Grid,
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
    prefix_sums(
        sizes
            .iter()
            .map(|(relative, absolute)| relative * k + absolute),
    )
}

/// `0` followed by the running sums of `lengths` (`np.cumsum([0, ...])`).
fn prefix_sums(lengths: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut total = 0.0;
    std::iter::once(0.0)
        .chain(lengths.map(|length| {
            total += length;
            total
        }))
        .collect()
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

/// Solves `a x = b` by Gaussian elimination with partial pivoting, as
/// numpy's `linalg.solve` does for the small systems of box dividers;
/// `None` when `a` is singular.
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        // The first of the largest candidates, like LAPACK's `idamax`.
        let pivot = (col..n).reduce(|best, row| {
            if a[row][col].abs() > a[best][col].abs() {
                row
            } else {
                best
            }
        })?;
        if a[pivot][col] == 0.0 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let (upper, lower) = a.split_at_mut(col + 1);
        let pivot_row = &upper[col];
        let (b_upper, b_lower) = b.split_at_mut(col + 1);
        for (row, rhs) in lower.iter_mut().zip(b_lower) {
            let factor = row[col] / pivot_row[col];
            for (value, &above) in row[col..].iter_mut().zip(&pivot_row[col..]) {
                *value -= factor * above;
            }
            *rhs -= factor * b_upper[col];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let known: f64 = a[row][row + 1..]
            .iter()
            .zip(&x[row + 1..])
            .map(|(coefficient, value)| coefficient * value)
            .sum();
        x[row] = (b[row] - known) / a[row][row];
    }
    Some(x)
}

/// A row of cells of equal height laid out by a box divider.
struct BoxRow {
    /// Lower left corner of the row, in frame fractions.
    origin: Point,
    /// Column edges from the row's left, in inches.
    offsets: Vec<f64>,
    /// The common height, in frame fractions.
    height: f64,
}

/// matplotlib's `_locate` helper of the box dividers, written for an
/// `HBoxDivider` (a `VBoxDivider` passes everything transposed): the
/// scales that give every `equal` size the same height while the
/// `summed` sizes fill the width of `rect` (`size` is the frame in
/// inches), the height capped at that of `rect`, anchored in `rect`.
fn box_row(
    rect: &Bbox,
    summed: &[(f64, f64)],
    equal: &[(f64, f64)],
    (fig_w, fig_h): (f64, f64),
    anchor: Anchor,
) -> Option<BoxRow> {
    let n = equal.len();
    if summed.len() != n || n == 0 {
        return None;
    }
    let max_height = fig_h * rect.height();
    // Unknowns k_0, ..., k_{n-1} and the height H:
    // eq_rel_i * k_i + eq_abs_i = H and sum(sm_rel_i * k_i + sm_abs_i) = width.
    let mut a = vec![vec![0.0; n + 1]; n + 1];
    let mut b = vec![0.0; n + 1];
    for (i, &(relative, absolute)) in equal.iter().enumerate() {
        a[i][i] = relative;
        a[i][n] = -1.0;
        b[i] = -absolute;
    }
    for (i, &(relative, _)) in summed.iter().enumerate() {
        a[n][i] = relative;
    }
    b[n] = fig_w * rect.width() - summed.iter().map(|&(_, absolute)| absolute).sum::<f64>();
    let mut k = solve(a, b)?;
    let height = k.pop()?;
    if height > max_height {
        k = equal
            .iter()
            .map(|(relative, absolute)| (max_height - absolute) / relative)
            .collect();
    }
    let offsets = prefix_sums(
        summed
            .iter()
            .zip(&k)
            .map(|((relative, absolute), k)| relative * k + absolute),
    );
    let width = (offsets.last().copied()? - offsets[0]) / fig_w;
    let height = (k[0] * equal[0].0 + equal[0].1) / fig_h;
    let shrunk = Bbox::from_bounds(rect.x0(), rect.y0(), width, height);
    Some(BoxRow {
        origin: shrunk.anchored(anchor, rect).p0,
        offsets,
        height,
    })
}

/// `bbox` with x and y swapped.
fn transposed(bbox: &Bbox) -> Bbox {
    let (x, y, w, h) = bbox.bounds();
    Bbox::from_bounds(y, x, h, w)
}

impl FigureState {
    /// The position of a divided axes in its frame's fractions
    /// (matplotlib's `Divider._locate`), or `None` if the divider or its
    /// rectangle is gone.
    pub(crate) fn locate_in_divider(&mut self, cell: DividerCell) -> Option<Bbox> {
        let divider = self.layout.dividers.get(cell.divider.0)?.clone();
        let frame = self.frame_bbox(divider.frame)?;
        let size = (frame.width() / self.dpi(), frame.height() / self.dpi());
        let rect = self.divider_rect(&divider)?;
        let horizontal: Vec<(f64, f64)> = divider
            .horizontal
            .iter()
            .map(|s| s.evaluate(self))
            .collect();
        let vertical: Vec<(f64, f64)> = divider.vertical.iter().map(|s| s.evaluate(self)).collect();
        let (x_ref, y_ref) = (signed(divider.x_ref), signed(divider.y_ref));
        let (fig_w, fig_h) = size;
        match divider.kind {
            DividerKind::Grid => {
                let keep_aspect = self.divider_keeps_aspect(&divider);
                grid_cell(
                    &rect,
                    &horizontal,
                    &vertical,
                    size,
                    keep_aspect,
                    divider.anchor,
                )
                .and_then(|(origin, ox, oy)| {
                    let (left, right) = (at(&ox, cell.nx + x_ref)?, at(&ox, cell.nx1 + x_ref)?);
                    let (bottom, top) = (at(&oy, cell.ny + y_ref)?, at(&oy, cell.ny1 + y_ref)?);
                    Some(Bbox::from_bounds(
                        origin.x + left / fig_w,
                        origin.y + bottom / fig_h,
                        (right - left) / fig_w,
                        (top - bottom) / fig_h,
                    ))
                })
            }
            DividerKind::HBox => {
                let row = box_row(&rect, &horizontal, &vertical, size, divider.anchor)?;
                let (left, right) = (
                    at(&row.offsets, cell.nx + x_ref)?,
                    at(&row.offsets, cell.nx1 + x_ref)?,
                );
                Some(Bbox::from_bounds(
                    row.origin.x + left / fig_w,
                    row.origin.y,
                    (right - left) / fig_w,
                    row.height,
                ))
            }
            DividerKind::VBox => {
                let column = box_row(
                    &transposed(&rect),
                    &vertical,
                    &horizontal,
                    (fig_h, fig_w),
                    divider.anchor,
                )?;
                let (bottom, top) = (
                    at(&column.offsets, cell.ny + y_ref)?,
                    at(&column.offsets, cell.ny1 + y_ref)?,
                );
                Some(Bbox::from_bounds(
                    column.origin.y,
                    column.origin.x + bottom / fig_h,
                    column.height,
                    (top - bottom) / fig_h,
                ))
            }
        }
    }

    /// The rectangle the divider `id` splits, in its frame's fractions
    /// (matplotlib's `Divider.get_position`).
    pub(crate) fn divider_position(&self, id: DividerId) -> Option<Bbox> {
        self.divider_rect(self.layout.dividers.get(id.0)?)
    }

    /// The rectangle a divider splits, in its frame's fractions.
    fn divider_rect(&self, divider: &DividerState) -> Option<Bbox> {
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

    /// The grid cells of the divider placing `axes`, if it divides some
    /// (matplotlib's `AxesLocator.get_subplotspec`): a subplot divider's
    /// own, or those of the axes an axes divider splits.
    pub(crate) fn divider_subplot_spec(&self, axes: ArtistId) -> Option<&SubplotSpec> {
        let cell = self.store.get_as::<AxesState>(axes)?.aspect.divider?;
        match &self.layout.dividers.get(cell.divider.0)?.rect {
            DividerRect::Subplot(spec) => Some(spec),
            DividerRect::Axes(divided) => self
                .store
                .get_as::<AxesState>(*divided)?
                .subplot_spec
                .as_ref(),
            DividerRect::Fixed(_) => None,
        }
    }

    /// Registers `divider` with the figure.
    pub(crate) fn add_divider(&mut self, divider: DividerState) -> DividerId {
        self.layout.dividers.push(divider);
        self.stale = true;
        DividerId(self.layout.dividers.len() - 1)
    }

    /// The divider `id`.
    pub(crate) fn divider(&self, id: DividerId) -> &DividerState {
        self.layout
            .dividers
            .get(id.0)
            .expect("dividers are never removed from their figure")
    }

    /// Whether `divider` keeps the proportions of its relative sizes: as
    /// set, or, when it is left to its axes, if the axes has a fixed
    /// aspect (matplotlib's `get_aspect`).
    pub(crate) fn divider_keeps_aspect(&self, divider: &DividerState) -> bool {
        match divider.aspect {
            DividerAspect::Fixed(keep) => keep,
            DividerAspect::OfAxes(axes) => self
                .store
                .get_as::<AxesState>(axes)
                .is_some_and(|state| state.aspect.ratio().is_some()),
        }
    }

    /// The divider `id`, mutably.
    pub(crate) fn divider_mut(&mut self, id: DividerId) -> &mut DividerState {
        self.stale = true;
        self.layout
            .dividers
            .get_mut(id.0)
            .expect("dividers are never removed from their figure")
    }

    /// Places `axes`, in the figure or removed from it, in `cell` of its
    /// divider from the next layout pass on (matplotlib's
    /// `set_axes_locator`).
    pub(crate) fn set_divider_cell(&mut self, axes: ArtistId, cell: DividerCell) {
        if let Some(state) = self.artist_as_mut::<AxesState>(axes) {
            state.aspect.divider = Some(cell);
        }
        self.stale = true;
    }
}

/// The lower left corner and the column and row edges (in inches) of a
/// grid divider's cells in `rect` (`size` is the frame in inches),
/// keeping the proportions of the relative sizes when `keep_aspect`.
fn grid_cell(
    rect: &Bbox,
    horizontal: &[(f64, f64)],
    vertical: &[(f64, f64)],
    (fig_w, fig_h): (f64, f64),
    keep_aspect: bool,
    anchor: Anchor,
) -> Option<(Point, Vec<f64>, Vec<f64>)> {
    let k_h = scale(horizontal, fig_w * rect.width());
    let k_v = scale(vertical, fig_h * rect.height());
    if !keep_aspect {
        return Some((rect.p0, offsets(horizontal, k_h), offsets(vertical, k_v)));
    }
    let k = k_h.min(k_v);
    let (ox, oy) = (offsets(horizontal, k), offsets(vertical, k));
    let span = |o: &[f64]| Some(o.last().copied()? - o[0]);
    let shrunk = Bbox::from_bounds(rect.x0(), rect.y0(), span(&ox)? / fig_w, span(&oy)? / fig_h);
    Some((shrunk.anchored(anchor, rect).p0, ox, oy))
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

    #[test]
    fn linear_systems_pivot_and_detect_singularity() {
        let x = solve(
            vec![
                vec![0.0, 2.0, 1.0],
                vec![1.0, 1.0, 0.0],
                vec![3.0, 0.0, 1.0],
            ],
            vec![5.0, 3.0, 4.0],
        )
        .expect("a regular system");
        for (actual, expected) in x.iter().zip([1.0, 2.0, 1.0]) {
            assert!((actual - expected).abs() < 1e-12, "{x:?}");
        }
        assert_eq!(
            solve(vec![vec![1.0, 2.0], vec![2.0, 4.0]], vec![1.0, 2.0]),
            None
        );
    }

    #[test]
    fn box_rows_give_their_cells_one_height() {
        // `demo_axes_hbox_divider`: images of 5 × 4 and 4 × 5 data units
        // with half an inch between them, in the 111 cell of a 6.4 × 4.8
        // inch figure (matplotlib 3.11).
        let rect = Bbox::from_bounds(0.125, 0.11, 0.775, 0.77);
        let summed = [(5.0, 0.0), (0.0, 0.5), (4.0, 0.0)];
        let equal = [(4.0, 0.0), (1.0, 0.0), (5.0, 0.0)];
        let row = box_row(&rect, &summed, &equal, (6.4, 4.8), Anchor::C).expect("a row");
        let k0 = row.offsets[1] / 5.0;
        let k2 = (row.offsets[3] - row.offsets[2]) / 4.0;
        assert!((4.0 * k0 - 5.0 * k2).abs() < 1e-12, "equal heights");
        assert!(
            (row.offsets[3] - 0.775 * 6.4).abs() < 1e-12,
            "the width is filled"
        );
        assert!((row.height - 4.0 * k0 / 4.8).abs() < 1e-12);
        assert!((row.origin.x - 0.125).abs() < 1e-12);
        assert_eq!(
            box_row(&rect, &summed, &equal[..2], (6.4, 4.8), Anchor::C).map(|r| r.height),
            None
        );
    }

    use crate::artist::ArtistHandle;

    #[test]
    fn cells_survive_sizes_inserted_before_them() {
        let frame = crate::figure::Figure::new().raw().id();
        let mut divider = DividerState::new(
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
