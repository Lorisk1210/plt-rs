//! The steps the renderers pltrs hands Python add to the drawing they
//! draw into, one function per drawing method of matplotlib's
//! `RendererBase`, and the drawing of a drawing onto a canvas.
//!
//! Python passes the arguments of the methods as pltrs (Rust) takes
//! them: graphics contexts as `_native_gc` describes them, transforms as
//! 3×3 matrices, colours as RGBA arrays, positions in display pixels from
//! the bottom-left corner.

use pltrs::artist::{DrawStep, Drawing, RawHandle};
use pltrs::colors::Color;
use pltrs::figure::{DrawErrors, Figure};
use pltrs::transforms::Point;
use pltrs_core::Rgba;
use pltrs_render::{Dashes, PathCollectionData, QuadMeshData, RasterRenderer, RgbaImage};

use super::super::artist::python_filter;
use super::super::backends::PyGc;
use super::super::font_manager::FontProps;
use super::super::path::PyPath;
use super::super::text_overrides::IsMath;
use super::super::transforms::{affine, points};
use crate::binding::prelude::*;

/// Adds `step` to `drawing`.
fn add(drawing: &Native<Drawing>, step: DrawStep) {
    drawing.borrow_mut().push(step);
}

/// The face colour `rgbFace`, resolved.
fn face(color: Option<Color>) -> PyResult<Option<Rgba>> {
    color
        .map(|color| {
            color
                .resolve()
                .map_err(|err| PyErr::value_error(err.to_string()))
        })
        .transpose()
}

/// The colours of an `(N, 4)` RGBA array (none for an empty one).
fn colors(array: &NdArray) -> PyResult<Vec<Rgba>> {
    if array.data.is_empty() {
        return Ok(Vec::new());
    }
    match array.shape[..] {
        [_, 4] | [4] => Ok(array
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[r, g, b, a]| Rgba::new(r, g, b, a))
            .collect()),
        _ => Err(PyErr::value_error(format!(
            "colours are an (N, 4) RGBA array, not one of shape {:?}",
            array.shape
        ))),
    }
}

/// `draw_path(gc, path, transform, rgbFace)`: `path` mapped into display
/// pixels by the affine `matrix`, stroked as `gc` says, filled with
/// `fill` if given.
fn draw_path(
    drawing: Native<Drawing>,
    gc: PyGc,
    path: PyPath,
    matrix: NdArray,
    fill: Option<Color>,
) -> PyResult<()> {
    let step = DrawStep::Path {
        gc: gc.0,
        path: path.path,
        transform: affine(&matrix)?,
        fill: face(fill)?,
    };
    add(&drawing, step);
    Ok(())
}

/// `draw_markers(gc, marker_path, marker_trans, path, trans, rgbFace)`:
/// the marker (mapped to pixel offsets by `marker_matrix`) at every vertex
/// of `path` (mapped into display pixels by `matrix`).
fn draw_markers(
    drawing: Native<Drawing>,
    gc: PyGc,
    marker: PyPath,
    marker_matrix: NdArray,
    path: PyPath,
    matrix: NdArray,
    fill: Option<Color>,
) -> PyResult<()> {
    let step = DrawStep::Markers {
        gc: gc.0,
        marker: marker.path,
        marker_transform: affine(&marker_matrix)?,
        path: path.path,
        transform: affine(&matrix)?,
        fill: face(fill)?,
    };
    add(&drawing, step);
    Ok(())
}

/// The shapes of a collection and where they go: the master transform's
/// matrix, the paths, their own transforms (an `(N, 3, 3)` array) and the
/// offsets in display pixels (an `(N, 2)` array).
type CollectionShapes = (NdArray, Vec<PyPath>, NdArray, NdArray);

/// The colours of a collection: `(facecolors, edgecolors, hatchcolors)`,
/// `(N, 4)` RGBA arrays.
type CollectionColors = (NdArray, NdArray, NdArray);

/// The strokes of a collection: `(linewidths, linestyles, antialiaseds,
/// urls)`, the line styles as `(offset, dashes)` in points.
type CollectionStrokes = (
    Vec<f64>,
    Vec<(f64, Option<Vec<f64>>)>,
    Vec<bool>,
    Vec<Option<String>>,
);

/// `draw_path_collection(gc, master_transform, paths, all_transforms,
/// offsets, offset_trans, facecolors, edgecolors, linewidths, linestyles,
/// antialiaseds, urls, offset_position, hatchcolors=...)`, the offsets
/// already through `offset_trans`.
fn draw_path_collection(
    drawing: Native<Drawing>,
    gc: PyGc,
    shapes: CollectionShapes,
    colours: CollectionColors,
    strokes: CollectionStrokes,
) -> PyResult<()> {
    let (master, paths, transforms, offsets) = shapes;
    let (facecolors, edgecolors, hatchcolors) = colours;
    let (linewidths, linestyles, antialiased, urls) = strokes;
    let path_transforms = transforms
        .data
        .as_chunks::<9>()
        .0
        .iter()
        .map(|matrix| affine(&NdArray::with_shape(matrix.to_vec(), vec![3, 3])?))
        .collect::<PyResult<_>>()?;
    let collection = PathCollectionData {
        master_transform: affine(&master)?,
        paths: paths.into_iter().map(|path| path.path).collect(),
        path_transforms,
        offsets: if offsets.data.is_empty() {
            Vec::new()
        } else {
            points(&offsets)?
        },
        facecolors: colors(&facecolors)?,
        edgecolors: colors(&edgecolors)?,
        hatchcolors: colors(&hatchcolors)?,
        linewidths,
        dashes: linestyles
            .into_iter()
            .map(|(offset, dashes)| match dashes {
                Some(pattern) => Dashes::new(offset, pattern),
                None => Dashes::solid(),
            })
            .collect(),
        antialiased,
        urls,
    };
    add(
        &drawing,
        DrawStep::PathCollection {
            gc: gc.0,
            collection: Box::new(collection),
        },
    );
    Ok(())
}

/// `draw_quad_mesh(gc, master_transform, meshWidth, meshHeight,
/// coordinates, offsets, offsetTrans, facecolors, antialiased,
/// edgecolors)`: the `(rows + 1, columns + 1, 2)` vertices `coordinates`
/// mapped into display pixels by `master`.
fn draw_quad_mesh(
    drawing: Native<Drawing>,
    gc: PyGc,
    master: NdArray,
    coordinates: NdArray,
    facecolors: NdArray,
    antialiased: bool,
    edgecolors: NdArray,
) -> PyResult<()> {
    let &[_, columns, 2] = &coordinates.shape[..] else {
        return Err(PyErr::value_error(format!(
            "the coordinates of a quad mesh are a (rows + 1, columns + 1, 2) array, not one of \
             shape {:?}",
            coordinates.shape
        )));
    };
    let flat = NdArray::with_shape(coordinates.data, vec![coordinates.shape[0] * columns, 2])?;
    let mesh = QuadMeshData {
        master_transform: affine(&master)?,
        columns: columns.saturating_sub(1),
        coordinates: points(&flat)?,
        facecolors: colors(&facecolors)?,
        edgecolors: colors(&edgecolors)?,
        antialiased,
    };
    let mesh = Box::new(mesh);
    add(&drawing, DrawStep::QuadMesh { gc: gc.0, mesh });
    Ok(())
}

/// `draw_gouraud_triangles(gc, triangles_array, colors_array,
/// transform)`: `(N, 3, 2)` vertices with `(N, 3, 4)` colours.
fn draw_gouraud_triangles(
    drawing: Native<Drawing>,
    gc: PyGc,
    triangles: NdArray,
    vertex_colors: NdArray,
    matrix: NdArray,
) -> PyResult<()> {
    let triangles = triangles
        .data
        .as_chunks::<6>()
        .0
        .iter()
        .map(|&[x0, y0, x1, y1, x2, y2]| {
            [Point::new(x0, y0), Point::new(x1, y1), Point::new(x2, y2)]
        })
        .collect();
    let corners = colors(&NdArray::with_shape(
        vertex_colors.data,
        vec![vertex_colors.shape.iter().product::<usize>() / 4, 4],
    )?)?;
    let step = DrawStep::GouraudTriangles {
        gc: gc.0,
        triangles,
        colors: corners.as_chunks::<3>().0.to_vec(),
        transform: affine(&matrix)?,
    };
    add(&drawing, step);
    Ok(())
}

/// The image of an `(H, W, 4)` RGBA array whose first row is drawn at
/// the bottom, as `draw_image` receives it.
fn image(array: &NdArray<u8>) -> PyResult<RgbaImage> {
    let &[height, width, 4] = &array.shape[..] else {
        return Err(PyErr::value_error(format!(
            "an image to draw is an (H, W, 4) array, not one of shape {:?}",
            array.shape
        )));
    };
    let rows: Vec<u8> = array
        .data
        .chunks(width * 4)
        .rev()
        .flatten()
        .copied()
        .collect();
    RgbaImage::new(width, height, rows).map_err(|err| PyErr::value_error(err.to_string()))
}

/// `draw_image(gc, x, y, im, transform)`: the image with its bottom-left
/// corner at `origin`, or its unit square mapped by `matrix`.
fn draw_image(
    drawing: Native<Drawing>,
    gc: PyGc,
    origin: (f64, f64),
    pixels: NdArray<u8>,
    matrix: Option<NdArray>,
) -> PyResult<()> {
    let step = DrawStep::Image {
        gc: gc.0,
        origin: Point::new(origin.0, origin.1),
        image: image(&pixels)?,
        transform: matrix.as_ref().map(affine).transpose()?,
    };
    add(&drawing, step);
    Ok(())
}

/// A line of text as `draw_text` receives it: `(s, prop, ismath)`.
type Line = (String, FontProps, IsMath);

/// `draw_text(gc, x, y, s, prop, angle, ismath)` and `draw_tex`: the
/// line typeset as its `ismath` says at `dpi`, the left end of its
/// baseline at `origin`, rotated by `angle` degrees.
fn draw_text(
    drawing: Native<Drawing>,
    gc: PyGc,
    origin: (f64, f64),
    (text, props, ismath): Line,
    angle: f64,
    dpi: f64,
) -> PyResult<()> {
    let run = pltrs::text::typeset_line(&text, &props.0, ismath.0, dpi, &pltrs::rc::params())?;
    let step = DrawStep::Text {
        gc: gc.0,
        run: Box::new(run),
        origin: Point::new(origin.0, origin.1),
        angle,
    };
    add(&drawing, step);
    Ok(())
}

/// `open_group(s, gid)`.
fn open_group(drawing: Native<Drawing>, name: String, gid: Option<String>) -> PyResult<()> {
    add(&drawing, DrawStep::OpenGroup { name, gid });
    Ok(())
}

/// `close_group(s)`.
fn close_group(drawing: Native<Drawing>, name: String) -> PyResult<()> {
    add(&drawing, DrawStep::CloseGroup { name });
    Ok(())
}

/// `start_rasterizing()`.
fn start_rasterizing(drawing: Native<Drawing>) -> PyResult<()> {
    add(&drawing, DrawStep::StartRasterizing);
    Ok(())
}

/// `stop_rasterizing()`.
fn stop_rasterizing(drawing: Native<Drawing>) -> PyResult<()> {
    add(&drawing, DrawStep::StopRasterizing);
    Ok(())
}

/// `start_filter()`.
fn start_filter(drawing: Native<Drawing>) -> PyResult<()> {
    add(&drawing, DrawStep::StartFilter);
    Ok(())
}

/// `stop_filter(post_processing)`: what was drawn since `start_filter`
/// goes through the Python function `function`, as an agg filter does.
fn stop_filter(drawing: Native<Drawing>, function: Callback) -> PyResult<()> {
    add(&drawing, DrawStep::StopFilter(python_filter(function)));
    Ok(())
}

/// `artist.draw(renderer)`: the artist as pltrs draws it.
fn draw_artist(drawing: Native<Drawing>, artist: RawHandle) -> PyResult<()> {
    add(&drawing, DrawStep::Artist(artist));
    Ok(())
}

/// Draws `drawing` onto the figure's canvas over what it holds (the
/// drawing methods of the canvas's `RendererAgg` between draws).
fn draw_onto_canvas(figure: Figure, drawing: Native<Drawing>) -> PyResult<()> {
    Ok(figure.canvas().draw_drawing(&drawing.borrow())?)
}

/// Draws `drawing` onto the raster canvas `raster` (the drawing methods
/// of a `RendererAgg` the program made).
fn draw_onto_raster(raster: Native<RasterRenderer>, drawing: Native<Drawing>) -> PyResult<()> {
    let problems = drawing.borrow().draw_onto(&mut *raster.borrow_mut());
    if problems.is_empty() {
        Ok(())
    } else {
        Err(DrawErrors(problems).into())
    }
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("draw_path", draw_path)?;
    module.add("draw_markers", draw_markers)?;
    module.add("draw_path_collection", draw_path_collection)?;
    module.add("draw_quad_mesh", draw_quad_mesh)?;
    module.add("draw_gouraud_triangles", draw_gouraud_triangles)?;
    module.add("draw_image", draw_image)?;
    module.add("draw_text", draw_text)?;
    module.add("open_group", open_group)?;
    module.add("close_group", close_group)?;
    module.add("start_rasterizing", start_rasterizing)?;
    module.add("stop_rasterizing", stop_rasterizing)?;
    module.add("start_filter", start_filter)?;
    module.add("stop_filter", stop_filter)?;
    module.add("draw_artist", draw_artist)?;
    module.add("draw_onto_canvas", draw_onto_canvas)?;
    module.add("draw_onto_raster", draw_onto_raster)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::with_python;

    #[test]
    fn images_turn_their_first_row_to_the_bottom() {
        with_python(|_| {
            let array =
                NdArray::with_shape([[255, 0, 0, 255], [0, 0, 255, 255]].concat(), vec![2, 1, 4])
                    .unwrap();
            let drawn = image(&array).unwrap();
            assert_eq!(drawn.pixel(0, 0), Some([0, 0, 255, 255]), "the top row");
            assert_eq!(drawn.pixel(0, 1), Some([255, 0, 0, 255]));
            let flat = NdArray::with_shape(vec![0; 4], vec![1, 4]).unwrap();
            assert!(image(&flat).is_err(), "not an RGBA image");
        });
    }

    #[test]
    fn colours_are_rgba_rows() {
        let array = NdArray::with_shape(vec![1.0, 0.0, 0.0, 1.0], vec![1, 4]).unwrap();
        assert_eq!(colors(&array).unwrap(), [Rgba::new(1.0, 0.0, 0.0, 1.0)]);
        assert_eq!(colors(&NdArray::from_vec(Vec::new())).unwrap(), Vec::new());
        let three = NdArray::with_shape(vec![0.0; 3], vec![1, 3]).unwrap();
        assert!(colors(&three).is_err());
    }
}
