//! What the last tick update of an axis gave its tick helpers, for
//! `_tick_state`: matplotlib's formatters and locators keep what they
//! computed in `Axis._update_ticks` (a formatter's offset, the frequency
//! an `AutoDateLocator` picked), which the Python objects of the helpers
//! pltrs (Rust) ticks with natively compute again from it when read.

use pltrs::axes::Labelling;
use pltrs::ticker::AxisInfo;

use super::helpers::{TickHelper, tick_helpers};
use super::owner::{AnyAxis, AxisOwner};
use crate::binding::prelude::*;

/// The number of the tick update that placed the ticks of `axis` last,
/// when it labelled its major (or `minor`) level (`TickSet::labelled`).
fn level_labelled(axis: &AnyAxis, minor: bool) -> Option<u64> {
    match axis {
        AnyAxis::Flat(axis) => axis.labelled(minor),
        AnyAxis::Deep(axis) => axis.labelled(minor),
    }
}

/// Where `helper` is in use on axis `name` and when a tick update of the
/// axis last gave it work: whether its level is the minor one, and the
/// number of that update (`TickSet::labelled`). A formatter labels its
/// level; a locator places the ticks of both, as matplotlib's major
/// locator is called for the minor ticks too. `None` when the helper is
/// not in use on the axis, or no update gave it work.
fn labelled(owner: AxisOwner, name: String, helper: TickHelper) -> PyResult<Option<(bool, u64)>> {
    let axis = owner.axis(&name)?;
    let id = helper.id();
    let [
        major_locator,
        minor_locator,
        major_formatter,
        minor_formatter,
    ] = tick_helpers(&axis).map(|helper| helper.id() == id);
    let (major, minor) = (level_labelled(&axis, false), level_labelled(&axis, true));
    Ok(if minor_formatter {
        minor.map(|serial| (true, serial))
    } else if major_formatter {
        major.map(|serial| (false, serial))
    } else if minor_locator {
        minor.map(|serial| (true, serial))
    } else if major_locator {
        major.max(minor).map(|serial| (false, serial))
    } else {
        None
    })
}

/// What the last tick update of axis `name` gave the formatter of its
/// major (or `minor`) level: the locations it labelled and the
/// description of the axis then; `None` when it labelled none.
fn labelling(owner: AxisOwner, name: String, minor: bool) -> PyResult<Option<(NdArray, AxisInfo)>> {
    let labelling = match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.labelling(minor),
        AnyAxis::Deep(axis) => axis.labelling(minor),
    };
    Ok(labelling.map(|Labelling { locs, info, .. }| (NdArray::from_vec(locs), info)))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("labelled", labelled)?;
    module.add("labelling", labelling)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;
    use pltrs::ticker::NullLocator;

    #[test]
    fn helpers_are_given_work_by_the_updates_of_their_level() {
        let ax = Figure::new().subplot();
        // No minor ticks, which matplotlib's updates then leave unlabelled.
        ax.xaxis().set_minor_locator(NullLocator);
        let owner = || AxisOwner::Flat(ax.clone());
        let formatter = TickHelper::Formatter(ax.xaxis().major_formatter());
        assert_eq!(
            labelled(owner(), "x".into(), formatter.clone()).unwrap(),
            None
        );
        ax.xaxis().place_ticks().unwrap();
        let (minor, serial) = labelled(owner(), "x".into(), formatter.clone())
            .unwrap()
            .expect("the major ticks were labelled");
        assert!(!minor);
        let locator = TickHelper::Locator(ax.xaxis().major_locator());
        assert_eq!(
            labelled(owner(), "x".into(), locator).unwrap(),
            Some((false, serial))
        );
        let other = TickHelper::Formatter(ax.yaxis().major_formatter());
        assert_eq!(labelled(owner(), "x".into(), other).unwrap(), None);
        let (locs, info) = labelling(owner(), "x".into(), false)
            .unwrap()
            .expect("the major ticks were labelled");
        assert_eq!(locs.data, ax.xaxis().ticklocs(false).unwrap());
        assert_eq!(info.major_ticks, locs.data);
        assert!(labelling(owner(), "x".into(), true).unwrap().is_none());
    }
}
