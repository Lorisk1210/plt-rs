//! `_native.patheffects`: the path effects of matplotlib's `patheffects`
//! drawn by `pltrs::patheffects`, and their geometry.
//!
//! A path effect crosses as the tuple its Python class describes itself
//! with (`AbstractPathEffect._native_spec`), the first item naming its
//! kind: `("normal",)`, `("stroke", then_normal, offset, gc)`,
//! `("patch_shadow", then_normal, offset, color, alpha, rho, gc)`,
//! `("line_shadow", then_normal, offset, color, alpha, rho, gc)`,
//! `("ticked", then_normal, offset, spacing, angle, length, gc)` and
//! `("path_patch", offset, patch_kwargs)`. `then_normal` is true for the
//! `with...` classes, which draw the path as it is afterwards; `gc` is the
//! dict of graphics context properties the effect sets.

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::patheffects::{
    Effect, GcUpdate, LineShadow, PatchEffect, PatchShadow, PathEffect, Stroke, TickedStroke,
};

use super::lines::{cap_style, join_style};
use super::patches::PatchKwargs;
use super::path::{PathArrays, path_arrays};
use super::transforms::points;
use crate::binding::dict_items;
use crate::binding::prelude::*;

/// The graphics context changes of a dict of `GraphicsContextBase`
/// properties (`linewidth`, `foreground`, `alpha`, `capstyle`,
/// `joinstyle`), with matplotlib's `AttributeError` for others.
struct PyGcUpdate(GcUpdate);

impl FromPy<'_> for PyGcUpdate {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let mut gc = GcUpdate::default();
        for (key, value) in dict_items(object)? {
            let key: String = key.extract()?;
            match key.as_str() {
                "linewidth" => gc.linewidth = Some(value.extract()?),
                "foreground" => gc.foreground = Some(value.extract()?),
                "alpha" => gc.alpha = value.extract()?,
                "capstyle" => gc.capstyle = Some(cap_style(&value)?),
                "joinstyle" => gc.joinstyle = Some(join_style(&value)?),
                _ => {
                    return Err(PyErr::new(
                        ExceptionKind::AttributeError,
                        format!("Unknown property {key}"),
                    ));
                }
            }
        }
        Ok(Self(gc))
    }
}

/// A path effect described by Python (see the module documentation).
struct PyPathEffect(PathEffect);

impl FromPy<'_> for PyPathEffect {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let items: Vec<Bound<'_>> = object.extract()?;
        let kind: String = items
            .first()
            .ok_or_else(|| PyErr::value_error("an empty path effect description"))?
            .extract()?;
        let gc = |item: &Bound<'_>| Ok::<_, PyErr>(item.extract::<PyGcUpdate>()?.0);
        let (effect, then_normal) = match (kind.as_str(), &items[1..]) {
            ("normal", []) => (Effect::Normal, false),
            ("stroke", [then_normal, offset, update]) => (
                Effect::Stroke(Stroke {
                    offset: offset.extract()?,
                    gc: gc(update)?,
                }),
                then_normal.extract()?,
            ),
            ("patch_shadow", [then_normal, offset, color, alpha, rho, update]) => (
                Effect::PatchShadow(PatchShadow {
                    offset: offset.extract()?,
                    color: color.extract()?,
                    alpha: alpha.extract()?,
                    rho: rho.extract()?,
                    gc: gc(update)?,
                }),
                then_normal.extract()?,
            ),
            ("line_shadow", [then_normal, offset, color, alpha, rho, update]) => (
                Effect::LineShadow(LineShadow {
                    offset: offset.extract()?,
                    color: color.extract()?,
                    alpha: alpha.extract()?,
                    rho: rho.extract()?,
                    gc: gc(update)?,
                }),
                then_normal.extract()?,
            ),
            ("ticked", [then_normal, offset, spacing, angle, length, update]) => (
                Effect::TickedStroke(TickedStroke {
                    offset: offset.extract()?,
                    spacing: spacing.extract()?,
                    angle: angle.extract()?,
                    length: length.extract()?,
                    gc: gc(update)?,
                }),
                then_normal.extract()?,
            ),
            ("path_patch", [offset, kwargs]) => {
                let PatchKwargs(props) = kwargs.extract()?;
                let patch = PatchEffect::new(offset.extract()?, props)
                    .map_err(|err| PyErr::value_error(err.to_string()))?;
                (Effect::PathPatch(patch), false)
            }
            _ => {
                return Err(PyErr::value_error(format!(
                    "a malformed path effect description of kind {kind:?}"
                )));
            }
        };
        Ok(Self(PathEffect {
            effect,
            then_normal,
        }))
    }
}

/// The path effects of a list of descriptions (a `path_effects` keyword
/// argument of a property table).
pub(super) fn path_effects(descriptions: &Bound<'_>) -> PyResult<Vec<PathEffect>> {
    let effects: Vec<PyPathEffect> = descriptions.extract()?;
    Ok(effects.into_iter().map(|effect| effect.0).collect())
}

/// Draws the paths of `artist` through `effects`, in order (matplotlib's
/// `set_path_effects`; none draws them as they are).
fn set_path_effects(artist: RawHandle, effects: Owned) -> PyResult<()> {
    let effects = Python::with_gil(|py| path_effects(&effects.bind(py)))?;
    artist.set_path_effects(effects);
    Ok(())
}

/// The ticks `TickedStroke` draws along a polyline (display pixels):
/// `spacing` points apart (`spacing_px` pixels), at `angle` degrees to the
/// line, `length` times the spacing long; `None` for a single point.
fn ticks(
    polyline: NdArray,
    (spacing, angle, length): (f64, f64, f64),
    spacing_px: f64,
) -> PyResult<Option<PathArrays>> {
    let stroke = TickedStroke {
        spacing,
        angle,
        length,
        ..TickedStroke::default()
    };
    stroke
        .ticks(&points(&polyline)?, spacing_px)
        .map(|path| path_arrays(&path, false))
        .transpose()
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_path_effects", set_path_effects)?;
    module.add("ticks", ticks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, extension, with_python};

    #[test]
    fn descriptions_become_path_effects() {
        with_python(|py| {
            extension(py);
            let stroke = eval(
                py,
                "",
                "('stroke', True, (1.0, -1.0), {'linewidth': 3, 'foreground': 'w'})",
            );
            let PyPathEffect(effect) = stroke.extract().unwrap();
            assert!(effect.then_normal);
            let Effect::Stroke(stroke) = effect.effect else {
                panic!("a stroke, not {:?}", effect.effect);
            };
            assert_eq!(stroke.offset, (1.0, -1.0));
            assert_eq!(stroke.gc.linewidth, Some(3.0));
            let shadow = eval(
                py,
                "",
                "('line_shadow', False, (2, -2), None, 0.3, 0.3, {})",
            );
            let PyPathEffect(effect) = shadow.extract().unwrap();
            assert!(matches!(
                effect.effect,
                Effect::LineShadow(LineShadow { color: None, .. })
            ));
            let unknown = eval(py, "", "('stroke', False, (0, 0), {'dashes': 1})");
            let err = unknown.extract::<PyPathEffect>().err().unwrap();
            assert_eq!(err.class_name(py), "AttributeError");
            let malformed = eval(py, "", "('stroke', False)");
            assert!(malformed.extract::<PyPathEffect>().is_err());
        });
    }
}
