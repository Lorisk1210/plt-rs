//! matplotlib's colour mapping objects as pltrs (Rust)'s: a Python
//! `Colormap` becomes the [`Colormap`] of its lookup table, a Python norm
//! the pltrs (Rust) [`Norm`] of the same kind with the same parameters.
//!
//! The Python classes compute with their own state; pltrs (Rust) gets a
//! copy whenever it draws something mapped through them (a colorbar, an
//! image), and the Python side hands it a new copy when they change.

use pltrs::colors::{
    AsinhNorm, BoundaryNorm, CenteredNorm, Colormap, Extend, LogNorm, NoNorm, Norm, NormError,
    Normalize, PowerNorm, Rgba, ScaleNorm, SymLogNorm, TwoSlopeNorm,
};
use pltrs::scale::{LogTransform, LogitTransform, NonPositive};

use super::python_norm::PythonNorm;
use crate::binding::prelude::*;

impl From<NormError> for PyErr {
    /// matplotlib's `ValueError` for invalid norm parameters.
    fn from(err: NormError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// The extension a colormap or norm names (`'neither'`, `'both'`,
/// `'min'`, `'max'`).
pub(crate) fn extend(name: &str) -> PyResult<Extend> {
    Ok(match name {
        "neither" => Extend::Neither,
        "both" => Extend::Both,
        "min" => Extend::Min,
        "max" => Extend::Max,
        _ => {
            return Err(PyErr::value_error(format!(
                "{name:?} is not a valid value for extend; supported values are \
                 'neither', 'both', 'min', 'max'"
            )));
        }
    })
}

impl FromPy<'_> for Colormap {
    /// A Python `Colormap` (a `ListedColormap`, a
    /// `LinearSegmentedColormap`, or any subclass keeping matplotlib's
    /// lookup table) as the colormap of its table: its `N` colours, then
    /// its under, over and bad colours.
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if !object.getattr("_isinit")?.is_truthy()? {
            object.call_method("_init", &[])?;
        }
        let name: String = object.getattr("name")?.extract()?;
        let lut: NdArray = object.getattr("_lut")?.extract()?;
        let rows: Vec<Rgba> = lut
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[r, g, b, a]| Rgba::new(r, g, b, a))
            .collect();
        let [colors @ .., under, over, bad] = &rows[..] else {
            return Err(PyErr::value_error(format!(
                "the lookup table of colormap {name:?} is empty"
            )));
        };
        let extend_name: Bound<'_> = object.getattr("colorbar_extend")?;
        let colorbar_extend = if extend_name.is_truthy()? {
            Some(extend(&extend_name.extract::<String>()?)?)
        } else {
            None
        };
        Ok(Colormap::listed(name, colors.to_vec())?
            .with_under(*under)?
            .with_over(*over)?
            .with_bad(*bad)?
            .with_colorbar_extend(colorbar_extend))
    }
}

/// An attribute of a Python object.
fn attr<'py, T: FromPy<'py>>(object: &Bound<'py>, name: &str) -> PyResult<T> {
    object.getattr(name)?.extract()
}

/// The pltrs (Rust) norm of a Python norm made from a scale (matplotlib's
/// `make_norm_from_scale`: `LogNorm`, `SymLogNorm`, `AsinhNorm`, and the
/// norms named by scale), after the scale `scale` of class `kind`; `None`
/// for a scale pltrs (Rust) has no norm for.
fn norm_of_scale(scale: &Bound<'_>, kind: &str) -> PyResult<Option<Box<dyn Norm>>> {
    Ok(Some(match kind {
        "LogScale" => {
            let transform = LogTransform::new(attr(scale, "base")?, NonPositive::Mask)?;
            Box::new(LogNorm::from_transform(transform))
        }
        "SymmetricalLogScale" => Box::new(SymLogNorm::new(
            attr(scale, "linthresh")?,
            attr(scale, "linscale")?,
            attr(scale, "base")?,
        )?),
        "AsinhScale" => Box::new(AsinhNorm::new(attr(scale, "linear_width")?)?),
        "LogitScale" => Box::new(ScaleNorm::from_transform(LogitTransform::new(
            NonPositive::Mask,
        ))),
        _ => return Ok(None),
    }))
}

/// The pltrs (Rust) norm of the kind of the Python norm `norm`, named by
/// its class, before its limits are set; `None` for a kind pltrs (Rust)
/// has no norm for.
fn norm_of_kind(norm: &Bound<'_>, kind: &str) -> PyResult<Option<Box<dyn Norm>>> {
    if let Ok(scale) = norm.getattr("_scale") {
        let scale_kind: String = attr(&scale.getattr("__class__")?, "__name__")?;
        if let Some(norm) = norm_of_scale(&scale, &scale_kind)? {
            return Ok(Some(norm));
        }
    }
    Ok(Some(match kind {
        "Normalize" => Box::new(Normalize::default()),
        "NoNorm" => Box::new(NoNorm::default()),
        "PowerNorm" => Box::new(PowerNorm::new(attr(norm, "gamma")?)),
        "BoundaryNorm" => {
            let boundaries: Vec<f64> = attr(norm, "boundaries")?;
            let extend_name: String = attr(norm, "extend")?;
            Box::new(BoundaryNorm::new(
                boundaries,
                attr(norm, "Ncmap")?,
                extend(&extend_name)?,
            )?)
        }
        "TwoSlopeNorm" => Box::new(TwoSlopeNorm::new(attr(norm, "vcenter")?, None, None)?),
        "CenteredNorm" => Box::new(CenteredNorm::new(
            attr(norm, "vcenter")?,
            attr(norm, "halfrange")?,
        )),
        _ => return Ok(None),
    }))
}

impl FromPy<'_> for Box<dyn Norm> {
    /// A Python norm of one of matplotlib's classes (not a subclass, which
    /// may compute differently) as the pltrs (Rust) norm of that kind, with
    /// its limits and clipping; any other norm of one component as itself,
    /// called from pltrs (Rust) (see [`PythonNorm`]).
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let class = object.getattr("__class__")?;
        let module: String = attr(&class, "__module__")?;
        let kind: String = attr(&class, "__name__")?;
        let matplotlib = module.ends_with(".colors") || module == "colors";
        let copy = if matplotlib {
            norm_of_kind(object, &kind)?
        } else {
            None
        };
        let Some(mut norm) = copy else {
            let components: usize = attr(object, "n_components")?;
            if components != 1 {
                // A norm of several variates mapping the values of one
                // (their colours are mapped in Python, `Collection._map_variates`):
                // matplotlib's colorizer refuses it as it does a colormap of
                // another number of variates.
                return Err(PyErr::value_error(format!(
                    "The {module}.{kind} of {components} variates cannot map data of 1 variate"
                )));
            }
            return Ok(Box::new(PythonNorm::of(object)?));
        };
        norm.set_vmin(attr(object, "vmin")?);
        norm.set_vmax(attr(object, "vmax")?);
        norm.set_clip(object.getattr("clip")?.is_truthy()?);
        Ok(norm)
    }
}
