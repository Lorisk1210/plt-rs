//! Stem widths: measured in font units on a style's standard character,
//! clustered (`af_sort_and_quantize_widths`), scaled, and used to snap
//! the widths of stems while hinting (`af_latin_snap_width`, which the
//! CJK hinter shares).

use crate::fixed::pix_round;

/// The most standard widths an axis keeps (`AF_LATIN_MAX_WIDTHS`,
/// `AF_CJK_MAX_WIDTHS`).
pub(crate) const MAX_WIDTHS: usize = 16;

/// A distance in font units with its scaled and grid-fitted values
/// (`AF_WidthRec`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Width {
    /// Original value in font units.
    pub(crate) org: i64,
    /// Scaled value in 26.6.
    pub(crate) cur: i64,
    /// Grid-fitted value in 26.6.
    pub(crate) fit: i64,
}

/// The standard stem widths of one axis, as a style's metrics keep them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AxisWidths {
    /// The widths, smallest first; only the first `count` are used.
    pub(crate) widths: [Width; MAX_WIDTHS],
    pub(crate) count: usize,
    /// How close segments must be to join one edge, in font units.
    pub(crate) edge_distance_threshold: i64,
    /// The default stem thickness in font units.
    pub(crate) standard_width: i64,
}

impl AxisWidths {
    /// The widths in use.
    pub(crate) fn used(&self) -> &[Width] {
        &self.widths[..self.count]
    }

    /// Fills in the standard width and edge distance threshold from the
    /// smallest width, or from `fallback` when none was measured.
    pub(crate) fn set_standard(&mut self, fallback: i64) {
        let standard = if self.count > 0 {
            self.widths[0].org
        } else {
            fallback
        };
        // 20% of the smallest width.
        self.edge_distance_threshold = standard / 5;
        self.standard_width = standard;
    }

    /// Sorts the first `count` widths and replaces clusters of widths no
    /// further apart than `threshold` by their mean
    /// (`af_sort_and_quantize_widths`).
    ///
    /// FreeType's quirks are kept: the mean divides by the index of the
    /// cluster's end rather than its size, and an empty list becomes a
    /// single zero width.
    pub(crate) fn quantize(&mut self, count: usize, threshold: i64) {
        self.count = count;
        if count == 1 {
            return;
        }
        let table = &mut self.widths;
        table[..count].sort_by_key(|width| width.org);
        let mut cur_idx = 0;
        let mut cur_val = table[0].org;
        let mut i = 1;
        while i < count {
            if table[i].org - cur_val > threshold || i == count - 1 {
                if table[i].org - cur_val <= threshold && i == count - 1 {
                    i += 1;
                }
                let mut sum = 0;
                for width in &mut table[cur_idx..i] {
                    sum += width.org;
                    width.org = 0;
                }
                table[cur_idx].org = sum / i as i64;
                if i + 1 < count {
                    cur_idx = i + 1;
                    cur_val = table[cur_idx].org;
                }
            }
            i += 1;
        }
        let mut kept = 1;
        for index in 1..count {
            if table[index].org != 0 {
                table[kept] = table[index];
                kept += 1;
            }
        }
        self.count = kept;
    }
}

/// Snaps `width` (26.6) to the nearest scaled standard width when it is
/// close enough, as measured against its rounded value
/// (`af_latin_snap_width`).
pub(crate) fn snap_width(widths: &[Width], width: i64) -> i64 {
    let mut best = 64 + 32 + 2;
    let mut reference = width;
    for standard in widths {
        let dist = (width - standard.cur).abs();
        if dist < best {
            best = dist;
            reference = standard.cur;
        }
    }
    let scaled = pix_round(reference);
    if width >= reference {
        if width < scaled + 48 {
            return reference;
        }
    } else if width > scaled - 48 {
        return reference;
    }
    width
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widths(values: &[i64]) -> AxisWidths {
        let mut axis = AxisWidths::default();
        for (slot, &org) in axis.widths.iter_mut().zip(values) {
            slot.org = org;
        }
        axis.quantize(values.len(), 10);
        axis
    }

    fn orgs(axis: &AxisWidths) -> Vec<i64> {
        axis.used().iter().map(|width| width.org).collect()
    }

    #[test]
    fn quantizing_keeps_freetype_means() {
        assert_eq!(orgs(&widths(&[180, 176])), [178]);
        assert_eq!(orgs(&widths(&[200, 100])), [100, 200]);
        // After a cluster FreeType skips a width, and the next cluster's
        // mean divides by its end index (4), not by its size (1).
        assert_eq!(orgs(&widths(&[100, 102, 300, 304])), [101, 300, 76]);
        assert_eq!(orgs(&widths(&[])), [0], "an empty list gains a zero");
        assert_eq!(orgs(&widths(&[42])), [42]);
    }

    #[test]
    fn standard_width_and_threshold() {
        let mut axis = widths(&[]);
        axis.set_standard(24);
        assert_eq!((axis.standard_width, axis.edge_distance_threshold), (0, 0));
        let mut none = AxisWidths::default();
        none.set_standard(24);
        assert_eq!((none.standard_width, none.edge_distance_threshold), (24, 4));
    }

    #[test]
    fn snapping_to_standard_widths() {
        let standard = [Width {
            org: 0,
            cur: 100,
            fit: 100,
        }];
        assert_eq!(snap_width(&standard, 110), 100);
        assert_eq!(snap_width(&standard, 200), 200, "too far");
        assert_eq!(snap_width(&standard, 90), 100);
        assert_eq!(snap_width(&[], 77), 77);
    }
}
