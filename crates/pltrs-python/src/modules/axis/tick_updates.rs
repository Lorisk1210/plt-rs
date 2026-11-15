//! The program's tick updates: pltrs (Rust) has the axes whose tick
//! helpers or scale are Python's ticked by the figure's updater
//! ([`pltrs::axes::TickUpdater`]) at matplotlib's call sites, lent the
//! figure. The updater is `_tick_updates`'s, which runs matplotlib's
//! `Axis._update_ticks` with the program's objects: `update(axes, name,
//! info, major, minor, quiet, called)` returns the locations, labels and
//! drawn flags of each level computed (None for the others; *called* says
//! whether matplotlib calls `_update_ticks` there), `offset(axes, name, info)`
//! the major formatter's offset text, `spines(axes)` the spines of an
//! axes of a projection pltrs (Rust) does not know, `(name, shown, axis
//! name or None)`, and `inverted(axes, name)` whether an axis whose class
//! answers its view interval itself is inverted.

use pltrs::axes::{
    Axes, SpineAxis, TickLevels, TickUpdater, UpdateSite, UpdatedLevel, UpdatedTicks,
};
use pltrs::figure::Figure;
use pltrs::ticker::{AxisInfo, AxisName, TickerError};

use crate::binding::prelude::*;

/// One level of an update as Python gives it: `(locs, labels, drawn)`.
type Level = (Vec<f64>, Vec<String>, Vec<bool>);

/// The figure's updater, calling the Python one's methods.
#[derive(Debug)]
struct PyTickUpdater {
    update: Callback,
    locate: Callback,
    offset: Callback,
    spines: Callback,
    inverted: Callback,
}

impl TickUpdater for PyTickUpdater {
    /// The update's levels; none when the Python update raised (its
    /// exception surfaces from the native call that drew).
    fn update_ticks(
        &self,
        axes: &Axes,
        axis: AxisName,
        info: &AxisInfo,
        levels: TickLevels,
        site: UpdateSite,
    ) -> Result<UpdatedTicks, TickerError> {
        let level = |(locs, labels, drawn): Level| UpdatedLevel {
            locs,
            labels,
            drawn,
        };
        Ok(self
            .update
            .call::<(Option<Level>, Option<Level>)>((
                axes.clone(),
                axis.name(),
                info.clone(),
                levels.major,
                levels.minor,
                pltrs::warnings::capturing(),
                site == UpdateSite::Matplotlib,
            ))
            .map(|(major, minor)| UpdatedTicks {
                major: major.map(level),
                minor: minor.map(level),
            })
            .unwrap_or_default())
    }

    /// The Python updater's `locate(axes, name, info)` (its exception
    /// surfaces from the native call that picked).
    fn locate(&self, axes: &Axes, axis: AxisName, info: &AxisInfo) {
        let _ = self
            .locate
            .call::<Owned>((axes.clone(), axis.name(), info.clone()));
    }

    fn offset(&self, axes: &Axes, axis: AxisName, info: &AxisInfo) -> String {
        self.offset
            .call((axes.clone(), axis.name(), info.clone()))
            .unwrap_or_default()
    }

    fn spines(&self, axes: &Axes) -> Vec<SpineAxis> {
        let named = |name: Option<String>| match name.as_deref() {
            Some("x") => Some(AxisName::X),
            Some("y") => Some(AxisName::Y),
            _ => None,
        };
        self.spines
            .call::<Vec<(String, bool, Option<String>)>>((axes.clone(),))
            .unwrap_or_default()
            .into_iter()
            .map(|(name, shown, axis)| SpineAxis {
                name,
                shown,
                axis: named(axis),
            })
            .collect()
    }

    /// The Python updater's `inverted(axes, name)`: the axis'
    /// `get_inverted()`, None once it is gone (or when it raised: its
    /// exception surfaces from the native call laying out).
    fn inverted(&self, axes: &Axes, axis: AxisName) -> Option<bool> {
        self.inverted
            .call::<Option<bool>>((axes.clone(), axis.name()))
            .flatten()
    }
}

/// Has the methods `update`, `locate`, `offset`, `spines` and `inverted` of
/// `updater` tick the
/// axes of `figure` whose helpers or scale are Python's (None removes it).
fn set_tick_updater(figure: Figure, updater: Option<Owned>) -> PyResult<()> {
    let updater = updater
        .map(|updater| {
            Python::with_gil(|py| {
                let updater = updater.bind(py);
                Ok::<_, PyErr>(PyTickUpdater {
                    update: updater.getattr("update")?.extract()?,
                    locate: updater.getattr("locate")?.extract()?,
                    offset: updater.getattr("offset")?.extract()?,
                    spines: updater.getattr("spines")?.extract()?,
                    inverted: updater.getattr("inverted")?.extract()?,
                })
            })
        })
        .transpose()?;
    figure.set_tick_updater(updater.map(|updater| std::rc::Rc::new(updater) as _));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_tick_updater", set_tick_updater)
}
