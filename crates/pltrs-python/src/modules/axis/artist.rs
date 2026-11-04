//! What an axis is as an artist, on 2-D and 3-D axes alike: its
//! visibility, the gids naming its SVG groups and those of its ticks
//! (matplotlib's `Axis.get_visible`, `Axis.get_gid` and `Tick.get_gid`;
//! the lines and texts of a tick and the axis' texts carry theirs on their
//! mirrors, see [`super::mirrors`]), and the placing of its `Tick`
//! objects.

use pltrs::axes::AxisGroup;

use super::owner::{AnyAxis, AxisOwner};
use crate::binding::prelude::*;

impl AnyAxis {
    /// The gid of `group` of the axis.
    pub(super) fn gid(&self, group: AxisGroup) -> Option<String> {
        match self {
            Self::Flat(axis) => axis.gid(group),
            Self::Deep(axis) => axis.gid(group),
        }
    }

    /// Gives `group` of the axis the gid `gid`, or takes it away.
    pub(super) fn set_gid(&self, group: AxisGroup, gid: Option<String>) {
        match self {
            Self::Flat(axis) => {
                axis.set_gid(group, gid);
            }
            Self::Deep(axis) => {
                axis.set_gid(group, gid);
            }
        }
    }
}

/// Whether the axis is drawn (a 3-D axis: whether tight boxes count it).
fn get_visible(owner: AxisOwner, name: String) -> PyResult<bool> {
    Ok(match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.visible(),
        AnyAxis::Deep(axis) => axis.visible(),
    })
}

/// Shows or hides the whole axis.
fn set_visible(owner: AxisOwner, name: String, visible: bool) -> PyResult<()> {
    match owner.axis(&name)? {
        AnyAxis::Flat(axis) => {
            axis.set_visible(visible);
        }
        AnyAxis::Deep(axis) => {
            axis.set_visible(visible);
        }
    }
    Ok(())
}

/// The gid of the axis' group.
fn get_gid(owner: AxisOwner, name: String) -> PyResult<Option<String>> {
    Ok(owner.axis(&name)?.gid(AxisGroup::Axis))
}

/// Names the axis' group `gid`, or numbers it (`None`).
fn set_gid(owner: AxisOwner, name: String, gid: Option<String>) -> PyResult<()> {
    owner.axis(&name)?.set_gid(AxisGroup::Axis, gid);
    Ok(())
}

/// The gid of the group of the major (or `minor`) tick at `index`.
fn get_tick_gid(
    owner: AxisOwner,
    name: String,
    minor: bool,
    index: usize,
) -> PyResult<Option<String>> {
    Ok(owner.axis(&name)?.gid(AxisGroup::Tick { minor, index }))
}

/// Names the group of the major (or `minor`) tick at `index` `gid`, or
/// numbers it (`None`).
fn set_tick_gid(
    owner: AxisOwner,
    name: String,
    minor: bool,
    index: usize,
    gid: Option<String>,
) -> PyResult<()> {
    owner
        .axis(&name)?
        .set_gid(AxisGroup::Tick { minor, index }, gid);
    Ok(())
}

/// Places the ticks of the axis, so that its `Tick` objects mirror them
/// (matplotlib's `Axis._update_ticks`); the ticks of a 3-D axis always
/// are, from its last layout, which this brings up to date, raising what
/// its locator or formatter raises.
fn place_ticks(owner: AxisOwner, name: String) -> PyResult<()> {
    match owner.axis(&name)? {
        AnyAxis::Flat(axis) => {
            axis.place_ticks()?;
        }
        AnyAxis::Deep(axis) => {
            axis.ticks()?;
        }
    }
    Ok(())
}

/// Places the ticks of the `minor` level of the axis (or the major one),
/// the other level's left as they were (matplotlib's `set_ticklabels`); a
/// 3-D axis places both. Returns how many ticks the level has, inside the
/// view or not (the locations matplotlib's `set_ticklabels` labels).
fn place_level(owner: AxisOwner, name: String, minor: bool) -> PyResult<usize> {
    let ticks = match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.place_level_ticks(minor)?.placed_ticks(),
        AnyAxis::Deep(axis) => Some(axis.ticks()?),
    };
    Ok(ticks.map_or(0, |ticks| {
        if minor {
            ticks.minor_unclipped.len()
        } else {
            ticks.major_unclipped.len()
        }
    }))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_visible", get_visible)?;
    module.add("set_visible", set_visible)?;
    module.add("get_gid", get_gid)?;
    module.add("set_gid", set_gid)?;
    module.add("get_tick_gid", get_tick_gid)?;
    module.add("set_tick_gid", set_tick_gid)?;
    module.add("place_ticks", place_ticks)?;
    module.add("place_level", place_level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn axes_of_both_kinds_keep_their_visibility_and_gids() {
        let fig = Figure::new();
        let owners = [
            AxisOwner::Flat(fig.subplot()),
            AxisOwner::Deep(fig.subplot_3d()),
        ];
        for owner in owners {
            let y = || "y".to_owned();
            set_visible(owner.clone(), y(), false).unwrap();
            assert!(!get_visible(owner.clone(), y()).unwrap());
            assert!(get_visible(owner.clone(), "x".into()).unwrap());
            set_gid(owner.clone(), y(), Some("height".into())).unwrap();
            assert_eq!(
                get_gid(owner.clone(), y()).unwrap().as_deref(),
                Some("height")
            );
            set_tick_gid(owner.clone(), y(), true, 2, Some("minor".into())).unwrap();
            assert_eq!(get_tick_gid(owner.clone(), y(), false, 2).unwrap(), None);
            let minor = get_tick_gid(owner.clone(), y(), true, 2).unwrap();
            assert_eq!(minor.as_deref(), Some("minor"));
            set_gid(owner.clone(), y(), None).unwrap();
            assert_eq!(get_gid(owner, y()).unwrap(), None);
        }
    }
}
