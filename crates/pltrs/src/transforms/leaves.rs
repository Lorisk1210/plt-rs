//! The leaves of matplotlib's transform trees: the values its `Bbox` and
//! `Affine2D` objects hold at the bottom of `ax.transData`, `ax.bbox`,
//! `fig.transFigure` and their friends, as a figure has them now.
//!
//! matplotlib builds `transData` as `transScale + (transLimits +
//! transAxes)` over an axes' `_position`, `_viewLim` and scales, the
//! figure's `bbox_inches` and `dpi_scale_trans`, and a subfigure's
//! `bbox_relative`, and keeps them up to date as the figure changes.
//! pltrs resolves its [`Transform`](super::Transform)s from its state
//! instead; [`Figure::tree_leaves`] reads that state as matplotlib's
//! leaves, so that code keeping matplotlib's trees (pltrs (Python)) can
//! build them and follow the figure (reading them again for the figures
//! [`take_changed_figures`](crate::figure::take_changed_figures) gives).

use pltrs_core::Bbox;
use pltrs_core::path::DataLimits;

use super::{TransformContext, unit_to_bbox};
use crate::artist::{Artist, ArtistId};
use crate::axes::AxesState;
use crate::figure::{Figure, FigureFrame, FigureState};
use crate::scale::ScaleName;

/// The leaves of a figure's trees: the figure's own and those of the
/// subfigures and axes asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeLeaves {
    /// The resolution (`dpi_scale_trans` scales by it).
    pub dpi: f64,
    /// The size in inches (`bbox_inches` spans it).
    pub size_inches: (f64, f64),
    /// The subfigures asked for that are in the figure.
    pub frames: Vec<FrameLeaves>,
    /// The axes asked for that are in the figure.
    pub axes: Vec<AxesLeaves>,
}

/// The leaf of a subfigure's tree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameLeaves {
    /// The subfigure.
    pub id: ArtistId,
    /// Its box in the fractions of its parent (`bbox_relative`).
    pub relative: Bbox,
}

/// The leaves of an axes' trees.
#[derive(Clone, Debug, PartialEq)]
pub struct AxesLeaves {
    /// The axes.
    pub id: ArtistId,
    /// Where it is drawn, in the fractions of its (sub)figure
    /// (`_position`, after any aspect adjustment).
    pub position: Bbox,
    /// Where it was placed, before any aspect adjustment
    /// (`_originalPosition`).
    pub original: Bbox,
    /// The view limits as they are, without pending autoscaling
    /// (`_viewLim`).
    pub view: Bbox,
    /// The limits of the data and their smallest positive coordinates
    /// (`dataLim`).
    pub data: DataLimits,
    /// The scales of the x and y axis (`transScale` blends their
    /// transforms).
    pub scales: [ScaleName; 2],
    /// What the trees of its projection rest on besides.
    pub projection: ProjectionLeaves,
}

/// The leaves of the trees of an axes' projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectionLeaves {
    /// Rectilinear axes, or a projection whose trees rest on no more.
    Plain,
    /// Polar axes (`PolarAxes._set_lim_and_transforms`).
    Polar {
        /// Where θ = 0 points, in radians (`_theta_offset`).
        theta_offset: f64,
        /// `1` anticlockwise, `-1` clockwise (`_direction`).
        direction: f64,
        /// The radius at the centre, `None` following the lower radial
        /// limit (`_originViewLim.locked_y0`).
        rorigin: Option<f64>,
        /// The angle of the radial tick labels in radians
        /// (`_r_label_position`).
        rlabel_position: f64,
    },
    /// Geographic axes (`GeoAxes._set_lim_and_transforms`).
    Geo {
        /// The latitude, in radians, beyond which longitude gridlines stop
        /// (`_longitude_cap`).
        longitude_cap: f64,
    },
}

impl Figure {
    /// The leaves of matplotlib's transform trees of this figure, of its
    /// subfigures `frames` and of its axes `axes` (ids of other kinds of
    /// artists are passed over), as the figure is now: neither laid out
    /// nor autoscaled first, as matplotlib's trees read the deferred
    /// state too. `None` while the figure is being changed (borrowed by
    /// a draw running).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::figure::Figure;
    /// let fig = Figure::with_size(4.0, 3.0, 50.0);
    /// let ax = fig.add_axes([0.1, 0.2, 0.5, 0.5])?;
    /// ax.set_xlim(0.0, 10.0)?;
    /// let leaves = fig.tree_leaves(&[], &[ax.id()]).expect("not being drawn");
    /// assert_eq!((leaves.dpi, leaves.size_inches), (50.0, (4.0, 3.0)));
    /// assert_eq!(leaves.axes[0].position.extents(), (0.1, 0.2, 0.6, 0.7));
    /// assert_eq!(leaves.axes[0].view.interval_x(), (0.0, 10.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn tree_leaves(&self, frames: &[ArtistId], axes: &[ArtistId]) -> Option<TreeLeaves> {
        let state = self.state_cell().try_borrow()?;
        Some(TreeLeaves {
            dpi: state.dpi(),
            size_inches: state.size_inches,
            frames: frames
                .iter()
                .filter_map(|&id| frame_leaves(&state, id))
                .collect(),
            axes: axes
                .iter()
                .filter_map(|&id| axes_leaves(&state, id))
                .collect(),
        })
    }
}

fn frame_leaves(state: &FigureState, id: ArtistId) -> Option<FrameLeaves> {
    let frame = state.store.get_as::<FigureFrame>(id)?;
    (id != state.root).then_some(FrameLeaves {
        id,
        relative: frame.relative_extent,
    })
}

fn axes_leaves(state: &FigureState, id: ArtistId) -> Option<AxesLeaves> {
    let axes = state.store.get_as::<AxesState>(id)?;
    let (position, original) = positions(state, id, axes)?;
    let projection = if let Some(polar) = axes.polar() {
        ProjectionLeaves::Polar {
            theta_offset: polar.theta_offset,
            direction: polar.direction.sign(),
            rorigin: polar.rorigin,
            rlabel_position: polar.rlabel_position,
        }
    } else if let Some(geo) = axes.geo() {
        ProjectionLeaves::Geo {
            longitude_cap: geo.longitude_cap,
        }
    } else {
        ProjectionLeaves::Plain
    };
    Some(AxesLeaves {
        id,
        position,
        original,
        view: axes.view_lim,
        data: axes.data_lim,
        scales: [axes.xaxis.scale().name(), axes.yaxis.scale().name()],
        projection,
    })
}

/// The active and the original position of an axes in the fractions of
/// its (sub)figure: those it keeps when placed in a (sub)figure, where it
/// is drawn when placed in another axes (an inset, a parasite, a 3-D
/// plane), as matplotlib's locators place those in the figure.
fn positions(state: &FigureState, id: ArtistId, axes: &AxesState) -> Option<(Bbox, Bbox)> {
    let parent = axes.base().parent.or(axes.home).unwrap_or(state.root);
    if state.store.get_as::<FigureFrame>(parent).is_some() {
        let active = axes.aspect.active_position(&axes.position);
        return Some((active, axes.position));
    }
    // An inset or a secondary axes is where its locator placed it, in
    // figure fractions; an axes covering its host (a parasite axes) where
    // the host's box is.
    if let Some((placed, _)) = state.hosted_position(id) {
        return Some((placed, placed));
    }
    let display = state.axes_bbox(id)?;
    let figure = state.figure_bbox_of(parent)?;
    let placed = display.transformed(&unit_to_bbox(&figure).inverted()?);
    Some((placed, placed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::SubfigureOptions;

    #[test]
    fn subfigures_insets_and_projections_give_their_leaves() {
        let fig = Figure::with_size(4.0, 2.0, 100.0);
        let parts = fig.subfigures(1, 2, SubfigureOptions::default()).unwrap();
        let right = &parts[(0, 1)];
        let host = right.add_axes([0.0, 0.0, 1.0, 1.0]).unwrap();
        let inset = host.inset_axes([0.5, 0.5, 0.5, 0.5]);
        let polar = fig.add_polar_axes([0.0, 0.0, 0.5, 0.5]).unwrap();
        polar.set_theta_offset(1.5);
        let leaves = fig
            .tree_leaves(&[right.id(), fig.id()], &[inset.id(), polar.id(), fig.id()])
            .unwrap();
        assert_eq!(leaves.frames.len(), 1, "the root is no subfigure");
        assert_eq!(leaves.frames[0].relative.extents(), (0.5, 0.0, 1.0, 1.0));
        assert_eq!(leaves.axes.len(), 2, "a figure is no axes");
        let inset_leaves = &leaves.axes[0];
        assert_eq!(
            inset_leaves.position.extents(),
            (0.5, 0.5, 1.0, 1.0),
            "in the fractions of the subfigure"
        );
        assert_eq!(inset_leaves.projection, ProjectionLeaves::Plain);
        let ProjectionLeaves::Polar {
            theta_offset,
            direction,
            ..
        } = leaves.axes[1].projection
        else {
            panic!("polar leaves, not {:?}", leaves.axes[1].projection);
        };
        assert_eq!((theta_offset, direction), (1.5, 1.0));
    }

    #[test]
    fn figures_being_changed_have_no_leaves() {
        let fig = Figure::new();
        let held = fig.state_mut();
        assert!(fig.tree_leaves(&[], &[]).is_none());
        drop(held);
        assert!(fig.tree_leaves(&[], &[]).is_some());
    }
}
