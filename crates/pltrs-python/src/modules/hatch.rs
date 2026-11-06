//! `_native.hatch`: matplotlib's hatch pattern classes (`HorizontalHatch`,
//! ..., `Shapes`), each the vertices of one piece of a hatch, made by
//! pltrs-core's `hatch`.
//!
//! A piece goes back as its `(N, 2)` vertices and its `N` codes, which
//! the class's `set_vertices_and_codes` writes into the caller's arrays.

use pltrs_core::hatch::{HatchLines, HatchShapes};
use pltrs_core::{Path, Point};

use super::path::PyPath;
use super::transforms::points_array;
use crate::binding::prelude::*;

/// A piece of a hatch: its vertices and codes.
type Piece = (NdArray, NdArray<u8>);

/// The family of lines named `family` (`"horizontal"`, `"vertical"`,
/// `"north_east"`, `"south_east"`).
fn lines_family(family: &str) -> PyResult<HatchLines> {
    Ok(match family {
        "horizontal" => HatchLines::Horizontal,
        "vertical" => HatchLines::Vertical,
        "north_east" => HatchLines::NorthEast,
        "south_east" => HatchLines::SouthEast,
        other => return Err(PyErr::value_error(format!("no hatch lines {other:?}"))),
    })
}

/// The arrays of `path`, its codes resolved (none for an empty path).
fn piece(path: &Path) -> PyResult<Piece> {
    let vertices: Vec<Point> = path.vertices().to_vec();
    let codes = path.resolved_codes().iter().map(|&c| c as u8).collect();
    let shape = vec![vertices.len(), 2];
    Ok((points_array(&vertices, shape)?, NdArray::from_vec(codes)))
}

/// The number of vertices of `lines` lines of `family` (`num_vertices`).
fn line_vertex_count(family: String, lines: usize) -> PyResult<usize> {
    Ok(lines_family(&family)?.vertex_count(lines))
}

/// `lines` lines of `family` (`set_vertices_and_codes`).
fn line_piece(family: String, lines: usize) -> PyResult<Piece> {
    piece(&lines_family(&family)?.path(lines))
}

/// The rows of a shape: `shape`, scaled by `size`, filled or not.
fn shapes(shape: PyPath, size: f64, filled: bool) -> HatchShapes {
    HatchShapes {
        shape: shape.path,
        size,
        filled,
        character: None,
    }
}

/// `rows` rows of `shape` (`Shapes.set_vertices_and_codes`).
fn shape_piece(shape: PyPath, (size, filled): (f64, bool), rows: usize) -> PyResult<Piece> {
    piece(&shapes(shape, size, filled).path(rows))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("line_vertex_count", line_vertex_count)?;
    module.add("line_piece", line_piece)?;
    module.add("shape_piece", shape_piece)
}
