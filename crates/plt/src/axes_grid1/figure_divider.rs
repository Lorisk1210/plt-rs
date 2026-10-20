//! Dividers of a rectangle of a figure, and axes placed in their cells
//! (matplotlib's `Divider`, `SubplotDivider`, `HBoxDivider` and
//! `VBoxDivider` with `ax.set_axes_locator(divider.new_locator(...))`).

use std::fmt;
use std::ops::Range;
use std::rc::Rc;

use plt_core::{Anchor, Bbox};

use super::divider::{DividerAspect, DividerId, DividerKind, DividerRect, DividerState};
use super::size::Size;
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::figure::Figure;
use crate::gridspec::SubplotSpec;

/// Why an axes cannot be placed by a divider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DividerError {
    /// The axes belongs to another figure than the divider.
    OtherFigure,
}

impl fmt::Display for DividerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OtherFigure => f.write_str("a divider only places axes of its own figure"),
        }
    }
}

impl std::error::Error for DividerError {}

/// A rectangle of a figure split into columns (left to right) and rows
/// (bottom to top) whose widths and heights are [`Size`]s; axes placed in
/// its cells are moved there at every layout pass, so absolute sizes stay
/// fixed in inches while relative ones share the rest of the rectangle.
///
/// A box divider ([`Divider::hbox`], [`Divider::vbox`]) lays its cells out
/// in a single row (column) and gives them all the same height (width).
///
/// ```
/// use plt::axes_grid1::{Divider, Size};
/// use plt::prelude::*;
/// let fig = Figure::with_size(6.0, 6.0, 100.0);
/// let horizontal = vec![Size::Fixed(1.0), Size::Fixed(4.5)];
/// let vertical = vec![Size::Fixed(0.7), Size::Fixed(5.0)];
/// let divider = Divider::new(&fig, [0.0, 0.0, 1.0, 1.0], horizontal, vertical, false);
/// let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
/// divider.place(&ax, 1, 1)?;
/// let (x, y, w, h) = ax.position().bounds();
/// assert!((x - 1.0 / 6.0).abs() < 1e-12 && (w - 0.75).abs() < 1e-12);
/// assert!((y - 0.7 / 6.0).abs() < 1e-12 && (h - 5.0 / 6.0).abs() < 1e-12);
/// # Ok::<(), plt::axes_grid1::DividerError>(())
/// ```
#[derive(Clone, Debug)]
pub struct Divider {
    figure: Figure,
    id: DividerId,
}

impl Divider {
    /// Divides `rect` (`[left, bottom, width, height]` in fractions of
    /// `fig`) by `horizontal` and `vertical`; with `aspect`, the relative
    /// sizes keep their proportions and the cells shrink to fit, centred
    /// (matplotlib's `Divider`).
    pub fn new(
        fig: &Figure,
        rect: [f64; 4],
        horizontal: Vec<Size>,
        vertical: Vec<Size>,
        aspect: bool,
    ) -> Self {
        Self::create(
            fig,
            DividerRect::Fixed(rect),
            horizontal,
            vertical,
            aspect,
            DividerKind::Grid,
        )
    }

    /// Divides the cells `spec` covers in the subplot grid of `fig`
    /// (matplotlib's `SubplotDivider`).
    pub fn subplot(
        fig: &Figure,
        spec: &SubplotSpec,
        horizontal: Vec<Size>,
        vertical: Vec<Size>,
        aspect: bool,
    ) -> Self {
        let rect = DividerRect::Subplot(spec.clone());
        Self::create(fig, rect, horizontal, vertical, aspect, DividerKind::Grid)
    }

    /// Divides the cells `spec` covers into one row: the n-th vertical
    /// size is the height of the n-th column, scaled so that every column
    /// is as tall as the others while the horizontal sizes fill the width
    /// (matplotlib's `HBoxDivider`). Rows are ignored when placing axes.
    pub fn hbox(
        fig: &Figure,
        spec: &SubplotSpec,
        horizontal: Vec<Size>,
        vertical: Vec<Size>,
    ) -> Self {
        let rect = DividerRect::Subplot(spec.clone());
        Self::create(fig, rect, horizontal, vertical, false, DividerKind::HBox)
    }

    /// [`Divider::hbox`] turned on its side: one column of rows of equal
    /// width (matplotlib's `VBoxDivider`). Columns are ignored when
    /// placing axes.
    pub fn vbox(
        fig: &Figure,
        spec: &SubplotSpec,
        horizontal: Vec<Size>,
        vertical: Vec<Size>,
    ) -> Self {
        let rect = DividerRect::Subplot(spec.clone());
        Self::create(fig, rect, horizontal, vertical, false, DividerKind::VBox)
    }

    fn create(
        fig: &Figure,
        rect: DividerRect,
        horizontal: Vec<Size>,
        vertical: Vec<Size>,
        aspect: bool,
        kind: DividerKind,
    ) -> Self {
        let divider = DividerState {
            horizontal,
            vertical,
            kind,
            ..DividerState::new(fig.raw().id(), rect, DividerAspect::Fixed(aspect))
        };
        let id = fig.raw().figure_state().borrow_mut().add_divider(divider);
        Self {
            figure: fig.clone(),
            id,
        }
    }

    /// Updates the divider and re-lays the figure out at the next pass.
    fn update(&self, change: impl FnOnce(&mut DividerState)) -> &Self {
        change(
            self.figure
                .raw()
                .figure_state()
                .borrow_mut()
                .divider_mut(self.id),
        );
        self
    }

    /// Replaces the column widths (matplotlib's `set_horizontal`).
    pub fn set_horizontal(&self, sizes: Vec<Size>) -> &Self {
        self.update(|divider| divider.horizontal = sizes)
    }

    /// Replaces the row heights (matplotlib's `set_vertical`).
    pub fn set_vertical(&self, sizes: Vec<Size>) -> &Self {
        self.update(|divider| divider.vertical = sizes)
    }

    /// Whether the relative sizes keep their proportions (matplotlib's
    /// `set_aspect`).
    pub fn set_aspect(&self, aspect: bool) -> &Self {
        self.update(|divider| divider.aspect = DividerAspect::Fixed(aspect))
    }

    /// Where the cells sit in the rectangle when they do not fill it
    /// (matplotlib's `set_anchor`; centred by default).
    pub fn set_anchor(&self, anchor: Anchor) -> &Self {
        self.update(|divider| divider.anchor = anchor)
    }

    /// The divided rectangle in fractions of the figure (matplotlib's
    /// `get_position`).
    pub fn position(&self) -> Bbox {
        self.figure
            .raw()
            .figure_state()
            .borrow()
            .divider_position(self.id)
            .unwrap_or(Bbox::UNIT)
    }

    /// Places `axes` in the cell of column `nx` and row `ny` from now on
    /// (matplotlib's `ax.set_axes_locator(divider.new_locator(nx, ny))`);
    /// negative indices count from the end.
    ///
    /// # Errors
    ///
    /// When `axes` belongs to another figure.
    pub fn place(&self, axes: &Axes, nx: isize, ny: isize) -> Result<&Self, DividerError> {
        self.place_spanning(axes, nx..nx + 1, ny..ny + 1)
    }

    /// Places `axes` over the block of cells of `columns` and `rows`
    /// (matplotlib's `new_locator(nx, ny, nx1, ny1)`); the range ends may
    /// count from the end when negative.
    ///
    /// # Errors
    ///
    /// When `axes` belongs to another figure.
    pub fn place_spanning(
        &self,
        axes: &Axes,
        columns: Range<isize>,
        rows: Range<isize>,
    ) -> Result<&Self, DividerError> {
        let figure = self.figure.raw().figure_state();
        if !Rc::ptr_eq(figure, axes.raw().figure_state()) {
            return Err(DividerError::OtherFigure);
        }
        let mut state = figure.borrow_mut();
        let cell = state.divider_mut(self.id).cell(
            self.id,
            (columns.start, columns.end),
            (rows.start, rows.end),
        );
        state.set_divider_cell(axes.id(), cell);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gridspec::GridSpec;

    fn bounds(ax: &Axes) -> [f64; 4] {
        let (x, y, w, h) = ax.position().bounds();
        [x, y, w, h]
    }

    fn assert_close(actual: [f64; 4], expected: [f64; 4]) {
        for (a, e) in actual.into_iter().zip(expected) {
            assert!((a - e).abs() < 1e-6, "{actual:?} vs {expected:?}");
        }
    }

    #[test]
    fn scaled_sizes_share_what_fixed_ones_leave() {
        // matplotlib 3.11, `simple_axes_divider1`'s second figure.
        let fig = Figure::with_size(6.0, 6.0, 100.0);
        let horizontal = vec![
            Size::Scaled(1.5),
            Size::Fixed(0.5),
            Size::Scaled(1.0),
            Size::Scaled(0.5),
        ];
        let vertical = vec![Size::Scaled(1.0), Size::Fixed(0.5), Size::Scaled(1.5)];
        let rect = [0.1, 0.1, 0.8, 0.8];
        let divider = Divider::new(&fig, rect, horizontal, vertical, false);
        let corner = fig.add_axes(rect);
        let wide = fig.add_axes(rect);
        divider.place(&corner, 0, 2).unwrap();
        divider.place_spanning(&wide, 2..4, 0..1).unwrap();
        assert_close(bounds(&corner), [0.1, 0.47, 0.358_333, 0.43]);
        assert_close(bounds(&wide), [0.541_667, 0.1, 0.358_333, 0.286_667]);
    }

    #[test]
    fn box_dividers_give_their_cells_one_height_or_width() {
        let fig = Figure::new();
        let spec = GridSpec::new(1, 1).unwrap().cell(0, 0).unwrap();
        let (ax1, ax2) = (
            fig.add_axes([0.0, 0.0, 1.0, 1.0]),
            fig.add_axes([0.0, 0.0, 1.0, 1.0]),
        );
        ax1.set_xlim(-0.5, 4.5).set_ylim(3.5, -0.5);
        ax2.set_xlim(-0.5, 3.5).set_ylim(4.5, -0.5);
        let horizontal = vec![
            Size::axes_x(ax1.id()),
            Size::Fixed(0.5),
            Size::axes_x(ax2.id()),
        ];
        let vertical = vec![
            Size::axes_y(ax1.id()),
            Size::Scaled(1.0),
            Size::axes_y(ax2.id()),
        ];
        let hbox = Divider::hbox(&fig, &spec, horizontal.clone(), vertical.clone());
        hbox.place(&ax1, 0, 0).unwrap().place(&ax2, 2, 0).unwrap();
        let ([_, y1, _, h1], [_, y2, _, h2]) = (bounds(&ax1), bounds(&ax2));
        assert!((y1 - y2).abs() < 1e-12 && (h1 - h2).abs() < 1e-12);
        let vbox = Divider::vbox(&fig, &spec, horizontal, vertical);
        vbox.place(&ax1, 0, 0).unwrap().place(&ax2, 0, 2).unwrap();
        let ([x1, _, w1, _], [x2, _, w2, _]) = (bounds(&ax1), bounds(&ax2));
        assert!((x1 - x2).abs() < 1e-12 && (w1 - w2).abs() < 1e-12);
        let other = Figure::new().subplot();
        assert_eq!(
            vbox.place(&other, 0, 0).unwrap_err(),
            DividerError::OtherFigure
        );
    }
}
