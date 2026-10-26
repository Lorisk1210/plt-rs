//! The shared, mutable state behind a [`Figure`](super::Figure) handle.

use std::cell::Cell;

use plt_core::{Affine2D, Bbox};
use plt_render::{ClipPath, Renderer};

use super::CanvasState;
use super::frame::FigureFrame;
use crate::artist::{Artist, ArtistId, ArtistStore, DetachedArtists, DrawContext, DrawError};
use crate::axes::{AxesState, LimitCallbacks, ShareGroups};
use crate::colors::Color;
use crate::events::{CallbackRegistry, CursorShape, LockOwner, TimerRegistry};
use crate::gridspec::{GridSpecError, SubplotOverrides, SubplotParams};
use crate::gui::tools::ToolRegistry;
use crate::layout::LayoutState;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, TransformContext, unit_to_bbox};

/// Everything a figure owns: its artists and its canvas geometry.
#[derive(Debug)]
pub(crate) struct FigureState {
    /// Every artist in the figure, including frames and axes.
    pub(crate) store: ArtistStore,
    /// The artists removed from the figure, kept for their handles (see
    /// [`FigureState::remove_artist`]).
    pub(crate) detached: DetachedArtists,
    /// The root frame (the figure itself).
    pub(crate) root: ArtistId,
    /// Canvas resolution in dots per inch (see [`FigureState::set_dpi`]).
    dpi: Cell<f64>,
    /// The resolution the figure was made at (matplotlib's
    /// `_original_dpi`): saving with `savefig.dpi: figure` uses it, and a
    /// canvas on a high-density display scales it, whatever the
    /// resolution became since.
    pub(crate) original_dpi: f64,
    /// Canvas size in inches.
    pub(crate) size_inches: (f64, f64),
    /// Set whenever something changed since the last draw.
    pub(crate) stale: bool,
    /// Problems found outside drawing (e.g. while autoscaling), reported
    /// with the next draw.
    pub(crate) pending_errors: Vec<DrawError>,
    /// Axes sharing their x or y axis.
    pub(crate) shared: ShareGroups,
    /// The layout engine and what it needs to know.
    pub(crate) layout: LayoutState,
    /// The axes `gca` returns: the last added or made current
    /// (matplotlib's `_axstack` order).
    pub(crate) current_axes: Option<ArtistId>,
    /// The callbacks of axes limit changes (`Axes::on_limits_changed`).
    pub(crate) limit_callbacks: LimitCallbacks,
    /// The callbacks connected with `Figure::connect`.
    pub(crate) callbacks: CallbackRegistry,
    /// The timers added with `Figure::add_timer`.
    pub(crate) timers: TimerRegistry,
    /// The axes receiving all mouse events (`Figure::grab_mouse`).
    pub(crate) mouse_grabber: Option<ArtistId>,
    /// Whether a widget takes key presses (`Figure::set_capture_keys`).
    pub(crate) capture_keys: bool,
    /// Who holds the widget lock (`Figure::lock_widgets`).
    pub(crate) widget_lock: Option<LockOwner>,
    /// The cursor tools ask the window for (`Figure::set_cursor`).
    pub(crate) cursor: CursorShape,
    /// The tools the program added to the figure's windows.
    pub(crate) tools: ToolRegistry,
    /// The part of the figure being printed, in inches, while saving with
    /// `bbox_inches` (matplotlib's `adjust_bbox`).
    pub(crate) print_window: Option<Bbox>,
    /// Whether the figure is being saved, when axes draw their animated
    /// artists too (matplotlib's `canvas.is_saving()`).
    pub(crate) saving: bool,
    /// Whether adjacent images are kept apart (`true`) or composited
    /// (`false`); `None` lets the renderer decide (matplotlib's
    /// `suppressComposite`).
    pub(crate) suppress_composite: Option<bool>,
    /// The resolution the figure was last drawn, or laid out as if drawn,
    /// at: arrows are measured at it, or at one pixel per point before the
    /// first draw (see
    /// [`LayoutContext::measured_dpi_cor`](crate::artist::LayoutContext::measured_dpi_cor)).
    pub(crate) last_draw_dpi: Option<f64>,
    /// The raster canvas kept between draws, and what the windows showing
    /// the figure have to present of it.
    pub(crate) canvas: CanvasState,
}

impl FigureState {
    /// The subplot parameters of figure frame `frame`.
    pub(crate) fn subplot_params_of(&self, frame: ArtistId) -> Option<SubplotParams> {
        self.store
            .get_as::<FigureFrame>(frame)
            .map(|frame| frame.subplot_params)
    }

    /// Gives the figure the subplot parameters `params` and moves every
    /// grid-placed axes accordingly (`fig.subplots_adjust(...)`). Like
    /// matplotlib's subfigures, which share the `SubplotParams` of their
    /// figure, every frame takes them.
    pub(crate) fn set_subplot_params(
        &mut self,
        params: SubplotParams,
    ) -> Result<(), GridSpecError> {
        let ids: Vec<_> = self.store.ids().collect();
        for id in ids {
            if let Some(frame) = self.store.get_as_mut::<FigureFrame>(id) {
                frame.subplot_params = params;
            } else if let Some(axes) = self.store.get_as_mut::<AxesState>(id) {
                if let Some(spec) = &axes.subplot_spec {
                    axes.position = spec.position(&params)?;
                }
            } else {
                crate::mplot3d::place_axes3d(&mut self.store, id, &params)?;
            }
        }
        self.stale = true;
        Ok(())
    }

    /// The axes of the whole figure, subfigures included, in creation
    /// order (the scope of the root figure's `Figure::axes`).
    pub(crate) fn all_axes(&self) -> Vec<ArtistId> {
        self.store
            .ids()
            .filter(|&id| {
                self.store.get_as::<AxesState>(id).is_some_and(|axes| {
                    axes.base()
                        .parent
                        .is_some_and(|parent| self.store.get_as::<FigureFrame>(parent).is_some())
                })
            })
            .collect()
    }

    /// A figure of `size_inches` at `dpi` with an empty root frame.
    pub(crate) fn new(
        size_inches: (f64, f64),
        dpi: f64,
        facecolor: Color,
        edgecolor: Color,
    ) -> Self {
        let mut store = ArtistStore::default();
        let rc = crate::rc::params();
        let params = SubplotParams::from_rc(&rc);
        let root = store.insert(Box::new(FigureFrame::new(
            Bbox::UNIT,
            facecolor,
            edgecolor,
            params,
        )));
        Self {
            store,
            detached: DetachedArtists::default(),
            root,
            dpi: Cell::new(dpi),
            original_dpi: dpi,
            size_inches,
            stale: true,
            pending_errors: Vec::new(),
            shared: ShareGroups::default(),
            layout: LayoutState::from_rc(&rc),
            current_axes: None,
            limit_callbacks: LimitCallbacks::default(),
            callbacks: CallbackRegistry::default(),
            timers: TimerRegistry::default(),
            mouse_grabber: None,
            capture_keys: false,
            widget_lock: None,
            cursor: CursorShape::default(),
            tools: ToolRegistry::default(),
            print_window: None,
            saving: false,
            suppress_composite: None,
            last_draw_dpi: None,
            canvas: CanvasState::default(),
        }
    }

    /// Canvas resolution in dots per inch.
    pub(crate) fn dpi(&self) -> f64 {
        self.dpi.get()
    }

    /// Changes the canvas resolution. Drawing changes it through a shared
    /// borrow while rasterized artists are drawn into the raster canvas of
    /// a vector document, so that they compute their geometry in its
    /// pixels (matplotlib's `MixedModeRenderer` sets `figure.dpi`).
    pub(crate) fn set_dpi(&self, dpi: f64) {
        self.dpi.set(dpi);
    }

    /// Canvas size in pixels.
    pub(crate) fn size_pixels(&self) -> (f64, f64) {
        let dpi = self.dpi();
        (self.size_inches.0 * dpi, self.size_inches.1 * dpi)
    }

    /// The mutable pass before drawing: a due layout-engine pass, then
    /// autoscaling, aspect and ticks of every axes.
    pub(crate) fn prepare(&mut self) {
        self.run_pending_layout();
        let axes: Vec<ArtistId> = self
            .store
            .ids()
            .filter(|&id| self.store.get_as::<AxesState>(id).is_some())
            .collect();
        for &id in &axes {
            self.sync_secondary(id);
            self.lay_out_axes(id);
            crate::polar::update_spine_visibility(self, id);
            let Some(extent) = self.axes_bbox(id) else {
                continue;
            };
            let dpi = self.dpi();
            let shared = self.shared_tick_spaces(id);
            if let Some(state) = self.store.get_as_mut::<AxesState>(id) {
                state.update_ticks(&extent, dpi, shared);
            }
        }
        crate::mplot3d::prepare_axes3d(self);
        // Titles clear the decorations of twins and insets, whose ticks
        // are only all known now.
        for id in axes {
            self.update_title_position(id);
        }
        crate::legend::lay_out_legends(self);
        crate::offsetbox::lay_out_boxes(self);
    }

    /// Renders the whole figure, returning the problems encountered.
    pub(crate) fn draw(&self, renderer: &mut dyn Renderer) -> Vec<DrawError> {
        let mut ctx = DrawContext::new(self, renderer);
        ctx.draw_child(self.root);
        ctx.finish_rasterizing();
        let mut errors = self.pending_errors.clone();
        // matplotlib refuses to make a figure whose subplot parameters
        // leave no room; plt-rs reports them whenever the figure draws.
        let params = self.subplot_params_of(self.root);
        if let Some(Err(err)) = params.map(|p| p.updated(&SubplotOverrides::default())) {
            errors.push(DrawError {
                artist: self.root,
                message: err.to_string(),
            });
        }
        errors.extend(ctx.into_errors());
        errors
    }

    /// Where the background of `frame` is drawn: its extent, or the whole
    /// canvas for the root figure while a window of it is printed
    /// (matplotlib moves the figure patch onto the saved area).
    pub(crate) fn frame_patch_bbox(&self, frame: ArtistId) -> Option<Bbox> {
        match self.print_window {
            Some(window) if frame == self.root => {
                let dpi = self.dpi();
                Some(Bbox::from_bounds(
                    0.0,
                    0.0,
                    window.width() * dpi,
                    window.height() * dpi,
                ))
            }
            _ => self.frame_bbox(frame),
        }
    }

    /// The display outline of `axes` when its patch is not a rectangle
    /// (an outline of its own, the wedge of polar axes, the ellipse of
    /// geographic axes, the outline of a custom projection); `None`
    /// otherwise.
    pub(crate) fn axes_outline(&self, axes: ArtistId) -> Option<ClipPath> {
        let state = self.store.get_as::<AxesState>(axes)?;
        if let Some((path, transform)) = &state.patch_outline {
            let (path, transform) = transform.resolve(self).ok()?.split_path(path);
            return Some(ClipPath {
                path: path.into_owned(),
                transform,
            });
        }
        if state.projection.is_rectangular() {
            return None;
        }
        Some(ClipPath {
            path: state.patch_path(),
            transform: unit_to_bbox(&self.axes_bbox(axes)?),
        })
    }
}

impl TransformContext for FigureState {
    fn dpi(&self) -> f64 {
        self.dpi.get()
    }

    fn frame_bbox(&self, frame: ArtistId) -> Option<Bbox> {
        let node = self.store.get_as::<FigureFrame>(frame)?;
        if frame == self.root {
            let (w, h) = self.size_pixels();
            let dpi = self.dpi();
            let (x, y) = self.print_window.map_or((0.0, 0.0), |window| {
                (-window.x0() * dpi, -window.y0() * dpi)
            });
            return Some(Bbox::from_bounds(x, y, w, h));
        }
        let parent = self.frame_bbox(node.base().parent?)?;
        Some(fraction_of(&parent, &node.relative_extent))
    }

    /// Axes are placed in a figure frame, or (secondary axes and the
    /// projection planes of 3-D axes) in their host axes.
    fn axes_bbox(&self, axes: ArtistId) -> Option<Bbox> {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return crate::mplot3d::axes3d_bbox(self, axes);
        };
        let parent = state.base().parent?;
        let container = self.frame_bbox(parent).or_else(|| self.axes_bbox(parent))?;
        Some(fraction_of(
            &container,
            &state.aspect.active_position(&state.position),
        ))
    }

    /// Auxiliary axes map their data through their transform into the
    /// data of their host.
    fn data_transform(&self, axes: ArtistId) -> Option<ResolvedTransform> {
        let state = self.store.get_as::<AxesState>(axes)?;
        if let Some(aux) = &state.aux_transform {
            return Some(aux.clone().then(self.data_transform(state.base().parent?)?));
        }
        Some(state.data_transform(&self.axes_bbox(axes)?))
    }

    /// Polar axes map the fraction across the data axis onto its view
    /// range before their data transform: a radial fraction is a radius, an
    /// angular one an angle (matplotlib's `PolarAxes._xaxis_transform` and
    /// `_yaxis_transform`). Custom projections may give their own.
    fn axis_transform(&self, axes: ArtistId, data_axis: AxisName) -> Option<ResolvedTransform> {
        let state = self.store.get_as::<AxesState>(axes)?;
        if state.projection.custom().is_some() {
            return state.custom_axis_transform(data_axis, &self.axes_bbox(axes)?);
        }
        state.polar()?;
        let view = state.view_lim;
        let fraction_to_view = match data_axis {
            AxisName::X => Affine2D::from_values(1.0, 0.0, 0.0, view.height(), 0.0, view.y0()),
            AxisName::Y => Affine2D::from_values(view.width(), 0.0, 0.0, 1.0, view.x0(), 0.0),
        };
        Some(ResolvedTransform::affine(fraction_to_view).then(self.data_transform(axes)?))
    }
}

/// The part of `outer` described by `fraction` (a box in `outer`'s unit square).
fn fraction_of(outer: &Bbox, fraction: &Bbox) -> Bbox {
    fraction.transformed(&crate::transforms::unit_to_bbox(outer))
}
