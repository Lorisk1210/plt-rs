//! The sizes of `axes_size` as the Python side describes them
//! (`_Base._to_native` of `mpl_toolkits.axes_grid1.axes_size`).

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes::Axes;
use pltrs::axes_grid1::{Size, SizeAspect, SizeFn};
use pltrs::spines::SpineSide;

use crate::binding::prelude::*;

/// A [`Size`] from its description: `("fixed", inches)`, `("scaled",
/// factor)`, `("axes_x" | "axes_y", axes, aspect)` with `aspect` a number
/// or `("axes", reference_axes)`, `("fraction", fraction, size or None)`,
/// `("sum", [sizes])`, `("max_width" | "max_height", [artists])`,
/// `("decorations", [axes], side)` or `("python", size)` for a size of
/// the program's, asked `get_size(None)` at every layout (no renderer is
/// at hand while pltrs (Rust) lays the figure out).
pub(super) struct PySize(pub(super) Size);

impl FromPy<'_> for PySize {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let tag: String = object
            .call_method("__getitem__", &[0_i64.into_py(object.py())?])?
            .extract()?;
        let size = match tag.as_str() {
            "fixed" => Size::Fixed(object.extract::<(String, f64)>()?.1),
            "scaled" => Size::Scaled(object.extract::<(String, f64)>()?.1),
            "axes_x" | "axes_y" => {
                let (_, axes, aspect): (String, Axes, Bound<'_>) = object.extract()?;
                let aspect = size_aspect(&aspect)?;
                if tag == "axes_x" {
                    Size::AxesX(axes.id(), aspect)
                } else {
                    Size::AxesY(axes.id(), aspect)
                }
            }
            "fraction" => {
                let (_, fraction, reference): (String, f64, Option<PySize>) = object.extract()?;
                match reference {
                    Some(reference) => reference.0.times(fraction),
                    // matplotlib's `Fraction(f, None)` is `(f, 0)`.
                    None => Size::Scaled(fraction),
                }
            }
            "sum" => {
                let (_, sizes): (String, Vec<PySize>) = object.extract()?;
                Size::Sum(sizes.into_iter().map(|size| size.0).collect())
            }
            "max_width" | "max_height" => {
                let (_, artists): (String, Vec<RawHandle>) = object.extract()?;
                let ids = artists.iter().map(RawHandle::id).collect();
                if tag == "max_width" {
                    Size::MaxWidth(ids)
                } else {
                    Size::MaxHeight(ids)
                }
            }
            "decorations" => {
                let (_, axes, side): (String, Vec<Axes>, String) = object.extract()?;
                Size::AxesDecorations(axes.iter().map(Axes::id).collect(), side_named(&side)?)
            }
            "python" => {
                let (_, size): (String, Bound<'_>) = object.extract()?;
                let get_size: Callback = size.getattr("get_size")?.extract()?;
                Size::Program(SizeFn::new(move || {
                    get_size
                        .call::<(f64, f64)>((None::<f64>,))
                        .unwrap_or((0.0, 0.0))
                }))
            }
            _ => return Err(PyErr::type_error(format!("unknown size {tag:?}"))),
        };
        Ok(Self(size))
    }
}

/// The aspect of an `AxesX`/`AxesY` size: a factor, or `("axes",
/// reference)` for matplotlib's `aspect="axes"`.
fn size_aspect(aspect: &Bound<'_>) -> PyResult<SizeAspect> {
    if let Ok(factor) = aspect.extract::<f64>() {
        return Ok(SizeAspect::Factor(factor));
    }
    let (_, reference): (String, Axes) = aspect.extract()?;
    Ok(SizeAspect::RelativeTo(reference.id()))
}

/// The side called `name` (`"left"`, `"right"`, `"bottom"`, `"top"`).
pub(crate) fn side_named(name: &str) -> PyResult<SpineSide> {
    SpineSide::from_name(name).ok_or_else(|| {
        PyErr::value_error(format!(
            "{name:?} is not a valid value for position; supported values are 'left', \
             'right', 'bottom', 'top'"
        ))
    })
}

/// The sizes of `sizes`, in order.
pub(super) fn unwrap_sizes(sizes: Vec<PySize>) -> Vec<Size> {
    sizes.into_iter().map(|size| size.0).collect()
}
