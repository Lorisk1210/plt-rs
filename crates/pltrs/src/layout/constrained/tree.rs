//! The layout grids of a figure, its subfigures and their grid
//! specifications (matplotlib's `make_layoutgrids`).

use super::super::grid::{GridOwner, GridParent, LayoutGrids, SolvedGrid};
use super::super::kiwi::SolverError;
use crate::artist::{Artist, ArtistId};
use crate::axes::AxesState;
use crate::figure::{FigureFrame, FigureState};
use crate::gridspec::{GridSpec, SubplotSpec};

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
pub(crate) struct LayoutTree {
    pub(super) grids: LayoutGrids,
    /// What each grid lays out, by grid index.
    owners: Vec<GridOwner>,
    /// The grid of each grid specification, matched by identity.
    specs: Vec<(GridSpec, usize)>,
    /// The figure first, then its subfigures depth first.
    pub(super) frames: Vec<Frame>,
}

impl LayoutTree {
    /// matplotlib's `make_layoutgrids` over the figure laid out in
    /// `rect`; `None` when no axes or subfigure is on a grid.
    pub(super) fn new(fig: &FigureState, rect: [f64; 4]) -> Result<Option<Self>, SolverError> {
        let mut tree = Self {
            grids: LayoutGrids::default(),
            owners: Vec::new(),
            specs: Vec::new(),
            frames: Vec::new(),
        };
        let grid = tree.push_frame_grid(fig.root, &GridParent::Rect(rect))?;
        tree.add_frame(fig, fig.root, grid, None)?;
        Ok((!tree.specs.is_empty()).then_some(tree))
    }

    /// Every grid as solved last, the figure's first and every other
    /// after its parent.
    pub(crate) fn solved_grids(&self) -> Vec<SolvedGrid> {
        self.owners
            .iter()
            .enumerate()
            .map(|(index, owner)| self.grids.solved(index, owner.clone()))
            .collect()
    }

    /// Adds the one-cell grid of the (sub)figure frame `id` in `parent`.
    fn push_frame_grid(&mut self, id: ArtistId, parent: &GridParent) -> Result<usize, SolverError> {
        self.owners.push(GridOwner::Frame(id));
        self.grids.push_single(parent)
    }

    /// Adds the frame `id` with its grid, then, as matplotlib does, its
    /// subfigures and after them the grids of its axes; returns the
    /// frame's index.
    fn add_frame(
        &mut self,
        fig: &FigureState,
        id: ArtistId,
        grid: usize,
        spec: Option<SubplotSpec>,
    ) -> Result<usize, SolverError> {
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
        for &child in &children {
            let Some(spec) = fig
                .store
                .get_as::<FigureFrame>(child)
                .and_then(|subfig| subfig.subplot_spec.clone())
            else {
                continue;
            };
            let outer = self.add_gridspec(spec.gridspec(), grid)?;
            let sub_grid = self.push_frame_grid(
                child,
                &GridParent::Cells {
                    grid: outer,
                    rows: spec.rows(),
                    cols: spec.cols(),
                    inner: true,
                },
            )?;
            let sub = self.add_frame(fig, child, sub_grid, Some(spec))?;
            self.frames[index].subfigs.push(sub);
        }
        for child in children {
            let Some((spec, in_layout)) = placement(fig, child) else {
                continue;
            };
            if let Some(spec) = &spec {
                self.add_gridspec(spec.gridspec(), grid)?;
            }
            self.frames[index].axes.push(Placed {
                id: child,
                spec,
                in_layout,
            });
        }
        Ok(index)
    }

    /// matplotlib's `make_layoutgrids_gs`: the grid of `gridspec`, inside
    /// the frame grid `owner` or, when the specification subdivides a
    /// cell, in an outer one-cell grid over that cell.
    fn add_gridspec(&mut self, gridspec: &GridSpec, owner: usize) -> Result<usize, SolverError> {
        if let Some(index) = self.grid_of(gridspec) {
            return Ok(index);
        }
        let parent = match gridspec.parent() {
            None => GridParent::Cells {
                grid: owner,
                rows: 0..1,
                cols: 0..1,
                inner: true,
            },
            Some(cell) => {
                let outer = self.add_gridspec(cell.gridspec(), owner)?;
                self.owners.push(GridOwner::Nested(gridspec.clone()));
                let top = self.grids.push_single(&GridParent::Cells {
                    grid: outer,
                    rows: cell.rows(),
                    cols: cell.cols(),
                    inner: false,
                })?;
                GridParent::Cells {
                    grid: top,
                    rows: 0..1,
                    cols: 0..1,
                    inner: false,
                }
            }
        };
        self.owners.push(GridOwner::GridSpec(gridspec.clone()));
        let index =
            self.grids
                .push(&parent, &gridspec.width_ratios(), &gridspec.height_ratios())?;
        self.specs.push((gridspec.clone(), index));
        Ok(index)
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

/// The grid cell of the 2-D or 3-D axes `id`, on a grid the layout
/// places (none otherwise), and whether the axes takes part in the
/// layout; `None` for other artists.
fn placement(fig: &FigureState, id: ArtistId) -> Option<(Option<SubplotSpec>, bool)> {
    let (spec, in_layout) = match fig.store.get_as::<AxesState>(id) {
        Some(axes) => (axes.subplot_spec.clone(), axes.base().in_layout),
        None => crate::mplot3d::axes3d_placement(&fig.store, id)?,
    };
    Some((spec.filter(|spec| spec.gridspec().in_layout()), in_layout))
}

/// The grid cell of the 2-D or 3-D axes `id`, if it has one.
pub(super) fn subplot_spec(fig: &FigureState, id: ArtistId) -> Option<SubplotSpec> {
    placement(fig, id)?.0
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
        let mut tree = LayoutTree::new(&state, [0.0, 0.0, 1.0, 1.0])
            .unwrap()
            .unwrap();
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
            LayoutTree::new(&fig.raw().figure_state().borrow(), [0.0, 0.0, 1.0, 1.0])
                .unwrap()
                .is_none()
        );
        let rows = fig.subfigures(2, 1, SubfigureOptions::default()).unwrap();
        rows[(1, 0)].subplots(1, 2).unwrap();
        let state = fig.raw().figure_state().borrow();
        let tree = LayoutTree::new(&state, [0.0, 0.0, 1.0, 1.0])
            .unwrap()
            .unwrap();
        assert_eq!(tree.frames.len(), 3);
        assert_eq!(tree.frames[0].subfigs, vec![1, 2]);
        assert_eq!(tree.frames[2].axes.len(), 2);
        assert_eq!(tree.all_axes().count(), 2);
    }
}
