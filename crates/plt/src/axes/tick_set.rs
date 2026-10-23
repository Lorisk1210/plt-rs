//! The ticks an axis computes for its view, inside it or not, and what
//! matplotlib's tick getters read from them.

use super::ticks::Sides;

/// A tick with its label text.
#[derive(Clone, Debug, PartialEq)]
pub struct Tick {
    /// Location in data coordinates.
    pub loc: f64,
    /// Label text (possibly empty).
    pub label: String,
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
}

impl TickSet {
    /// The major (or minor) ticks of the view, inside it or not.
    fn unclipped(&self, minor: bool) -> &[Tick] {
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

    fn ticks(locs: &[f64]) -> Vec<Tick> {
        locs.iter()
            .map(|&loc| Tick {
                loc,
                label: format!("{loc}"),
            })
            .collect()
    }

    #[test]
    fn getters_read_the_unclipped_ticks_of_a_level() {
        let set = TickSet {
            major: ticks(&[1.0]),
            major_unclipped: ticks(&[0.0, 1.0, 2.0]),
            minor_unclipped: ticks(&[0.5]),
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
