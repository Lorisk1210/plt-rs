//! The native side of pltrs's modules: one file per matplotlib module,
//! each filling the submodule of `_native` its Python module calls
//! (`pltrs/lines.py` calls `_native.lines`).
//!
//! A file holds its module's native functions, the conversions of the
//! pltrs (Rust) types it owns ([`artist_handle!`] and [`native_value!`] for
//! handles and values, `FromPy` and `IntoPy` for the rest) and the `From`
//! impls turning its pltrs (Rust) errors into the Python exceptions matplotlib
//! raises. Adding a matplotlib module means adding a file here and one
//! line to [`register`].
//!
//! [`artist_handle!`]: crate::binding::artist_handle
//! [`native_value!`]: crate::binding::native_value

#![allow(
    clippy::needless_pass_by_value,
    clippy::unnecessary_wraps,
    reason = "native functions take their converted arguments by value and return \
              `PyResult`, the form `Module::add` registers"
)]

mod artist;
mod axes;
mod axes_base;
mod axis;
mod axisartist;
mod backends;
mod bezier;
mod c_internal_utils;
mod cbook;
mod cm;
mod collections;
mod colorbar;
mod colors;
mod contour;
mod dates;
mod dviread;
mod figure;
mod font_manager;
mod ft2font;
mod gridspec;
mod image;
mod legend;
mod lines;
mod markers;
mod mathtext;
mod mlab;
mod offsetbox;
mod patch_styles;
mod patches;
mod path;
mod patheffects;
mod plotting;
mod rcsetup;
mod scale;
mod spines;
mod testing;
mod texmanager;
mod text;
mod textpath;
mod ticker;
mod transforms;
mod tri;
mod warnings;

use crate::binding::{Module, PyResult};

/// Fills the extension module: the current thread's parameters start from
/// the look in force (pltrs' modern look unless `PLTRS_LOOK` or
/// `PLTRS_FIGURE_LOOK` chooses another, as in pltrs (Rust)), then each
/// matplotlib module gets its submodule.
pub(crate) fn register(module: &Module<'_>) -> PyResult<()> {
    pltrs::rc::restore_startup();
    artist::register(&module.submodule("artist")?)?;
    axes::register(&module.submodule("axes")?)?;
    axes_base::register(&module.submodule("axes_base")?)?;
    axis::register(&module.submodule("axis")?)?;
    axisartist::register(&module.submodule("axisartist")?)?;
    backends::register(&module.submodule("backends")?)?;
    bezier::register(&module.submodule("bezier")?)?;
    c_internal_utils::register(&module.submodule("c_internal_utils")?)?;
    cbook::register(&module.submodule("cbook")?)?;
    cm::register(&module.submodule("cm")?)?;
    collections::register(&module.submodule("collections")?)?;
    colorbar::register(&module.submodule("colorbar")?)?;
    colors::register(&module.submodule("colors")?)?;
    contour::register(&module.submodule("contour")?)?;
    dates::register(&module.submodule("dates")?)?;
    dviread::register(&module.submodule("dviread")?)?;
    figure::register(&module.submodule("figure")?)?;
    font_manager::register(&module.submodule("font_manager")?)?;
    ft2font::register(&module.submodule("ft2font")?)?;
    gridspec::register(&module.submodule("gridspec")?)?;
    image::register(&module.submodule("image")?)?;
    legend::register(&module.submodule("legend")?)?;
    lines::register(&module.submodule("lines")?)?;
    markers::register(&module.submodule("markers")?)?;
    mathtext::register(&module.submodule("mathtext")?)?;
    mlab::register(&module.submodule("mlab")?)?;
    offsetbox::register(&module.submodule("offsetbox")?)?;
    patch_styles::register(&module.submodule("patch_styles")?)?;
    patches::register(&module.submodule("patches")?)?;
    path::register(&module.submodule("path")?)?;
    plotting::register(&module.submodule("plotting")?)?;
    patheffects::register(&module.submodule("patheffects")?)?;
    rcsetup::register(&module.submodule("rcsetup")?)?;
    scale::register(&module.submodule("scale")?)?;
    spines::register(&module.submodule("spines")?)?;
    testing::register(&module.submodule("testing")?)?;
    texmanager::register(&module.submodule("texmanager")?)?;
    text::register(&module.submodule("text")?)?;
    textpath::register(&module.submodule("textpath")?)?;
    ticker::register(&module.submodule("ticker")?)?;
    transforms::register(&module.submodule("transforms")?)?;
    tri::register(&module.submodule("tri")?)?;
    warnings::register(&module.submodule("warnings")?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::binding::testing::{eval, extension, with_python};
    use crate::binding::{Bound, IntoPy, PyResult};

    /// Calls `_native.<module>.<name>(*args)`.
    fn call<'py>(
        native: &Bound<'py>,
        module: &str,
        name: &str,
        args: &[Bound<'py>],
    ) -> PyResult<Bound<'py>> {
        native.getattr(module)?.getattr(name)?.call(args, &[])
    }

    #[test]
    fn every_matplotlib_module_has_its_submodule() {
        with_python(|py| {
            let native = extension(py);
            for module in [
                "artist",
                "axes",
                "axisartist",
                "backends",
                "bezier",
                "c_internal_utils",
                "cbook",
                "colors",
                "dates",
                "dviread",
                "figure",
                "font_manager",
                "lines",
                "markers",
                "mlab",
                "path",
                "patheffects",
                "rcsetup",
                "scale",
                "testing",
                "ticker",
                "transforms",
            ] {
                assert!(native.getattr(module).is_ok(), "_native.{module}");
            }
            assert!(native.getattr("Native").is_ok() && native.getattr("register").is_ok());
        });
    }

    #[test]
    fn a_figure_is_made_plotted_and_saved_through_the_native_modules() {
        with_python(|py| {
            let native = extension(py);
            let number = |v: f64| v.into_py(py).unwrap();
            let figure = call(
                &native,
                "figure",
                "new",
                &[number(4.0), number(3.0), number(50.0)],
            )
            .unwrap();
            let axes = call(&native, "figure", "gca", std::slice::from_ref(&figure)).unwrap();
            let x = eval(py, "", "[0.0, 1.0, 2.0]");
            let kwargs = eval(py, "", "{'linewidth': 2.0, 'label': 'line'}");
            let line = call(
                &native,
                "plotting",
                "plot",
                &[axes, x.clone(), x, Bound::none(py), kwargs],
            )
            .unwrap();
            let name = "linewidth".into_py(py).unwrap();
            let width = call(&native, "lines", "get", &[line.clone(), name]).unwrap();
            assert_eq!(width.extract::<f64>().unwrap(), 2.0);
            let id = call(&native, "artist", "id", &[line])
                .unwrap()
                .extract::<u32>()
                .unwrap();
            assert!(id > 0, "the figure itself is artist 0");
            let png = "png".into_py(py).unwrap();
            let args = [figure, png, eval(py, "", "{'metadata': {'Title': 'line'}}")];
            let bytes = call(&native, "backends", "print_figure", &args).unwrap();
            assert!(bytes.repr().unwrap().starts_with("b'\\x89PNG"));
        });
    }
}
