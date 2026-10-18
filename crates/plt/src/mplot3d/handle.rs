//! The [`Axes3D`] handle, the figure methods creating 3-D axes, and the
//! view, limit and decoration settings of a 3-D axes.

use std::rc::Rc;

use plt_core::Bbox;

use super::art3d::Art3D;
use super::camera::{Camera, ProjType, ViewAngles};
use super::coords::Axis3;
use super::error::Axes3DError;
use super::math::Vec3;
use super::state::Axes3DState;
use crate::artist::{Artist, ArtistHandle, ArtistId, RawHandle};
use crate::axes::Axes;
use crate::colors::Color;
use crate::figure::{Figure, insert_axes};
use crate::gridspec::{GridSpec, SubplotSpec};
use crate::text::Text;
use crate::transforms::{Transform, TransformContext};

/// How a new 3-D axes is set up (matplotlib's `Axes3D` keyword
/// arguments).
#[derive(Clone, Copy, Debug, PartialEq)]
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
}

impl Default for Axes3DOptions {
    fn default() -> Self {
        Self {
            view: ViewAngles::default(),
            vertical_axis: Axis3::Z,
            proj_type: ProjType::default(),
            box_aspect: None,
            computed_zorder: true,
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
    /// use plt::figure::Figure;
    /// use plt::mplot3d::{Axes3DOptions, ProjType};
    /// let fig = Figure::new();
    /// let options = Axes3DOptions { proj_type: ProjType::Ortho, ..Axes3DOptions::default() };
    /// let ax = fig.subplot_3d_with(options)?;
    /// // Orthographic axes keep the `axes.zmargin` around the data.
    /// assert!(ax.zlim().0 < -0.05);
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the focal length or the box aspect is invalid.
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
        let spec = GridSpec::new(nrows, ncols)?.numbered(index)?;
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
    /// As [`Figure::subplot_3d_with`].
    pub fn add_axes_3d(
        &self,
        rect: [f64; 4],
        options: Axes3DOptions,
    ) -> Result<Axes3D, Axes3DError> {
        let [left, bottom, width, height] = rect;
        insert_axes3d(
            self,
            Bbox::from_bounds(left, bottom, width, height),
            None,
            options,
        )
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
    let (lo, hi) = Camera::view_window();
    plane.set_xlim(lo, hi).set_ylim(lo, hi);
    let mut axes = Axes3DState::new(position, plane.id(), camera, crate::rc::params());
    axes.subplot_spec = spec;
    axes.computed_zorder = options.computed_zorder;
    axes.autoscale_view()?;
    let mut figure_state = state.borrow_mut();
    let id = figure_state.adopt(frame, Box::new(axes));
    // The plane is drawn by the 3-D axes, not by the frame.
    if let Some(owner) = figure_state.store.get_mut(frame) {
        owner.remove_child(plane.id());
    }
    if let Some(plane) = figure_state.store.get_mut(plane.id()) {
        plane.base_mut().parent = Some(id);
    }
    drop(figure_state);
    Ok(Axes3D::from_raw(RawHandle::new(Rc::clone(state), id)))
}

/// A handle to a 3-D axes of a figure (matplotlib's `Axes3D`).
///
/// ```
/// use plt::figure::Figure;
/// let fig = Figure::new();
/// let ax = fig.subplot_3d();
/// ax.plot(&[0.0, 1.0], &[0.0, 2.0], &[0.0, 3.0])?;
/// ax.view_init(20.0, 30.0, 0.0);
/// assert_eq!(ax.zlim(), (-0.0625, 3.0625));
/// # Ok::<(), plt::mplot3d::Axes3DError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Axes3D {
    raw: RawHandle,
}

impl ArtistHandle for Axes3D {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl Axes3D {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    pub(crate) fn with<R>(&self, f: impl FnOnce(&Axes3DState) -> R) -> R {
        self.raw.with::<Axes3DState, _>(f)
    }

    pub(crate) fn with_mut<R>(&self, f: impl FnOnce(&mut Axes3DState) -> R) -> R {
        self.raw.with_mut::<Axes3DState, _>(f)
    }

    /// The 2-D axes whose data coordinates are the projected coordinates.
    pub(crate) fn plane(&self) -> Axes {
        let id = self.with(|axes| axes.plane);
        Axes::from_raw(RawHandle::new(Rc::clone(self.raw.figure_state()), id))
    }

    /// Adds a 2-D artist to the projection plane, clipped to the axes and
    /// in projected coordinates, without 3-D geometry yet.
    pub(crate) fn adopt_projected(&self, mut artist: impl Artist) -> RawHandle {
        let plane = self.plane();
        let base = artist.base_mut();
        base.clip_box = Some(plane.id());
        base.transform = Some(plane.trans_data());
        let figure = self.raw.figure_state();
        let id = figure.borrow_mut().adopt(plane.id(), Box::new(artist));
        RawHandle::new(Rc::clone(figure), id)
    }

    /// Adds a 2-D artist of the projection plane with its 3-D geometry.
    pub(crate) fn add_artist(&self, artist: impl Artist, art: Art3D) -> RawHandle {
        let raw = self.adopt_projected(artist);
        self.register(raw.id(), art);
        raw
    }

    /// Gives an artist already added to the plane its 3-D geometry.
    pub(crate) fn register(&self, id: ArtistId, art: Art3D) {
        self.with_mut(|axes| axes.artists.push((id, art)));
    }

    /// Whether data artists were added (matplotlib's `has_data`).
    pub(crate) fn has_data(&self) -> bool {
        self.with(Axes3DState::has_data)
    }

    /// Includes data in the limits and autoscales (matplotlib's
    /// `auto_scale_xyz`).
    pub(crate) fn auto_scale_xyz(
        &self,
        x: &[f64],
        y: &[f64],
        z: Option<&[f64]>,
        had_data: bool,
    ) -> Result<(), Axes3DError> {
        self.with_mut(|axes| axes.auto_scale_xyz(x, y, z, had_data))
    }

    /// Axes-fraction coordinates (`ax.transAxes`), for 2-D texts and
    /// artists placed over the 3-D scene.
    pub fn trans_axes(&self) -> Transform {
        Transform::Axes(self.with(|axes| axes.plane))
    }

    /// Placement as a fraction of the figure after the box was made
    /// square (matplotlib's `get_position`).
    pub fn position(&self) -> Bbox {
        let state = self.raw.figure_state().borrow();
        let axes = state
            .store
            .get_as::<Axes3DState>(self.id())
            .expect("a live 3-D axes handle refers to a 3-D axes");
        let frame = axes.base().parent.and_then(|f| state.frame_bbox(f));
        frame.map_or(axes.position, |frame| axes.active_position(&frame))
    }

    /// The grid cell the axes occupies, if it was placed on a grid.
    pub fn subplot_spec(&self) -> Option<SubplotSpec> {
        self.with(|axes| axes.subplot_spec.clone())
    }

    /// The view limits of one axis; decreasing when inverted.
    pub fn limits(&self, axis: Axis3) -> (f64, f64) {
        self.with(|axes| axes.limits.view(axis))
    }

    /// The x view limits.
    pub fn xlim(&self) -> (f64, f64) {
        self.limits(Axis3::X)
    }

    /// The y view limits.
    pub fn ylim(&self) -> (f64, f64) {
        self.limits(Axis3::Y)
    }

    /// The z view limits.
    pub fn zlim(&self) -> (f64, f64) {
        self.limits(Axis3::Z)
    }

    /// Sets the view limits of one axis and turns its autoscaling off;
    /// `low > high` inverts the axis (matplotlib's `set_xlim`).
    ///
    /// # Errors
    ///
    /// When a limit is infinite or NaN.
    pub fn set_limits(&self, axis: Axis3, low: f64, high: f64) -> Result<&Self, Axes3DError> {
        self.with_mut(|axes| axes.set_lim3d(axis, low, high, Some(false), None))?;
        Ok(self)
    }

    /// Sets the x view limits.
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_limits`].
    pub fn set_xlim(&self, left: f64, right: f64) -> Result<&Self, Axes3DError> {
        self.set_limits(Axis3::X, left, right)
    }

    /// Sets the y view limits.
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_limits`].
    pub fn set_ylim(&self, bottom: f64, top: f64) -> Result<&Self, Axes3DError> {
        self.set_limits(Axis3::Y, bottom, top)
    }

    /// Sets the z view limits.
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_limits`].
    pub fn set_zlim(&self, bottom: f64, top: f64) -> Result<&Self, Axes3DError> {
        self.set_limits(Axis3::Z, bottom, top)
    }

    /// Flips the direction of one axis (matplotlib's `invert_xaxis`).
    pub fn invert(&self, axis: Axis3) -> &Self {
        self.with_mut(|axes| axes.invert(axis));
        self
    }

    /// Whether one axis runs from high to low values.
    pub fn inverted(&self, axis: Axis3) -> bool {
        self.with(|axes| axes.limits.inverted(axis))
    }

    /// Whether one axis follows the data (matplotlib's
    /// `get_autoscalex_on`).
    pub fn autoscale(&self, axis: Axis3) -> bool {
        self.with(|axes| axes.limits.autoscale[axis.index()])
    }

    /// Turns autoscaling of one axis on or off.
    pub fn set_autoscale(&self, axis: Axis3, on: bool) -> &Self {
        self.with_mut(|axes| axes.limits.autoscale[axis.index()] = on);
        self
    }

    /// Sets the label of one axis (matplotlib's `set_xlabel`).
    pub fn set_label(&self, axis: Axis3, text: impl Into<String>) -> &Self {
        let text = text.into();
        self.with_mut(|axes| axes.axis_mut(axis).label.text = text);
        self
    }

    /// Sets the x label.
    pub fn set_xlabel(&self, text: impl Into<String>) -> &Self {
        self.set_label(Axis3::X, text)
    }

    /// Sets the y label.
    pub fn set_ylabel(&self, text: impl Into<String>) -> &Self {
        self.set_label(Axis3::Y, text)
    }

    /// Sets the z label.
    pub fn set_zlabel(&self, text: impl Into<String>) -> &Self {
        self.set_label(Axis3::Z, text)
    }

    /// The label of one axis.
    pub fn label(&self, axis: Axis3) -> String {
        self.with(|axes| axes.axis(axis).label.text.clone())
    }

    /// Whether the label of one axis follows the axis direction; `None`
    /// rotates labels longer than four characters (matplotlib's
    /// `set_rotate_label`).
    pub fn set_rotate_label(&self, axis: Axis3, rotate: Option<bool>) -> &Self {
        self.with_mut(|axes| axes.axis_mut(axis).label.rotate = rotate);
        self
    }

    /// Sets the centred title and returns it.
    pub fn set_title(&self, text: impl Into<String>) -> Text {
        self.plane().set_title(text)
    }

    /// The centred title (empty when unset).
    pub fn title(&self) -> String {
        self.plane().title(crate::axes::TitleLocation::Center)
    }

    /// Sets the view angles in degrees (matplotlib's `view_init`).
    pub fn view_init(&self, elev: f64, azim: f64, roll: f64) -> &Self {
        self.with_mut(|axes| axes.camera.angles = ViewAngles { elev, azim, roll });
        self
    }

    /// The view angles.
    pub fn view(&self) -> ViewAngles {
        self.with(|axes| axes.camera.angles)
    }

    /// Chooses the axis displayed vertically (matplotlib's
    /// `vertical_axis`).
    pub fn set_vertical_axis(&self, axis: Axis3) -> &Self {
        self.with_mut(|axes| axes.camera.vertical = axis);
        self
    }

    /// Switches between perspective and orthographic projection
    /// (matplotlib's `set_proj_type`).
    ///
    /// # Errors
    ///
    /// When a perspective focal length is not positive.
    pub fn set_proj_type(&self, proj_type: ProjType) -> Result<&Self, Axes3DError> {
        self.with_mut(|axes| axes.camera.set_proj_type(proj_type))?;
        Ok(self)
    }

    /// Sets the x:y:z ratio of the box edges (`None` = 4:4:3) and the
    /// zoom (matplotlib's `set_box_aspect`).
    ///
    /// # Errors
    ///
    /// When a ratio or the zoom is not positive.
    pub fn set_box_aspect(&self, aspect: Option<Vec3>, zoom: f64) -> Result<&Self, Axes3DError> {
        self.with_mut(|axes| axes.camera.set_box_aspect(aspect, zoom))?;
        Ok(self)
    }

    /// Shows or hides the gridlines on the panes (matplotlib's `grid`).
    pub fn grid(&self, visible: bool) -> &Self {
        self.with_mut(|axes| axes.grid_on = visible);
        self
    }

    /// Hides the panes, gridlines and axes, leaving the background and
    /// the plotted artists (matplotlib's `set_axis_off`).
    pub fn set_axis_off(&self) -> &Self {
        self.with_mut(|axes| axes.axis_on = false);
        self
    }

    /// Shows the panes, gridlines and axes again.
    pub fn set_axis_on(&self) -> &Self {
        self.with_mut(|axes| axes.axis_on = true);
        self
    }

    /// Whether the panes, gridlines and axes are drawn.
    pub fn axis_on(&self) -> bool {
        self.with(|axes| axes.axis_on)
    }

    /// Sets the colour of the pane behind the data normal to `axis`
    /// (matplotlib's `axis.set_pane_color`); its alpha also fades the
    /// pane edges.
    pub fn set_pane_color(&self, axis: Axis3, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.with_mut(|axes| axes.axis_mut(axis).pane_color = color);
        self
    }

    /// Orders collections and patches by depth (`true`) or by their own
    /// z-order (matplotlib's `computed_zorder`).
    pub fn set_computed_zorder(&self, computed: bool) -> &Self {
        self.with_mut(|axes| axes.computed_zorder = computed);
        self
    }

    /// Sets the autoscaling margin of one axis as a fraction of the data
    /// span and autoscales (matplotlib's `set_xmargin`).
    ///
    /// # Errors
    ///
    /// When the margin is not above -0.5 or a locator fails.
    pub fn set_margin(&self, axis: Axis3, margin: f64) -> Result<&Self, Axes3DError> {
        if margin.is_nan() || margin <= -0.5 {
            return Err(Axes3DError::Margin(margin));
        }
        self.with_mut(|axes| {
            axes.limits.margins[axis.index()] = margin;
            axes.autoscale_view()
        })?;
        Ok(self)
    }

    /// The autoscaling margin of one axis.
    pub fn margin(&self, axis: Axis3) -> f64 {
        self.with(|axes| axes.limits.margins[axis.index()])
    }
}
