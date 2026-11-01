//! What an axis keeps of the artists matplotlib makes it of: the group
//! ids of the axis, of its label and offset text, of its `Tick`s and of
//! their lines and labels in SVG output (their `get_gid`), and whether
//! its `Tick` objects were ever placed at the ticks.
//!
//! matplotlib draws an axis as a tree of artists, each opening a group of
//! its own: the axis (`matplotlib.axis`, or `axis3d` on 3-D axes), each
//! tick in it (`xtick`, `ytick`), and each visible line (`line2d`) or
//! non-empty text (`text`) of the tick, then the axis label and the offset
//! text. The SVG writer names a group after the gid of its artist, or
//! numbers it per group name. pltrs draws an axis from its state, so the
//! axis keeps the gids of those artists here, and draws the same groups.
//!
//! matplotlib's `Tick` objects sit at location 0 without a label until
//! `Axis._update_ticks` places them, which drawing the axis, measuring it
//! and reading its tick labels do; the ticks of a hidden axis, which is
//! neither drawn nor measured, stay there. The axis records whether that
//! happened, for the objects mirroring its ticks.

use std::collections::BTreeMap;

use super::Axis;
use super::tick_set::TickSet;
use crate::artist::DrawContext;
use crate::ticker::AxisName;

/// A part of a tick with a group of its own, in the order matplotlib's
/// `Tick.draw` draws them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TickPart {
    /// The gridline (`Tick.gridline`).
    Gridline,
    /// The mark on the first (bottom or left) side (`Tick.tick1line`).
    Tick1Line,
    /// The mark on the second (top or right) side (`Tick.tick2line`).
    Tick2Line,
    /// The label on the first side (`Tick.label1`).
    Label1,
    /// The label on the second side (`Tick.label2`).
    Label2,
}

pltrs_core::archive_enum!(TickPart {
    Gridline,
    Tick1Line,
    Tick2Line,
    Label1,
    Label2
});

impl TickPart {
    /// The group the SVG writer numbers the part in when it has no gid:
    /// `line2d` for a line, `text` for a label.
    pub const fn group_name(self) -> &'static str {
        match self {
            Self::Gridline | Self::Tick1Line | Self::Tick2Line => "line2d",
            Self::Label1 | Self::Label2 => "text",
        }
    }

    /// The mark on the first or the `second` side.
    pub const fn mark(second: bool) -> Self {
        if second {
            Self::Tick2Line
        } else {
            Self::Tick1Line
        }
    }

    /// The label on the first or the `second` side.
    pub const fn label(second: bool) -> Self {
        if second { Self::Label2 } else { Self::Label1 }
    }
}

/// An artist of an axis that draws in a group of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AxisGroup {
    /// The axis itself.
    Axis,
    /// The axis label.
    Label,
    /// The offset (or scale) text.
    OffsetText,
    /// The major (or `minor`) tick at `index` among those its locator
    /// places, as matplotlib's `Tick` objects are reused in that order.
    Tick {
        /// Whether the tick is a minor one.
        minor: bool,
        /// Its position among the ticks of its level.
        index: usize,
    },
    /// A part of that tick.
    TickPart {
        /// Whether the tick is a minor one.
        minor: bool,
        /// Its position among the ticks of its level.
        index: usize,
        /// Which part.
        part: TickPart,
    },
}

pltrs_core::archive_enum!(AxisGroup {
    Axis,
    Label,
    OffsetText,
    Tick { minor, index },
    TickPart { minor, index, part }
});

/// The gids an axis' artists were given, by group.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AxisGids(BTreeMap<AxisGroup, String>);

pltrs_core::archive_struct!(AxisGids(groups));

impl AxisGids {
    /// The gid of `group`, if it has one.
    pub(crate) fn get(&self, group: AxisGroup) -> Option<&str> {
        self.0.get(&group).map(String::as_str)
    }

    /// Gives `group` the gid `gid`, or takes its gid away (`None`).
    pub(crate) fn set(&mut self, group: AxisGroup, gid: Option<String>) {
        match gid {
            Some(gid) => self.0.insert(group, gid),
            None => self.0.remove(&group),
        };
    }

    /// Draws `draw` inside the group `name`, named after the gid of
    /// `group` if it has one (matplotlib's `open_group(name, gid)`).
    pub(crate) fn grouped<R>(
        &self,
        ctx: &mut DrawContext<'_>,
        name: &str,
        group: AxisGroup,
        draw: impl FnOnce(&mut DrawContext<'_>) -> R,
    ) -> R {
        ctx.renderer().open_group(name, self.get(group));
        let drawn = draw(ctx);
        ctx.renderer().close_group(name);
        drawn
    }

    /// [`AxisGids::grouped`] for a part of the axis (a tick, a line or
    /// label of one, the axis label, the offset text), which draws with
    /// the axis' clip regions: matplotlib clips the gridlines of an axis
    /// to one box (see `Renderer::open_part_group`).
    pub(crate) fn part_grouped<R>(
        &self,
        ctx: &mut DrawContext<'_>,
        name: &str,
        group: AxisGroup,
        draw: impl FnOnce(&mut DrawContext<'_>) -> R,
    ) -> R {
        ctx.renderer().open_part_group(name, self.get(group));
        let drawn = draw(ctx);
        ctx.renderer().close_part_group(name);
        drawn
    }

    /// Draws `draw` inside the group of the tick of axis `name` at `key`
    /// (see [`TickSet::drawn_key`]), handing it the groups of the tick's
    /// parts (matplotlib's `Tick.draw`).
    pub(crate) fn tick<R>(
        &self,
        ctx: &mut DrawContext<'_>,
        name: &str,
        key: (bool, usize),
        draw: impl FnOnce(&mut DrawContext<'_>, TickGroups<'_>) -> R,
    ) -> R {
        let (minor, index) = key;
        let groups = TickGroups { gids: self, key };
        self.part_grouped(ctx, name, AxisGroup::Tick { minor, index }, |ctx| {
            draw(ctx, groups)
        })
    }
}

/// The groups of the parts of one drawn tick.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TickGroups<'a> {
    gids: &'a AxisGids,
    /// The level and index of the tick (see [`TickSet::drawn_key`]).
    key: (bool, usize),
}

impl TickGroups<'_> {
    /// Draws `draw` inside the group of `part` (matplotlib's
    /// `Line2D.draw` and `Text.draw` opening theirs).
    pub(crate) fn part<R>(
        &self,
        ctx: &mut DrawContext<'_>,
        part: TickPart,
        draw: impl FnOnce(&mut DrawContext<'_>) -> R,
    ) -> R {
        let (minor, index) = self.key;
        let group = AxisGroup::TickPart { minor, index, part };
        self.gids.part_grouped(ctx, part.group_name(), group, draw)
    }
}

/// The group a tick of axis `name` draws in when it has no gid
/// (matplotlib's `XTick.__name__` and `YTick.__name__`; every tick of a
/// 3-D axis is an `XTick`).
pub(crate) const fn tick_group_name(name: AxisName) -> &'static str {
    match name {
        AxisName::X => "xtick",
        AxisName::Y => "ytick",
    }
}

impl TickSet {
    /// The level and the index among the ticks its locator places of the
    /// tick at `position` among those in view, the major ones followed by
    /// the minor ones (the tick whose `Tick` object matplotlib draws
    /// there).
    pub(crate) fn drawn_key(&self, position: usize) -> Option<(bool, usize)> {
        match position.checked_sub(self.major_index.len()) {
            None => Some((false, self.major_index[position])),
            Some(minor) => Some((true, *self.minor_index.get(minor)?)),
        }
    }
}

impl Axis {
    /// The gid of `group` in SVG output, which names the group it draws
    /// in: the axis, its label, its offset text, one of its ticks or a
    /// part of one (matplotlib's `get_gid` of those artists).
    ///
    /// ```
    /// use pltrs::axes::{AxisGroup, TickPart};
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let label = AxisGroup::TickPart { minor: false, index: 1, part: TickPart::Label1 };
    /// ax.xaxis().set_gid(AxisGroup::Axis, Some("time".into()));
    /// ax.xaxis().set_gid(label, Some("second-label".into()));
    /// assert_eq!(ax.xaxis().gid(AxisGroup::Axis).as_deref(), Some("time"));
    /// let svg = fig.to_svg()?;
    /// assert!(svg.contains("<g id=\"time\">") && svg.contains("<g id=\"second-label\">"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn gid(&self, group: AxisGroup) -> Option<String> {
        self.read(|axis| axis.gids.get(group).map(str::to_owned))
    }

    /// Gives `group` the gid `gid` (see [`Axis::gid`]), or takes its gid
    /// away (`None`).
    pub fn set_gid(&self, group: AxisGroup, gid: Option<String>) -> &Self {
        self.update(|axis| axis.gids.set(group, gid));
        self
    }

    /// Whether the axis' ticks were ever placed: by drawing or measuring
    /// the axis while it is shown, or by [`Axis::place_ticks`]
    /// (matplotlib's `Tick` objects sit at 0 until then).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.xaxis().set_visible(false);
    /// assert!(!ax.yaxis().ticks_placed());
    /// fig.to_svg()?;
    /// assert!(ax.yaxis().ticks_placed() && !ax.xaxis().ticks_placed());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn ticks_placed(&self) -> bool {
        self.read(|axis| axis.ticks_placed.get())
    }

    /// Places the axis' ticks (matplotlib's `Axis._update_ticks`, which
    /// reading the tick labels runs).
    pub fn place_ticks(&self) -> &Self {
        self.read(|axis| axis.ticks_placed.set(true));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gids_are_kept_per_group_until_taken_away() {
        let mut gids = AxisGids::default();
        let tick = AxisGroup::Tick {
            minor: false,
            index: 2,
        };
        gids.set(tick, Some("third".into()));
        gids.set(AxisGroup::Axis, Some("axis".into()));
        assert_eq!(gids.get(tick), Some("third"));
        assert_eq!(
            gids.get(AxisGroup::Tick {
                minor: true,
                index: 2
            }),
            None
        );
        gids.set(tick, None);
        assert_eq!(gids.get(tick), None);
        assert_eq!(gids.get(AxisGroup::Axis), Some("axis"));
    }

    #[test]
    fn parts_group_as_lines_or_texts() {
        assert_eq!(TickPart::mark(true).group_name(), "line2d");
        assert_eq!(TickPart::Gridline.group_name(), "line2d");
        assert_eq!(TickPart::label(false).group_name(), "text");
        assert_eq!(tick_group_name(AxisName::Y), "ytick");
    }

    #[test]
    fn drawn_ticks_are_keyed_by_their_level_and_index() {
        let ticks = TickSet {
            major_index: vec![1, 2],
            minor_index: vec![0, 4],
            ..TickSet::default()
        };
        assert_eq!(ticks.drawn_key(1), Some((false, 2)));
        assert_eq!(ticks.drawn_key(3), Some((true, 4)));
        assert_eq!(ticks.drawn_key(4), None);
    }

    /// The ids of the SVG groups `svg` opens, in order.
    fn group_ids(svg: &str) -> Vec<&str> {
        svg.split("<g id=\"")
            .skip(1)
            .filter_map(|rest| rest.split_once('"').map(|(id, _)| id))
            .collect()
    }

    #[test]
    fn backgrounds_axes_ticks_and_their_parts_draw_in_matplotlibs_groups() {
        use crate::figure::Figure;

        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xticks([0.5]).set_yticks(Vec::<f64>::new());
        ax.set_xlabel("x");
        let ids = group_ids(&fig.to_svg().unwrap()).join(" ");
        assert!(
            ids.starts_with(
                "figure_1 patch_1 axes_1 patch_2 matplotlib.axis_1 xtick_1 line2d_1 text_1 \
                 text_2 matplotlib.axis_2 patch_3"
            ),
            "{ids}"
        );
        let label = AxisGroup::TickPart {
            minor: false,
            index: 0,
            part: TickPart::Label1,
        };
        ax.xaxis()
            .set_gid(
                AxisGroup::Tick {
                    minor: false,
                    index: 0,
                },
                Some("half".into()),
            )
            .set_gid(label, Some("half-label".into()));
        ax.set_patch_gid(Some("backdrop".into()));
        let ids = group_ids(&fig.to_svg().unwrap()).join(" ");
        assert!(
            ids.contains("backdrop matplotlib.axis_1 half line2d_1 half-label text_1"),
            "{ids}"
        );
    }
}
