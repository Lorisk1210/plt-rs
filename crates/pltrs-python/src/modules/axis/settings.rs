//! What an axis is apart from its ticks: whether its tick helpers are the
//! scale's, minor ticks, its place in layouts, label pad and coordinates,
//! the side of its ticks and offset text, its extent, and its units
//! (matplotlib's `Axis.clear`, `isDefault_majloc`, `minorticks_on`,
//! `set_in_layout`, `labelpad`, `set_label_coords`, `tick_top`,
//! `get_tightbbox`, `have_units`, `update_units`, `axis_date`).

use pltrs::axes::{Axes, AxisBelow};
use pltrs::spines::SpineSide;
use pltrs::units::{Categories, DateConverter, UnitConverter};

use super::owner::{AnyAxis, AxisOwner};
use super::{axis, side};
use crate::binding::prelude::*;
use crate::modules::dates::Zone;
use crate::modules::plotting::values::PyAxisValues;

/// Whether the major or `minor` locator (`kind` `"locator"`) or formatter
/// (`"formatter"`) is still the one the scale installed.
fn is_default_helper(owner: AxisOwner, name: String, kind: String, minor: bool) -> PyResult<bool> {
    Ok(match (kind.as_str(), minor, owner.axis(&name)?) {
        ("locator", false, AnyAxis::Flat(axis)) => axis.is_default_major_locator(),
        ("locator", true, AnyAxis::Flat(axis)) => axis.is_default_minor_locator(),
        ("formatter", false, AnyAxis::Flat(axis)) => axis.is_default_major_formatter(),
        ("formatter", true, AnyAxis::Flat(axis)) => axis.is_default_minor_formatter(),
        ("locator", _, AnyAxis::Deep(axis)) => axis.is_default_locator(minor),
        ("formatter", _, AnyAxis::Deep(axis)) => axis.is_default_formatter(minor),
        _ => return Err(PyErr::value_error(format!("no tick helper kind {kind:?}"))),
    })
}

/// Whether the label text was never set (`isDefault_label`).
fn is_default_label(owner: AxisOwner, name: String) -> PyResult<bool> {
    Ok(match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.is_default_label(),
        AnyAxis::Deep(axis) => axis.is_default_label(),
    })
}

/// Sets the label text, which unit converters then leave alone, even when
/// it stays the same (`set_label_text`).
fn set_label_text(owner: AxisOwner, name: String, text: String) -> PyResult<()> {
    match owner.axis(&name)? {
        AnyAxis::Flat(axis) => {
            axis.set_label_text(text);
        }
        AnyAxis::Deep(axis) => {
            axis.axes().set_label(axis.dir(), text);
        }
    }
    Ok(())
}

/// Turns the minor ticks on with the scale's minor locator, or off
/// (`minorticks_on`, `minorticks_off`).
fn minorticks(owner: AxisOwner, name: String, on: bool) -> PyResult<()> {
    match (owner.axis(&name)?, on) {
        (AnyAxis::Flat(axis), true) => {
            axis.minorticks_on();
        }
        (AnyAxis::Flat(axis), false) => {
            axis.minorticks_off();
        }
        (AnyAxis::Deep(axis), true) => {
            axis.minorticks_on();
        }
        (AnyAxis::Deep(axis), false) => {
            axis.minorticks_off();
        }
    }
    Ok(())
}

/// Whether minor ticks on major ones are dropped.
fn get_remove_overlapping_locs(owner: AxisOwner, name: String) -> PyResult<bool> {
    Ok(match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.remove_overlapping_locs(),
        AnyAxis::Deep(axis) => axis.remove_overlapping_locs(),
    })
}

/// Drops minor ticks on major ones, or keeps them.
fn set_remove_overlapping_locs(owner: AxisOwner, name: String, remove: bool) -> PyResult<()> {
    match owner.axis(&name)? {
        AnyAxis::Flat(axis) => {
            axis.set_remove_overlapping_locs(remove);
        }
        AnyAxis::Deep(axis) => {
            axis.set_remove_overlapping_locs(remove);
        }
    }
    Ok(())
}

/// How far the tick marks reach out of the axes, in points.
fn get_tick_padding(axes: Axes, name: String) -> PyResult<f64> {
    Ok(axis(&axes, &name)?.tick_padding())
}

/// The gap between the axis label and the tick labels, in points.
fn get_labelpad(owner: AxisOwner, name: String) -> PyResult<f64> {
    Ok(match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.label_pad(),
        AnyAxis::Deep(axis) => axis.label_pad(),
    })
}

/// Sets the gap between the axis label and the tick labels.
fn set_labelpad(owner: AxisOwner, name: String, pad: f64) -> PyResult<()> {
    match owner.axis(&name)? {
        AnyAxis::Flat(axis) => {
            axis.set_label_pad(pad);
        }
        AnyAxis::Deep(axis) => {
            axis.set_label_pad(pad);
        }
    }
    Ok(())
}

/// Anchors the axis label at `(x, y)` in axes fractions.
fn set_label_coords(axes: Axes, name: String, x: f64, y: f64) -> PyResult<()> {
    axis(&axes, &name)?.set_label_coords(x, y);
    Ok(())
}

/// Whether layouts make room for the axis' texts.
fn get_in_layout(axes: Axes, name: String) -> PyResult<bool> {
    Ok(axis(&axes, &name)?.in_layout())
}

/// Includes the axis' texts in layouts, or leaves them out.
fn set_in_layout(axes: Axes, name: String, in_layout: bool) -> PyResult<()> {
    axis(&axes, &name)?.set_in_layout(in_layout);
    Ok(())
}

/// The extent of the axis' texts in display pixels, the axis label
/// collapsed along the axis when `for_layout_only` (`get_tightbbox`).
fn get_tightbbox(
    owner: AxisOwner,
    name: String,
    for_layout_only: bool,
) -> PyResult<Option<(f64, f64, f64, f64)>> {
    let bbox = match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.tight_bbox_with(for_layout_only),
        AnyAxis::Deep(axis) => axis
            .tight_bbox(for_layout_only)
            .map_err(|err| PyErr::value_error(err.to_string()))?,
    };
    Ok(bbox.map(|bbox| (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())))
}

/// Moves the ticks, and their labels unless both were hidden, to the side
/// `position` (`tick_top`, `tick_bottom`, `tick_left`, `tick_right`).
fn tick_to(axes: Axes, name: String, position: String) -> PyResult<()> {
    axis(&axes, &name)?.tick_to(side(&position)?)?;
    Ok(())
}

/// Puts the offset text on the side `position` (`set_offset_position`).
fn set_offset_position(axes: Axes, name: String, position: String) -> PyResult<()> {
    let side: SpineSide = side(&position)?;
    axis(&axes, &name)?.set_offset_position(side)?;
    Ok(())
}

/// Rescales the view along axis `name` at the next layout, `tight`
/// replacing the tight setting (`_request_autoscale_view`).
fn request_autoscale(axes: Axes, name: String, tight: Option<bool>) -> PyResult<()> {
    axis(&axes, &name)?.request_autoscale(tight);
    Ok(())
}

/// Where ticks and gridlines sit among the artists of `axes`: `True`
/// (below), `"line"` or `False` (above) (`get_axisbelow`).
fn get_axisbelow(axes: Axes) -> PyResult<Owned> {
    Python::with_gil(|py| {
        let value = match axes.axisbelow() {
            AxisBelow::Below => true.into_py(py)?,
            AxisBelow::Line => "line".into_py(py)?,
            AxisBelow::Above => false.into_py(py)?,
        };
        Ok(value.unbind())
    })
}

/// Puts ticks and gridlines below everything (`below` `"below"`), above
/// patches (`"line"`) or above everything (`"above"`) (`set_axisbelow`).
fn set_axisbelow(axes: Axes, below: String) -> PyResult<()> {
    axes.set_axisbelow(match below.as_str() {
        "below" => AxisBelow::Below,
        "line" => AxisBelow::Line,
        _ => AxisBelow::Above,
    });
    Ok(())
}

/// Counts the tick helpers in use as the scale's own, as after a scale of
/// the program's installed them.
fn mark_default_helpers(owner: AxisOwner, name: String) -> PyResult<()> {
    match owner.axis(&name)? {
        AnyAxis::Flat(axis) => {
            axis.mark_default_tick_helpers();
        }
        AnyAxis::Deep(axis) => {
            axis.mark_default_tick_helpers();
        }
    }
    Ok(())
}

/// The categories of axis `name` of a 2-D or a 3-D axes, if it holds
/// category names.
fn categories(owner: &AxisOwner, name: &str) -> PyResult<Option<Categories>> {
    Ok(match owner.axis(name)? {
        AnyAxis::Flat(axis) => axis.categories(),
        AnyAxis::Deep(axis) => axis.categories(),
    })
}

/// Whether the axis holds category names.
fn has_categories(owner: AxisOwner, name: String) -> PyResult<bool> {
    Ok(categories(&owner, &name)?.is_some())
}

/// The category names of the axis, in the order of their positions.
fn category_names(owner: AxisOwner, name: String) -> PyResult<Vec<String>> {
    Ok(categories(&owner, &name)?
        .map(|categories| categories.names().to_vec())
        .unwrap_or_default())
}

/// Whether the axis converts units.
fn have_units(owner: AxisOwner, name: String) -> PyResult<bool> {
    Ok(match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.have_units(),
        AnyAxis::Deep(axis) => axis.have_units(),
    })
}

/// Takes the units `values` bring (`update_units`); whether they have
/// units.
fn update_units(owner: AxisOwner, name: String, values: PyAxisValues) -> PyResult<bool> {
    Ok(match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.update_units(values.0),
        AnyAxis::Deep(axis) => axis.update_units(values.0),
    })
}

/// The kind of the unit converter set on the axis itself (matplotlib's
/// `get_converter`, which a 2-D axis sharing another's converter does not
/// give): `"dates"`, `"category"`, `"custom"`, or `None`.
fn converter_kind(owner: AxisOwner, name: String) -> PyResult<Option<&'static str>> {
    let converter = match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.own_converter(),
        AnyAxis::Deep(axis) => axis.converter(),
    };
    Ok(converter.map(|converter| match converter {
        UnitConverter::Dates | UnitConverter::DatesBy(_) => "dates",
        UnitConverter::Categories => "category",
        UnitConverter::Custom(_) => "custom",
    }))
}

/// Gives axis `name` the converter `kind` explicitly (`set_converter`):
/// dates with `"auto"` or `"concise"` ticks, or `"category"` names.
fn set_converter(owner: AxisOwner, name: String, kind: String) -> PyResult<()> {
    let converter = match kind.as_str() {
        "category" => UnitConverter::Categories,
        dates => UnitConverter::DatesBy(
            dates
                .parse::<DateConverter>()
                .map_err(|err| PyErr::value_error(err.to_string()))?,
        ),
    };
    let set = match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis
            .set_converter(converter)
            .map(|_| ())
            .map_err(|e| e.to_string()),
        AnyAxis::Deep(axis) => axis
            .set_converter(converter)
            .map(|_| ())
            .map_err(|e| e.to_string()),
    };
    set.map_err(|err| PyErr::new(ExceptionKind::RuntimeError, err))
}

/// Ticks the axis, of a 2-D or a 3-D axes, as an axis of dates in the
/// time zone `tz` (`axis_date`, `zaxis_date`).
fn axis_date(owner: AxisOwner, name: String, tz: Option<Zone>) -> PyResult<()> {
    let tz = tz.as_ref().map(|zone| &zone.0);
    match owner.axis(&name)? {
        AnyAxis::Flat(axis) => {
            axis.axis_date(tz);
        }
        AnyAxis::Deep(axis) => {
            axis.axis_date(tz);
        }
    }
    Ok(())
}

/// Resets axis `name` to a fresh one (`Axis.clear`).
fn clear(owner: AxisOwner, name: String) -> PyResult<()> {
    match owner.axis(&name)? {
        AnyAxis::Flat(axis) => {
            axis.clear();
        }
        AnyAxis::Deep(axis) => {
            axis.clear();
        }
    }
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("clear", clear)?;
    module.add("is_default_helper", is_default_helper)?;
    module.add("is_default_label", is_default_label)?;
    module.add("set_label_text", set_label_text)?;
    module.add("minorticks", minorticks)?;
    module.add("get_remove_overlapping_locs", get_remove_overlapping_locs)?;
    module.add("set_remove_overlapping_locs", set_remove_overlapping_locs)?;
    module.add("get_tick_padding", get_tick_padding)?;
    module.add("get_labelpad", get_labelpad)?;
    module.add("set_labelpad", set_labelpad)?;
    module.add("set_label_coords", set_label_coords)?;
    module.add("get_in_layout", get_in_layout)?;
    module.add("set_in_layout", set_in_layout)?;
    module.add("get_tightbbox", get_tightbbox)?;
    module.add("tick_to", tick_to)?;
    module.add("set_offset_position", set_offset_position)?;
    module.add("request_autoscale", request_autoscale)?;
    module.add("get_axisbelow", get_axisbelow)?;
    module.add("set_axisbelow", set_axisbelow)?;
    module.add("mark_default_helpers", mark_default_helpers)?;
    module.add("has_categories", has_categories)?;
    module.add("category_names", category_names)?;
    module.add("have_units", have_units)?;
    module.add("update_units", update_units)?;
    module.add("converter_kind", converter_kind)?;
    module.add("set_converter", set_converter)?;
    module.add("axis_date", axis_date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn settings_read_back_by_axis_name() {
        let ax = Figure::new().subplot();
        let x = || "x".to_owned();
        let owner = || AxisOwner::Flat(ax.clone());
        assert!(is_default_helper(owner(), x(), "locator".into(), false).unwrap());
        minorticks(owner(), x(), true).unwrap();
        assert!(!is_default_helper(owner(), x(), "locator".into(), true).unwrap());
        assert!(is_default_helper(owner(), x(), "artist".into(), true).is_err());
        set_labelpad(owner(), "y".into(), 9.0).unwrap();
        assert_eq!(get_labelpad(owner(), "y".into()).unwrap(), 9.0);
        assert!(tick_to(ax.clone(), x(), "left".into()).is_err());
        set_axisbelow(ax.clone(), "line".into()).unwrap();
        assert_eq!(ax.axisbelow(), AxisBelow::Line);
        assert!(!has_categories(owner(), x()).unwrap());
        let names = || PyAxisValues(vec!["a".to_owned(), "b".to_owned()].into());
        assert!(update_units(owner(), x(), names()).unwrap());
        assert_eq!(category_names(owner(), x()).unwrap(), ["a", "b"]);
        assert_eq!(converter_kind(owner(), x()).unwrap(), Some("category"));
    }

    #[test]
    fn units_and_overlaps_of_3d_axes() {
        let ax = Figure::new().subplot_3d();
        let z = || "z".to_owned();
        let owner = || AxisOwner::Deep(ax.clone());
        assert!(!have_units(owner(), z()).unwrap());
        let names = PyAxisValues(vec!["low".to_owned(), "high".to_owned()].into());
        assert!(update_units(owner(), z(), names).unwrap());
        assert_eq!(category_names(owner(), z()).unwrap(), ["low", "high"]);
        assert_eq!(converter_kind(owner(), z()).unwrap(), Some("category"));
        assert!(get_remove_overlapping_locs(owner(), z()).unwrap());
        set_remove_overlapping_locs(owner(), z(), false).unwrap();
        assert!(!get_remove_overlapping_locs(owner(), z()).unwrap());
    }
}
