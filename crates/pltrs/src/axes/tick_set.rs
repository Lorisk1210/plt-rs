//! The ticks an axis computes for its view, inside it or not, and what
//! matplotlib's tick getters read from them.

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

use super::ticks::Sides;
use crate::ticker::AxisInfo;

/// A tick with its label text.
#[derive(Clone, Debug, PartialEq)]
pub struct Tick {
    /// Location in data coordinates.
    pub loc: f64,
    /// Label text (possibly empty).
    pub label: String,
}

pltrs_core::archive_struct!(Tick { loc, label });

/// What a formatter was given by the last tick update of its axis (see
/// [`TickSet::labelling`]): what matplotlib's formatters keep computing
/// from after `Axis._update_ticks`.
#[derive(Clone, Debug, PartialEq)]
pub struct Labelling {
    /// The number of that update (see [`TickSet::labelled`]).
    pub serial: u64,
    /// The locations the formatter labelled, inside the view or not.
    pub locs: Vec<f64>,
    /// What the tick helpers knew of the axis then.
    pub info: AxisInfo,
}

/// The ticks an axis draws in the current view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickSet {
    /// Major ticks inside the view.
    pub major: Vec<Tick>,
    /// Minor ticks inside the view (excluding those on major ticks).
    pub minor: Vec<Tick>,
    /// Every minor tick inside the view, those on major ticks included,
    /// labelled together (what axisartist draws from matplotlib's
    /// `minor.locator()`).
    pub minor_all: Vec<Tick>,
    /// Every location the major locator gave, inside the view or not
    /// (axisartist draws its gridlines at matplotlib's `major.locator()`).
    pub major_locs: Vec<f64>,
    /// Every location the minor locator gave, inside the view or not.
    pub minor_locs: Vec<f64>,
    /// Every major tick with its label, inside the view or not
    /// (matplotlib's `get_majorticklocs` and `get_majorticklabels`).
    pub major_unclipped: Vec<Tick>,
    /// The position of each of `major` in `major_unclipped` (the index of
    /// its tick in matplotlib's `majorTicks`).
    pub(crate) major_index: Vec<usize>,
    /// Every minor tick not on a major one (unless overlapping locations
    /// are kept) with its label, inside the view or not (matplotlib's
    /// `get_minorticklocs` and `get_minorticklabels`).
    pub minor_unclipped: Vec<Tick>,
    /// The position of each of `minor` in `minor_unclipped`.
    pub(crate) minor_index: Vec<usize>,
    /// Offset or scale text shown at the end of the axis (e.g. `1e6`).
    pub offset: String,
    /// What the tick helpers knew of the axis when they computed these
    /// ticks, its major locations included; `None` for ticks given rather
    /// than computed.
    pub(crate) info: Option<AxisInfo>,
    /// Whether the computation labelled the major and the minor level.
    pub(crate) computed: [bool; 2],
    /// The number of the update that placed these ticks first, once one
    /// did (see [`TickSet::labelled`]).
    pub(crate) placed: Cell<Option<u64>>,
    /// Whether the last update that placed them labelled every minor
    /// location, those on major ticks included (axisartist's).
    pub(crate) whole_minor: Cell<bool>,
}

pltrs_core::archive_struct!(TickSet {
    major,
    minor,
    minor_all,
    major_locs,
    minor_locs,
    major_unclipped,
    major_index,
    minor_unclipped,
    minor_index,
    offset
} skip { info, computed, placed, whole_minor });

impl TickSet {
    /// Records that the computation of these ticks labelled the levels
    /// `computed` (major, minor) on the axis `info` describes.
    pub(crate) fn record(&mut self, info: &AxisInfo, computed: [bool; 2]) {
        self.info = Some(info.clone());
        self.computed = computed;
    }

    /// Counts these ticks as placed by one of matplotlib's tick updates
    /// (`Axis._update_ticks`, which drawing or measuring the axis and
    /// reading its tick labels run; with `whole_minor`, axisartist's
    /// `get_tick_iterators`, which labels every minor location): the first
    /// time, they get a number no other ticks get.
    pub(crate) fn place(&self, whole_minor: bool) {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        self.whole_minor.set(whole_minor);
        if self.placed.get().is_none() {
            self.placed.set(Some(NEXT.fetch_add(1, Ordering::Relaxed)));
        }
    }

    /// Takes the number of the update that placed `before` when these are
    /// the same ticks computed again (see [`TickSet::labelled`]): pltrs
    /// computes the ticks of an axis wherever it lays it out, matplotlib
    /// in its tick updates, and computing them for the same view changes
    /// nothing its helpers keep.
    pub(crate) fn succeed(&self, before: &Self) {
        let placed = self.placed.replace(before.placed.get());
        if self != before {
            self.placed.set(placed);
        }
    }

    /// The number of the tick update that placed these ticks first when
    /// their major (or `minor`) level was labelled, different for every
    /// set of ticks placed: matplotlib's formatters keep what they
    /// computed in the last update of their axis (`Axis._update_ticks`);
    /// `None` when the level was not labelled or the ticks were not placed
    /// (computed for another purpose: matplotlib's updates run where it
    /// draws or measures the axis, or reads its tick labels).
    ///
    /// ```
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.xaxis().place_ticks()?;
    /// let first = ax.xaxis().placed_ticks().and_then(|t| t.labelled(false));
    /// ax.set_xlim(0.0, 5.0)?;
    /// ax.xaxis().place_ticks()?;
    /// let second = ax.xaxis().placed_ticks().and_then(|t| t.labelled(false));
    /// assert!(first.is_some() && second > first);
    /// assert_eq!(ax.xaxis().placed_ticks().and_then(|t| t.labelled(true)), None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn labelled(&self, minor: bool) -> Option<u64> {
        self.computed[usize::from(minor)]
            .then(|| self.placed.get())
            .flatten()
    }

    /// What the tick helpers knew of the axis when they computed these
    /// ticks (its view, its major locations, ...); `None` for ticks never
    /// computed.
    pub fn info(&self) -> Option<&AxisInfo> {
        self.info.as_ref()
    }

    /// What the update that placed these ticks gave the formatter of the
    /// major (or `minor`) level; `None` when it did not label that level.
    pub fn labelling(&self, minor: bool) -> Option<Labelling> {
        Some(Labelling {
            serial: self.labelled(minor)?,
            locs: if minor && self.whole_minor.get() {
                self.minor_locs.clone()
            } else {
                self.unclipped_locs(minor)
            },
            info: self.info.clone()?,
        })
    }

    /// Takes the ticks of the `minor` level (or the major one) from
    /// `other`: the level an update left alone.
    pub(crate) fn keep_level(&mut self, other: &Self, minor: bool) {
        self.computed[usize::from(minor)] = other.computed[usize::from(minor)];
        if minor {
            self.minor.clone_from(&other.minor);
            self.minor_all.clone_from(&other.minor_all);
            self.minor_locs.clone_from(&other.minor_locs);
            self.minor_unclipped.clone_from(&other.minor_unclipped);
            self.minor_index.clone_from(&other.minor_index);
        } else {
            self.major.clone_from(&other.major);
            self.major_locs.clone_from(&other.major_locs);
            self.major_unclipped.clone_from(&other.major_unclipped);
            self.major_index.clone_from(&other.major_index);
            self.offset.clone_from(&other.offset);
        }
    }

    /// The major (or minor) ticks of the view, inside it or not.
    pub(crate) fn unclipped(&self, minor: bool) -> &[Tick] {
        if minor {
            &self.minor_unclipped
        } else {
            &self.major_unclipped
        }
    }

    /// The locations of the major (or minor) ticks of the view, inside it
    /// or not (matplotlib's `Axis.get_ticklocs`).
    pub(crate) fn unclipped_locs(&self, minor: bool) -> Vec<f64> {
        self.unclipped(minor).iter().map(|tick| tick.loc).collect()
    }

    /// The labels of those ticks on each side that shows them, those on
    /// the first side first (matplotlib's `Axis.get_ticklabels`).
    pub(crate) fn unclipped_labels(&self, minor: bool, sides: Sides) -> Vec<String> {
        let labels = self.unclipped(minor).iter().map(|tick| tick.label.clone());
        [sides.first, sides.second]
            .into_iter()
            .filter(|&shown| shown)
            .flat_map(|_| labels.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::ticks_at;

    #[test]
    fn getters_read_the_unclipped_ticks_of_a_level() {
        let set = TickSet {
            major: ticks_at(&[1.0]),
            major_unclipped: ticks_at(&[0.0, 1.0, 2.0]),
            minor_unclipped: ticks_at(&[0.5]),
            ..TickSet::default()
        };
        assert_eq!(set.unclipped_locs(false), [0.0, 1.0, 2.0]);
        assert_eq!(set.unclipped_locs(true), [0.5]);
        let both = Sides {
            first: true,
            second: true,
        };
        assert_eq!(set.unclipped_labels(true, both), ["0.5", "0.5"]);
        let second = Sides {
            first: false,
            second: true,
        };
        assert_eq!(set.unclipped_labels(false, second), ["0", "1", "2"]);
        assert_eq!(
            set.unclipped_labels(false, Sides::default()),
            Vec::<String>::new()
        );
        assert_eq!(TickSet::default().unclipped_locs(false), Vec::<f64>::new());
    }
}
