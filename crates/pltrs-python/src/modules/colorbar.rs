//! `_native.colorbar`: colorbars (matplotlib's `colorbar`).
//!
//! A colorbar shows the colour mapping of its Python mappable: the
//! mappable's colormap and norm convert to pltrs (Rust)'s (see
//! `colors::mapping`), and the Python `Colorbar` hands them over again
//! when the mappable changes (`update_normal`); a colorbar of an image
//! follows the image pltrs (Rust) draws, its opacity included. pltrs
//! (Rust) lays the colorbar out, in an axes of its own or in the one
//! given, and draws it.

use std::rc::Rc;

use pltrs::axes::{Axes, LabelLoc};
use pltrs::colorbar::{
    Colorbar, ColorbarAxes, ColorbarError, ColorbarFormat, ColorbarOptions, ColorbarTicks,
    ExtendFrac, Mappable, ParentAnchor, Spacing,
};
use pltrs::colors::{Colormap, Extend, Norm, ScalarMappable};
use pltrs::figure::Figure;
use pltrs::image::AxesImage;
use pltrs::orientation::Orientation;
use pltrs::spines::SpineSide;
use pltrs::ticker::{FuncFormatter, LabelFormat};

use super::colors::mapping::extend;
use super::transforms::anchor;
use crate::binding::dict_value;
use crate::binding::prelude::*;

native_value!(Colorbar);

impl From<ColorbarError> for PyErr {
    /// matplotlib's `ValueError` for colorbars it cannot make.
    fn from(err: ColorbarError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// The side a location names.
fn side(name: &str) -> PyResult<SpineSide> {
    Ok(match name {
        "left" => SpineSide::Left,
        "right" => SpineSide::Right,
        "top" => SpineSide::Top,
        "bottom" => SpineSide::Bottom,
        _ => {
            return Err(PyErr::value_error(format!(
                "{name:?} is not a valid value for location; supported values are \
                 'left', 'right', 'top', 'bottom'"
            )));
        }
    })
}

/// The orientation a name gives.
fn orientation(name: &str) -> PyResult<Orientation> {
    name.parse()
        .map_err(|err: pltrs::orientation::OrientationError| PyErr::value_error(err.to_string()))
}

/// matplotlib's `extendfrac`: `None`, `'auto'`, a fraction, or one per end.
fn extendfrac(value: &Bound<'_>) -> PyResult<ExtendFrac> {
    if value.is_none() {
        return Ok(ExtendFrac::Default);
    }
    if let Ok(name) = value.extract::<String>() {
        return match name.as_str() {
            "auto" => Ok(ExtendFrac::Auto),
            _ => Err(PyErr::value_error("invalid value for extendfrac")),
        };
    }
    if let Ok(both) = value.extract::<f64>() {
        return Ok(ExtendFrac::Both(both));
    }
    match value.extract::<(f64, f64)>() {
        Ok((lower, upper)) if !lower.is_nan() && !upper.is_nan() => {
            Ok(ExtendFrac::Each(lower, upper))
        }
        _ => Err(PyErr::value_error("invalid value for extendfrac")),
    }
}

/// matplotlib's `format`: a `%`-style or `str.format` template, or a
/// Python formatter (any callable taking `(x, pos)`).
fn format(value: &Bound<'_>) -> PyResult<Option<ColorbarFormat>> {
    if value.is_none() {
        return Ok(None);
    }
    if let Ok(template) = value.extract::<String>() {
        return Ok(Some(ColorbarFormat::Label(LabelFormat::from(template))));
    }
    let formatter: Callback = value.extract()?;
    Ok(Some(ColorbarFormat::Formatter(Rc::new(
        FuncFormatter::new(move |value, position| {
            formatter
                .call::<String>((value, position))
                .unwrap_or_default()
        }),
    ))))
}

/// matplotlib's `panchor`: `None` for the default (towards the colorbar),
/// `False` to leave the parent's anchor alone, or an anchor.
fn parent_anchor(value: &Bound<'_>) -> PyResult<ParentAnchor> {
    if value.is_none() {
        return Ok(ParentAnchor::Default);
    }
    if value.type_name() == "bool" && !value.is_truthy()? {
        return Ok(ParentAnchor::Unchanged);
    }
    Ok(ParentAnchor::At(anchor(value)?))
}

/// The options of `Figure.colorbar` given as a dict of matplotlib's
/// keywords (see `colorbar.py`), drawing in `cax` or beside `parents`.
fn options(
    cax: Option<Axes>,
    parents: Option<Vec<Axes>>,
    spec: &Bound<'_>,
) -> PyResult<ColorbarOptions> {
    let named = |key: &str| -> PyResult<Option<String>> { dict_value(spec, key) };
    Ok(ColorbarOptions {
        axes: match (cax, parents) {
            (Some(cax), _) => ColorbarAxes::Cax(cax),
            (None, Some(parents)) => ColorbarAxes::Parents(parents),
            (None, None) => ColorbarAxes::FromMappable,
        },
        use_gridspec: dict_value(spec, "use_gridspec")?,
        location: named("location")?.as_deref().map(side).transpose()?,
        orientation: named("orientation")?
            .as_deref()
            .map(orientation)
            .transpose()?,
        ticklocation: named("ticklocation")?
            .filter(|name| name != "auto")
            .as_deref()
            .map(side)
            .transpose()?,
        fraction: dict_value(spec, "fraction")?,
        shrink: dict_value(spec, "shrink")?,
        aspect: dict_value(spec, "aspect")?,
        pad: dict_value(spec, "pad")?,
        anchor: dict_value::<Option<Bound<'_>>>(spec, "anchor")?
            .as_ref()
            .map(anchor)
            .transpose()?,
        panchor: parent_anchor(&dict_value(spec, "panchor")?)?,
        extend: named("extend")?.as_deref().map(extend).transpose()?,
        extendfrac: extendfrac(&dict_value(spec, "extendfrac")?)?,
        extendrect: dict_value(spec, "extendrect")?,
        format: format(&dict_value(spec, "format")?)?,
        ticks: dict_value::<Option<Vec<f64>>>(spec, "ticks")?.map(ColorbarTicks::Values),
        drawedges: dict_value(spec, "drawedges")?,
        label: dict_value(spec, "label")?,
        spacing: match named("spacing")?.as_deref() {
            Some("proportional") => Spacing::Proportional,
            _ => Spacing::Uniform,
        },
        boundaries: dict_value(spec, "boundaries")?,
        values: dict_value(spec, "values")?,
    })
}

/// What a colorbar shows: the mapping through `cmap` and `norm`, which it
/// follows as the image `image` (its opacity included) when the mappable
/// is one.
struct Shown {
    mapping: ScalarMappable,
    image: Option<AxesImage>,
}

impl Shown {
    fn new(cmap: Colormap, norm: Box<dyn Norm>, image: Option<AxesImage>) -> Self {
        Self {
            mapping: ScalarMappable::with_norm(cmap, norm),
            image,
        }
    }

    fn mappable(&self) -> Mappable<'_> {
        self.image
            .as_ref()
            .map_or_else(|| Mappable::from(&self.mapping), Mappable::from)
    }
}

/// A colorbar in `figure` showing the mapping through `cmap` and `norm`
/// (of the image `image`, if the mappable is one), drawn in `cax` or in a
/// new axes taking space from `parents`, with the options of the dict
/// `spec`.
fn new(
    figure: Figure,
    (cmap, norm, image): (Colormap, Box<dyn Norm>, Option<AxesImage>),
    cax: Option<Axes>,
    parents: Option<Vec<Axes>>,
    spec: Owned,
) -> PyResult<Colorbar> {
    let options = Python::with_gil(|py| options(cax, parents, &spec.bind(py)))?;
    let shown = Shown::new(cmap, norm, image);
    Ok(figure.colorbar(shown.mappable(), options)?)
}

/// The axes the colorbar is drawn in.
fn ax(colorbar: Colorbar) -> PyResult<Axes> {
    Ok(colorbar.ax().clone())
}

/// `'vertical'` or `'horizontal'`.
fn get_orientation(colorbar: Colorbar) -> PyResult<&'static str> {
    Ok(match colorbar.orientation() {
        Orientation::Vertical => "vertical",
        Orientation::Horizontal => "horizontal",
    })
}

/// The ends extended beyond the range.
fn get_extend(colorbar: Colorbar) -> PyResult<&'static str> {
    Ok(match colorbar.extend() {
        Extend::Neither => "neither",
        Extend::Both => "both",
        Extend::Min => "min",
        Extend::Max => "max",
    })
}

/// The edges of the colour blocks, in data units.
fn boundaries(colorbar: Colorbar) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(colorbar.boundaries()))
}

/// The value at the middle of each colour block.
fn values(colorbar: Colorbar) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(colorbar.values()))
}

/// The limits of the colorbar's data range.
fn limits(colorbar: Colorbar) -> PyResult<(f64, f64)> {
    Ok((colorbar.vmin(), colorbar.vmax()))
}

/// Labels the long axis at `loc` (`'left'`/`'bottom'`, `'center'`,
/// `'right'`/`'top'`; `None` for the rc default).
fn set_label(colorbar: Colorbar, label: String, loc: Option<String>) -> PyResult<()> {
    let loc = match loc.as_deref() {
        None => None,
        Some("left" | "bottom") => Some(LabelLoc::Start),
        Some("center") => Some(LabelLoc::Center),
        Some("right" | "top") => Some(LabelLoc::End),
        Some(other) => {
            return Err(PyErr::value_error(format!(
                "{other:?} is not a valid value for loc"
            )));
        }
    };
    colorbar.set_label(label, loc);
    Ok(())
}

/// Puts the major ticks at `ticks`.
fn set_ticks(colorbar: Colorbar, ticks: Vec<f64>) -> PyResult<()> {
    colorbar.set_ticks(ticks);
    Ok(())
}

/// The major (or, `minor`, minor) tick locations along the colorbar.
fn get_ticks(colorbar: Colorbar, minor: bool) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(colorbar.long_axis().ticklocs(minor)?))
}

/// Turns the minor ticks on (`on`) or off.
fn set_minorticks(colorbar: Colorbar, on: bool) -> PyResult<()> {
    if on {
        colorbar.minorticks_on();
    } else {
        colorbar.minorticks_off();
    }
    Ok(())
}

/// Shows the mapping through `cmap` and `norm`, of the image `image` if
/// the mappable is one (the mappable changed).
fn update_normal(
    colorbar: Colorbar,
    (cmap, norm, image): (Colormap, Box<dyn Norm>, Option<AxesImage>),
) -> PyResult<()> {
    colorbar.update_normal(Shown::new(cmap, norm, image).mappable())?;
    Ok(())
}

/// Removes the colorbar and its axes, giving the space back.
fn remove(colorbar: Colorbar) -> PyResult<()> {
    colorbar.remove();
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new", new)?;
    module.add("ax", ax)?;
    module.add("get_orientation", get_orientation)?;
    module.add("get_extend", get_extend)?;
    module.add("boundaries", boundaries)?;
    module.add("values", values)?;
    module.add("limits", limits)?;
    module.add("set_label", set_label)?;
    module.add("set_ticks", set_ticks)?;
    module.add("get_ticks", get_ticks)?;
    module.add("set_minorticks", set_minorticks)?;
    module.add("update_normal", update_normal)?;
    module.add("remove", remove)
}
