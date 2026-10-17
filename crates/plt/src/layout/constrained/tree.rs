//! The layout grids of a figure, its subfigures and their grid
//! specifications (matplotlib's `make_layoutgrids`), and the conversions
//! between figure fractions and the fractions of a (sub)figure.

use plt_core::Bbox;

use super::super::grid::{LayoutGrid, LayoutGrids, Parent};
use crate::artist::{Artist, ArtistId};
use crate::axes::AxesState;
use crate::figure::{FigureFrame, FigureState};
use crate::gridspec::{GridSpec, SubplotSpec};
use crate::transforms::TransformContext;

/// The index of the figure's own grid.
pub(super) const FIGURE: usize = 0;

/// An axes of a (sub)figure as the layout sees it.
#[derive(Clone, Debug)]
pub(super) struct Placed {
    pub(super) id: ArtistId,
    pub(super) spec: Option<SubplotSpec>,
    pub(super) in_layout: bool,
}

impl Placed {
    /// The grid cells of an axes taking part in the layout.
    pub(super) fn cells(&self) -> Option<&SubplotSpec> {
        self.spec.as_ref().filter(|_| self.in_layout)
    }
}

/// The figure or a subfigure as the layout sees it.
#[derive(Clone, Debug)]
pub(super) struct Frame {
    /// The frame artist.
    pub(super) id: ArtistId,
    /// Its layout grid (one cell; its margins hold the figure's title and
    /// labels).
    pub(super) grid: usize,
    /// For subfigures: the cells covered in the parent's grid.
    pub(super) spec: Option<SubplotSpec>,
    /// Its subfigures, as indices into [`LayoutTree::frames`].
    pub(super) subfigs: Vec<usize>,
    /// Its own axes, in the order they were added.
    pub(super) axes: Vec<Placed>,
}

/// The layout grids of the figure, its subfigures and every grid
/// specification in use (matplotlib's `layoutgrids` dictionary).
#[derive(Debug)]
pub(super) struct LayoutTree {
    pub(super) grids: LayoutGrids,
    /// The grid of each grid specification, matched by identity.
    specs: Vec<(GridSpec, usize)>,
    /// The figure first, then its subfigures depth first.
    pub(super) frames: Vec<Frame>,
}

impl LayoutTree {
    /// matplotlib's `make_layoutgrids` over the figure laid out in
    /// `rect`; `None` when no axes or subfigure is on a grid.
    pub(super) fn new(fig: &FigureState, rect: [f64; 4]) -> Option<Self> {
        let mut tree = Self {
            grids: LayoutGrids::default(),
            specs: Vec::new(),
            frames: Vec::new(),
        };
        let grid = tree.grids.push(LayoutGrid::single(Parent::Rect(rect)));
        tree.add_frame(fig, fig.root, grid, None);
        (!tree.specs.is_empty()).then_some(tree)
    }

    /// Adds the frame `id` with its grid, then its subfigures and the
    /// grids of its axes; returns the frame's index.
    fn add_frame(
        &mut self,
        fig: &FigureState,
        id: ArtistId,
        grid: usize,
        spec: Option<SubplotSpec>,
    ) -> usize {
        let index = self.frames.len();
        self.frames.push(Frame {
            id,
            grid,
            spec,
            subfigs: Vec::new(),
            axes: Vec::new(),
        });
        let children = fig
            .store
            .get_as::<FigureFrame>(id)
            .map(|frame| frame.children.clone())
            .unwrap_or_default();
        for child in children {
            if let Some(subfig) = fig.store.get_as::<FigureFrame>(child) {
                let Some(spec) = subfig.subplot_spec.clone() else {
                    continue;
                };
                let outer = self.add_gridspec(spec.gridspec(), grid);
                let sub_grid = self.grids.push(LayoutGrid::single(Parent::Cells {
                    grid: outer,
                    rows: spec.rows(),
                    cols: spec.cols(),
                    inner: true,
                }));
                let sub = self.add_frame(fig, child, sub_grid, Some(spec));
                self.frames[index].subfigs.push(sub);
            } else if let Some(axes) = fig.store.get_as::<AxesState>(child) {
                if let Some(spec) = &axes.subplot_spec {
                    self.add_gridspec(spec.gridspec(), grid);
                }
                self.frames[index].axes.push(Placed {
                    id: child,
                    spec: axes.subplot_spec.clone(),
                    in_layout: axes.base().in_layout,
                });
            }
        }
        index
    }

    /// matplotlib's `make_layoutgrids_gs`: the grid of `gridspec`, inside
    /// the frame grid `owner` or, when the specification subdivides a
    /// cell, in an outer one-cell grid over that cell.
    fn add_gridspec(&mut self, gridspec: &GridSpec, owner: usize) -> usize {
        if let Some(index) = self.grid_of(gridspec) {
            return index;
        }
        let (nrows, ncols) = gridspec.shape();
        let parent = match gridspec.parent() {
            None => Parent::Cells {
                grid: owner,
                rows: 0..1,
                cols: 0..1,
                inner: true,
            },
            Some(cell) => {
                let outer = self.add_gridspec(cell.gridspec(), owner);
                let top = self.grids.push(LayoutGrid::single(Parent::Cells {
                    grid: outer,
                    rows: cell.rows(),
                    cols: cell.cols(),
                    inner: false,
                }));
                Parent::Cells {
                    grid: top,
                    rows: 0..1,
                    cols: 0..1,
                    inner: false,
                }
            }
        };
        let index = self.grids.push(LayoutGrid::new(
            parent,
            nrows,
            ncols,
            gridspec.width_ratios().to_vec(),
            gridspec.height_ratios().to_vec(),
        ));
        self.specs.push((gridspec.clone(), index));
        index
    }

    /// The grid of `gridspec`, if it has one.
    pub(super) fn grid_of(&self, gridspec: &GridSpec) -> Option<usize> {
        self.specs
            .iter()
            .find(|(known, _)| known.is_same(gridspec))
            .map(|&(_, index)| index)
    }

    /// Every axes of every frame.
    pub(super) fn all_axes(&self) -> impl Iterator<Item = &Placed> {
        self.frames.iter().flat_map(|frame| &frame.axes)
    }
}

/// The extent of the frame `id` in figure fractions (the unit box for the
/// figure itself).
pub(super) fn frame_extent(fig: &FigureState, id: ArtistId) -> Option<Bbox> {
    let bbox = fig.frame_bbox(id)?;
    let (width, height) = fig.size_pixels();
    Some(Bbox::from_extents(
        bbox.x0() / width,
        bbox.y0() / height,
        bbox.x1() / width,
        bbox.y1() / height,
    ))
}

/// `bbox` (figure fractions) in the fractions of a frame spanning
/// `frame` (matplotlib's `transFigure - transSubfigure`).
pub(super) fn to_frame(bbox: &Bbox, frame: &Bbox) -> Bbox {
    let x = |v: f64| (v - frame.x0()) / frame.width();
    let y = |v: f64| (v - frame.y0()) / frame.height();
    Bbox::from_extents(x(bbox.x0()), y(bbox.y0()), x(bbox.x1()), y(bbox.y1()))
}

/// `bbox` in the fractions of a frame spanning `frame`, in figure
/// fractions (matplotlib's `transSubfigure - transFigure`).
pub(super) fn from_frame(bbox: &Bbox, frame: &Bbox) -> Bbox {
    let x = |v: f64| frame.x0() + v * frame.width();
    let y = |v: f64| frame.y0() + v * frame.height();
    Bbox::from_extents(x(bbox.x0()), y(bbox.y0()), x(bbox.x1()), y(bbox.y1()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::{Figure, SubfigureOptions};

    #[test]
    fn nested_grids_sit_in_an_outer_cell() {
        let fig = Figure::new();
        let outer = GridSpec::new(1, 2).unwrap();
        let inner =
            GridSpec::nested(&outer.cell(0, 1).unwrap(), 2, 1, None, None, (None, None)).unwrap();
        fig.add_subplot_spec(&inner.cell(0, 0).unwrap()).unwrap();
        fig.add_subplot_spec(&outer.cell(0, 0).unwrap()).unwrap();
        let state = fig.raw().figure_state().borrow();
        let mut tree = LayoutTree::new(&state, [0.0, 0.0, 1.0, 1.0]).unwrap();
        assert_eq!(tree.specs.len(), 2, "the outer and the nested grid");
        tree.grids.solve();
        let nested = tree.grid_of(&inner).unwrap();
        let top = tree.grids.get(nested).outer_bbox(&(0..1), &(0..1));
        assert_eq!(
            (top.x0(), top.x1(), top.y0(), top.y1()),
            (0.5, 1.0, 0.5, 1.0)
        );
    }

    #[test]
    fn subfigures_get_frames_in_their_parent_grid() {
        let fig = Figure::new();
        assert!(
            LayoutTree::new(&fig.raw().figure_state().borrow(), [0.0, 0.0, 1.0, 1.0]).is_none()
        );
        let rows = fig.subfigures(2, 1, &SubfigureOptions::default()).unwrap();
        rows[(1, 0)].subplots(1, 2).unwrap();
        let state = fig.raw().figure_state().borrow();
        let tree = LayoutTree::new(&state, [0.0, 0.0, 1.0, 1.0]).unwrap();
        assert_eq!(tree.frames.len(), 3);
        assert_eq!(tree.frames[0].subfigs, vec![1, 2]);
        assert_eq!(tree.frames[2].axes.len(), 2);
        assert_eq!(tree.all_axes().count(), 2);
    }

    #[test]
    fn frame_fractions_round_trip() {
        let frame = Bbox::from_extents(0.5, 0.0, 1.0, 0.5);
        let bbox = Bbox::from_extents(0.6, 0.1, 0.9, 0.4);
        let local = to_frame(&bbox, &frame);
        assert!((local.x0() - 0.2).abs() < 1e-12 && (local.y1() - 0.8).abs() < 1e-12);
        let back = from_frame(&local, &frame);
        assert!((back.x1() - 0.9).abs() < 1e-12);
    }
}
