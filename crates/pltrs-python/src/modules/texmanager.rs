//! `_native.texmanager`: matplotlib's `TexManager`, by pltrs-text's TeX
//! typesetter (`pltrs_text::tex::TexManager`), configured from the current
//! rcParams (`font.*`, `text.latex.preamble`, `text.latex.engine`) as
//! matplotlib's reads them at each call.

use pltrs_text::tex::TexManager;

use crate::binding::prelude::*;

/// The typesetter of the current rcParams.
fn manager() -> PyResult<TexManager> {
    Ok(pltrs::text::tex_manager(&pltrs::rc::params())?)
}

/// `_get_tex_source(tex, fontsize)`: the LaTeX document typesetting `tex`.
fn tex_source(tex: String, fontsize: f64) -> PyResult<String> {
    Ok(manager()?.source(&tex, fontsize))
}

/// `get_font_preamble()`: the font packages and their configuration.
fn font_preamble() -> PyResult<String> {
    Ok(manager()?.fonts().preamble_and_command().0)
}

/// `_get_base_path(tex, fontsize, dpi)`: the cache path of a document,
/// without extension.
fn base_path(tex: String, fontsize: f64, dpi: Option<f64>) -> PyResult<String> {
    Ok(manager()?
        .base_path(&tex, fontsize, dpi)?
        .display()
        .to_string())
}

/// `make_tex(tex, fontsize)`: writes the document; returns its path.
fn make_tex(tex: String, fontsize: f64) -> PyResult<String> {
    Ok(manager()?.make_tex(&tex, fontsize)?.display().to_string())
}

/// `make_dvi(tex, fontsize)`: typesets the document; returns the DVI
/// file's path.
fn make_dvi(tex: String, fontsize: f64) -> PyResult<String> {
    Ok(manager()?.make_dvi(&tex, fontsize)?.display().to_string())
}

/// `make_png(tex, fontsize, dpi)`: renders the document with `dvipng`;
/// returns the image's path.
fn make_png(tex: String, fontsize: f64, dpi: f64) -> PyResult<String> {
    Ok(manager()?
        .make_png(&tex, fontsize, dpi)?
        .display()
        .to_string())
}

/// `get_grey(tex, fontsize, dpi)`: the coverage of the rendered document,
/// rows from the top, as bytes (`0..=255`).
fn grey(tex: String, fontsize: f64, dpi: f64) -> PyResult<NdArray<u8>> {
    let image = manager()?.grey(&tex, fontsize, dpi)?;
    NdArray::with_shape(
        image.data.clone(),
        vec![image.height as usize, image.width as usize],
    )
}

/// `get_text_width_height_descent(tex, fontsize)`: the extent of the
/// typeset text in pixels of `pixels_per_point` pixels a point.
fn text_width_height_descent(
    tex: String,
    fontsize: f64,
    pixels_per_point: f64,
) -> PyResult<(f64, f64, f64)> {
    Ok(manager()?.text_width_height_descent(&tex, fontsize, pixels_per_point)?)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("tex_source", tex_source)?;
    module.add("font_preamble", font_preamble)?;
    module.add("base_path", base_path)?;
    module.add("make_tex", make_tex)?;
    module.add("make_dvi", make_dvi)?;
    module.add("make_png", make_png)?;
    module.add("grey", grey)?;
    module.add("text_width_height_descent", text_width_height_descent)
}
