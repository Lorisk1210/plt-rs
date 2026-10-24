//! Test helpers shared by the crate's unit tests.

use plt_core::{Affine2D, Path, Point, Rgba};
use plt_render::{GraphicsContext, Renderer, RgbaImage};

use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxesState, AxisName, Tick};
use crate::colors::Colormap;
use crate::dates::DateTime;
use crate::figure::Figure;
use crate::image::ScalarField;
use crate::numeric::Grid;

/// Whether two numbers agree to within `1e-9`.
pub(crate) fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// Whether two points agree to within `1e-9`.
pub(crate) fn close_points(a: Point, b: Point) -> bool {
    (a - b).length() < 1e-9
}

/// A 640 × 480 px figure with one axes over `(0, 10)²` at `(80, 52.8)`,
/// 496 × 369.6 px: 49.6 px per unit across and 36.96 px up.
pub(crate) fn data_axes() -> (Figure, Axes) {
    let fig = Figure::new();
    let ax = fig
        .add_axes([0.125, 0.11, 0.775, 0.77])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 10.0))
        .expect("finite limits");
    (fig, ax)
}

/// The display position of the data point `(x, y)` of [`data_axes`].
pub(crate) fn data_to_display(x: f64, y: f64) -> Point {
    Point::new(80.0 + x * 49.6, 52.8 + y * 36.96)
}

/// Whether the major ticks and their labels of axis `name` of `ax` are on
/// the (first, second) side: `((ticks…), (labels…))`.
pub(crate) fn major_tick_sides(ax: &Axes, name: AxisName) -> ((bool, bool), (bool, bool)) {
    ax.raw().with::<AxesState, _>(|state| {
        let style = &state.axis(name).major_style;
        (
            (style.ticks.first, style.ticks.second),
            (style.labels.first, style.labels.second),
        )
    })
}

/// Ticks at `locs`, each labelled with its value.
pub(crate) fn ticks_at(locs: &[f64]) -> Vec<Tick> {
    locs.iter()
        .map(|&loc| Tick {
            loc,
            label: format!("{loc}"),
        })
        .collect()
}

/// The listed colormap `red, green, blue`.
pub(crate) fn rgb_colormap() -> Colormap {
    Colormap::listed("rgb", vec!["red", "green", "blue"]).expect("valid colours")
}

/// A `rows × cols` field counting up from 0 row by row.
pub(crate) fn counting_field(rows: usize, cols: usize) -> ScalarField {
    ScalarField::new(Grid::from_fn(rows, cols, |r, c| (r * cols + c) as f64))
}

/// A valid date and time.
pub(crate) fn datetime(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32, us: u32) -> DateTime {
    DateTime::new(y, mo, d, h, mi, s, us).expect("a valid date and time")
}

/// `coords` as points.
pub(crate) fn points(coords: &[(f64, f64)]) -> Vec<Point> {
    coords.iter().map(|&(x, y)| Point::new(x, y)).collect()
}

/// The vertices of `path` as `(x, y)` pairs.
pub(crate) fn vertex_pairs(path: &Path) -> Vec<(f64, f64)> {
    path.vertices().iter().map(|p| (p.x, p.y)).collect()
}

/// The horizontal segment from `(0, 0)` to `(100, 0)`.
pub(crate) fn horizontal_segment() -> Path {
    Path::polyline(vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)])
}

/// One recorded renderer call.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Call {
    /// `draw_path` with the path's vertices already transformed to display space.
    Path {
        /// Transformed vertices.
        points: Vec<Point>,
        /// Fill colour.
        fill: Option<Rgba>,
        /// Stroke width in points.
        linewidth: f64,
    },
    /// `draw_image` origin and size.
    Image {
        /// Bottom-left corner.
        origin: Point,
        /// Width and height in pixels.
        size: (usize, usize),
    },
    /// `draw_gouraud_triangles` triangle count.
    Gouraud(usize),
}

/// A renderer that records every call, for asserting on draw output.
#[derive(Debug)]
pub(crate) struct RecordingRenderer {
    pub(crate) size: (f64, f64),
    pub(crate) dpi: f64,
    pub(crate) calls: Vec<Call>,
}

impl RecordingRenderer {
    pub(crate) fn new(width: f64, height: f64, dpi: f64) -> Self {
        Self {
            size: (width, height),
            dpi,
            calls: Vec::new(),
        }
    }
}

impl Renderer for RecordingRenderer {
    fn canvas_size(&self) -> (f64, f64) {
        self.size
    }

    fn dpi(&self) -> f64 {
        self.dpi
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.calls.push(Call::Path {
            points: path.transformed(transform).vertices().to_vec(),
            fill,
            linewidth: gc.linewidth,
        });
    }

    fn draw_gouraud_triangles(
        &mut self,
        _gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        _colors: &[[Rgba; 3]],
        _transform: &Affine2D,
    ) {
        self.calls.push(Call::Gouraud(triangles.len()));
    }

    fn draw_image(
        &mut self,
        _gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        _transform: Option<&Affine2D>,
    ) {
        self.calls.push(Call::Image {
            origin,
            size: (image.width(), image.height()),
        });
    }
}
