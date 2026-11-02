//! Subfigures (matplotlib's `SubFigure`, `subfigures` and
//! `add_subfigure`): figure frames nested in a figure, each with its own
//! background, titles and subplots.

use std::rc::Rc;

use pltrs_core::{Bbox, Rgba};

use super::{Figure, FigureFrame};
use crate::artist::{ArtistId, ArtistLookup, RawHandle};
use crate::colors::Color;
use crate::gridspec::{GridSpec, GridSpecError, SubplotOverrides, SubplotSpec};
use crate::numeric::Grid;

/// Options of [`Figure::subfigures`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubfigureOptions {
    /// Space between columns, as a fraction of the average width (`None`:
    /// subfigures touch).
    pub wspace: Option<f64>,
    /// Space between rows, as a fraction of the average height.
    pub hspace: Option<f64>,
    /// Relative column widths.
    pub width_ratios: Option<Vec<f64>>,
    /// Relative row heights.
    pub height_ratios: Option<Vec<f64>>,
}

impl Figure {
    /// A grid of subfigures filling this figure (matplotlib's
    /// `subfigures`); each is a [`Figure`] handle for its frame, so it has
    /// its own subplots, titles and background.
    ///
    /// ```
    /// use pltrs::figure::{Figure, SubfigureOptions};
    /// let fig = Figure::new();
    /// let parts = fig.subfigures(1, 2, SubfigureOptions::default())?;
    /// parts[(0, 1)].subplot();
    /// assert_eq!(fig.axes().len(), 1);
    /// assert!(parts[(0, 0)].axes().is_empty());
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the grid is empty or the ratios do not match it.
    pub fn subfigures(
        &self,
        nrows: usize,
        ncols: usize,
        options: SubfigureOptions,
    ) -> Result<Grid<Self>, GridSpecError> {
        let grid = GridSpec::with_ratios(
            nrows,
            ncols,
            options.width_ratios.clone(),
            options.height_ratios.clone(),
            SubplotOverrides {
                left: Some(0.0),
                right: Some(1.0),
                bottom: Some(0.0),
                top: Some(1.0),
                wspace: options.wspace,
                hspace: options.hspace,
            },
        )?;
        // matplotlib places subfigures by their ratios alone, unless
        // spacing is given and no layout engine (which leaves the room
        // between them itself) is set: then by the grid's cell positions.
        let spaced = (options.wspace.is_some() || options.hspace.is_some())
            && self.layout_engine().is_none();
        let positions = if spaced {
            Some(grid.grid_positions(&self.subplot_params())?)
        } else {
            None
        };
        let mut cells = Vec::with_capacity(nrows * ncols);
        for row in 0..nrows {
            for col in 0..ncols {
                let spec = grid.cell(row, col)?;
                let extent = match &positions {
                    Some(p) => {
                        Bbox::from_extents(p.lefts[col], p.bottoms[row], p.rights[col], p.tops[row])
                    }
                    None => ratio_extent(&spec),
                };
                cells.push(self.insert_subfigure(extent, spec));
            }
        }
        Ok(Grid::from_vec(nrows, ncols, cells).expect("one subfigure per grid cell"))
    }

    /// A subfigure covering `spec` of this figure (matplotlib's
    /// `add_subfigure`).
    #[must_use]
    pub fn add_subfigure(&self, spec: &SubplotSpec) -> Self {
        self.insert_subfigure(ratio_extent(spec), spec.clone())
    }

    /// The subfigures made directly in this figure (or subfigure), in the
    /// order they were made (matplotlib's `subfigs`).
    ///
    /// ```
    /// use pltrs::figure::{Figure, SubfigureOptions};
    /// let fig = Figure::new();
    /// let parts = fig.subfigures(2, 1, SubfigureOptions::default())?;
    /// parts[(1, 0)].subfigures(1, 2, SubfigureOptions::default())?;
    /// assert_eq!(fig.subfigs().len(), 2);
    /// assert_eq!(parts[(1, 0)].subfigs().len(), 2);
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    pub fn subfigs(&self) -> Vec<Self> {
        let children = self
            .root
            .with::<FigureFrame, _>(|frame| frame.children.clone());
        let state = self.state.borrow();
        children
            .into_iter()
            .filter(|&id| state.artist_as::<FigureFrame>(id).is_some())
            .map(|id| Self {
                state: Rc::clone(&self.state),
                root: RawHandle::new(Rc::clone(&self.state), id),
            })
            .collect()
    }

    /// Every subfigure inside this figure, nested ones included.
    pub(crate) fn subfigures_nested(&self) -> Vec<Self> {
        let state = self.state.borrow();
        let inside = |mut id: ArtistId| loop {
            match state.store.get(id).and_then(|artist| artist.base().parent) {
                Some(parent) if parent == self.root.id() => break true,
                Some(parent) => id = parent,
                None => break false,
            }
        };
        state
            .store
            .ids()
            .filter(|&id| state.store.get_as::<FigureFrame>(id).is_some() && inside(id))
            .map(|id| Self {
                state: Rc::clone(&self.state),
                root: RawHandle::new(Rc::clone(&self.state), id),
            })
            .collect()
    }

    fn insert_subfigure(&self, extent: Bbox, spec: SubplotSpec) -> Self {
        let rc = crate::rc::params();
        let mut frame = FigureFrame::new(
            extent,
            Color::from(Rgba::TRANSPARENT),
            Color::from(rc.figure_edgecolor.resolve().unwrap_or(Rgba::WHITE)),
            self.subplot_params(),
        );
        frame.frameon = rc.figure_frameon;
        frame.subplot_spec = Some(spec);
        let id = self
            .state
            .borrow_mut()
            .adopt(self.root.id(), Box::new(frame));
        Self {
            state: Rc::clone(&self.state),
            root: RawHandle::new(Rc::clone(&self.state), id),
        }
    }
}

/// The part of its figure a subfigure on `spec` covers, from the grid's
/// ratios (matplotlib's `SubFigure._redo_transform_rel_fig`).
fn ratio_extent(spec: &SubplotSpec) -> Bbox {
    let grid = spec.gridspec();
    let (widths, heights) = (grid.width_ratios(), grid.height_ratios());
    let total = |ratios: &[f64]| ratios.iter().sum::<f64>();
    let span = |ratios: &[f64], range: std::ops::Range<usize>| total(&ratios[range]);
    let x0 = span(&widths, 0..spec.cols().start) / total(&widths);
    let dx = span(&widths, spec.cols()) / total(&widths);
    let y0 = 1.0 - span(&heights, 0..spec.rows().end) / total(&heights);
    let dy = span(&heights, spec.rows()) / total(&heights);
    Bbox::from_extents(x0, y0, x0 + dx, y0 + dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_extents_follow_matplotlib() {
        let grid = GridSpec::with_ratios(
            2,
            2,
            Some(vec![2.0, 1.0]),
            None,
            SubplotOverrides::default(),
        )
        .unwrap();
        let top_left = ratio_extent(&grid.cell(0, 0).unwrap());
        assert_eq!(top_left.extents(), (0.0, 0.5, 2.0 / 3.0, 1.0));
        let bottom_right = ratio_extent(&grid.cell(1, 1).unwrap());
        assert_eq!(bottom_right.extents(), (2.0 / 3.0, 0.0, 1.0, 0.5));
    }

    #[test]
    fn subfigures_share_the_subplot_parameters_of_their_figure() {
        let fig = Figure::new();
        let parts = fig.subfigures(1, 2, SubfigureOptions::default()).unwrap();
        let before = parts[(0, 0)].subplot();
        fig.subplots_adjust(SubplotOverrides {
            left: Some(0.05),
            ..SubplotOverrides::default()
        })
        .unwrap();
        let after = parts[(0, 1)].subplot();
        assert_eq!(parts[(0, 1)].subplot_params().left, 0.05);
        assert_eq!(before.position().x0(), 0.05);
        assert_eq!(after.position().x0(), 0.05);
    }

    #[test]
    fn spacing_uses_grid_positions() {
        let fig = Figure::new();
        let options = SubfigureOptions {
            wspace: Some(0.1),
            ..SubfigureOptions::default()
        };
        let parts = fig.subfigures(1, 2, options).unwrap();
        let left = parts[(0, 0)]
            .root
            .with::<FigureFrame, _>(|f| f.relative_extent);
        assert!(left.x1() < 0.5);
    }

    #[test]
    fn layout_engines_leave_the_spacing_to_themselves() {
        let fig = Figure::new();
        fig.set_layout_engine(Some(crate::layout::LayoutEngine::Constrained(
            crate::layout::ConstrainedLayout::default(),
        )))
        .unwrap();
        let options = SubfigureOptions {
            wspace: Some(0.1),
            ..SubfigureOptions::default()
        };
        let parts = fig.subfigures(1, 2, options).unwrap();
        let left = parts[(0, 0)]
            .root
            .with::<FigureFrame, _>(|f| f.relative_extent);
        assert_eq!(left.x1(), 0.5, "placed by the ratios alone");
    }
}
