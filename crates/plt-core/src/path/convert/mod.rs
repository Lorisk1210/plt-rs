//! The path-conditioning pipeline of matplotlib's `src/path_converters.h`.
//!
//! Before drawing, every matplotlib backend pushes a path through a chain of
//! *vertex sources*: pull-based iterators that each read vertices from the
//! previous stage, exactly like Agg's `vertex(&x, &y)` protocol. Agg's
//! `RendererAgg::draw_path` chains them as
//!
//! ```text
//! transform → remove_nans → clip_to → snap → simplify → flatten_curves → sketch
//! ```
//!
//! and other entry points (markers, path collections, `cleanup_path`,
//! `convert_to_string`) use different subsets in the same order. Each stage
//! here is a separate adapter with the same constructor parameters as its C++
//! counterpart, so a backend can chain exactly what matplotlib chains:
//!
//! ```
//! use plt_core::path::convert::{SnapMode, VertexSource};
//! use plt_core::{Affine2D, Bbox, Path, Point};
//!
//! let line = Path::polyline(vec![Point::new(0.2, 0.3), Point::new(5.7, 0.3)]);
//! let canvas = Bbox::from_extents(0.0, 0.0, 100.0, 100.0);
//! let pixels: Path = line
//!     .iter_vertices()
//!     .transform(Affine2D::identity())
//!     .remove_nans(true, line.codes().is_some())
//!     .clip_to(Some(canvas))
//!     .snap(SnapMode::Auto, line.len(), 1.0)
//!     .simplify(None)
//!     .collect_path();
//! assert_eq!(pixels.vertices(), &[Point::new(0.5, 0.5), Point::new(6.5, 0.5)]);
//! ```
//!
//! Streams are made of [`Vertex`] values (one per stored vertex, so a cubic
//! curve contributes three vertices tagged [`PathCode::Curve4`]); the end of
//! iteration is Agg's `path_cmd_stop`. [`VertexSource::segments`] groups a
//! stream into [`Segment`](crate::Segment)s and
//! [`VertexSource::collect_path`] stores it as a [`Path`].
//!
//! [`Path::cleaned`] and [`Path::iter_segments`] reproduce matplotlib's
//! methods of the same name on top of the pipeline, and
//! [`Path::convert_to_string`] the string serialisation used by the vector
//! backends.

mod cleanup;
mod clip;
mod curve;
mod nan;
mod queue;
mod simplify;
mod sketch;
mod snap;
mod source;
mod string;

pub use cleanup::{CleanupOptions, Conditioned, SimplifySettings, VertexSegments};
pub use clip::Clipper;
pub use curve::CurveFlattener;
pub use nan::NanRemover;
pub use simplify::Simplifier;
pub use sketch::{SketchParams, Sketcher};
pub use snap::{SnapMode, Snapper};
pub use source::{PathVertices, Transformed};
pub use string::{StringCodes, StringOptions};

use super::{Path, PathCode};
use crate::{Affine2D, Bbox, Point};

/// One vertex of a conditioning stream: a point and the command consuming it.
///
/// For [`PathCode::ClosePoly`] the point is whatever the source stored there
/// (matplotlib keeps and sometimes uses those coordinates).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    /// The command this vertex belongs to.
    pub code: PathCode,
    /// The vertex coordinates.
    pub point: Point,
}

impl Vertex {
    /// Creates a vertex.
    pub const fn new(code: PathCode, point: Point) -> Self {
        Self { code, point }
    }

    /// A [`PathCode::MoveTo`] vertex.
    pub const fn move_to(point: Point) -> Self {
        Self::new(PathCode::MoveTo, point)
    }

    /// A [`PathCode::LineTo`] vertex.
    pub const fn line_to(point: Point) -> Self {
        Self::new(PathCode::LineTo, point)
    }

    /// Agg's `is_vertex`: every code except [`PathCode::ClosePoly`].
    pub const fn is_vertex(&self) -> bool {
        !matches!(self.code, PathCode::ClosePoly)
    }
}

/// Number of vertices that follow the first one of a command (Agg's
/// `num_extra_points_map`).
const fn extra_vertices(code: PathCode) -> usize {
    match code {
        PathCode::Curve3 => 1,
        PathCode::Curve4 => 2,
        PathCode::MoveTo | PathCode::LineTo | PathCode::ClosePoly => 0,
    }
}

/// A rewindable stream of [`Vertex`] values, with matplotlib's conditioning
/// stages as chainable adapters.
///
/// `rewind` restarts the stream like Agg's `rewind(0)`: it resets exactly
/// the state the corresponding C++ class resets, so a stage that scans its
/// input first (the [`Snapper`]) observes the same carried-over state as in
/// matplotlib.
pub trait VertexSource: Iterator<Item = Vertex> + Sized {
    /// Restarts the stream from its first vertex.
    fn rewind(&mut self);

    /// Applies an affine transformation to every vertex except closures
    /// (Agg's `conv_transform`).
    fn transform(self, transform: Affine2D) -> Transformed<Self> {
        Transformed::new(self, transform)
    }

    /// Breaks the path at non-finite vertices (`PathNanRemover`). Pass the
    /// source path's `codes().is_some()` as `has_codes`.
    fn remove_nans(self, enabled: bool, has_codes: bool) -> NanRemover<Self> {
        NanRemover::new(self, enabled, has_codes)
    }

    /// Clips line segments to `rect` grown by one unit on every side
    /// (`PathClipper`); `None` disables clipping.
    fn clip_to(self, rect: Option<Bbox>) -> Clipper<Self> {
        Clipper::new(self, rect)
    }

    /// Rounds vertices to pixel centres (`PathSnapper`). `total_vertices` is
    /// the vertex count of the original path.
    fn snap(self, mode: SnapMode, total_vertices: usize, stroke_width: f64) -> Snapper<Self> {
        Snapper::new(self, mode, total_vertices, stroke_width)
    }

    /// Merges nearly collinear line segments (`PathSimplifier`);
    /// `Some(threshold)` enables simplification.
    fn simplify(self, threshold: Option<f64>) -> Simplifier<Self> {
        Simplifier::new(self, threshold)
    }

    /// Replaces Bézier curves by line segments with Agg's `curve_div`
    /// subdivision (`conv_curve`); disabled stages pass curves through.
    fn flatten_curves(self, enabled: bool) -> CurveFlattener<Self> {
        CurveFlattener::new(self, enabled)
    }

    /// Applies matplotlib's hand-drawn "sketch" wiggle (`Sketch`). Expects
    /// flattened input; `None` (or a zero scale) passes vertices through.
    fn sketch(self, params: Option<SketchParams>) -> Sketcher<Self> {
        Sketcher::new(self, params)
    }

    /// Groups the stream into [`Segment`](crate::Segment)s, consuming as
    /// many vertices as each command's first code requires.
    fn segments(self) -> VertexSegments<Self> {
        VertexSegments::new(self)
    }

    /// Stores the stream as a path (codes are always present).
    fn collect_path(self) -> Path {
        let mut path = Path::default();
        self.collect_path_into(&mut path);
        path
    }

    /// Stores the stream in `path`, replacing what it held but keeping its
    /// allocations, as [`collect_path`](Self::collect_path) would.
    fn collect_path_into(self, path: &mut Path) {
        let codes = path.codes.get_or_insert_with(Vec::new);
        path.vertices.clear();
        codes.clear();
        for vertex in self {
            path.vertices.push(vertex.point);
            codes.push(vertex.code);
        }
        path.interpolation_steps = 1;
    }
}
