//! The intervals of an axis (matplotlib's `get_view_interval`,
//! `set_view_interval`, `get_data_interval`, `set_data_interval`,
//! `get_minpos`, `get_tick_space`) and the description tick helpers
//! consult: the live one of an axis, and the fields of the one pltrs
//! (Rust) hands a Python helper while it draws.

use pltrs::axes::Axes;
use pltrs::ticker::AxisInfo;

use super::axis;
use super::owner::{AnyAxis, AxisOwner};
use crate::binding::FloatArray;
use crate::binding::prelude::*;

/// The view limits, in the order they were set.
fn get_view_interval(axes: Axes, name: String) -> PyResult<(f64, f64)> {
    Ok(axis(&axes, &name)?.view_interval())
}

/// Sets the view limits (`ignore`) or widens them to include `vmin` and
/// `vmax`.
fn set_view_interval(axes: Axes, name: String, vmin: f64, vmax: f64, ignore: bool) -> PyResult<()> {
    axis(&axes, &name)?.set_view_interval(vmin, vmax, ignore);
    Ok(())
}

/// The interval the data span.
fn get_data_interval(axes: Axes, name: String) -> PyResult<(f64, f64)> {
    Ok(axis(&axes, &name)?.data_interval())
}

/// Sets the data interval (`ignore`) or widens it to include `vmin` and
/// `vmax`.
fn set_data_interval(axes: Axes, name: String, vmin: f64, vmax: f64, ignore: bool) -> PyResult<()> {
    axis(&axes, &name)?.set_data_interval(vmin, vmax, ignore);
    Ok(())
}

/// The smallest positive datum.
fn get_minpos(axes: Axes, name: String) -> PyResult<f64> {
    Ok(axis(&axes, &name)?.minpos())
}

/// How many ticks fit along the axis; matplotlib's `ValueError` for an
/// axis whose length is no number.
fn get_tick_space(axes: Axes, name: String) -> PyResult<i64> {
    Ok(axis(&axes, &name)?.tick_space().count()?)
}

/// How many ticks fit along the axis the description `info` describes.
fn info_tick_space(info: AxisInfo) -> PyResult<i64> {
    Ok(info.tick_space.count()?)
}

/// What the helpers of axis `name` know of it (see
/// [`pltrs::axes::Axis::info`] and [`pltrs::mplot3d::Axis3D::info`]).
fn axis_info(owner: AxisOwner, name: String) -> PyResult<AxisInfo> {
    Ok(match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.info()?,
        AnyAxis::Deep(axis) => axis.info()?,
    })
}

/// The fields of an axis description: view and data intervals, smallest
/// positive datum and major tick locations (its tick space is
/// `info_tick_space`'s, which may fail).
type InfoFields = ((f64, f64), (f64, f64), f64, NdArray);

/// The fields of the axis description `info`.
fn info_fields(info: AxisInfo) -> PyResult<InfoFields> {
    Ok((
        info.view_interval,
        info.data_interval,
        info.minpos,
        NdArray::from_vec(info.major_ticks),
    ))
}

/// The axis description `info` with the major tick locations `major`,
/// which minor locators place their ticks between (matplotlib's
/// `AutoMinorLocator` reads them from the axis).
fn with_major_ticks(mut info: AxisInfo, major: FloatArray) -> PyResult<AxisInfo> {
    info.major_ticks = major.0.into_vec()?;
    Ok(info)
}

/// `info` with the view interval `(low, high)`, as the program's class of
/// the axis answers it (its `get_view_interval`).
fn with_view_interval(mut info: AxisInfo, low: f64, high: f64) -> PyResult<AxisInfo> {
    info.view_interval = (low, high);
    Ok(info)
}

/// `values` through the scale of axis `name` (matplotlib's
/// `axis.get_transform().transform(values)`).
fn scaled(axes: Axes, name: String, values: NdArray) -> PyResult<NdArray> {
    let scaled = axis(&axes, &name)?.scaled(&values.data);
    NdArray::with_shape(scaled, values.shape)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("scaled", scaled)?;
    module.add("with_major_ticks", with_major_ticks)?;
    module.add("with_view_interval", with_view_interval)?;
    module.add("get_view_interval", get_view_interval)?;
    module.add("set_view_interval", set_view_interval)?;
    module.add("get_data_interval", get_data_interval)?;
    module.add("set_data_interval", set_data_interval)?;
    module.add("get_minpos", get_minpos)?;
    module.add("get_tick_space", get_tick_space)?;
    module.add("info_tick_space", info_tick_space)?;
    module.add("axis_info", axis_info)?;
    module.add("info_fields", info_fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn intervals_cross_by_axis_name() {
        let ax = Figure::new().subplot();
        set_view_interval(ax.clone(), "x".into(), 2.0, -1.0, true).unwrap();
        assert_eq!(
            get_view_interval(ax.clone(), "x".into()).unwrap(),
            (2.0, -1.0)
        );
        set_data_interval(ax.clone(), "y".into(), 1.0, 3.0, true).unwrap();
        assert_eq!(
            get_data_interval(ax.clone(), "y".into()).unwrap(),
            (1.0, 3.0)
        );
        assert!(get_tick_space(ax.clone(), "x".into()).unwrap() > 0);
        let info = axis_info(AxisOwner::Flat(ax), "x".into()).unwrap();
        assert!(info_tick_space(info.clone()).unwrap() > 0);
        let (view, _, _, major) = info_fields(info).unwrap();
        assert_eq!(view, (2.0, -1.0));
        assert_ne!(major.into_vec().unwrap(), [] as [f64; 0]);
    }
}
