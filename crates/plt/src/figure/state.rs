//! The shared, mutable state behind a [`Figure`](super::Figure) handle.

use plt_core::{Bbox, Rgba};
use plt_render::{ClipPath, Renderer};

use super::frame::FigureFrame;
use crate::artist::{Artist, ArtistId, ArtistStore, DrawContext, DrawError};
use crate::axes::{AxesState, ShareGroups};
use crate::gridspec::SubplotParams;
use crate::layout::LayoutState;
use crate::transforms::{ResolvedTransform, TransformContext, unit_to_bbox};

/// Everything a figure owns: its artists and its canvas geometry.
#[derive(Debug)]
pub(crate) struct FigureState {
    /// Every artist of the figure, including frames and axes.
    pub(crate) store: ArtistStore,
    /// The root frame (the figure itself).
    pub(crate) root: ArtistId,
    /// Canvas resolution in dots per inch.
    pub(crate) dpi: f64,
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
}

impl FigureState {
    /// A figure of `size_inches` at `dpi` with an empty root frame.
    pub(crate) fn new(size_inches: (f64, f64), dpi: f64, facecolor: Rgba, edgecolor: Rgba) -> Self {
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
            root,
            dpi,
            size_inches,
            stale: true,
            pending_errors: Vec::new(),
            shared: ShareGroups::default(),
            layout: LayoutState::from_rc(&rc),
            current_axes: None,
        }
    }

    /// Canvas size in pixels.
    pub(crate) fn size_pixels(&self) -> (f64, f64) {
        (self.size_inches.0 * self.dpi, self.size_inches.1 * self.dpi)
    }

    /// Inserts `artist` as a child of the container `parent`.
    pub(crate) fn adopt(
        &mut self,
        parent: ArtistId,
        mut artist: Box<dyn crate::artist::Artist>,
    ) -> ArtistId {
        artist.base_mut().parent = Some(parent);
        let id = self.store.insert(artist);
        if let Some(frame) = self.store.get_as_mut::<FigureFrame>(parent) {
            frame.children.push(id);
        } else if let Some(axes) = self.store.get_as_mut::<AxesState>(parent) {
            axes.children.push(id);
        }
        self.stale = true;
        self.layout.pending = true;
        id
    }

    /// Detaches an artist from its parent and deletes it (with its children).
    pub(crate) fn remove_artist(&mut self, id: ArtistId) {
        let parent = self.store.get(id).and_then(|a| a.base().parent);
        if let Some(parent) = parent.and_then(|p| self.store.get_mut(p)) {
            parent.remove_child(id);
        }
        self.store.remove(id);
        self.shared.remove(id);
        self.stale = true;
        self.layout.pending = true;
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
            let Some(extent) = self.axes_bbox(id) else {
                continue;
            };
            let dpi = self.dpi;
            if let Some(state) = self.store.get_as_mut::<AxesState>(id) {
                state.update_ticks(&extent, dpi);
            }
        }
        crate::mplot3d::prepare_axes3d(self);
        // Titles clear the decorations of twins and insets, whose ticks
        // are only all known now.
        for id in axes {
            self.update_title_position(id);
        }
    }

    /// Renders the whole figure, returning the problems encountered.
    pub(crate) fn draw(&self, renderer: &mut dyn Renderer) -> Vec<DrawError> {
        let mut ctx = DrawContext::new(self, renderer);
        ctx.draw_child(self.root);
        let mut errors = self.pending_errors.clone();
        errors.extend(ctx.into_errors());
        errors
    }

    /// The display outline of `axes` when its patch is not a rectangle
    /// (the wedge of polar axes); `None` otherwise.
    pub(crate) fn axes_outline(&self, axes: ArtistId) -> Option<ClipPath> {
        let state = self.store.get_as::<AxesState>(axes)?;
        state.polar()?;
        Some(ClipPath {
            path: state.patch_path(),
            transform: unit_to_bbox(&self.axes_bbox(axes)?),
        })
    }
}

impl TransformContext for FigureState {
    fn dpi(&self) -> f64 {
        self.dpi
    }

    fn frame_bbox(&self, frame: ArtistId) -> Option<Bbox> {
        let node = self.store.get_as::<FigureFrame>(frame)?;
        if frame == self.root {
            let (w, h) = self.size_pixels();
            return Some(Bbox::from_bounds(0.0, 0.0, w, h));
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

    fn data_transform(&self, axes: ArtistId) -> Option<ResolvedTransform> {
        let display = self.axes_bbox(axes)?;
        Some(
            self.store
                .get_as::<AxesState>(axes)?
                .data_transform(&display),
        )
    }
}

/// The part of `outer` described by `fraction` (a box in `outer`'s unit square).
fn fraction_of(outer: &Bbox, fraction: &Bbox) -> Bbox {
    fraction.transformed(&crate::transforms::unit_to_bbox(outer))
}
