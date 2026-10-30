//! The tick helpers of an axis as Python sees them (matplotlib's
//! `axis.major.locator`, `get_major_formatter`, `set_minor_locator`, ...).
//!
//! pltrs (Rust) keeps the locators and formatters of each axis, shared
//! between the axes sharing it. Python refers to one through a token, a
//! [`TickHelper`] holding the shared helper, whose identity (`id`, the
//! helper's address, unique while a token holds it) lets `_tick_helpers`
//! give back the same Python object every time. A Python helper is
//! installed either from its spec, as the pltrs (Rust) helper of that kind
//! (`ticker._spec()`), or through an adapter calling it
//! ([`super::python_helpers`]). A helper pltrs (Rust) installed by itself
//! is described by its spec ([`describe`]), or computed through its token
//! when it has no Python counterpart.

use std::rc::Rc;

use pltrs::axes::{Axes, Axis};
use pltrs::ticker::{Formatter, Locator, ScalarFormatter, TickerSettings};
use pltrs::units::UnitAxisInfo;

use super::axis;
use super::python_helpers::{PyFormatter, PyLocator};
use crate::binding::prelude::*;
use crate::modules::ticker;

/// A locator or formatter in use on an axis.
#[derive(Clone, Debug)]
pub(crate) enum TickHelper {
    /// A locator.
    Locator(Rc<dyn Locator>),
    /// A formatter.
    Formatter(Rc<dyn Formatter>),
}

native_value!(TickHelper);

impl TickHelper {
    /// The helper's identity: its address, unique while it lives.
    fn id(&self) -> usize {
        match self {
            Self::Locator(locator) => Rc::as_ptr(locator).cast::<()>().addr(),
            Self::Formatter(formatter) => Rc::as_ptr(formatter).cast::<()>().addr(),
        }
    }

    /// The locator, or `TypeError` for a formatter.
    fn locator(&self) -> PyResult<&Rc<dyn Locator>> {
        match self {
            Self::Locator(locator) => Ok(locator),
            Self::Formatter(_) => Err(PyErr::type_error("expected a locator")),
        }
    }

    /// The formatter, or `TypeError` for a locator.
    fn formatter(&self) -> PyResult<&Rc<dyn Formatter>> {
        match self {
            Self::Formatter(formatter) => Ok(formatter),
            Self::Locator(_) => Err(PyErr::type_error("expected a formatter")),
        }
    }
}

/// The identity and token of `helper`.
type Identified = (usize, TickHelper);

/// `helper` with its identity.
fn identified(helper: TickHelper) -> Identified {
    (helper.id(), helper)
}

/// The locator (`kind` `"locator"`) or formatter (`"formatter"`) of the
/// major or `minor` ticks of axis `name`, with its identity.
fn helper(axes: Axes, name: String, kind: String, minor: bool) -> PyResult<Identified> {
    let axis = axis(&axes, &name)?;
    let helper = match (kind.as_str(), minor) {
        ("locator", false) => TickHelper::Locator(axis.major_locator()),
        ("locator", true) => TickHelper::Locator(axis.minor_locator()),
        ("formatter", false) => TickHelper::Formatter(axis.major_formatter()),
        ("formatter", true) => TickHelper::Formatter(axis.minor_formatter()),
        _ => return Err(PyErr::value_error(format!("no tick helper kind {kind:?}"))),
    };
    Ok(identified(helper))
}

/// Whether anything but tokens still uses `helper`.
fn in_use(helper: TickHelper) -> PyResult<bool> {
    let count = match &helper {
        TickHelper::Locator(locator) => Rc::strong_count(locator),
        TickHelper::Formatter(formatter) => Rc::strong_count(formatter),
    };
    // The argument and the token it was taken from hold two references.
    Ok(count > 2)
}

/// The spec of `helper`: `{"kind": "python", "adapter": ...}` for one
/// installed from Python through an adapter, the spec of its kind for
/// one of pltrs (Rust), `None` for one without a Python counterpart.
fn describe(helper: TickHelper) -> PyResult<Option<Owned>> {
    Python::with_gil(|py| {
        let any: &dyn std::any::Any = match &helper {
            TickHelper::Locator(locator) => locator.as_ref(),
            TickHelper::Formatter(formatter) => formatter.as_ref(),
        };
        let adapter = (any.downcast_ref::<PyLocator>().map(|l| &l.adapter))
            .or_else(|| any.downcast_ref::<PyFormatter>().map(|f| &f.adapter));
        if let Some(adapter) = adapter {
            let spec = crate::binding::new_dict(py)?;
            spec.set_item("kind", "python")?;
            spec.set_item("adapter", adapter)?;
            return Ok(Some(spec.unbind()));
        }
        let spec = match &helper {
            TickHelper::Locator(locator) => ticker::describe::locator(py, locator.as_ref())?,
            TickHelper::Formatter(formatter) => {
                ticker::describe::formatter(py, formatter.as_ref())?
            }
        };
        Ok(spec.map(Bound::unbind))
    })
}

/// The locator a Python helper describes: built from its spec when
/// `native`, else calling its adapter.
fn new_locator(description: &Owned, native: bool) -> PyResult<Rc<dyn Locator>> {
    Python::with_gil(|py| {
        let description = description.bind(py);
        Ok(if native {
            Rc::from(ticker::locator(&description)?)
        } else {
            Rc::new(PyLocator::of(&description)?) as Rc<dyn Locator>
        })
    })
}

/// The formatter a Python helper describes, as [`new_locator`].
fn new_formatter(description: &Owned, native: bool) -> PyResult<Rc<dyn Formatter>> {
    Python::with_gil(|py| {
        let description = description.bind(py);
        Ok(if native {
            Rc::from(ticker::formatter(&description)?)
        } else {
            Rc::new(PyFormatter::of(&description)?) as Rc<dyn Formatter>
        })
    })
}

/// Installs the locator `description` describes (see [`new_locator`]) on
/// the major or `minor` ticks of axis `name`, as matplotlib's
/// `set_major_locator` does; with `replacing`, in place of that locator
/// wherever it is in use instead, as a helper changed in place. Returns
/// the locator in use and its identity, `None` when `replacing` was in
/// use nowhere.
fn install_locator(
    axes: Axes,
    name: String,
    minor: bool,
    description: Owned,
    native: bool,
    replacing: Option<TickHelper>,
) -> PyResult<Option<Identified>> {
    let axis = axis(&axes, &name)?;
    let locator = new_locator(&description, native)?;
    let installed = match replacing {
        Some(old) => axis.replace_locator(old.locator()?, &locator),
        None if minor => Some(axis.set_shared_minor_locator(&locator).minor_locator()),
        None => Some(axis.set_shared_major_locator(&locator).major_locator()),
    };
    Ok(installed.map(|locator| identified(TickHelper::Locator(locator))))
}

/// Installs the formatter `description` describes, as [`install_locator`]
/// does locators.
fn install_formatter(
    axes: Axes,
    name: String,
    minor: bool,
    description: Owned,
    native: bool,
    replacing: Option<TickHelper>,
) -> PyResult<Option<Identified>> {
    let axis = axis(&axes, &name)?;
    let formatter = new_formatter(&description, native)?;
    let installed = match replacing {
        Some(old) => axis
            .replace_formatter(old.formatter()?, &formatter)
            .then_some(formatter),
        None if minor => Some(
            axis.set_shared_minor_formatter(&formatter)
                .minor_formatter(),
        ),
        None => Some(
            axis.set_shared_major_formatter(&formatter)
                .major_formatter(),
        ),
    };
    Ok(installed.map(|formatter| identified(TickHelper::Formatter(formatter))))
}

/// The four tick helpers of `axis`: the major and minor locators, then
/// the major and minor formatters.
fn tick_helpers(axis: &Axis) -> [TickHelper; 4] {
    [
        TickHelper::Locator(axis.major_locator()),
        TickHelper::Locator(axis.minor_locator()),
        TickHelper::Formatter(axis.major_formatter()),
        TickHelper::Formatter(axis.minor_formatter()),
    ]
}

/// Takes the tick helpers, label and default view a unit converter of
/// the program's gives axis `name` (matplotlib's `_update_axisinfo`, see
/// `Axis::take_axis_info`). `helpers` holds a description for each of
/// the four helpers of [`tick_helpers`], as [`install_locator`] takes
/// them, or `None`. Returns, for each helper given, the identity of the
/// helper in use if it was installed, `None` where the axis kept its own.
fn use_axis_info(
    axes: Axes,
    name: String,
    helpers: Vec<Option<(Owned, bool)>>,
    label: Option<String>,
    default_limits: Option<(f64, f64)>,
) -> PyResult<Vec<Option<Identified>>> {
    let [majloc, minloc, majfmt, minfmt] = <[_; 4]>::try_from(helpers).map_err(|helpers| {
        PyErr::value_error(format!("expected 4 tick helpers, got {}", helpers.len()))
    })?;
    let locator = |helper: Option<(Owned, bool)>| {
        helper
            .map(|(description, native)| new_locator(&description, native))
            .transpose()
    };
    let formatter = |helper: Option<(Owned, bool)>| {
        helper
            .map(|(description, native)| new_formatter(&description, native))
            .transpose()
    };
    let info = UnitAxisInfo {
        majloc: locator(majloc)?,
        minloc: locator(minloc)?,
        majfmt: formatter(majfmt)?,
        minfmt: formatter(minfmt)?,
        label,
        default_limits,
    };
    let given = [
        info.majloc.is_some(),
        info.minloc.is_some(),
        info.majfmt.is_some(),
        info.minfmt.is_some(),
    ];
    let axis = axis(&axes, &name)?;
    let before = tick_helpers(&axis);
    axis.take_axis_info(&info);
    Ok(tick_helpers(&axis)
        .into_iter()
        .zip(before)
        .zip(given)
        .map(|((after, before), given)| {
            (given && after.id() != before.id()).then(|| identified(after))
        })
        .collect())
}

/// The ticks of the locator `helper` on the axis `info` (`__call__`).
fn helper_locate(helper: TickHelper, info: Option<Owned>) -> PyResult<NdArray> {
    let locator = Rc::clone(helper.locator()?);
    ticker::with_context(info, |ctx| Ok(NdArray::from_vec(locator.locate(ctx)?)))
}

/// `tick_values(vmin, vmax)` of the locator `helper` on the axis `info`.
fn helper_tick_values(
    helper: TickHelper,
    vmin: f64,
    vmax: f64,
    info: Option<Owned>,
) -> PyResult<NdArray> {
    let locator = Rc::clone(helper.locator()?);
    ticker::with_context(info, |ctx| {
        Ok(NdArray::from_vec(locator.tick_values(vmin, vmax, ctx)?))
    })
}

/// The labels and offset text the formatter `helper` gives `values` on
/// the axis `info` (`format_ticks`, then `get_offset`).
fn helper_format_ticks(
    helper: TickHelper,
    values: NdArray,
    info: Option<Owned>,
) -> PyResult<(Vec<String>, String)> {
    let formatter = Rc::clone(helper.formatter()?);
    ticker::with_context(info, |ctx| {
        let formatted = formatter.format_ticks(&values.data, ctx)?;
        Ok((formatted.labels, formatted.offset))
    })
}

/// `format_data(value)` (`short`: `format_data_short(value)`) of the
/// formatter `helper` on the axis `info`.
fn helper_format_data(
    helper: TickHelper,
    value: f64,
    short: bool,
    info: Option<Owned>,
) -> PyResult<String> {
    let formatter = Rc::clone(helper.formatter()?);
    ticker::with_context(info, |ctx| {
        Ok(if short {
            formatter.format_data_short(value, ctx)?
        } else {
            formatter.format_data(value, ctx)?
        })
    })
}

/// The value `value` along axis `name` as a `ScalarFormatter` writes it
/// for the cursor: with as many digits as a screen pixel resolves around
/// it (matplotlib's `format_data_short` on an axis); `None` while the
/// axes has no extent.
fn format_data_short(axes: Axes, name: String, value: f64) -> PyResult<Option<String>> {
    let name = axis(&axes, &name)?.name();
    let settings = TickerSettings::from(&*pltrs::rc::params());
    axes.pixel_resolution(name, value)
        .map(|resolution| ScalarFormatter::format_data_resolved(value, resolution, &settings))
        .transpose()
        .map_err(PyErr::from)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("helper", helper)?;
    module.add("helper_in_use", in_use)?;
    module.add("describe_helper", describe)?;
    module.add("install_locator", install_locator)?;
    module.add("install_formatter", install_formatter)?;
    module.add("use_axis_info", use_axis_info)?;
    module.add("helper_locate", helper_locate)?;
    module.add("helper_tick_values", helper_tick_values)?;
    module.add("helper_format_ticks", helper_format_ticks)?;
    module.add("helper_format_data", helper_format_data)?;
    module.add("format_data_short", format_data_short)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::with_python;
    use pltrs::figure::Figure;

    #[test]
    fn helpers_keep_their_identity_until_replaced() {
        let ax = Figure::new().subplot();
        let (id, token) = helper(ax.clone(), "x".into(), "locator".into(), false).unwrap();
        let (again, _) = helper(ax.clone(), "x".into(), "locator".into(), false).unwrap();
        assert_eq!(id, again);
        assert!(in_use(token.clone()).unwrap());
        let spec = with_python(|py| {
            let spec = crate::binding::new_dict(py).unwrap();
            spec.set_item("kind", "null").unwrap();
            spec.unbind()
        });
        let (new_id, _) = install_locator(ax.clone(), "x".into(), false, spec, true, Some(token))
            .unwrap()
            .expect("the default locator was in use");
        assert_ne!(new_id, id);
        assert!(ax.xaxis().is_default_major_locator());
        assert_eq!(ax.xaxis().ticklocs(false).unwrap(), Vec::<f64>::new());
        assert!(helper(ax, "x".into(), "artist".into(), false).is_err());
    }
}
