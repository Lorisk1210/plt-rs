//! The paint of a patch, read and set by property name (matplotlib's
//! `Patch` getters and `Patch.set`): of a patch of any shape, and of a
//! spine, which matplotlib's `Spine` is a `Patch` too.

use pltrs::artist::{ArtistHandle, BaseProps, RawHandle};
use pltrs::colors::{Color, ColorError};
use pltrs::patches::{Patch, PatchColors, PatchProps, PatchShape, PatchStyle};
use pltrs::rc::ColorOrEdge;
use pltrs::spines::Spine;

use super::super::lines::{PyLineStyle, cap_style, join_style};
use super::AnyPatch;
use crate::binding::dict_items;
use crate::binding::prelude::*;

/// What every artist painted as a patch does: a patch of any shape, or
/// a spine.
pub(super) trait StyleOps {
    /// The handle common to all artists.
    fn raw_handle(&self) -> &RawHandle;
    /// A copy of the edge/face/hatch properties.
    fn style(&self) -> PatchStyle;
    /// Changes the edge/face/hatch properties.
    fn update_style(&self, f: &mut dyn FnMut(&mut PatchStyle));
    /// Sets the properties every artist has.
    fn update_base(&self, props: BaseProps);
    /// The edge width, defaults applied.
    fn linewidth(&self) -> f64;
    /// The colours painted, defaults and alpha applied.
    fn colors(&self) -> Result<PatchColors, ColorError>;
    /// Sets the hatch line width.
    fn set_hatch_linewidth(&self, width: f64);
}

impl<S: PatchShape> StyleOps for Patch<S> {
    fn raw_handle(&self) -> &RawHandle {
        self.raw()
    }

    fn style(&self) -> PatchStyle {
        Patch::style(self)
    }

    fn update_style(&self, f: &mut dyn FnMut(&mut PatchStyle)) {
        Patch::update_style(self, f);
    }

    fn update_base(&self, props: BaseProps) {
        Patch::update_props(
            self,
            PatchProps {
                base: props,
                ..PatchProps::default()
            },
        );
    }

    fn linewidth(&self) -> f64 {
        Patch::linewidth(self)
    }

    fn colors(&self) -> Result<PatchColors, ColorError> {
        Patch::colors(self)
    }

    fn set_hatch_linewidth(&self, width: f64) {
        Patch::set_hatch_linewidth(self, width);
    }
}

impl StyleOps for Spine {
    fn raw_handle(&self) -> &RawHandle {
        self.raw()
    }

    fn style(&self) -> PatchStyle {
        Spine::style(self)
    }

    fn update_style(&self, f: &mut dyn FnMut(&mut PatchStyle)) {
        Spine::update_style(self, f);
    }

    fn update_base(&self, props: BaseProps) {
        Spine::update_base(self, props);
    }

    fn linewidth(&self) -> f64 {
        Spine::linewidth(self)
    }

    fn colors(&self) -> Result<PatchColors, ColorError> {
        Spine::colors(self)
    }

    fn set_hatch_linewidth(&self, width: f64) {
        Spine::update_style(self, |style| style.hatch_linewidth = width);
    }
}

/// The native object of a patch of any shape or of a spine.
pub(super) struct AnyStyled(Box<dyn StyleOps>);

impl FromPy<'_> for AnyStyled {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(spine) = object.extract::<Spine>() {
            return Ok(Self(Box::new(spine)));
        }
        let patch: AnyPatch = object.extract()?;
        Ok(Self(patch.into_style()))
    }
}

/// A colour, `None` (the default) when the Python value is `None`.
fn optional_color(value: &Bound<'_>) -> PyResult<Option<Color>> {
    value.extract()
}

/// Applies the patch property `key` set to `value` (a resolved name of
/// matplotlib's `Patch.set`) to `patch`.
fn set_property(patch: &dyn StyleOps, key: &str, value: &Bound<'_>) -> PyResult<()> {
    let mut props = BaseProps::default();
    match key {
        "color" => {
            let color = optional_color(value)?;
            patch.update_style(&mut |style| {
                style.facecolor.clone_from(&color);
                style.edgecolor.clone_from(&color);
            });
        }
        "facecolor" => {
            let color = optional_color(value)?;
            patch.update_style(&mut |style| style.facecolor.clone_from(&color));
        }
        "edgecolor" => {
            let color = optional_color(value)?;
            patch.update_style(&mut |style| style.edgecolor.clone_from(&color));
        }
        "linewidth" => {
            let width: Option<f64> = value.extract()?;
            patch.update_style(&mut |style| style.linewidth = width);
        }
        "linestyle" => {
            let PyLineStyle(linestyle) = value.extract()?;
            patch.update_style(&mut |style| style.linestyle = linestyle.clone());
        }
        "fill" => {
            let fill: bool = value.extract()?;
            patch.update_style(&mut |style| style.fill = fill);
        }
        "hatch" => {
            let hatch: Option<String> = value.extract()?;
            patch.update_style(&mut |style| style.hatch.clone_from(&hatch));
        }
        "hatchcolor" => {
            let color = match value.extract::<Option<String>>() {
                Ok(Some(edge)) if edge == "edge" => ColorOrEdge::Edge,
                _ => ColorOrEdge::Color(value.extract()?),
            };
            patch.update_style(&mut |style| style.hatchcolor = color.clone());
        }
        "hatch_linewidth" => patch.set_hatch_linewidth(value.extract()?),
        "edgegapcolor" => {
            let color = optional_color(value)?.unwrap_or_else(Color::none);
            patch.update_style(&mut |style| style.edgegapcolor = color.clone());
        }
        "antialiased" => {
            let antialiased: Option<bool> = value.extract()?;
            patch.update_style(&mut |style| style.antialiased = antialiased);
        }
        "capstyle" => {
            let cap = cap_style(value)?;
            patch.update_style(&mut |style| style.capstyle = cap);
        }
        "joinstyle" => {
            let join = join_style(value)?;
            patch.update_style(&mut |style| style.joinstyle = join);
        }
        "label" => {
            props.label = Some(value.str()?);
            patch.update_base(props);
        }
        "zorder" => {
            props.zorder = Some(value.extract()?);
            patch.update_base(props);
        }
        "visible" => {
            props.visible = Some(value.extract()?);
            patch.update_base(props);
        }
        "clip_on" => {
            props.clip_on = Some(value.extract()?);
            patch.update_base(props);
        }
        "alpha" => {
            patch
                .raw_handle()
                .set_alpha(value.extract::<Option<f64>>()?);
        }
        _ => {
            return Err(PyErr::new(
                ExceptionKind::AttributeError,
                format!("Patch.set() got an unexpected keyword argument '{key}'"),
            ));
        }
    }
    Ok(())
}

/// Sets the properties of a dict of keyword arguments whose aliases the
/// Python side resolved (matplotlib's `Patch.set`), in order.
pub(super) fn update(patch: AnyStyled, kwargs: Owned) -> PyResult<()> {
    Python::with_gil(|py| {
        for (key, value) in dict_items(&kwargs.bind(py))? {
            let key: String = key.extract()?;
            set_property(patch.0.as_ref(), &key, &value)?;
        }
        Ok(())
    })
}

/// `(r, g, b, a)`.
fn rgba_tuple(rgba: pltrs::colors::Rgba) -> (f64, f64, f64, f64) {
    (rgba.r, rgba.g, rgba.b, rgba.a)
}

/// The patch property `name` as matplotlib's `get_<name>` returns it:
/// colours as RGBA tuples with the defaults and alpha applied, styles
/// by name.
pub(super) fn get(patch: AnyStyled, name: String) -> PyResult<Owned> {
    let patch = patch.0;
    Python::with_gil(|py| {
        let style = patch.style();
        let value = match name.as_str() {
            "facecolor" => rgba_tuple(patch.colors()?.face).into_py(py)?,
            "edgecolor" => rgba_tuple(patch.colors()?.edge).into_py(py)?,
            "hatchcolor" => rgba_tuple(patch.colors()?.hatch).into_py(py)?,
            "linewidth" => patch.linewidth().into_py(py)?,
            "linestyle" => match style.linestyle {
                pltrs::lines::LineStyle::Dashes { offset, pattern } => {
                    (offset, pattern).into_py(py)?
                }
                named => named.to_string().into_py(py)?,
            },
            "fill" => style.fill.into_py(py)?,
            "hatch" => style.hatch.into_py(py)?,
            "hatch_linewidth" => style.hatch_linewidth.into_py(py)?,
            "edgegapcolor" => (!style.edgegapcolor.is_none())
                .then_some(style.edgegapcolor)
                .into_py(py)?,
            "antialiased" => style.antialiased.into_py(py)?,
            "capstyle" => style.capstyle.name().into_py(py)?,
            "joinstyle" => style.joinstyle.name().into_py(py)?,
            _ => {
                return Err(PyErr::new(
                    ExceptionKind::AttributeError,
                    format!("'Patch' object has no property '{name}'"),
                ));
            }
        };
        Ok(value.unbind())
    })
}
