//! How grids and their cell blocks print: matplotlib's `repr` of a
//! `GridSpec` (or `GridSpecFromSubplotSpec`) and of a `SubplotSpec`.

use std::fmt;

use super::{GridSpec, SubplotSpec};

/// Ratios as matplotlib prints ratios given as a tuple: `(3, 1)`, whole
/// numbers without a decimal point.
struct Ratios<'a>(&'a [f64]);

impl fmt::Display for Ratios<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(")?;
        for (i, ratio) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{ratio}")?;
        }
        f.write_str(")")
    }
}

/// Whether `ratios` hold more than one distinct value, which is when
/// matplotlib prints them.
fn uneven(ratios: &[f64]) -> bool {
    ratios
        .windows(2)
        .any(|pair| pair[0].total_cmp(&pair[1]).is_ne())
}

impl fmt::Display for GridSpec {
    /// `GridSpec(2, 2, height_ratios=(3, 1), width_ratios=(1, 3))`, with
    /// the ratios only when they differ, and `GridSpecFromSubplotSpec(...)`
    /// for a nested grid.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = if self.parent().is_some() {
            "GridSpecFromSubplotSpec"
        } else {
            "GridSpec"
        };
        let (nrows, ncols) = self.shape();
        write!(f, "{name}({nrows}, {ncols}")?;
        if uneven(self.height_ratios()) {
            write!(f, ", height_ratios={}", Ratios(self.height_ratios()))?;
        }
        if uneven(self.width_ratios()) {
            write!(f, ", width_ratios={}", Ratios(self.width_ratios()))?;
        }
        f.write_str(")")
    }
}

impl fmt::Display for SubplotSpec {
    /// The grid followed by the rows and columns covered:
    /// `GridSpec(3, 3)[2:3, 1:3]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (rows, cols) = (self.rows(), self.cols());
        write!(
            f,
            "{}[{}:{}, {}:{}]",
            self.gridspec(),
            rows.start,
            rows.end,
            cols.start,
            cols.end
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gridspec::SubplotOverrides;

    fn widths(ratios: Vec<f64>) -> GridSpec {
        GridSpec::with_ratios(
            1,
            ratios.len(),
            Some(ratios),
            None,
            SubplotOverrides::default(),
        )
        .unwrap()
    }

    #[test]
    fn grids_print_like_matplotlib() {
        let plain = GridSpec::new(3, 3).unwrap();
        assert_eq!(plain.to_string(), "GridSpec(3, 3)");
        assert_eq!(
            plain.span(2..3, 1..3).unwrap().to_string(),
            "GridSpec(3, 3)[2:3, 1:3]"
        );
        let ratios = GridSpec::with_ratios(
            2,
            2,
            Some(vec![1.0, 3.0]),
            Some(vec![3.0, 1.0]),
            SubplotOverrides::default(),
        )
        .unwrap();
        assert_eq!(
            ratios.to_string(),
            "GridSpec(2, 2, height_ratios=(3, 1), width_ratios=(1, 3))"
        );
        assert_eq!(
            widths(vec![2.0, 2.0]).to_string(),
            "GridSpec(1, 2)",
            "equal ratios are left out"
        );
        assert_eq!(
            widths(vec![0.5, 1.5]).to_string(),
            "GridSpec(1, 2, width_ratios=(0.5, 1.5))"
        );
    }

    #[test]
    fn nested_grids_name_their_class() {
        let cell = GridSpec::new(1, 2).unwrap().cell(0, 0).unwrap();
        let inner = GridSpec::nested(&cell, 2, 1, None, None, (None, None)).unwrap();
        assert_eq!(inner.to_string(), "GridSpecFromSubplotSpec(2, 1)");
        let lower = inner.cell(1, 0).unwrap();
        assert_eq!(lower.to_string(), "GridSpecFromSubplotSpec(2, 1)[1:2, 0:1]");
        assert_eq!(
            lower.topmost_subplotspec().to_string(),
            "GridSpec(1, 2)[0:1, 0:1]"
        );
        assert_eq!(cell.topmost_subplotspec(), cell);
        assert_eq!(GridSpec::new(1, 1).unwrap().topmost_subplotspec(), None);
    }
}
