//! The [`Axes3D`] handle, the figure methods creating 3-D axes, and the
//! view and decoration settings of a 3-D axes.

use std::rc::Rc;

use plt_core::{Anchor, Bbox};

use super::art3d::{Art3D, Artist3D};
use super::camera::{Camera, ProjType, ViewAngles};
use super::coords::Axis3;
use super::error::Axes3DError;
use super::math::{Matrix4, Vec3};
use super::share::replay;
use super::state::Axes3DState;
use super::units::Sources3D;
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup, RawHandle};
use crate::axes::{Axes, AxisName};
use crate::colors::Color;
use crate::figure::{Figure, axes_bounds, insert_axes};
use crate::gridspec::SubplotSpec;
use crate::text::Text;
use crate::transforms::{Transform, TransformContext};

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

    /// Changes the axes, then lets the axes sharing its limits follow the
    /// limits it set (matplotlib's `emit=True`).
    pub(crate) fn with_mut<R>(&self, f: impl FnOnce(&mut Axes3DState) -> R) -> R {
        let (result, changes) = self.raw.with_mut::<Axes3DState, _>(|axes| {
            let result = f(axes);
            (result, axes.take_limit_changes())
        });
        if !changes.is_empty() {
            replay(&mut self.raw.figure_state().borrow_mut(), &changes);
        }
        result
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

    /// Adds a 2-D artist of the projection plane with its 3-D geometry,
    /// hiding the points outside the view limits with `axlim_clip`.
    pub(crate) fn add_artist(
        &self,
        artist: impl Artist,
        art: Art3D,
        axlim_clip: bool,
    ) -> RawHandle {
        let raw = self.adopt_projected(artist);
        self.register(raw.id(), art, axlim_clip);
        raw
    }

    /// Gives an artist already added to the plane its 3-D geometry,
    /// hiding the points outside the view limits with `axlim_clip`.
    pub(crate) fn register(&self, id: ArtistId, art: Art3D, axlim_clip: bool) {
        self.with_mut(|axes| {
            axes.artists.push(Artist3D {
                id,
                art,
                axlim_clip,
                sources: Sources3D::default(),
            });
        });
    }

    /// Keeps the user values the geometry of artist `id` came from, to
    /// convert them again when an axis changes unit.
    pub(crate) fn keep_unit_sources(&self, id: ArtistId, sources: Sources3D) {
        self.with_mut(|axes| {
            if let Some(artist) = axes.artists.iter_mut().find(|a| a.id == id) {
                artist.sources = sources;
            }
        });
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
    /// square, in the figure's current layout (laid out first, so that a
    /// layout engine has placed the axes: matplotlib's `get_position`
    /// once the figure was drawn).
    pub fn position(&self) -> Bbox {
        self.raw.figure_state().borrow_mut().prepare();
        let state = self.raw.figure_state().borrow();
        self.with(|axes| {
            let frame = axes.base().parent.and_then(|f| state.frame_bbox(f));
            frame.map_or(axes.position, |frame| axes.active_position(&frame))
        })
    }

    /// The grid cell the axes occupies, if it was placed on a grid.
    pub fn subplot_spec(&self) -> Option<SubplotSpec> {
        self.with(|axes| axes.subplot_spec.clone())
    }

    /// The placement as a fraction of the figure before the box is made
    /// square (matplotlib's `get_position(original=True)`).
    pub fn original_position(&self) -> Bbox {
        self.with(|axes| axes.position)
    }

    /// Where the square box goes in its placement (matplotlib's
    /// `set_anchor`; [`Anchor::C`] by default).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::transforms::Anchor;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_anchor(Anchor::W);
    /// assert_eq!(ax.position().x0(), ax.original_position().x0());
    /// ```
    pub fn set_anchor(&self, anchor: Anchor) -> &Self {
        self.with_mut(|axes| axes.anchor = anchor);
        self
    }

    /// Moves the axes to `position` (figure fractions), in grid cell
    /// `spec` when given (what making room for a colorbar does).
    pub(crate) fn place(&self, position: Bbox, spec: Option<SubplotSpec>) {
        self.with_mut(|axes| {
            axes.position = position;
            if spec.is_some() {
                axes.subplot_spec = spec;
            }
        });
    }

    /// The 3-D axes whose projection plane `ax` is, if any.
    pub(crate) fn of_plane(ax: &Axes) -> Option<Self> {
        let figure = ax.raw().figure_state();
        let parent = {
            let state = figure.borrow();
            let parent = state.artist(ax.id())?.base().parent?;
            state.artist_as::<Axes3DState>(parent).map(|_| parent)
        }?;
        Some(Self::from_raw(RawHandle::new(Rc::clone(figure), parent)))
    }

    /// Sets the label of one axis (matplotlib's `set_xlabel`).
    pub fn set_label(&self, axis: Axis3, text: impl Into<String>) -> &Self {
        let text = text.into();
        self.with_mut(|axes| {
            let label = &mut axes.axis_mut(axis).label;
            label.text = text;
            label.is_default = false;
        });
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

    /// Whether the label of one axis follows the axis direction: as set
    /// with [`set_rotate_label`](Self::set_rotate_label), else when the
    /// label is longer than four characters (matplotlib's
    /// `get_rotate_label` of the axis label).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_label(Axis3::Z, "z");
    /// assert!(!ax.rotate_label(Axis3::Z));
    /// ax.set_rotate_label(Axis3::Z, Some(true));
    /// assert!(ax.rotate_label(Axis3::Z));
    /// ```
    pub fn rotate_label(&self, axis: Axis3) -> bool {
        self.with(|axes| axes.axis(axis).label.rotated())
    }

    /// Sets the centred title and returns it: like matplotlib's
    /// `Axes3D.set_title`, centred whatever `axes.titlelocation` says.
    pub fn set_title(&self, text: impl Into<String>) -> Text {
        self.set_title_at(crate::axes::TitleLocation::Center, text)
    }

    /// Sets the title at `location` (left, centre or right) and returns
    /// it (matplotlib's `Axes3D.set_title(label, loc=...)`).
    ///
    /// ```
    /// use plt::axes::TitleLocation;
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let left = ax.set_title_at(TitleLocation::Left, "left");
    /// assert_eq!(left.text(), "left");
    /// assert_eq!(ax.title(), "", "the centred title stays empty");
    /// ```
    pub fn set_title_at(
        &self,
        location: crate::axes::TitleLocation,
        text: impl Into<String>,
    ) -> Text {
        self.plane().set_title_at(location, text)
    }

    /// The title at `location`, empty when unset (matplotlib's
    /// `get_title(loc=...)`).
    pub fn title_at(&self, location: crate::axes::TitleLocation) -> String {
        self.plane().title(location)
    }

    /// The centred title (empty when unset).
    pub fn title(&self) -> String {
        self.title_at(crate::axes::TitleLocation::Center)
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

    /// The projection matrix of the current view and limits, from
    /// transformed data coordinates to projected ones (matplotlib's
    /// `get_proj`).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let projected = ax.proj().project([0.5, 0.5, 0.5]);
    /// assert!(projected.iter().all(|c| c.is_finite()));
    /// ```
    pub fn proj(&self) -> Matrix4 {
        self.with(|axes| axes.projection().matrix)
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

    /// The x, y and z lengths of the box edges in the units the camera
    /// sees them (matplotlib's `_box_aspect` rolled to the vertical axis).
    pub fn box_aspect(&self) -> Vec3 {
        self.with(|axes| axes.camera.box_aspect())
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
}

#[cfg(test)]
mod tests {
    use crate::axes::TitleLocation;
    use crate::figure::Figure;

    #[test]
    fn titles_are_centred_whatever_the_title_location() {
        crate::rc::context(|| {
            crate::rc::set("axes.titlelocation", "left").unwrap();
            let ax = Figure::new().subplot_3d();
            ax.set_title("view");
            assert_eq!(ax.title(), "view");
            assert_eq!(ax.plane().title(TitleLocation::Left), "");
        });
    }
}
