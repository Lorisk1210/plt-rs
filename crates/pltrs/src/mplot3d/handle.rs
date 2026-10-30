//! The [`Axes3D`] handle, and the view and decoration settings of a 3-D
//! axes.

use std::rc::Rc;

use pltrs_core::{Anchor, Bbox};

use super::art3d::{Art3D, Artist3D};
use super::camera::{ProjType, ViewAngles};
use super::coords::Axis3;
use super::error::Axes3DError;
use super::math::{Matrix4, Vec3};
use super::share::replay;
use super::state::Axes3DState;
use super::units::Sources3D;
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup, RawHandle, TypedHandle};
use crate::axes::Axes;
use crate::colors::Color;
use crate::gridspec::SubplotSpec;
use crate::text::Text;
use crate::transforms::{Transform, TransformContext};

/// A handle to a 3-D axes of a figure (matplotlib's `Axes3D`).
///
/// ```
/// use pltrs::figure::Figure;
/// let fig = Figure::new();
/// let ax = fig.subplot_3d();
/// ax.plot(&[0.0, 1.0], &[0.0, 2.0], &[0.0, 3.0])?;
/// ax.view_init(20.0, 30.0, 0.0);
/// assert_eq!(ax.zlim(), (-0.0625, 3.0625));
/// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
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

impl TypedHandle for Axes3D {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<Axes3DState>().then(|| Self::from_raw(raw.clone()))
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

    /// The 2-D axes whose data coordinates are the projected coordinates
    /// (what matplotlib's `Axes3D` is as an `Axes`): titles, 2-D texts and
    /// artists added in projected coordinates belong to it.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// assert_ne!(ax.projection_plane().id(), ax.id());
    /// assert_eq!(fig.gca(), ax.projection_plane(), "a new 3-D axes is current");
    /// ```
    pub fn projection_plane(&self) -> Axes {
        self.plane()
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
    /// use pltrs::figure::Figure;
    /// use pltrs::transforms::Anchor;
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
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
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
    /// use pltrs::axes::TitleLocation;
    /// use pltrs::figure::Figure;
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
    /// use pltrs::figure::Figure;
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
