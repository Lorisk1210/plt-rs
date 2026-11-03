//! `_native.backends`: what the canvases of matplotlib's `backends`
//! package do with a figure: drawing it onto the raster canvas it keeps
//! (`FigureCanvasAgg`, with blitting), printing it in a file format
//! (`print_png`, `print_pdf`, ...) and collecting pages (`PdfPages`).
//! `ft2font` subsets the fonts the documents embed (`_backend_pdf_ps`).

mod agg_text;
mod canvas;
mod documents;
mod gc;
mod pages;
mod pdf_file;
mod pdf_pages;
mod print;

pub(super) use gc::PyGc;
pub(super) use print::DocumentMetadata;

use pltrs::figure::{CanvasError, Figure};

use crate::binding::prelude::*;

impl From<CanvasError> for PyErr {
    /// matplotlib's `ValueError` for boxes and pixel ratios it refuses,
    /// `RuntimeError` for what the canvas cannot do now, and the problems
    /// drawing met as `DrawErrors` raise them.
    fn from(err: CanvasError) -> Self {
        let kind = match err {
            CanvasError::Draw(errors) => return errors.into(),
            CanvasError::Text(err) => return err.into(),
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

/// Whether the figure keeps images and rasterized artists apart instead
/// of compositing them (`Figure.suppressComposite`; `None`: as the
/// renderer prefers).
fn get_suppress_composite(figure: Figure) -> PyResult<Option<bool>> {
    Ok(figure.suppress_composite())
}

/// Sets `Figure.suppressComposite`.
fn set_suppress_composite(figure: Figure, suppress: Option<bool>) -> PyResult<()> {
    figure.set_suppress_composite(suppress);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("client_script", client_script)?;
    module.add("get_suppress_composite", get_suppress_composite)?;
    module.add("set_suppress_composite", set_suppress_composite)?;
    canvas::register(module)?;
    agg_text::register(module)?;
    print::register(module)?;
    documents::register(module)?;
    pdf_pages::register(module)?;
    pages::register(module)?;
    pdf_file::register(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compositing_is_the_renderers_choice_until_set() {
        let fig = Figure::new();
        assert_eq!(get_suppress_composite(fig.clone()).unwrap(), None);
        set_suppress_composite(fig.clone(), Some(true)).unwrap();
        assert_eq!(get_suppress_composite(fig).unwrap(), Some(true));
    }
}
