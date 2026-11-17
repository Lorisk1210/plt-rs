//! The shared, mutable state behind a [`Figure`](super::Figure) handle.

mod archive;
mod links;

use std::cell::Cell;
use std::rc::Rc;

use pltrs_core::{Affine2D, Bbox};
use pltrs_render::ClipPath;

use super::frame::FigureFrame;
use super::{CanvasState, FreshArtists};
use crate::artist::{
    Artist, ArtistId, ArtistLookup, ArtistStore, DetachedArtists, DrawError, DrawErrorKind,
};
use crate::axes::{
    AxesState, LimitCallbacks, PatchOutline, ShareGroups, SharesAcross, TickUpdater,
};
use crate::colors::Color;
use crate::events::{CallbackRegistry, CursorShape, LockOwner, PickedAxes, TimerRegistry};
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
    /// The artists drawn since they last changed (see
    /// [`FigureState::is_stale`]).
    pub(crate) fresh: FreshArtists,
    /// Problems found outside drawing (e.g. while autoscaling), reported
    /// with the next draw.
    pub(crate) pending_errors: Vec<DrawError>,
    /// The axes whose aspect could not be applied since the last draw,
    /// each with the limits it failed on (see
    /// [`AspectInputs`](crate::axes::AspectInputs)): matplotlib's draw
    /// ends at the first application that fails, so they are not applied
    /// to the same limits again before the next draw reports it.
    pub(crate) aspects_failed: Vec<(ArtistId, crate::axes::AspectInputs)>,
    /// Axes sharing their x or y axis.
    pub(crate) shared: ShareGroups,
    /// Which of them share it with axes of other figures.
    pub(crate) across: SharesAcross,
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
    /// The magnification of the images the program's code makes in the
    /// draw before a print (matplotlib's vector renderers make them at the
    /// save resolution, their `image_magnification`), 1 outside prints.
    pub(crate) print_image_magnification: f64,
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
    /// What the program runs at every draw (`Figure::set_draw_hook`).
    pub(crate) draw_hook: Option<super::DrawHook>,
    /// What the program runs before every layout pass of a draw
    /// (`Figure::set_layout_hook`).
    pub(crate) layout_hook: Option<super::DrawHook>,
    /// What is handed the evaluations of matplotlib's transform graph the
    /// figure's draws stand for (`Figure::set_evaluation_hook`).
    pub(crate) evaluation_hook: Option<crate::transforms::EvaluationHook>,
    /// What ticks the axes whose tick helpers or scale call the program
    /// (`Figure::set_tick_updater`).
    pub(crate) tick_updater: Option<Rc<dyn TickUpdater>>,
    /// How many of matplotlib's draws the layout of the next draw stands
    /// for, at whose call sites the program's tick helpers are called: 2
    /// for the print of a whole figure with a layout engine (matplotlib
    /// draws it once to lay it out, then prints it), else 1. The draw
    /// takes it.
    pub(crate) draws_per_layout: usize,
    /// A figure, or the holder of an artist made outside any figure.
    pub(crate) role: StateRole,
}

/// What a [`FigureState`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StateRole {
    /// A figure.
    Figure,
    /// No figure but the holder of an artist made outside any figure (see
    /// [`RawHandle::is_loose`](crate::artist::RawHandle::is_loose)).
    LooseHolder,
}

/// What a figure is prepared for (see [`FigureState::prepare`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Preparation {
    /// Read as it is between draws: no layout engine, no aspect applied,
    /// and secondary axes keep the limits they had.
    AsIs,
    /// Read as it is between draws for a pick, which computes the ticks
    /// of the axes whose children it reaches alone (matplotlib's
    /// `Artist.pick`): what the arithmetic of the others' warns about is
    /// not said.
    Picking(PickedAxes),
    /// Measured (a tight box, a layout).
    Measuring,
    /// Drawn: the boxes of 3-D axes are made square too.
    Drawing(Painting),
}

/// Whether a draw goes on to draw the figure's artists once it is laid
/// out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Painting {
    /// The artists are drawn.
    Painted,
    /// The program's code stopped the draw before (see
    /// [`DrawHook::stopping`](super::DrawHook::stopping)): what is laid
    /// out as matplotlib's artists draw themselves is left as it was.
    Stopped,
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
            fresh: FreshArtists::default(),
            pending_errors: Vec::new(),
            aspects_failed: Vec::new(),
            shared: ShareGroups::default(),
            across: SharesAcross::default(),
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
            print_image_magnification: 1.0,
            saving: false,
            suppress_composite: None,
            last_draw_dpi: None,
            canvas: CanvasState::default(),
            draw_hook: None,
            layout_hook: None,
            evaluation_hook: None,
            tick_updater: None,
            draws_per_layout: 1,
            role: StateRole::Figure,
        }
    }

    /// The holder of an artist made outside any figure: a state with
    /// matplotlib's default figure geometry that is never drawn.
    pub(crate) fn holder() -> Self {
        Self {
            role: StateRole::LooseHolder,
            ..Self::new(
                (6.4, 4.8),
                100.0,
                Color::from("white"),
                Color::from("white"),
            )
        }
    }

    /// What a cell holds while its state is worked on outside it (see
    /// [`FigureCell::work_on`](super::FigureCell::work_on)): an empty
    /// state, made without allocating, which no handle reaches.
    pub(crate) fn placeholder() -> Self {
        Self {
            store: ArtistStore::default(),
            detached: DetachedArtists::default(),
            root: ArtistId::from_index(0),
            dpi: Cell::new(0.0),
            original_dpi: 0.0,
            size_inches: (0.0, 0.0),
            stale: false,
            fresh: FreshArtists::default(),
            pending_errors: Vec::new(),
            aspects_failed: Vec::new(),
            shared: ShareGroups::default(),
            across: SharesAcross::default(),
            layout: LayoutState::default(),
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
            print_image_magnification: 1.0,
            saving: false,
            suppress_composite: None,
            last_draw_dpi: None,
            canvas: CanvasState::default(),
            draw_hook: None,
            layout_hook: None,
            evaluation_hook: None,
            tick_updater: None,
            draws_per_layout: 1,
            role: StateRole::LooseHolder,
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

    /// The figure's size in inches as matplotlib's figure has it now: the
    /// size of the window a print running now is cut to (its print makes
    /// `fig.bbox_inches` the window's size), else the figure's own.
    pub(crate) fn printed_size_inches(&self) -> (f64, f64) {
        self.print_window
            .map_or(self.size_inches, |window| (window.width(), window.height()))
    }

    /// Canvas size in pixels.
    pub(crate) fn size_pixels(&self) -> (f64, f64) {
        let dpi = self.dpi();
        (self.size_inches.0 * dpi, self.size_inches.1 * dpi)
    }

    /// The box of the (sub)figure `artist` is in, or is.
    pub(crate) fn figure_bbox_of(&self, artist: ArtistId) -> Option<Bbox> {
        self.frame_bbox(artist)
            .or_else(|| self.figure_bbox_of(self.store.get(artist)?.base().parent?))
    }

    /// Brings the ticks of axis `name` of `axes` up to date as matplotlib
    /// computes them between draws (`Axis._update_ticks`, of that axis
    /// alone): its view autoscaled (reading the view limits does), then
    /// ticked over the axes where it is now, neither the layout engine nor
    /// the aspect placing it first (they do when the figure is drawn or
    /// laid out).
    pub(crate) fn update_ticks_as_is(&mut self, axes: ArtistId, name: AxisName) {
        self.unstale_axes(axes);
        let Some(extent) = self.axes_bbox(axes) else {
            return;
        };
        let dpi = self.dpi();
        let bound = self.bound_axes(axes)[usize::from(name == AxisName::Y)];
        self.tick_axis_now(axes, name, &extent, dpi, bound);
    }

    /// [`update_ticks_as_is`](Self::update_ticks_as_is) as matplotlib's
    /// `Axis._update_ticks` places the ticks, which the program's helpers
    /// see: an axis the program ticks is ticked by it again.
    pub(crate) fn place_ticks_as_is(&mut self, axes: ArtistId, name: AxisName) {
        self.unstale_axes(axes);
        if !self.update_program_ticks(axes, name) {
            self.update_ticks_as_is(axes, name);
        }
    }

    /// [`place_ticks_as_is`](Self::place_ticks_as_is) for the `minor`
    /// level alone (or the major one), as matplotlib's `set_ticklabels`
    /// places them.
    pub(crate) fn place_level_as_is(&mut self, axes: ArtistId, name: AxisName, minor: bool) {
        self.unstale_axes(axes);
        if self.update_program_level(axes, name, minor) {
            return;
        }
        let Some(extent) = self.axes_bbox(axes) else {
            return;
        };
        let dpi = self.dpi();
        let bound = self.bound_axes(axes)[usize::from(name == AxisName::Y)];
        if let Some(state) = self.store.get_as_mut::<AxesState>(axes) {
            let geometry = state.axis_geometry(name, &extent, dpi, bound);
            state.axis_mut(name).update_level_ticks(&geometry, minor);
        }
    }

    /// Places the ticks of the 2-D `axes` over its box as it is now: those
    /// of the axes the program ticks (`program`), or of the others. An
    /// animated axis, which matplotlib's draw leaves out (but printing),
    /// is not ticked: its helpers, which may be the program's, are not
    /// asked.
    fn tick_axes(&mut self, axes: ArtistId, program: bool) {
        if program && !self.ticks_by_program() {
            return;
        }
        let Some(extent) = self.axes_bbox(axes) else {
            return;
        };
        let dpi = self.dpi();
        let bound = self.bound_axes(axes);
        for (name, bound) in [AxisName::X, AxisName::Y].into_iter().zip(bound) {
            if self.program_ticker(axes, name).is_some() != program
                || self.axis_left_out(axes, name)
            {
                continue;
            }
            if self.axis_shown(axes, name)
                || crate::axisartist::reads_hidden_axes(&self.store, axes)
            {
                self.tick_axis_now(axes, name, &extent, dpi, bound);
            } else {
                // matplotlib ticks no hidden axis (but for the axis
                // artists that read it): what its arithmetic warns about is
                // not said.
                crate::warnings::capturing_runtime(|| {
                    self.tick_axis_now(axes, name, &extent, dpi, bound);
                });
            }
        }
    }

    /// The part of [`prepare`](Self::prepare) for the 2-D `axes`, which
    /// returns whether it was laid out; an axes whose aspect fails is not
    /// ticked (matplotlib's draw raises first), nor are the axes a draw
    /// reaches `after_failure` of an axes' ticks (matplotlib's draw stops
    /// there): what their arithmetic warns about is not said, and the
    /// program's helpers are not asked. Prepared as it is, the
    /// axes is ticked where it is now: matplotlib applies the
    /// aspect (and a secondary axis takes its host's limits) when it draws
    /// or measures the axes, not when it computes ticks between draws. A
    /// draw leaves the axes the program ticks to
    /// [`program_figure_drawn`](Self::program_figure_drawn).
    fn prepare_axes(
        &mut self,
        id: ArtistId,
        preparation: Preparation,
        after_failure: bool,
    ) -> bool {
        let laid_out = if matches!(preparation, Preparation::AsIs | Preparation::Picking(_)) {
            self.unstale_axes(id);
            true
        } else {
            self.lay_out_axes(id)
        };
        crate::polar::update_spine_visibility(self, id);
        if laid_out {
            let picked_elsewhere =
                matches!(preparation, Preparation::Picking(reached) if !reached.reaches(id));
            if self.drawn_by_figure(id) && !after_failure && !picked_elsewhere {
                self.tick_axes(id, false);
            } else {
                // matplotlib computes no ticks of the axes its draw leaves
                // out (hidden or animated ones), nor of those a pick does
                // not reach: what their arithmetic warns about is not said
                // there.
                crate::warnings::capturing_runtime(|| self.tick_axes(id, false));
            }
            match preparation {
                // matplotlib's pick asks the locators of the axes it
                // reaches for their locations, no formatter, and no helper
                // of the others.
                Preparation::Picking(reached) if reached.reaches(id) => {
                    self.locate_program_ticks(id);
                }
                Preparation::Picking(_) | Preparation::Drawing(_) => {}
                Preparation::AsIs | Preparation::Measuring => self.tick_axes(id, true),
            }
        }
        laid_out
    }

    /// [`prepare`](Self::prepare) without the layout engine: the figure
    /// as it is now, its axes where they are until the next draw or
    /// layout pass places them and applies their aspect (matplotlib
    /// computes ticks between draws without either). A due layout pass
    /// stays due.
    pub(crate) fn prepare_as_is(&mut self) {
        self.prepare_between_draws(Preparation::AsIs);
    }

    /// Brings `axes` up to date to be drawn alone, as matplotlib's
    /// `Axes.draw` does: its view autoscaled, its ticks placed over its
    /// box as it is now, by the program's helpers too; neither the layout
    /// engine nor the aspect runs.
    pub(crate) fn prepare_axes_alone(&mut self, axes: ArtistId) {
        self.prepare_axes(axes, Preparation::AsIs, false);
    }

    /// [`prepare_as_is`](Self::prepare_as_is) for a pick reaching the
    /// children of the axes `reached` says.
    pub(crate) fn prepare_for_pick(&mut self, reached: PickedAxes) {
        self.prepare_between_draws(Preparation::Picking(reached));
    }

    /// `preparation`, one of those between draws, with a due layout pass
    /// left due.
    fn prepare_between_draws(&mut self, preparation: Preparation) {
        let pending = std::mem::take(&mut self.layout.pending);
        self.prepare_with(preparation, 0);
        self.layout.pending |= pending;
    }

    /// The mutable pass before drawing: a due layout-engine pass, then
    /// autoscaling, aspect and ticks of every axes. 3-D axes stay where
    /// they stand: only [`prepare_for_draw`](Self::prepare_for_draw)
    /// makes their boxes square.
    pub(crate) fn prepare(&mut self) {
        self.prepare_with(Preparation::Measuring, 0);
    }

    /// [`prepare`](Self::prepare) for a draw: the layout engine runs, and
    /// the boxes of 3-D axes are made square, as matplotlib applies their
    /// aspect when it draws them (an axis of a 3-D axes measured before
    /// the first draw is where its unsquared axes puts it).
    /// `painting` tells whether the artists are drawn after.
    pub(crate) fn prepare_for_draw(&mut self, painting: Painting) {
        self.layout.pending = true;
        // A draw the program makes meanwhile (from a formatter) is one.
        let draws = std::mem::replace(&mut self.draws_per_layout, 1);
        self.prepare_with(Preparation::Drawing(painting), draws);
    }

    /// [`prepare`](Self::prepare) for `preparation`, `draws` being how
    /// many draws the program saw since the last layout of a draw.
    fn prepare_with(&mut self, preparation: Preparation, draws: usize) {
        // What the program's scales answered before stands for no part of
        // this layout (see `transforms::program_calls`).
        crate::transforms::program_calls::forget();
        let failed_before = crate::layout::failures::failed();
        let axes = self.axes_ids();
        if matches!(preparation, Preparation::Drawing(_)) {
            // `Figure.draw` applies the aspect of every axes, its secondary
            // axes after their host (`_get_draw_artists`), before its
            // layout engine runs: a projection giving its own data ratio
            // is asked for it there too.
            for &id in &axes {
                self.aspect_applied(id);
                if self.asks_data_ratio(id) {
                    self.apply_aspect(id);
                }
            }
        }
        if matches!(preparation, Preparation::Drawing(_)) {
            crate::text::font_notes::layout_engine(|| self.run_pending_layout());
        } else {
            self.run_measured_layout();
        }
        if matches!(preparation, Preparation::Drawing(_)) && !failed_before {
            // matplotlib's `Figure.draw` passes over a `ValueError` its
            // layout engine raised, and draws as the pass left the figure.
            crate::layout::failures::pass_over(crate::layout::failures::ErrorClass::ValueError);
        }
        let drawing = matches!(preparation, Preparation::Drawing(_));
        if drawing && self.layout_failed() {
            // matplotlib's draw raised in its layout engine, before it
            // drew anything (see `take_layout_failure`).
            return;
        }
        let mut failed = false;
        let mut ticked = Vec::new();
        let laid_out: Vec<ArtistId> = (axes.iter().copied())
            .filter(|&id| {
                let laid_out = self.prepare_axes(id, preparation, failed);
                if drawing && laid_out {
                    self.place_parasites_at_host(id);
                }
                if !failed {
                    ticked.push(id);
                }
                failed |= drawing && self.drawn_by_figure(id) && self.ticks_fail_draw(id);
                laid_out
            })
            .collect();
        let painting = match preparation {
            Preparation::Drawing(painting) => Some(painting),
            Preparation::AsIs | Preparation::Picking(_) | Preparation::Measuring => None,
        };
        if painting.is_some() {
            self.program_figure_drawn(&axes, draws);
            // The axes the program ticks that no axis draw ticked (hidden
            // ones of the axes drawn, which other artists may still read;
            // none of those along a curvilinear grid does).
            for &id in &laid_out {
                if self.drawn_by_figure(id)
                    && ticked.contains(&id)
                    && !crate::axisartist::on_curvilinear_grid(&self.store, id)
                {
                    self.tick_axes(id, true);
                }
            }
        }
        crate::mplot3d::prepare_axes3d(self, painting);
        // Titles clear the decorations of twins and insets, whose ticks
        // are only all known now. matplotlib places them when it draws
        // (and measures an axes' tight box): the fonts they look up count
        // for a draw only.
        for &id in &axes {
            if painting.is_some() {
                crate::text::font_notes::measured_before(|| self.update_title_position(id));
            } else {
                crate::text::font_notes::unnoted(|| self.update_title_position(id));
            }
        }
        crate::legend::lay_out_legends(self);
        crate::offsetbox::lay_out_boxes(self);
        crate::inset::place_indicators(self);
        // Where the axes stand now (the layout engine and the other axes
        // placed them), for the spines of the program's the draw places
        // (matplotlib adjusts them only where it draws or measures them,
        // not where it picks or tests a point).
        if painting.is_some() {
            for id in axes {
                self.span_program_spines(id);
            }
        }
    }

    /// The ids of the figure's axes (the artists with an [`AxesState`]).
    fn axes_ids(&self) -> Vec<ArtistId> {
        self.store
            .ids()
            .filter(|&id| self.store.get_as::<AxesState>(id).is_some())
            .collect()
    }

    /// Whether the layout a draw ran failed as matplotlib's engine fails
    /// (raising a `ZeroDivisionError`).
    fn layout_failed(&self) -> bool {
        self.pending_errors.iter().any(Self::fails_layout)
    }

    /// The failure of the layout a draw ran (see `layout_failed`), taken
    /// out of the problems it reports.
    pub(crate) fn take_layout_failure(&mut self) -> Option<DrawError> {
        let at = self.pending_errors.iter().position(Self::fails_layout)?;
        Some(self.pending_errors.remove(at))
    }

    /// Whether `error` is a layout engine's failure.
    fn fails_layout(error: &DrawError) -> bool {
        error.kind == DrawErrorKind::DivisionByZero
    }

    /// The problems a draw reports besides those met drawing: those of
    /// the layout, and subplot parameters leaving no room, which matplotlib
    /// refuses when the figure is made and pltrs reports whenever the
    /// figure draws.
    pub(crate) fn draw_problems(&self) -> Vec<DrawError> {
        let mut errors = self.pending_errors.clone();
        let params = self.subplot_params_of(self.root);
        if let Some(Err(err)) = params.map(|p| p.updated(SubplotOverrides::default())) {
            errors.push(DrawError::new(self.root, err.to_string()));
        }
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

    /// The display outline of `axes` when what it clips is clipped to its
    /// patch's outline (an outline of its own clipping as an outline, the
    /// wedge of polar axes, the ellipse of geographic axes, the outline of
    /// a custom projection); `None` otherwise.
    pub(crate) fn axes_outline(&self, axes: ArtistId) -> Option<ClipPath> {
        let state = self.store.get_as::<AxesState>(axes)?;
        if let Some(outline) = &state.patch_outline {
            return (!outline.clips_as_box)
                .then(|| self.resolved_outline(outline))
                .flatten();
        }
        self.projection_outline(axes, state)
    }

    /// The display → data transform of `axes` as matplotlib inverts its
    /// `transData` (see `AxesState::data_transform_inverse`); auxiliary
    /// axes invert theirs whole.
    pub(crate) fn data_transform_inverse(&self, axes: ArtistId) -> Option<ResolvedTransform> {
        let state = self.artist_as::<AxesState>(axes)?;
        if state.aux_transform.is_some() {
            return self.data_transform(axes)?.inverted();
        }
        state.data_transform_inverse(&self.axes_bbox(axes)?)
    }

    /// The display outline the background of `axes` is drawn along when
    /// it is no rectangle of the axes box: an outline of its own, else
    /// [`axes_outline`](Self::axes_outline)'s.
    pub(crate) fn background_outline(&self, axes: ArtistId) -> Option<ClipPath> {
        let state = self.store.get_as::<AxesState>(axes)?;
        match &state.patch_outline {
            Some(outline) => self.resolved_outline(outline),
            None => self.projection_outline(axes, state),
        }
    }

    /// The display box what `axes` clips is clipped to when its own
    /// outline clips as a box (matplotlib's `Rectangle` patch).
    pub(crate) fn outline_clip_box(&self, axes: ArtistId) -> Option<Bbox> {
        let outline = self
            .store
            .get_as::<AxesState>(axes)?
            .patch_outline
            .as_ref()
            .filter(|outline| outline.clips_as_box)?;
        let resolved = outline.transform.resolve(self).ok()?;
        Some(resolved.transform_path(&outline.path).extents())
    }

    /// `outline` in display coordinates.
    fn resolved_outline(&self, outline: &PatchOutline) -> Option<ClipPath> {
        let resolved = outline.transform.resolve(self).ok()?;
        let (path, transform) = resolved.split_path(&outline.path);
        Some(ClipPath::new(path.into_owned(), transform))
    }

    /// The display outline of the projection of `axes`, whose state is
    /// `state`, unless it is the axes box.
    fn projection_outline(&self, axes: ArtistId, state: &AxesState) -> Option<ClipPath> {
        if state.projection.is_rectangular() {
            return None;
        }
        Some(ClipPath::new(
            state.patch_path(),
            unit_to_bbox(&self.axes_bbox(axes)?),
        ))
    }

    /// Where axes `axes` placed in another (an inset, a secondary axes)
    /// is, in the fractions of the (sub)figure of its host, with that
    /// figure's box in pixels: the bits matplotlib's
    /// `_TransformedBoundsLocator` gives the axes' `_position` (see
    /// [`host_to_figure`]), its aspect applied once a draw or a layout
    /// placed it. `None` for axes in a figure frame and for axes covering
    /// their host.
    pub(crate) fn hosted_position(&self, axes: ArtistId) -> Option<(Bbox, Bbox)> {
        let state = self.artist_as::<AxesState>(axes)?;
        let parent = state.base().parent.or(state.home).unwrap_or(self.root);
        let position = state.aspect.active_position(&state.position);
        if position == Bbox::UNIT || self.frame_bbox(parent).is_some() {
            return None;
        }
        let host = self.axes_bbox(parent)?;
        let figure = self.figure_bbox_of(parent)?;
        let placed = match state
            .aspect
            .active_in_figure(&state.position, &host, &figure)
        {
            Some(active) => active,
            None => position.transformed(&host_to_figure(&host, &figure)?),
        };
        Some((placed, figure))
    }
}

impl TransformContext for FigureState {
    fn dpi(&self) -> f64 {
        self.dpi.get()
    }

    fn frame_bbox(&self, frame: ArtistId) -> Option<Bbox> {
        let node = self.artist_as::<FigureFrame>(frame)?;
        if frame == self.root {
            let (w, h) = self.size_pixels();
            let dpi = self.dpi();
            let (x, y) = self.print_window.map_or((0.0, 0.0), |window| {
                (-window.x0() * dpi, -window.y0() * dpi)
            });
            return Some(Bbox::from_bounds(x, y, w, h));
        }
        // A removed subfigure stays where it was in its figure.
        let parent = self.frame_bbox(node.base().parent.or(node.home)?)?;
        Some(fraction_of(&parent, &node.relative_extent))
    }

    /// Axes are placed in a figure frame, or (insets, secondary axes,
    /// parasite axes and the projection planes of 3-D axes) in their host
    /// axes. A removed axes, owned by none, keeps its position in the
    /// figure, as matplotlib's keeps its transforms.
    fn axes_bbox(&self, axes: ArtistId) -> Option<Bbox> {
        let Some(state) = self.artist_as::<AxesState>(axes) else {
            return crate::mplot3d::axes3d_bbox(self, axes);
        };
        if let Some(extent) = self.parasite_extent(axes) {
            return Some(extent);
        }
        let parent = state.base().parent.or(state.home).unwrap_or(self.root);
        let position = state.aspect.active_position(&state.position);
        if let Some(frame) = self.frame_bbox(parent) {
            return Some(fraction_of(&frame, &position));
        }
        if position == Bbox::UNIT {
            // Covering its host (parasite axes, 3-D planes).
            return self.axes_bbox(parent);
        }
        let (placed, figure) = self.hosted_position(axes)?;
        Some(fraction_of(&figure, &placed))
    }

    /// Auxiliary axes map their data through their transform into the
    /// data of their host.
    fn data_transform(&self, axes: ArtistId) -> Option<ResolvedTransform> {
        let state = self.artist_as::<AxesState>(axes)?;
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

/// The part of `outer` described by `fraction` (a box in `outer`'s unit
/// square), as matplotlib's `TransformedBbox` of an axes' position bounds
/// it: by Python's `min` and `max` of its corners, so that a NaN edge
/// after a finite one collapses the box onto that edge.
fn fraction_of(outer: &Bbox, fraction: &Bbox) -> Bbox {
    fraction.transformed_enclosing(&crate::transforms::unit_to_bbox(outer))
}

/// The map from the fractions of the box `host` of an axes to those of
/// the (sub)figure `figure` (at `figure` in display pixels), for an axes
/// placed inside it (an inset, a secondary axes), with matplotlib's
/// arithmetic: its `_TransformedBoundsLocator` maps the axes' bounds
/// through `transAxes - transSubfigure`, the host's box transform followed
/// by numpy's inverse of the figure's; the axes' box is then its position
/// through `transSubfigure`. `None` for a figure of no area.
pub(crate) fn host_to_figure(host: &Bbox, figure: &Bbox) -> Option<Affine2D> {
    Some(unit_to_bbox(host).then(unit_to_bbox(figure).inverted()?))
}
