//! Distributing boxes along one direction and aligning them across it
//! (matplotlib's `_get_packed_offsets` and `_get_aligned_offsets`).
//!
//! The descriptions speak of a horizontal layout (widths packed left to
//! right, heights aligned); both functions work the same way vertically.

/// How a packer distributes its children along the packing direction
/// (matplotlib's `mode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PackMode {
    /// Packed tight from the start with `sep` between neighbours.
    #[default]
    Fixed,
    /// Spread so the first box starts at 0 and the last ends at the total
    /// length, with equal gaps (`sep` is ignored).
    Expand,
    /// Every box gets an equal share of the total length and sits at the
    /// start of it.
    Equal,
}

pltrs_core::archive_enum!(PackMode {
    Fixed,
    Expand,
    Equal,
});

/// Where children sit across the packing direction (matplotlib's `align`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PackAlign {
    /// Each child keeps its own origin (its baseline for text).
    #[default]
    Baseline,
    /// Flush with the left edge (in a vertical packer).
    Left,
    /// Flush with the top edge (in a horizontal packer).
    Top,
    /// Flush with the right edge (in a vertical packer).
    Right,
    /// Flush with the bottom edge (in a horizontal packer).
    Bottom,
    /// Centred.
    Center,
}

pltrs_core::archive_enum!(PackAlign {
    Baseline,
    Left,
    Top,
    Right,
    Bottom,
    Center,
});

/// Offsets of boxes of the given `widths` packed with `mode`, and the total
/// length they need (matplotlib's `_get_packed_offsets`).
///
/// In [`PackMode::Fixed`] a `total` is returned unchecked, otherwise it is
/// the end of the last box; [`PackMode::Expand`] treats a missing total as 1.
///
/// ```
/// use pltrs::offsetbox::{PackMode, packed_offsets};
/// assert_eq!(packed_offsets(&[1.0, 2.0], None, 0.5, PackMode::Fixed), (3.5, vec![0.0, 1.5]));
/// assert_eq!(packed_offsets(&[1.0, 2.0], Some(5.0), 0.0, PackMode::Expand), (5.0, vec![0.0, 3.0]));
/// ```
pub fn packed_offsets(
    widths: &[f64],
    total: Option<f64>,
    sep: f64,
    mode: PackMode,
) -> (f64, Vec<f64>) {
    match mode {
        PackMode::Fixed => {
            let (ends, offsets) = cumulative_starts(widths, sep);
            (total.unwrap_or(ends - sep), offsets)
        }
        PackMode::Expand => {
            let total = total.unwrap_or(1.0);
            let sep = if widths.len() > 1 {
                (total - widths.iter().sum::<f64>()) / (widths.len() - 1) as f64
            } else {
                0.0
            };
            (total, cumulative_starts(widths, sep).1)
        }
        PackMode::Equal if widths.is_empty() => (total.unwrap_or(0.0), Vec::new()),
        PackMode::Equal => {
            let widest = widths.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let count = widths.len() as f64;
            let (total, sep) = match total {
                Some(total) => (total, total / count - widest),
                None => ((widest + sep) * count, sep),
            };
            let offsets = (0..widths.len())
                .map(|i| (widest + sep) * i as f64)
                .collect();
            (total, offsets)
        }
    }
}

/// The running sums `0, w0 + sep, w0 + w1 + 2 sep, ...` without the last
/// one, and the last one (numpy's `cumsum([0] + [w + sep ...])`).
fn cumulative_starts(widths: &[f64], sep: f64) -> (f64, Vec<f64>) {
    let mut end = 0.0;
    let offsets = widths
        .iter()
        .map(|width| {
            let start = end;
            end += width + sep;
            start
        })
        .collect();
    (end, offsets)
}

/// The span `(y0, y1)` covered by boxes spanning `yspans` once aligned with
/// `align`, and the bottom offset of each (matplotlib's
/// `_get_aligned_offsets`).
///
/// Without a `height` the tallest box sets it. For every alignment but
/// [`PackAlign::Baseline`] the span is `(0, height)`, unchecked.
///
/// ```
/// use pltrs::offsetbox::{PackAlign, aligned_offsets};
/// let spans = [(-1.0, 3.0), (0.0, 2.0)];
/// assert_eq!(aligned_offsets(&spans, None, PackAlign::Baseline), ((-1.0, 3.0), vec![0.0, 0.0]));
/// assert_eq!(aligned_offsets(&spans, None, PackAlign::Center), ((0.0, 4.0), vec![1.0, 1.0]));
/// ```
pub fn aligned_offsets(
    yspans: &[(f64, f64)],
    height: Option<f64>,
    align: PackAlign,
) -> ((f64, f64), Vec<f64>) {
    let height = height.unwrap_or_else(|| {
        yspans
            .iter()
            .map(|(y0, y1)| y1 - y0)
            .fold(f64::NEG_INFINITY, f64::max)
    });
    match align {
        PackAlign::Baseline => {
            let low = yspans.iter().map(|s| s.0).fold(f64::INFINITY, f64::min);
            let high = yspans.iter().map(|s| s.1).fold(f64::NEG_INFINITY, f64::max);
            ((low, high), vec![0.0; yspans.len()])
        }
        PackAlign::Left | PackAlign::Bottom => {
            ((0.0, height), yspans.iter().map(|(y0, _)| -y0).collect())
        }
        PackAlign::Right | PackAlign::Top => (
            (0.0, height),
            yspans.iter().map(|(_, y1)| height - y1).collect(),
        ),
        PackAlign::Center => (
            (0.0, height),
            yspans
                .iter()
                .map(|(y0, y1)| (height - (y1 - y0)) * 0.5 - y0)
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_packing_keeps_an_explicit_total() {
        assert_eq!(
            packed_offsets(&[1.0, 2.0, 3.0], Some(2.0), 1.0, PackMode::Fixed),
            (2.0, vec![0.0, 2.0, 5.0])
        );
        assert_eq!(
            packed_offsets(&[], None, 1.0, PackMode::Fixed),
            (-1.0, vec![]),
            "matplotlib returns -sep for nothing"
        );
    }

    #[test]
    fn expand_spreads_the_gaps() {
        assert_eq!(
            packed_offsets(&[1.0, 1.0, 1.0], Some(7.0), 9.0, PackMode::Expand),
            (7.0, vec![0.0, 3.0, 6.0])
        );
        assert_eq!(
            packed_offsets(&[2.0], None, 9.0, PackMode::Expand),
            (1.0, vec![0.0]),
            "a lone box ignores the separation"
        );
    }

    #[test]
    fn equal_shares() {
        assert_eq!(
            packed_offsets(&[1.0, 3.0], None, 1.0, PackMode::Equal),
            (8.0, vec![0.0, 4.0])
        );
        assert_eq!(
            packed_offsets(&[1.0, 3.0], Some(10.0), 0.0, PackMode::Equal),
            (10.0, vec![0.0, 5.0])
        );
    }

    #[test]
    fn alignments() {
        let spans = [(-1.0, 3.0), (0.0, 2.0)];
        assert_eq!(
            aligned_offsets(&spans, Some(6.0), PackAlign::Bottom),
            ((0.0, 6.0), vec![1.0, 0.0])
        );
        assert_eq!(
            aligned_offsets(&spans, None, PackAlign::Top),
            ((0.0, 4.0), vec![1.0, 2.0])
        );
        assert_eq!(
            aligned_offsets(&spans, Some(5.0), PackAlign::Right),
            ((0.0, 5.0), vec![2.0, 3.0])
        );
    }
}
