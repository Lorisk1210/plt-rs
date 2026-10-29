//! Creating 3-D axes in a figure: the figure methods adding them, the
//! options they are set up with, and their projection plane.

use std::rc::Rc;

use pltrs_core::Bbox;

use super::camera::{Camera, ProjType, ViewAngles};
use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::math::Vec3;
use super::state::Axes3DState;
use crate::artist::{ArtistHandle, RawHandle};
use crate::axes::{Axes, AxisName};
use crate::figure::{Figure, axes_bounds, insert_axes};
use crate::gridspec::SubplotSpec;

/// How a new 3-D axes is set up (matplotlib's `Axes3D` keyword
/// arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct Axes3DOptions {
    /// The initial view angles.
    pub view: ViewAngles,
    /// The axis displayed vertically.
    pub vertical_axis: Axis3,
    /// Perspective or orthographic projection.
    pub proj_type: ProjType,
    /// The x:y:z ratio of the box edges; `None` is 4:4:3.
    pub box_aspect: Option<Vec3>,
    /// Whether collections and patches are drawn in depth order rather
    /// than by their own z-order.
    pub computed_zorder: bool,
    /// An axes of the same figure to share the x axis with (matplotlib's
    /// `sharex`).
    pub sharex: Option<Axes3D>,
    /// An axes of the same figure to share the y axis with (`sharey`).
    pub sharey: Option<Axes3D>,
    /// An axes of the same figure to share the z axis with (`sharez`).
    pub sharez: Option<Axes3D>,
    /// An axes of the same figure to rotate together with (`shareview`);
    /// the new axes keeps its own initial view until either is turned.
    pub shareview: Option<Axes3D>,
}

impl Default for Axes3DOptions {
    fn default() -> Self {
        Self {
            view: ViewAngles::default(),
            vertical_axis: Axis3::Z,
            proj_type: ProjType::default(),
            box_aspect: None,
            computed_zorder: true,
            sharex: None,
            sharey: None,
            sharez: None,
            shareview: None,
        }
    }
}

impl Figure {
    /// A 3-D axes filling the default subplot area
    /// (`fig.add_subplot(projection="3d")`).
    pub fn subplot_3d(&self) -> Axes3D {
        self.subplot_3d_with(Axes3DOptions::default())
            .expect("the default 3-D axes options are valid")
    }

    /// A 3-D axes filling the default subplot area, set up by `options`.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::{Axes3DOptions, ProjType};
    /// let fig = Figure::new();
    /// let options = Axes3DOptions { proj_type: ProjType::Ortho, ..Axes3DOptions::default() };
    /// let ax = fig.subplot_3d_with(options)?;
    /// // Orthographic axes keep the `axes.zmargin` around the data.
    /// assert!(ax.zlim().0 < -0.05);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the focal length or the box aspect is invalid, or an axes to
    /// share with belongs to another figure.
    pub fn subplot_3d_with(&self, options: Axes3DOptions) -> Result<Axes3D, Axes3DError> {
        self.add_subplot_3d(1, 1, 1, options)
    }

    /// A 3-D axes in cell `index` (1-based, row-major) of an
    /// `nrows × ncols` grid (`fig.add_subplot(nrows, ncols, index,
    /// projection="3d")`).
    ///
    /// # Errors
    ///
    /// When the grid or cell is invalid, or as [`Figure::subplot_3d_with`].
    pub fn add_subplot_3d(
        &self,
        nrows: usize,
        ncols: usize,
        index: usize,
        options: Axes3DOptions,
    ) -> Result<Axes3D, Axes3DError> {
        let spec = self.numbered_cell(nrows, ncols, index)?;
        self.add_subplot_spec_3d(&spec, options)
    }

    /// A 3-D axes covering a grid cell or block.
    ///
    /// # Errors
    ///
    /// When the cell does not fit the subplot parameters, or as
    /// [`Figure::subplot_3d_with`].
    pub fn add_subplot_spec_3d(
        &self,
        spec: &SubplotSpec,
        options: Axes3DOptions,
    ) -> Result<Axes3D, Axes3DError> {
        let position = spec.position(&self.subplot_params())?;
        insert_axes3d(self, position, Some(spec.clone()), options)
    }

    /// A 3-D axes at `[left, bottom, width, height]` in figure fractions
    /// (`fig.add_axes(rect, projection="3d")`).
    ///
    /// # Errors
    ///
    /// As [`Figure::add_axes`] for `rect`, and as
    /// [`Figure::subplot_3d_with`] for the options.
    pub fn add_axes_3d(
        &self,
        rect: [f64; 4],
        options: Axes3DOptions,
    ) -> Result<Axes3D, Axes3DError> {
        insert_axes3d(self, axes_bounds(rect)?, None, options)
    }

    /// Every 3-D axes of the figure (and of its subfigures), in creation
    /// order.
    pub fn axes3d(&self) -> Vec<Axes3D> {
        let figure = self.raw().figure_state();
        self.scoped::<Axes3DState>()
            .into_iter()
            .map(|id| Axes3D::from_raw(RawHandle::new(Rc::clone(figure), id)))
            .collect()
    }
}

/// Sets both view limits of a projection plane to the view window
/// (matplotlib's `set_xlim(-0.95/dist, 0.9/dist)` and the same for y on a
/// new or cleared 3-D axes).
pub(super) fn show_view_window(plane: &Axes) {
    let (lo, hi) = Camera::view_window();
    for axis in [AxisName::X, AxisName::Y] {
        plane
            .set_limits(axis, lo, hi)
            .expect("the view window is finite");
    }
}

/// Creates a 3-D axes and its projection plane in `figure`.
fn insert_axes3d(
    figure: &Figure,
    position: Bbox,
    spec: Option<SubplotSpec>,
    options: Axes3DOptions,
) -> Result<Axes3D, Axes3DError> {
    let mut camera = Camera::new(options.proj_type)?;
    camera.angles = options.view;
    camera.vertical = options.vertical_axis;
    camera.set_box_aspect(options.box_aspect, 1.0)?;
    let state = figure.raw().figure_state();
    let frame = figure.raw().id();
    let plane = insert_axes(state, frame, Bbox::UNIT, None);
    show_view_window(&plane);
    // mplot3d draws its own frame: matplotlib hides the axes' spines.
    for spine in plane.spines().iter() {
        spine.set_visible(false);
    }
    let mut axes = Axes3DState::new(position, plane.id(), camera, crate::rc::params());
    axes.subplot_spec = spec;
    axes.computed_zorder = options.computed_zorder;
    let mut figure_state = state.borrow_mut();
    let id = figure_state.adopt(frame, Box::new(axes));
    // The plane is drawn by the 3-D axes, not by the frame.
    if let Some(owner) = figure_state.artist_mut(frame) {
        owner.remove_child(plane.id());
    }
    if let Some(plane) = figure_state.artist_mut(plane.id()) {
        plane.base_mut().parent = Some(id);
    }
    drop(figure_state);
    let axes = Axes3D::from_raw(RawHandle::new(Rc::clone(state), id));
    // As in matplotlib, the axes joins its groups before its first
    // autoscaling, which the axes sharing its limits follow.
    let shared = [options.sharex, options.sharey, options.sharez];
    axes.join_initial(
        shared.each_ref().map(Option::as_ref),
        options.shareview.as_ref(),
    )?;
    axes.with_mut(Axes3DState::autoscale_view)?;
    Ok(axes)
}
