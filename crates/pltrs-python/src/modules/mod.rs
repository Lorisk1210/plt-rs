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

mod annotation;
mod archive;
mod artist;
mod axes;
mod axes_base;
mod axes_grid1;
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
mod drawing;
mod dviread;
mod figure;
mod figure_layout;
mod font_manager;
mod ft2font;
mod gridspec;
mod gui;
mod hatch;
mod image;
mod insets;
mod layoutgrid;
mod legend;
mod lines;
mod markers;
mod mathtext;
mod mlab;
mod mplot3d;
mod navigation;
mod offsetbox;
mod patch_styles;
mod patches;
mod path;
mod patheffects;
mod plotting;
mod polar;
mod projections;
mod rcsetup;
mod sankey;
mod scale;
mod spines;
mod table;
mod testing;
mod texmanager;
mod text;
mod text_helpers;
mod text_overrides;
mod textpath;
mod ticker;
mod transforms;
mod tri;
mod type1font;
mod warnings;
mod web;

use crate::binding::{Module, PyResult};

/// Fills the extension module: the current thread's parameters start from
/// the look in force (pltrs' modern look unless `PLTRS_LOOK` or
/// `PLTRS_FIGURE_LOOK` chooses another, as in pltrs (Rust)), then each
/// matplotlib module gets its submodule.
pub(crate) fn register(module: &Module<'_>) -> PyResult<()> {
    pltrs::config::honour_matplotlib_config_dir();
    pltrs::rc::restore_startup();
    crate::binding::install_after_call(vec![text_helpers::warn_missing_glyphs]);
    annotation::register(&module.submodule("annotation")?)?;
    archive::register(&module.submodule("archive")?)?;
    artist::register(&module.submodule("artist")?)?;
    axes::register(&module.submodule("axes")?)?;
    axes_base::register(&module.submodule("axes_base")?)?;
    axes_grid1::register(&module.submodule("axes_grid1")?)?;
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
    figure_layout::register(&module.submodule("figure_layout")?)?;
    font_manager::register(&module.submodule("font_manager")?)?;
    ft2font::register(&module.submodule("ft2font")?)?;
    gridspec::register(&module.submodule("gridspec")?)?;
    gui::register(&module.submodule("gui")?)?;
    hatch::register(&module.submodule("hatch")?)?;
    image::register(&module.submodule("image")?)?;
    insets::register(&module.submodule("insets")?)?;
    legend::register(&module.submodule("legend")?)?;
    lines::register(&module.submodule("lines")?)?;
    markers::register(&module.submodule("markers")?)?;
    mathtext::register(&module.submodule("mathtext")?)?;
    mlab::register(&module.submodule("mlab")?)?;
    mplot3d::register(&module.submodule("mplot3d")?)?;
    navigation::register(&module.submodule("navigation")?)?;
    offsetbox::register(&module.submodule("offsetbox")?)?;
    patch_styles::register(&module.submodule("patch_styles")?)?;
    patches::register(&module.submodule("patches")?)?;
    path::register(&module.submodule("path")?)?;
    plotting::register(&module.submodule("plotting")?)?;
    polar::register(&module.submodule("polar")?)?;
    projections::register(&module.submodule("projections")?)?;
    patheffects::register(&module.submodule("patheffects")?)?;
    drawing::register(&module.submodule("drawing")?)?;
    rcsetup::register(&module.submodule("rcsetup")?)?;
    sankey::register(&module.submodule("sankey")?)?;
    scale::register(&module.submodule("scale")?)?;
    spines::register(&module.submodule("spines")?)?;
    table::register(&module.submodule("table")?)?;
    testing::register(&module.submodule("testing")?)?;
    texmanager::register(&module.submodule("texmanager")?)?;
    text::register(&module.submodule("text")?)?;
    text_overrides::register(&module.submodule("text_overrides")?)?;
    textpath::register(&module.submodule("textpath")?)?;
    ticker::register(&module.submodule("ticker")?)?;
    transforms::register(&module.submodule("transforms")?)?;
    tri::register(&module.submodule("tri")?)?;
    warnings::register(&module.submodule("warnings")?)?;
    web::register(&module.submodule("web")?)?;
    type1font::register(&module.submodule("type1font")?)?;
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
            let rect = eval(py, "", "(0.125, 0.11, 0.775, 0.77)");
            let axes = call(&native, "figure", "add_axes", &[figure.clone(), rect]).unwrap();
            let x = eval(py, "", "[0.0, 1.0, 2.0]");
            let kwargs = eval(py, "", "{'linewidth': 2.0, 'label': 'line'}");
            let rescale = eval(py, "", "(True, True)");
            let line = call(
                &native,
                "plotting",
                "plot",
                &[axes, x.clone(), x, Bound::none(py), kwargs, rescale],
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
