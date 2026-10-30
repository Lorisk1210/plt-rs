//! `_native.cm`: the built-in colormaps (matplotlib's `cm`, with the
//! data of its `_cm`, `_cm_listed`, `_cm_multivar` and `_cm_bivar`).
//!
//! pltrs (Rust) holds the definitions of matplotlib's colormaps; the
//! Python registries rebuild each as matplotlib's own class from its
//! *spec*: a dict with the colormap's `name`, its `N`, and either its
//! `colors` (a `ListedColormap`) or its `segmentdata` (a
//! `LinearSegmentedColormap`), whose channels are anchor rows `(x, y0,
//! y1)` or `None` for a channel defined by a function, which
//! [`builtin_segment`] evaluates.

use std::collections::BTreeMap;

use pltrs::colors::multivar::{
    BivarDefinition, BivarShape, CombinationMode, bivar_colormap_definition, bivar_colormap_names,
    multivar_colormap, multivar_colormap_names,
};
use pltrs::colors::{Channel, Colormap, ColormapError, Rgba, Segment, colormaps};

use crate::binding::prelude::*;

/// The spec of a pltrs (Rust) colormap, from which Python rebuilds it.
struct ColormapSpec(Colormap);

/// The channels of segment data, by matplotlib's names.
const CHANNELS: [Channel; 4] = [Channel::Red, Channel::Green, Channel::Blue, Channel::Alpha];

impl IntoPy for ColormapSpec {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let cmap = self.0;
        let mut spec = BTreeMap::new();
        spec.insert("name".to_owned(), cmap.name().into_py(py)?);
        spec.insert("N".to_owned(), cmap.n().into_py(py)?);
        match cmap.segment_data() {
            None => {
                spec.insert("colors".to_owned(), colors_array(cmap.lut())?.into_py(py)?);
            }
            Some(data) => {
                let channels: BTreeMap<String, Option<NdArray>> = CHANNELS
                    .into_iter()
                    .filter_map(|channel| {
                        let anchors = match data.channel(channel)? {
                            Segment::Anchors(rows) => Some(rows.concat()),
                            Segment::Func(_) => None,
                        };
                        Some((channel, anchors))
                    })
                    .map(|(channel, anchors)| {
                        let rows = anchors
                            .map(|values| {
                                let count = values.len() / 3;
                                NdArray::with_shape(values, vec![count, 3])
                            })
                            .transpose()?;
                        Ok((channel.name().to_owned(), rows))
                    })
                    .collect::<PyResult<_>>()?;
                spec.insert("segmentdata".to_owned(), channels.into_py(py)?);
            }
        }
        spec.into_py(py)
    }
}

/// The colormap of a mapping pltrs (Rust) made (a contour set's levels,
/// a collection's values), for Python's colorizer of it: `(spec, under,
/// over, bad)`, the spec as [`ColormapSpec`] gives it and the extreme
/// colours as RGBA tuples.
pub(super) struct NativeColormap(pub(super) Colormap);

impl IntoPy for NativeColormap {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let rgba = |c: Rgba| (c.r, c.g, c.b, c.a);
        let (under, over, bad) = (
            rgba(self.0.under()),
            rgba(self.0.over()),
            rgba(self.0.bad()),
        );
        (ColormapSpec(self.0), under, over, bad).into_py(py)
    }
}

/// The colours as rows: RGB when all are opaque (as matplotlib lists its
/// built-in colours), RGBA otherwise.
fn colors_array(colors: &[Rgba]) -> PyResult<NdArray> {
    let opaque = colors.iter().all(|color| color.a >= 1.0);
    let width = if opaque { 3 } else { 4 };
    let values = colors
        .iter()
        .flat_map(|color| [color.r, color.g, color.b, color.a].into_iter().take(width))
        .collect();
    NdArray::with_shape(values, vec![colors.len(), width])
}

/// The names of the built-in colormaps and their aliases, in matplotlib's
/// order, without their reversed `_r` forms (the Python registry makes
/// those, as matplotlib does).
fn colormap_names() -> PyResult<Vec<String>> {
    let names = colormaps::names();
    Ok(names[..names.len() / 2].to_vec())
}

/// The built-in colormap `name`.
fn builtin(name: &str) -> PyResult<Colormap> {
    colormaps::get(name).map_err(|err| match err {
        ColormapError::Unknown(name) => {
            PyErr::key_error(format!("{name:?} is not a known colormap name"))
        }
        other => other.into(),
    })
}

/// The spec of the built-in colormap `name`.
fn builtin_colormap(name: String) -> PyResult<ColormapSpec> {
    builtin(&name).map(ColormapSpec)
}

/// The values at `x` of the channel `channel` (`"red"`, ...) of the
/// built-in colormap `name`, which a function defines.
fn builtin_segment(name: String, channel: String, x: NdArray) -> PyResult<NdArray> {
    let cmap = builtin(&name)?;
    let segment = CHANNELS
        .into_iter()
        .find(|candidate| candidate.name() == channel)
        .and_then(|channel| cmap.segment_data()?.channel(channel));
    let Some(Segment::Func(function)) = segment else {
        return Err(PyErr::value_error(format!(
            "the {channel} channel of {name} is not a function"
        )));
    };
    let values = x.data.iter().map(|&x| function.eval(x)).collect();
    NdArray::with_shape(values, x.shape)
}

/// The names of the multivariate colormaps.
fn multivar_names() -> PyResult<Vec<&'static str>> {
    Ok(multivar_colormap_names().to_vec())
}

/// The multivariate colormap `name`: its combination mode and the specs
/// of its colormaps.
fn multivar(name: String) -> PyResult<(&'static str, Vec<ColormapSpec>)> {
    let cmap = multivar_colormap(&name)?;
    let mode = match cmap.combination_mode() {
        CombinationMode::Add => "sRGB_add",
        CombinationMode::Sub => "sRGB_sub",
    };
    let specs = cmap.colormaps().iter().cloned().map(ColormapSpec).collect();
    Ok((mode, specs))
}

/// The names of the bivariate colormaps.
fn bivar_names() -> PyResult<Vec<&'static str>> {
    Ok(bivar_colormap_names().to_vec())
}

/// matplotlib's name of a bivariate shape.
fn shape_name(shape: BivarShape) -> &'static str {
    match shape {
        BivarShape::Square => "square",
        BivarShape::Circle => "circle",
        BivarShape::Ignore => "ignore",
        BivarShape::CircleIgnore => "circleignore",
    }
}

/// The bivariate colormap `name`: its RGB patch (`(M, M, 3)`), shape and
/// origin, the arguments of matplotlib's `SegmentedBivarColormap`.
fn bivar(name: String) -> PyResult<(NdArray, &'static str, (f64, f64))> {
    let BivarDefinition {
        patch,
        shape,
        origin,
    } = bivar_colormap_definition(&name)?;
    let (rows, cols) = patch.shape();
    let values = patch.into_vec().concat();
    Ok((
        NdArray::with_shape(values, vec![rows, cols, 3])?,
        shape_name(shape),
        origin,
    ))
}

/// Registers `cmap` under `name` with pltrs (Rust) too, replacing what was
/// registered there (`ColormapRegistry.register`): the `image.cmap`
/// rcParam may then name it.
fn register_colormap(name: String, cmap: Colormap) -> PyResult<()> {
    colormaps::unregister(&name)?;
    colormaps::register(cmap.with_name(name))?;
    Ok(())
}

/// Removes the colormap registered under `name` from pltrs (Rust)'s
/// registry (`ColormapRegistry.unregister`).
fn unregister_colormap(name: String) -> PyResult<()> {
    Ok(colormaps::unregister(&name)?)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("register", register_colormap)?;
    module.add("unregister", unregister_colormap)?;
    module.add("colormap_names", colormap_names)?;
    module.add("builtin_colormap", builtin_colormap)?;
    module.add("builtin_segment", builtin_segment)?;
    module.add("multivar_names", multivar_names)?;
    module.add("multivar", multivar)?;
    module.add("bivar_names", bivar_names)?;
    module.add("bivar", bivar)
}
