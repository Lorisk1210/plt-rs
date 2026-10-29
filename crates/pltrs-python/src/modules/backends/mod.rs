//! `_native.backends`: what the canvases of matplotlib's `backends`
//! package do with a figure: drawing it onto the raster canvas it keeps
//! (`FigureCanvasAgg`, with blitting), printing it in a file format
//! (`print_png`, `print_pdf`, ...) and collecting pages (`PdfPages`).

mod canvas;
mod pdf_pages;
mod print;

use pltrs::figure::CanvasError;

use crate::binding::prelude::*;

impl From<CanvasError> for PyErr {
    /// matplotlib's `ValueError` for boxes and pixel ratios it refuses,
    /// `RuntimeError` for what the canvas cannot do now.
    fn from(err: CanvasError) -> Self {
        let kind = match err {
            CanvasError::InvalidBox(_) | CanvasError::InvalidPixelRatio(_) => {
                ExceptionKind::ValueError
            }
            _ => ExceptionKind::RuntimeError,
        };
        PyErr::new(kind, err.to_string())
    }
}

/// matplotlib's browser client script (its `web_backend/js/mpl.js`),
/// which the `WebAgg` backends serve.
fn client_script() -> PyResult<&'static str> {
    Ok(pltrs::web::client_script())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("client_script", client_script)?;
    canvas::register(module)?;
    print::register(module)?;
    pdf_pages::register(module)
}
