//! Adding axes to a figure: free placement, grid cells and subplot grids
//! with shared axes (matplotlib's `add_axes`, `add_subplot`, `subplots`).

use std::cell::RefCell;
use std::rc::Rc;

use plt_core::{Bbox, Point};
use plt_text::HAlign;

use super::{Figure, FigureState};
use crate::artist::{Artist, ArtistId, RawHandle};
use crate::axes::{Axes, AxesState, AxisName, TickLabelProps, TickLevel, TickParams};
use crate::fonts::FontProperties;
use crate::gridspec::{GridSpec, GridSpecError, SubplotOverrides, SubplotSpec};
use crate::numeric::Grid;
use crate::rc::{ColorOrAuto, RcParams};
use crate::spines::{SpineArtist, SpineSide};
use crate::text::TextArtist;
use crate::transforms::Transform;

/// Which axes of a subplot grid share an axis (matplotlib's `sharex` and
/// `sharey` arguments of `subplots`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Share {
    /// Every axes has its own axis.
    #[default]
    None,
    /// All axes share the axis.
    All,
    /// The axes of each row share the axis.
    Row,
    /// The axes of each column share the axis.
    Col,
}

impl Share {
    /// The index of the earlier cell that cell `(row, col)` of a grid with
    /// `ncols` columns shares with, if it has been created already.
    fn target(self, row: usize, col: usize, ncols: usize) -> Option<usize> {
        let target = match self {
            Self::None => return None,
            Self::All => 0,
            Self::Row => row * ncols,
            Self::Col => col,
        };
        (target < row * ncols + col).then_some(target)
    }
}

impl Figure {
    /// Adds axes occupying `[left, bottom, width, height]` in figure fractions.
    pub fn add_axes(&self, rect: [f64; 4]) -> Axes {
        let [left, bottom, width, height] = rect;
        insert_axes(
            &self.state,
            self.root.id(),
            Bbox::from_bounds(left, bottom, width, height),
            None,
        )
    }

    /// Adds axes filling the default subplot area (`fig.add_subplot()`).
    pub fn subplot(&self) -> Axes {
        let spec = GridSpec::new(1, 1)
            .and_then(|grid| grid.cell(0, 0))
            .expect("a 1x1 grid has a cell (0, 0)");
        self.add_subplot_spec(&spec)
            .expect("the default subplot parameters are valid")
    }

    /// Adds axes in cell `index` (1-based, row-major) of an
    /// `nrows × ncols` grid (`fig.add_subplot(nrows, ncols, index)`).
    pub fn add_subplot(
        &self,
        nrows: usize,
        ncols: usize,
        index: usize,
    ) -> Result<Axes, GridSpecError> {
        let spec = GridSpec::new(nrows, ncols)?.numbered(index)?;
        self.add_subplot_spec(&spec)
    }

    /// Adds axes covering a grid cell or block (`fig.add_subplot(gs[...])`).
    pub fn add_subplot_spec(&self, spec: &SubplotSpec) -> Result<Axes, GridSpecError> {
        let position = spec.position(&self.subplot_params())?;
        Ok(insert_axes(
            &self.state,
            self.root.id(),
            position,
            Some(spec.clone()),
        ))
    }

    /// A grid of `nrows × ncols` axes (`fig.subplots(nrows, ncols)`),
    /// indexed by `(row, col)`.
    pub fn subplots(&self, nrows: usize, ncols: usize) -> Result<Grid<Axes>, GridSpecError> {
        self.subplots_shared(nrows, ncols, Share::None, Share::None)
    }

    /// A grid of axes sharing their x and y axes as requested
    /// (`fig.subplots(nrows, ncols, sharex=..., sharey=...)`).
    ///
    /// ```
    /// use plt::figure::{Figure, Share};
    /// let fig = Figure::new();
    /// let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    /// axs[(0, 0)].set_xlim(0.0, 5.0);
    /// assert_eq!(axs[(1, 0)].xlim(), (0.0, 5.0));
    /// # Ok::<(), plt::gridspec::GridSpecError>(())
    /// ```
    pub fn subplots_shared(
        &self,
        nrows: usize,
        ncols: usize,
        sharex: Share,
        sharey: Share,
    ) -> Result<Grid<Axes>, GridSpecError> {
        self.subplots_on(&GridSpec::new(nrows, ncols)?, sharex, sharey)
    }

    /// One axes per cell of `grid`, sharing axes as requested
    /// (matplotlib's `GridSpec.subplots`). When columns share x (or rows
    /// share y), only the outer axes keep their tick labels.
    pub fn subplots_on(
        &self,
        grid: &GridSpec,
        sharex: Share,
        sharey: Share,
    ) -> Result<Grid<Axes>, GridSpecError> {
        let (nrows, ncols) = grid.shape();
        let mut cells: Vec<Axes> = Vec::with_capacity(nrows * ncols);
        for row in 0..nrows {
            for col in 0..ncols {
                let axes = self.add_subplot_spec(&grid.cell(row, col)?)?;
                for (share, name) in [(sharex, AxisName::X), (sharey, AxisName::Y)] {
                    if let Some(target) = share.target(row, col, ncols) {
                        axes.share(name, &cells[target])
                            .expect("a new axes in this figure shares nothing yet");
                    }
                }
                cells.push(axes);
            }
        }
        for (share, name) in [(sharex, AxisName::X), (sharey, AxisName::Y)] {
            let outer = match name {
                AxisName::X => matches!(share, Share::Col | Share::All),
                AxisName::Y => matches!(share, Share::Row | Share::All),
            };
            if outer {
                for axes in &cells {
                    axes.label_outer_axis(name, false);
                }
            }
        }
        Ok(Grid::from_vec(nrows, ncols, cells).expect("one axes per grid cell"))
    }

    /// Makes room for long date tick labels (matplotlib's `autofmt_xdate`):
    /// the x tick labels are rotated by `rotation` degrees and aligned
    /// `halign`; with several subplots only the bottom row keeps them (and
    /// its x labels); grid-placed figures get `bottom` as their bottom
    /// margin.
    ///
    /// # Errors
    ///
    /// When `bottom` leaves no room for the subplots.
    pub fn autofmt_xdate(
        &self,
        bottom: f64,
        rotation: f64,
        halign: HAlign,
    ) -> Result<&Self, GridSpecError> {
        let axes = self.axes();
        let rotated = TickLabelProps {
            rotation: Some(rotation),
            halign: Some(halign),
            ..TickLabelProps::default()
        };
        let all_subplots = axes.iter().all(|ax| ax.subplot_spec().is_some());
        for ax in &axes {
            let bottom_row = ax.subplot_spec().is_none_or(|spec| spec.is_last_row());
            if axes.len() == 1 || bottom_row {
                ax.xaxis().set_ticklabel_props(TickLevel::Major, &rotated);
            } else if all_subplots {
                ax.xaxis().set_tick_params(&TickParams {
                    labelbottom: Some(false),
                    labeltop: Some(false),
                    ..TickParams::default()
                });
                ax.set_xlabel("");
            }
        }
        if all_subplots {
            self.subplots_adjust(&SubplotOverrides {
                bottom: Some(bottom),
                ..SubplotOverrides::default()
            })?;
        }
        Ok(self)
    }

    /// A grid specification using this figure's subplot parameters.
    pub fn add_gridspec(&self, nrows: usize, ncols: usize) -> Result<GridSpec, GridSpecError> {
        GridSpec::new(nrows, ncols)
    }
}

/// Adds an axes at `position` (fractions of `frame`) with its spines and
/// titles.
pub(crate) fn insert_axes(
    figure: &Rc<RefCell<FigureState>>,
    frame: ArtistId,
    position: Bbox,
    spec: Option<SubplotSpec>,
) -> Axes {
    let rc = crate::rc::params();
    let mut axes = AxesState::new(position, Rc::clone(&rc));
    axes.subplot_spec = spec;
    let mut state = figure.borrow_mut();
    let id = state.adopt(frame, Box::new(axes));
    let spines: Vec<_> = SpineSide::ALL
        .into_iter()
        .map(|side| state.adopt(id, Box::new(SpineArtist::new(side, id, Rc::clone(&rc)))))
        .collect();
    let titles: Vec<_> = [
        (0.0, HAlign::Left),
        (0.5, HAlign::Center),
        (1.0, HAlign::Right),
    ]
    .into_iter()
    .map(|(x, halign)| state.adopt(id, Box::new(axes_title(id, x, halign, &rc))))
    .collect();
    if let Some(axes) = state.store.get_as_mut::<AxesState>(id) {
        axes.spines = spines;
        axes.titles = titles;
    }
    drop(state);
    Axes::from_raw(RawHandle::new(Rc::clone(figure), id))
}

/// An empty axes title at axes fraction `x` (matplotlib's `Axes.title`,
/// `_left_title` and `_right_title`).
fn axes_title(axes: ArtistId, x: f64, halign: HAlign, rc: &Rc<RcParams>) -> TextArtist {
    let y = rc.axes_titley.unwrap_or(1.0);
    let mut title = TextArtist::new(Point::new(x, y), "", Rc::clone(rc));
    title.halign = halign;
    title.font = FontProperties::from_rc(rc)
        .with_size(rc.axes_titlesize, rc.font_size)
        .with_weight(rc.axes_titleweight);
    if let ColorOrAuto::Color(color) = &rc.axes_titlecolor {
        title.color = color.clone();
    }
    title.base_mut().transform = Some(Transform::Axes(axes).offset_points(0.0, rc.axes_titlepad));
    title
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_targets_follow_matplotlib() {
        // Cell (1, 2) of a 2x3 grid.
        assert_eq!(Share::All.target(1, 2, 3), Some(0));
        assert_eq!(Share::Row.target(1, 2, 3), Some(3));
        assert_eq!(Share::Col.target(1, 2, 3), Some(2));
        assert_eq!(Share::None.target(1, 2, 3), None);
        // The first cell of each group has nothing to share with yet.
        assert_eq!(Share::All.target(0, 0, 3), None);
        assert_eq!(Share::Row.target(1, 0, 3), None);
        assert_eq!(Share::Col.target(0, 2, 3), None);
    }
}
