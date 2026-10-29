//! `_native.textpath`: the outlines of text (matplotlib's `textpath`), by
//! `pltrs::text::text_path` and `text_path_tex`.

use pltrs::text::{text_path, text_path_tex};
use pltrs_core::{PathCode, Point};
use pltrs_text::TEXT_PATH_FONT_SCALE;

use super::font_manager::FontProps;
use crate::binding::prelude::*;

/// `TextToPath.get_text_path(prop, s, ismath)`: the outline of `s` at
/// `TextToPath.FONT_SCALE` points, `(vertices, codes)`, typeset by LaTeX
/// when `usetex` is set.
fn get_text_path(s: String, props: FontProps, usetex: bool) -> PyResult<(NdArray, NdArray<u8>)> {
    let at = Point::new(0.0, 0.0);
    let size = Some(TEXT_PATH_FONT_SCALE);
    let path = if usetex {
        text_path_tex(at, &s, size, &props.0)
    } else {
        text_path(at, &s, size, &props.0)
    }
    .map_err(|err| PyErr::value_error(err.to_string()))?;
    let vertices: Vec<f64> = path.vertices().iter().flat_map(|p| [p.x, p.y]).collect();
    let count = path.vertices().len();
    let codes = path.codes().map_or_else(
        || {
            let mut codes = vec![PathCode::LineTo as u8; count];
            if let Some(first) = codes.first_mut() {
                *first = PathCode::MoveTo as u8;
            }
            codes
        },
        |codes| codes.iter().map(|&code| code as u8).collect(),
    );
    Ok((
        NdArray::with_shape(vertices, vec![count, 2])?,
        NdArray::from_vec(codes),
    ))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_text_path", get_text_path)
}
