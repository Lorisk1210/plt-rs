//! Renderers of the program's (matplotlib's `RendererBase` subclasses
//! pltrs does not define): pltrs (Rust) draws an artist, or a whole
//! figure, through their methods, as matplotlib's artists call them
//! (`artist.draw(renderer)`, `figure.draw(renderer)`).
//!
//! Every drawing call goes through the package's
//! `forward_drawing.<method>(renderer, ...)`, which turns pltrs (Rust)'s
//! arguments into matplotlib's (a graphics context from
//! `renderer.new_gc()`, `Path`, `Affine2D`) and calls the renderer's
//! method; the renderer's own methods answer what pltrs asks (its canvas
//! size, `points_to_pixels`, its options). Text reaches the renderer as
//! one path per line, through `draw_path`, as `RendererBase.draw_text`
//! draws it.

use pltrs::artist::{ArtistHandle, RawHandle, Renderer};
use pltrs::figure::Figure;
use pltrs_core::{Affine2D, Path, Point, Rgba};
use pltrs_render::{GraphicsContext, PathCollection, QuadMesh, RgbaImage, draw_text_as_path};
use pltrs_text::TextRun;

use super::super::backends::PyGc;
use super::super::path::path_arrays;
use super::super::transforms::{matrix, points_array};
use crate::binding::prelude::*;
use crate::binding::{lookup, report};

/// A renderer of the program's.
struct PythonRenderer {
    object: Owned,
}

/// `color` as an `(r, g, b, a)` tuple.
fn rgba(color: Rgba) -> (f64, f64, f64, f64) {
    (color.r, color.g, color.b, color.a)
}

/// `colors` as an `(N, 4)` array.
fn rgba_array(colors: &[Rgba]) -> PyResult<NdArray> {
    let data = colors.iter().flat_map(|c| [c.r, c.g, c.b, c.a]).collect();
    NdArray::with_shape(data, vec![colors.len(), 4])
}

/// The matrices of `transforms` as an `(N, 3, 3)` array.
fn matrices(transforms: &[Affine2D]) -> PyResult<NdArray> {
    let data = transforms
        .iter()
        .flat_map(|t| t.matrix().concat())
        .collect();
    NdArray::with_shape(data, vec![transforms.len(), 3, 3])
}

/// `image` as an `(H, W, 4)` array whose first row is drawn at the
/// bottom, as `draw_image` takes it.
fn image_array(image: &RgbaImage) -> PyResult<NdArray<u8>> {
    let (width, height) = (image.width(), image.height());
    let rows = image
        .data()
        .chunks(width * 4)
        .rev()
        .flatten()
        .copied()
        .collect();
    NdArray::with_shape(rows, vec![height, width, 4])
}

impl PythonRenderer {
    /// Runs `call` with the renderer; what it gives, or `None` when it
    /// raised (its exception raises from the native call drawing).
    fn call<R>(&self, call: impl FnOnce(&Bound<'_>) -> PyResult<R>) -> Option<R> {
        Python::with_gil(|py| match call(&self.object.bind(py)) {
            Ok(value) => Some(value),
            Err(err) => {
                report(err);
                None
            }
        })
    }

    /// Calls `forward_drawing.<method>(renderer, *args)`, `args` made with
    /// the GIL held.
    fn forward(&self, method: &str, args: impl FnOnce(Python<'_>) -> PyResult<Vec<Bound<'_>>>) {
        self.call(|renderer| {
            let py = renderer.py();
            let mut all = vec![renderer.clone()];
            all.extend(args(py)?);
            lookup(py, "forward_drawing")?
                .getattr(method)?
                .call(&all, &[])
                .map(drop)
        });
    }

    /// The graphics context `gc` for Python.
    fn gc<'py>(py: Python<'py>, gc: &GraphicsContext) -> PyResult<Bound<'py>> {
        PyGc(gc.clone()).into_py(py)
    }

    /// `path` as Python's `(vertices, codes)`.
    fn path<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py>> {
        path_arrays(path, false)?.into_py(py)
    }

    /// The 3×3 matrix of `transform`.
    fn affine<'py>(py: Python<'py>, transform: &Affine2D) -> PyResult<Bound<'py>> {
        matrix(*transform)?.into_py(py)
    }
}

impl Renderer for PythonRenderer {
    fn canvas_size(&self) -> (f64, f64) {
        self.call(|r| r.call_method("get_canvas_width_height", &[])?.extract())
            .unwrap_or((1.0, 1.0))
    }

    fn dpi(&self) -> f64 {
        self.points_to_pixels(72.0)
    }

    fn points_to_pixels(&self, points: f64) -> f64 {
        self.call(|r| {
            r.call_method("points_to_pixels", &[points.into_py(r.py())?])?
                .extract()
        })
        .unwrap_or(points)
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.forward("draw_path", |py| {
            Ok(vec![
                Self::gc(py, gc)?,
                Self::path(py, path)?,
                Self::affine(py, transform)?,
                fill.map(rgba).into_py(py)?,
            ])
        });
    }

    fn draw_markers(
        &mut self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: &Affine2D,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.forward("draw_markers", |py| {
            Ok(vec![
                Self::gc(py, gc)?,
                Self::path(py, marker)?,
                Self::affine(py, marker_transform)?,
                Self::path(py, path)?,
                Self::affine(py, transform)?,
                fill.map(rgba).into_py(py)?,
            ])
        });
    }

    fn draw_path_collection(&mut self, gc: &GraphicsContext, collection: &PathCollection<'_>) {
        self.forward("draw_path_collection", |py| {
            let paths = collection
                .paths
                .iter()
                .map(|path| path_arrays(path, false))
                .collect::<PyResult<Vec<_>>>()?;
            let dashes: Vec<(f64, Option<Vec<f64>>)> = collection
                .dashes
                .iter()
                .map(|dashes| (dashes.offset, dashes.pattern.clone()))
                .collect();
            Ok(vec![
                Self::gc(py, gc)?,
                Self::affine(py, &collection.master_transform)?,
                paths.into_py(py)?,
                matrices(collection.path_transforms)?.into_py(py)?,
                points_array(collection.offsets, vec![collection.offsets.len(), 2])?.into_py(py)?,
                (
                    rgba_array(collection.facecolors)?,
                    rgba_array(collection.edgecolors)?,
                    rgba_array(collection.hatchcolors)?,
                )
                    .into_py(py)?,
                (
                    collection.linewidths.to_vec(),
                    dashes,
                    collection.antialiased.to_vec(),
                    collection.urls.to_vec(),
                )
                    .into_py(py)?,
            ])
        });
    }

    fn draw_quad_mesh(&mut self, gc: &GraphicsContext, mesh: &QuadMesh<'_>) {
        self.forward("draw_quad_mesh", |py| {
            let columns = mesh.columns + 1;
            let rows = mesh.coordinates.len() / columns;
            let coordinates = points_array(mesh.coordinates, vec![rows, columns, 2])?;
            Ok(vec![
                Self::gc(py, gc)?,
                Self::affine(py, &mesh.master_transform)?,
                coordinates.into_py(py)?,
                rgba_array(mesh.facecolors)?.into_py(py)?,
                mesh.antialiased.into_py(py)?,
                rgba_array(mesh.edgecolors)?.into_py(py)?,
            ])
        });
    }

    fn draw_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: &Affine2D,
    ) {
        self.forward("draw_gouraud_triangles", |py| {
            let corners: Vec<Point> = triangles.iter().flatten().copied().collect();
            let shades: Vec<Rgba> = colors.iter().flatten().copied().collect();
            let mut shades = rgba_array(&shades)?;
            shades.shape = vec![colors.len(), 3, 4];
            Ok(vec![
                Self::gc(py, gc)?,
                points_array(&corners, vec![triangles.len(), 3, 2])?.into_py(py)?,
                shades.into_py(py)?,
                Self::affine(py, transform)?,
            ])
        });
    }

    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    ) {
        self.forward("draw_image", |py| {
            Ok(vec![
                Self::gc(py, gc)?,
                (origin.x, origin.y).into_py(py)?,
                image_array(image)?.into_py(py)?,
                transform
                    .map(|transform| matrix(*transform))
                    .transpose()?
                    .into_py(py)?,
            ])
        });
    }

    /// Text reaches the renderer as one path per line, as matplotlib's
    /// `RendererBase.draw_text` draws it (`_draw_text_as_path`).
    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        draw_text_as_path(self, gc, run, origin, angle);
    }

    fn open_group(&mut self, name: &str, gid: Option<&str>) {
        self.call(|r| {
            let py = r.py();
            r.call_method("open_group", &[name.into_py(py)?, gid.into_py(py)?])
                .map(drop)
        });
    }

    fn close_group(&mut self, name: &str) {
        self.call(|r| {
            r.call_method("close_group", &[name.into_py(r.py())?])
                .map(drop)
        });
    }

    fn start_rasterizing(&mut self) {
        self.call(|r| r.call_method("start_rasterizing", &[]).map(drop));
    }

    fn stop_rasterizing(&mut self) {
        self.call(|r| r.call_method("stop_rasterizing", &[]).map(drop));
    }

    /// The resolution a `MixedModeRenderer` rasterizes at while it does.
    fn raster_dpi(&self) -> Option<f64> {
        self.call(|r| match r.getattr("_raster_dpi") {
            Ok(method) => method.call(&[], &[])?.extract(),
            Err(_) => Ok(None),
        })
        .flatten()
    }

    fn option_image_nocomposite(&self) -> bool {
        self.call(|r| r.call_method("option_image_nocomposite", &[])?.is_truthy())
            .unwrap_or(false)
    }

    fn option_scale_image(&self) -> bool {
        self.call(|r| r.call_method("option_scale_image", &[])?.is_truthy())
            .unwrap_or(false)
    }

    fn image_magnification(&self) -> f64 {
        self.call(|r| r.call_method("get_image_magnification", &[])?.extract())
            .unwrap_or(1.0)
    }
}

/// Draws `artist` through the program's renderer `renderer` (matplotlib's
/// `artist.draw(renderer)`): as pltrs draws it, its painter left out; a
/// root figure is laid out and drawn whole (`figure.draw(renderer)`).
fn draw_through(renderer: Owned, artist: RawHandle) -> PyResult<()> {
    let mut python = PythonRenderer { object: renderer };
    let figure = artist.figure();
    if figure.id() == artist.id() {
        return Ok(Figure::draw(&figure, &mut python)?);
    }
    Ok(artist.draw_unpainted_onto(&mut python)?)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("draw_through", draw_through)
}
