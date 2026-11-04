//! Tick parameters (matplotlib's `Axis.set_tick_params`): the keywords
//! `Axis._translate_tick_params` gives (`size`, `tickdir`, `tick1On`,
//! `labelrotation`, `grid_color`, ...) as pltrs (Rust)'s [`TickParams`].

use pltrs::artist::ClipPathSpec;
use pltrs::axes::{AxisName, GridClip, GridOptions, TickLevel, TickParams};
use pltrs::fonts::FontSize;
use pltrs::lines::LineStyle;
use pltrs::mplot3d::{Axis3, Axis3D};

use pltrs_render::Sketch;

use super::owner::{AnyAxis, AxisOwner};
use crate::binding::dict_items;
use crate::binding::prelude::*;
use crate::modules::artist::ClipSpec;
use crate::modules::artist::filter::python_filter;
use crate::modules::lines::{PyLineStyle, cap_style, drawstyle, join_style, parse};
use crate::modules::patheffects::path_effects;

/// A clip region of the gridlines: `None` for none, else a clip path
/// description (see `artist::ClipSpec`).
fn grid_clip(value: &Bound<'_>) -> PyResult<GridClip> {
    Ok(value
        .extract::<Option<ClipSpec>>()?
        .map_or(GridClip::Off, |spec| GridClip::Given(spec.0)))
}

/// A font size given as points or as a keyword (`"large"`).
fn font_size(value: &Bound<'_>) -> PyResult<FontSize> {
    match value.extract::<f64>() {
        Ok(points) => Ok(FontSize::Points(points)),
        Err(_) => parse(value),
    }
}

/// Sets `params.labelrotation` from matplotlib's `labelrotation`: an
/// angle, `"auto"`, `"default"` or `(mode, angle)`.
fn label_rotation(params: &mut TickParams, value: &Bound<'_>) -> PyResult<()> {
    let (mode, angle) = if let Ok(angle) = value.extract::<f64>() {
        ("default".to_owned(), angle)
    } else if let Ok(mode) = value.extract::<String>() {
        (mode, 0.0)
    } else {
        value.extract::<(String, f64)>()?
    };
    params.labelrotation_auto = Some(match mode.as_str() {
        "auto" => true,
        "default" => false,
        _ => {
            return Err(PyErr::value_error(format!(
                "{mode:?} is not a valid value for labelrotation; supported values are \
                 'auto', 'default'"
            )));
        }
    });
    params.labelrotation = Some(angle);
    Ok(())
}

/// The line style `Line2D.set_dashes(seq)` gives: solid for an empty
/// sequence or `(None, None)`, else the dashes `(0, seq)`.
fn dashes(value: &Bound<'_>) -> PyResult<LineStyle> {
    let none_none = value
        .extract::<(Option<f64>, Option<f64>)>()
        .is_ok_and(|pair| pair == (None, None));
    if none_none || value.try_iter()?.next().is_none() {
        return Ok(LineStyle::Solid);
    }
    Ok((0.0, value.clone())
        .into_py(value.py())?
        .extract::<PyLineStyle>()?
        .0)
}

/// The sketch matplotlib's gridline takes from `set(sketch_params=value)`
/// (its `set_sketch_params(value)`): none for `None`, else the wobble of
/// scale `value` at the default length and randomness.
fn sketch(value: &Bound<'_>) -> PyResult<Option<Sketch>> {
    if value.is_none() {
        return Ok(None);
    }
    let scale: f64 = value.extract().map_err(|_| {
        PyErr::type_error(
            "grid_sketch_params gives the gridlines' sketch scale, a number (or None)",
        )
    })?;
    Ok(Some(Sketch {
        scale,
        length: 128.0,
        randomness: 16.0,
    }))
}

/// Sets the side switch `key` (`tick1On`, `label2On`, ...) of axis
/// `name`.
fn side_switch(params: &mut TickParams, name: AxisName, key: &str, on: bool) {
    let slot = match (key, name) {
        ("tick1On", AxisName::X) => &mut params.bottom,
        ("tick2On", AxisName::X) => &mut params.top,
        ("label1On", AxisName::X) => &mut params.labelbottom,
        ("label2On", AxisName::X) => &mut params.labeltop,
        ("tick1On", AxisName::Y) => &mut params.left,
        ("tick2On", AxisName::Y) => &mut params.right,
        ("label1On", AxisName::Y) => &mut params.labelleft,
        _ => &mut params.labelright,
    };
    *slot = Some(on);
}

/// The tick parameters of the translated keywords `kwargs` for axis
/// `name`, and the gridline switch (`gridOn`) when given.
fn tick_params(name: AxisName, kwargs: &Bound<'_>) -> PyResult<(TickParams, Option<bool>)> {
    let mut params = TickParams::default();
    let mut grid_on = None;
    for (key, value) in dict_items(kwargs)? {
        let key: String = key.extract()?;
        match key.as_str() {
            "size" => params.length = Some(value.extract()?),
            "width" => params.width = Some(value.extract()?),
            "pad" => params.pad = Some(value.extract()?),
            "color" => params.color = Some(value.extract()?),
            "labelcolor" => params.labelcolor = Some(value.extract()?),
            "tickdir" => params.direction = Some(parse(&value)?),
            "labelsize" => params.labelsize = Some(font_size(&value)?),
            "labelfontfamily" => params.labelfontfamily = Some(value.extract()?),
            "labelrotation" => label_rotation(&mut params, &value)?,
            "labelrotation_mode" => params.labelrotation_mode = Some(parse(&value)?),
            "gridOn" => grid_on = Some(value.is_truthy()?),
            "tick1On" | "tick2On" | "label1On" | "label2On" => {
                side_switch(&mut params, name, &key, value.is_truthy()?);
            }
            "grid_color" | "grid_c" => params.grid_color = Some(value.extract()?),
            "grid_alpha" => params.grid_alpha = Some(value.extract()?),
            "grid_linewidth" | "grid_lw" => params.grid_linewidth = Some(value.extract()?),
            "grid_linestyle" | "grid_ls" => {
                params.grid_linestyle = Some(value.extract::<PyLineStyle>()?.0);
            }
            "grid_dashes" => params.grid_linestyle = Some(dashes(&value)?),
            "grid_path_effects" => params.grid_path_effects = Some(path_effects(&value)?),
            "grid_zorder" => params.grid_zorder = Some(value.extract()?),
            "grid_antialiased" | "grid_aa" => {
                params.grid_stroke.antialiased = Some(value.is_truthy()?);
            }
            "grid_solid_capstyle" => params.grid_stroke.solid_capstyle = Some(cap_style(&value)?),
            "grid_dash_capstyle" => params.grid_stroke.dash_capstyle = Some(cap_style(&value)?),
            "grid_solid_joinstyle" => {
                params.grid_stroke.solid_joinstyle = Some(join_style(&value)?);
            }
            "grid_dash_joinstyle" => params.grid_stroke.dash_joinstyle = Some(join_style(&value)?),
            "grid_drawstyle" | "grid_ds" => params.grid_line.drawstyle = Some(drawstyle(&value)?),
            "grid_gapcolor" => params.grid_line.gapcolor = Some(value.extract()?),
            "grid_snap" => params.grid_line.snap = Some(value.extract()?),
            "grid_sketch_params" => params.grid_line.sketch = Some(sketch(&value)?),
            "grid_clip_on" => params.grid_line.clip_on = Some(value.is_truthy()?),
            "grid_url" => params.grid_line.url = Some(value.extract()?),
            "grid_gid" => params.grid_line.gid = Some(value.extract()?),
            "grid_rasterized" => params.grid_line.rasterized = Some(value.is_truthy()?),
            "grid_animated" => params.grid_line.animated = Some(value.is_truthy()?),
            "grid_agg_filter" => {
                let filter: Option<Callback> = value.extract()?;
                params.grid_line.agg_filter = Some(filter.map(python_filter));
            }
            // Clip regions as `Axis.set_tick_params` describes them (see
            // `_artist_clipping.clip_path_spec`); a rectangle given as the
            // clip path clips as a box, as matplotlib makes it one.
            "grid_clip_box" => params.grid_line.clip_box = Some(grid_clip(&value)?),
            "grid_clip_path" => match grid_clip(&value)? {
                GridClip::Given(spec @ ClipPathSpec::BoundingBox { .. }) => {
                    params.grid_line.clip_box = Some(GridClip::Given(spec));
                    params.grid_line.clip_path = Some(GridClip::Off);
                }
                clip => params.grid_line.clip_path = Some(clip),
            },
            // Line properties the gridlines do not draw with: matplotlib's
            // ticks make their gridline without a marker and place it
            // themselves, and neither layouts nor the mouse look at it.
            "grid_markersize"
            | "grid_ms"
            | "grid_markeredgecolor"
            | "grid_mec"
            | "grid_markeredgewidth"
            | "grid_mew"
            | "grid_markerfacecolor"
            | "grid_mfc"
            | "grid_markerfacecoloralt"
            | "grid_mfcalt"
            | "grid_fillstyle"
            | "grid_markevery"
            | "grid_pickradius"
            | "grid_picker"
            | "grid_label"
            | "grid_data"
            | "grid_xdata"
            | "grid_ydata"
            | "grid_transform"
            | "grid_in_layout"
            | "grid_mouseover"
            | "grid_figure" => {}
            // matplotlib's ticks make their gridline with a marker and a
            // visibility of their own, which these would give twice: the
            // next tick the level makes fails.
            "grid_marker" => params.grid_line.refused = Some("marker"),
            "grid_visible" => params.grid_line.refused = Some("visible"),
            _ => {
                return Err(PyErr::value_error(format!(
                    "keyword {key} is not recognized"
                )));
            }
        }
    }
    Ok((params, grid_on))
}

/// Sets the tick parameters `kwargs` (translated keywords) of the major,
/// minor or both (`which`) tick levels of axis `name`, after restoring
/// the rc defaults when `reset` (`Axis.set_tick_params`).
fn set_tick_params(
    owner: AxisOwner,
    name: String,
    which: String,
    reset: bool,
    kwargs: Owned,
) -> PyResult<()> {
    let which: TickLevel =
        which
            .parse()
            .map_err(|err: <TickLevel as std::str::FromStr>::Err| {
                PyErr::value_error(err.to_string())
            })?;
    let axis = match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis,
        AnyAxis::Deep(axis) => return set_deep_tick_params(&axis, which, reset, &kwargs),
    };
    let (params, grid_on) = Python::with_gil(|py| tick_params(axis.name(), &kwargs.bind(py)))?;
    axis.set_tick_params(TickParams {
        which,
        reset,
        ..params
    });
    if let Some(visible) = grid_on {
        axis.grid(
            visible,
            GridOptions {
                which,
                ..GridOptions::default()
            },
        );
    }
    Ok(())
}

/// [`set_tick_params`] on an axis of a 3-D axes, whose x axis takes the
/// switches of an x axis and whose y and z axes those of a y axis, as
/// `Axes3D.tick_params` routes them; its gridlines are the axes' own.
fn set_deep_tick_params(
    axis: &Axis3D,
    which: TickLevel,
    reset: bool,
    kwargs: &Owned,
) -> PyResult<()> {
    let sides = if axis.dir() == Axis3::X {
        AxisName::X
    } else {
        AxisName::Y
    };
    // A 3-D axis draws its gridlines by the grid switch of its axes, as
    // matplotlib's `Axis3D.draw_grid` does, whatever its ticks' `gridOn`.
    let (params, _) = Python::with_gil(|py| tick_params(sides, &kwargs.bind(py)))?;
    axis.set_tick_params(TickParams {
        which,
        reset,
        ..params
    });
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_tick_params", set_tick_params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::new_dict;
    use crate::binding::testing::with_python;
    use pltrs::figure::Figure;

    #[test]
    fn translated_keywords_become_tick_params() {
        let ax = Figure::new().subplot();
        let kwargs = with_python(|py| {
            let kwargs = new_dict(py).unwrap();
            kwargs.set_item("size", 7.0).unwrap();
            kwargs.set_item("tick2On", true).unwrap();
            kwargs.set_item("gridOn", true).unwrap();
            kwargs.unbind()
        });
        set_tick_params(
            AxisOwner::Flat(ax.clone()),
            "x".into(),
            "major".into(),
            false,
            kwargs,
        )
        .unwrap();
        let style = ax.xaxis().tick_style(false);
        assert_eq!(style.size, 7.0);
        assert!(style.ticks.second && style.grid_on);
        let unknown = with_python(|py| {
            let kwargs = new_dict(py).unwrap();
            kwargs.set_item("zorder", 3.0).unwrap();
            kwargs.unbind()
        });
        assert!(
            set_tick_params(
                AxisOwner::Flat(ax),
                "y".into(),
                "both".into(),
                false,
                unknown
            )
            .is_err()
        );
    }
}
