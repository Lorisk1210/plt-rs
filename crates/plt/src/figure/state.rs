//! The shared, mutable state behind a [`Figure`](super::Figure) handle.

use plt_core::{Bbox, Rgba};
use plt_render::Renderer;

use super::frame::FigureFrame;
use crate::artist::{Artist, ArtistId, ArtistStore, DrawContext, DrawError};
use crate::axes::AxesState;
use crate::gridspec::SubplotParams;
use crate::transforms::{ResolvedTransform, TransformContext};

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
}

impl FigureState {
    /// A figure of `size_inches` at `dpi` with an empty root frame.
    pub(crate) fn new(size_inches: (f64, f64), dpi: f64, facecolor: Rgba, edgecolor: Rgba) -> Self {
        let mut store = ArtistStore::default();
        let params = SubplotParams::from_rc(&crate::rc::params());
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
        id
    }

    /// Detaches an artist from its parent and deletes it (with its children).
    pub(crate) fn remove_artist(&mut self, id: ArtistId) {
        let parent = self.store.get(id).and_then(|a| a.base().parent);
        if let Some(parent) = parent.and_then(|p| self.store.get_mut(p)) {
            parent.remove_child(id);
        }
        self.store.remove(id);
        self.stale = true;
    }

    /// Recomputes the stale autoscaled view limits of one axes, honouring
    /// the sticky edges of its children.
    pub(crate) fn unstale_axes(&mut self, axes: ArtistId) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        if !state.needs_autoscale() {
            return;
        }
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        for child in state.children.iter().filter_map(|&id| self.store.get(id)) {
            let (x, y) = &child.base().sticky_edges;
            xs.extend_from_slice(x);
            ys.extend_from_slice(y);
        }
        xs.sort_by(f64::total_cmp);
        ys.sort_by(f64::total_cmp);
        if let Some(state) = self.store.get_as_mut::<AxesState>(axes) {
            state.unstale_view((&xs, &ys));
        }
    }

    /// The mutable pass before drawing: autoscaling (and, later, ticks
    /// and layout).
    pub(crate) fn prepare(&mut self) {
        let axes: Vec<ArtistId> = self
            .store
            .ids()
            .filter(|&id| self.store.get_as::<AxesState>(id).is_some())
            .collect();
        for id in axes {
            self.unstale_axes(id);
        }
    }

    /// Renders the whole figure, returning the problems encountered.
    pub(crate) fn draw(&self, renderer: &mut dyn Renderer) -> Vec<DrawError> {
        let mut ctx = DrawContext::new(self, renderer);
        ctx.draw_child(self.root);
        ctx.into_errors()
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

    fn axes_bbox(&self, axes: ArtistId) -> Option<Bbox> {
        let state = self.store.get_as::<AxesState>(axes)?;
        let frame = self.frame_bbox(state.base().parent?)?;
        Some(fraction_of(&frame, &state.position))
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
