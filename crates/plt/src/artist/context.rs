//! Contexts handed to artists while measuring and drawing.

use plt_core::Bbox;
use plt_render::{ClipPath, GraphicsContext, Renderer, SnapMode};

use super::{ArtistBase, ArtistId, ArtistStore, ClipPathSpec};
use crate::figure::FigureState;
use crate::transforms::{ResolvedTransform, Transform, TransformContext};

/// Read-only access to figure geometry, for measuring and hit testing.
#[derive(Clone, Copy, Debug)]
pub struct LayoutContext<'a> {
    figure: &'a FigureState,
}

impl<'a> LayoutContext<'a> {
    pub(crate) fn new(figure: &'a FigureState) -> Self {
        Self { figure }
    }

    /// Every artist of the figure.
    pub fn store(&self) -> &'a ArtistStore {
        &self.figure.store
    }

    /// Canvas resolution.
    pub fn dpi(&self) -> f64 {
        self.figure.dpi
    }

    /// Converts points to pixels at the canvas resolution.
    pub fn points_to_pixels(&self, points: f64) -> f64 {
        points * self.figure.dpi / 72.0
    }

    /// Resolves a transform; `None` if it refers to removed artists.
    pub fn resolve(&self, transform: &Transform) -> Option<ResolvedTransform> {
        transform.resolve(self.figure).ok()
    }

    /// Display-space extent of a figure or subfigure frame.
    pub fn frame_extent(&self, frame: ArtistId) -> Option<Bbox> {
        self.figure.frame_bbox(frame)
    }

    /// Display-space extent of an axes.
    pub fn axes_extent(&self, axes: ArtistId) -> Option<Bbox> {
        self.figure.axes_bbox(axes)
    }

    /// Display-space extent of an axes or figure frame used as a clip box.
    pub fn clip_extent(&self, owner: ArtistId) -> Option<Bbox> {
        self.axes_extent(owner).or_else(|| self.frame_extent(owner))
    }
}

/// Everything an artist needs while drawing: the renderer plus read-only
/// access to the rest of the figure.
pub struct DrawContext<'a> {
    layout: LayoutContext<'a>,
    renderer: &'a mut dyn Renderer,
}

impl std::fmt::Debug for DrawContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DrawContext")
            .field("layout", &self.layout)
            .finish_non_exhaustive()
    }
}

impl<'a> DrawContext<'a> {
    pub(crate) fn new(figure: &'a FigureState, renderer: &'a mut dyn Renderer) -> Self {
        Self {
            layout: LayoutContext::new(figure),
            renderer,
        }
    }

    /// Read-only geometry access.
    pub fn layout(&self) -> &LayoutContext<'a> {
        &self.layout
    }

    /// The output backend.
    pub fn renderer(&mut self) -> &mut dyn Renderer {
        self.renderer
    }

    /// Resolves a transform; `None` if it refers to removed artists.
    pub fn resolve(&self, transform: &Transform) -> Option<ResolvedTransform> {
        self.layout.resolve(transform)
    }

    /// Converts points to pixels at the canvas resolution.
    pub fn points_to_pixels(&self, points: f64) -> f64 {
        self.layout.points_to_pixels(points)
    }

    /// Draws a child artist if it exists, is visible and not animated.
    pub fn draw_child(&mut self, id: ArtistId) {
        let Some(artist) = self.layout.store().get(id) else {
            return;
        };
        let base = artist.base();
        if base.visible && !base.animated {
            artist.draw(self);
        }
    }

    /// Draws children in z-order (stable for equal z-orders).
    pub fn draw_children_sorted(&mut self, children: &[ArtistId]) {
        let store = self.layout.store();
        let mut ordered: Vec<(f64, ArtistId)> = children
            .iter()
            .filter_map(|&id| Some((store.get(id)?.base().zorder, id)))
            .collect();
        ordered.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, id) in ordered {
            self.draw_child(id);
        }
    }

    /// A graphics context initialised from an artist's common properties:
    /// clipping, snapping, sketch, alpha override, url and gid.
    pub fn base_gc(&self, base: &ArtistBase) -> GraphicsContext {
        let mut gc = GraphicsContext {
            alpha: base.alpha,
            snap: match base.snap {
                None => SnapMode::Auto,
                Some(true) => SnapMode::Always,
                Some(false) => SnapMode::Never,
            },
            sketch: base.sketch,
            url: base.url.clone(),
            gid: base.gid.clone(),
            ..GraphicsContext::default()
        };
        if base.clip_on {
            gc.clip_rect = base
                .clip_box
                .and_then(|owner| self.layout.clip_extent(owner));
            gc.clip_path = base
                .clip_path
                .as_ref()
                .and_then(|spec| self.clip_path(spec));
        }
        gc
    }

    fn clip_path(&self, spec: &ClipPathSpec) -> Option<ClipPath> {
        match spec {
            ClipPathSpec::Path { path, transform } => {
                let resolved = self.resolve(transform)?;
                let (path, transform) = resolved.split_path(path);
                Some(ClipPath {
                    path: path.into_owned(),
                    transform,
                })
            }
            ClipPathSpec::Artist(id) => self.layout.store().get(*id)?.outline(&self.layout),
        }
    }
}
