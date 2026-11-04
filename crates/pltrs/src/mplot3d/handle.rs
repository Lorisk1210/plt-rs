//! The [`Axes3D`] handle, and the view and decoration settings of a 3-D
//! axes.

use std::rc::Rc;

use pltrs_core::path::DataLimits;
use pltrs_core::{Anchor, Bbox};

use super::art3d::{Art3D, Artist3D};
use super::camera::{ProjType, ViewAngles};
use super::coords::Axis3;
use super::error::Axes3DError;
use super::math::{Matrix4, Vec3};
use super::share::replay;
use super::state::{Axes3DState, BoxFit};
use super::units::Sources3D;
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup, RawHandle, TypedHandle};
use crate::axes::{Axes, AxesState};
use crate::collections::{CollectionArtist, CollectionKind};
use crate::colors::Color;
use crate::figure::FigureState;
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
        let plane_minpos = super::state::plane_minpos(&self.raw.figure_state().borrow(), self.id());
        let (result, changes) = self.raw.with_mut::<Axes3DState, _>(|axes| {
            axes.limits.plane_minpos = plane_minpos;
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

    /// Runs `plot` on the projection plane, its 2-D data limits left as
    /// they were: matplotlib's 2-D plotting methods extend them through
    /// `update_datalim`, which `Axes3D` makes a no-op (the collections and
    /// patches of its plotting methods leave them alone, its lines extend
    /// them directly).
    pub(crate) fn on_plane_keeping_limits<R>(&self, plot: impl FnOnce(&Axes) -> R) -> R {
        let plane = self.plane();
        let kept = plane.data_lim();
        let result = plot(&plane);
        self.restore_plane_data_lim(kept);
        result
    }

    /// Gives the projection plane back the 2-D data limits `limits`, taken
    /// (`projection_plane().data_lim()`) before a 2-D plotting method
    /// extended them through matplotlib's `update_datalim`, which
    /// `Axes3D` makes a no-op.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let kept = ax.projection_plane().data_lim();
    /// ax.projection_plane().bar(vec![1.0, 2.0], vec![3.0, 4.0], Default::default())?;
    /// ax.restore_plane_data_lim(kept);
    /// assert_eq!(ax.projection_plane().data_lim(), kept);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn restore_plane_data_lim(&self, limits: DataLimits) -> &Self {
        self.plane()
            .raw()
            .with_mut::<AxesState, _>(|axes| axes.data_lim = limits);
        self
    }

    /// Adds a 2-D artist to the projection plane, clipped to the axes and
    /// in projected coordinates, without 3-D geometry yet.
    pub(crate) fn adopt_projected(&self, mut artist: impl Artist) -> RawHandle {
        let plane = self.plane();
        let label = plane.child_label(&artist);
        let base = artist.base_mut();
        base.clip_box = Some(plane.id());
        base.transform = Some(plane.trans_data());
        if let Some(label) = label {
            base.label = label;
        }
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
    /// hiding the points outside the view limits with `axlim_clip`; a
    /// collection takes the name of matplotlib's 3-D class for it.
    pub(crate) fn register(&self, id: ArtistId, art: Art3D, axlim_clip: bool) {
        let raw = RawHandle::new(Rc::clone(self.raw.figure_state()), id);
        if raw.is::<CollectionArtist>() {
            raw.with_mut::<CollectionArtist, _>(|c| {
                c.class_name = art.collection_class(c.kind);
                // matplotlib's patch, path and polygon collections map their
                // values when they are lifted (`set_3d_properties`).
                if c.kind != CollectionKind::Line {
                    c.update_scalarmappable();
                }
            });
        }
        self.with_mut(|axes| {
            axes.artists.push(Artist3D {
                id,
                art,
                axlim_clip,
                sources: Sources3D::default(),
                drawn: false,
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
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// assert!(!ax.has_data());
    /// ax.plot(&[0.0], &[0.0], &[0.0])?;
    /// assert!(ax.has_data());
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    pub fn has_data(&self) -> bool {
        self.with(Axes3DState::has_data)
    }

    /// Includes the points `(x, y, z)` in the data limits, which they
    /// replace unless `had_data`, and autoscales the axes that autoscale
    /// (matplotlib's `auto_scale_xyz`); values outside an axis' scale are
    /// ignored, and without `z` the z data limits stay as they are.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.auto_scale_xyz(&[0.0, 10.0], &[0.0, 1.0], None, false)?;
    /// assert!(ax.xlim().1 >= 10.0);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a locator fails while autoscaling.
    pub fn auto_scale_xyz(
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

    /// Placement as a fraction of the figure, the box made square, in the
    /// figure's current layout (laid out first, so that a layout engine
    /// has placed the axes: matplotlib's `get_position`, which applies the
    /// aspect).
    pub fn position(&self) -> Bbox {
        self.raw.figure_state().work_on(FigureState::prepare);
        self.squared_position()
    }

    /// Makes the box square in its placement where the axes is now, the
    /// figure not laid out, and gives the placement as a fraction of the
    /// figure (matplotlib's `get_position`, which applies the aspect).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::with_size(6.4, 4.8, 100.0);
    /// let ax = fig.subplot_3d();
    /// let square = ax.squared_position();
    /// let pixels = (square.width() * 6.4, square.height() * 4.8);
    /// assert!((pixels.0 - pixels.1).abs() < 1e-9, "{pixels:?}");
    /// ```
    pub fn squared_position(&self) -> Bbox {
        self.with_mut(|axes| axes.fit = BoxFit::Squared);
        self.active_position()
    }

    /// The active position in the fractions of the axes' frame.
    fn active_position(&self) -> Bbox {
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

    /// The tight bounding box of the axes in display coordinates, the
    /// figure brought up to date first: its box, and the lines, ticks,
    /// tick labels, offset texts and labels of its axes when shown
    /// (without the labels when `for_layout_only`), and its titles;
    /// `None` for an invisible axes (matplotlib's `Axes3D.get_tightbbox`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let tight = ax.tight_bbox(false).expect("a visible axes");
    /// assert!(tight.width() > 0.0 && tight.height() > 0.0);
    /// ```
    pub fn tight_bbox(&self, for_layout_only: bool) -> Option<Bbox> {
        let mut state = self.raw.figure_state().prepared();
        super::state::axes3d_fresh_tight_bbox(&mut state, self.id(), for_layout_only)
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

    /// Moves the axes to `position` in the fractions of its frame, where
    /// its box is made square at the next draw (matplotlib's
    /// `set_position` of the original position, which the active one
    /// follows).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::transforms::Bbox;
    /// let ax = Figure::with_size(6.4, 4.8, 100.0).subplot_3d();
    /// ax.set_position(Bbox::from_bounds(0.1, 0.1, 0.5, 0.5));
    /// assert_eq!(ax.original_position(), Bbox::from_bounds(0.1, 0.1, 0.5, 0.5));
    /// assert!((ax.squared_position().width() - 0.375).abs() < 1e-12);
    /// ```
    pub fn set_position(&self, position: Bbox) -> &Self {
        self.place(position, None);
        self
    }

    /// Moves the axes to `position` (figure fractions), in grid cell
    /// `spec` when given (what making room for a colorbar does).
    pub(crate) fn place(&self, position: Bbox, spec: Option<SubplotSpec>) {
        self.with_mut(|axes| {
            axes.position = position;
            axes.fit = BoxFit::Placed;
            if spec.is_some() {
                axes.subplot_spec = spec;
            }
        });
    }

    /// The 3-D axes whose projection plane `ax` is, if any.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axes3D;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// assert_eq!(Axes3D::of_plane(&ax.projection_plane()), Some(ax));
    /// assert_eq!(Axes3D::of_plane(&fig.subplot()), None);
    /// ```
    pub fn of_plane(ax: &Axes) -> Option<Self> {
        let figure = ax.raw().figure_state();
        let parent = {
            let state = figure.borrow();
            let parent = state.artist(ax.id())?.base().parent?;
            state.artist_as::<Axes3DState>(parent).map(|_| parent)
        }?;
        Some(Self::from_raw(RawHandle::new(Rc::clone(figure), parent)))
    }

    /// The 3-D axes `artist` is drawn in: the axes itself, its projection
    /// plane or an artist under them; `None` for the artists of 2-D axes
    /// and of the figure, and for a removed artist (matplotlib's
    /// `artist.axes` when the axes is an `Axes3D`).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axes3D;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// let line = ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// assert_eq!(Axes3D::of_artist(line.raw()), Some(ax.clone()));
    /// assert_eq!(Axes3D::of_artist(ax.projection_plane().raw()), Some(ax));
    /// line.remove();
    /// assert_eq!(Axes3D::of_artist(line.raw()), None);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    pub fn of_artist(artist: &RawHandle) -> Option<Self> {
        let figure = artist.figure_state();
        let id = {
            let state = figure.borrow();
            if state.is_removed(artist.id()) {
                return None;
            }
            let mut id = artist.id();
            while state.artist_as::<Axes3DState>(id).is_none() {
                id = state.artist(id)?.base().parent?;
            }
            id
        };
        Some(Self::from_raw(RawHandle::new(Rc::clone(figure), id)))
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

    /// Whether a label `text` of one axis would follow the axis direction:
    /// as set with [`set_rotate_label`](Self::set_rotate_label), else when
    /// `text` is longer than four characters (matplotlib's
    /// `get_rotate_label(text)`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// assert!(ax.rotates_label(Axis3::X, "longer"));
    /// ax.set_rotate_label(Axis3::X, Some(false));
    /// assert!(!ax.rotates_label(Axis3::X, "longer"));
    /// ```
    pub fn rotates_label(&self, axis: Axis3, text: &str) -> bool {
        self.with(|axes| axes.axis(axis).label.rotates(text))
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

    /// The focal length of the virtual camera: infinite for an
    /// orthographic projection (matplotlib's `_focal_length`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::ProjType;
    /// let ax = Figure::new().subplot_3d();
    /// assert_eq!(ax.focal_length(), 1.0);
    /// ax.set_proj_type(ProjType::Ortho)?;
    /// assert_eq!(ax.focal_length(), f64::INFINITY);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    pub fn focal_length(&self) -> f64 {
        self.with(|axes| axes.camera.focal_length())
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

    /// The box aspect as matplotlib keeps it (`_box_aspect`, which
    /// `get_box_aspect` returns): [`Axes3D::box_aspect`] rolled back from
    /// the vertical axis, so that its last entry is that of the vertical
    /// axis.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_vertical_axis(Axis3::X);
    /// ax.set_box_aspect(None, 1.0)?;
    /// let [x, y, z] = ax.box_aspect();
    /// assert_eq!(ax.stored_box_aspect(), [y, z, x]);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    pub fn stored_box_aspect(&self) -> Vec3 {
        self.with(|axes| axes.camera.stored_box_aspect())
    }

    /// Shows or hides the gridlines on the panes (matplotlib's `grid`).
    pub fn grid(&self, visible: bool) -> &Self {
        self.with_mut(|axes| axes.grid_on = visible);
        self
    }

    /// Whether the gridlines on the panes are drawn (matplotlib's
    /// `_draw_grid`).
    pub fn grid_on(&self) -> bool {
        self.with(|axes| axes.grid_on)
    }

    /// The colour of the pane normal to `axis` (matplotlib's
    /// `axis._axinfo['color']`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_pane_color(Axis3::X, "red");
    /// assert_eq!(ax.pane_color(Axis3::X).resolve()?, pltrs::colors::Rgba::opaque(1.0, 0.0, 0.0));
    /// # Ok::<(), pltrs::colors::ColorError>(())
    /// ```
    pub fn pane_color(&self, axis: Axis3) -> Color {
        self.with(|axes| axes.axis(axis).pane_color.clone())
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
