//! The item styles a 3-D projection sorts by depth, and why it may fail
//! to.

use std::fmt;

use pltrs_core::{Point, Rgba};

use crate::artist::DrawErrorKind;

/// The item styles of a collection worked out by a 3-D projection, in
/// depth order (matplotlib's `Path3DCollection` sorting its colours, sizes
/// and widths at each projection); the collection keeps its own in data
/// order.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DepthSorted {
    /// Face and edge colours, shaded with depth (`None` when the
    /// collection's colours cannot be resolved, which drawing reports).
    pub(crate) colors: Option<(Vec<Rgba>, Vec<Rgba>)>,
    /// Marker areas (`None`: the collection's own).
    pub(crate) sizes: Option<Vec<f64>>,
    /// Edge widths (`None`: the collection's own).
    pub(crate) linewidths: Option<Vec<f64>>,
    /// The offsets in drawing order (`None`: the collection's own, which
    /// keep the order of the data, as picking reports it).
    pub(crate) offsets: Option<Vec<Point>>,
    /// Why the styles could not be sorted, which drawing reports.
    pub(crate) problem: Option<DepthSortError>,
    /// Whether, without colours of the projection, the collection's own
    /// lose their last one when there are several: matplotlib's depth
    /// order is `slice(-1)` until a projection sorts the markers (from
    /// `set_3d_properties` on), and for good for patches, which it never
    /// sorts.
    pub(crate) without_last: bool,
}

pltrs_core::archive_struct!(DepthSorted {
    colors,
    sizes,
    linewidths,
    offsets,
    problem,
    without_last
});

impl DepthSorted {
    /// The styles before the markers are projected: the collection's own,
    /// its colours without their last when there are several.
    pub(crate) fn unprojected() -> Self {
        Self {
            without_last: true,
            ..Self::default()
        }
    }
}

/// `colors` without their last one when there are several (matplotlib's
/// colours indexed by the depth order `slice(-1)`).
pub(crate) fn without_last(mut colors: Vec<Rgba>) -> Vec<Rgba> {
    if colors.len() > 1 {
        colors.pop();
    }
    colors
}

/// Why the markers of a 3-D scatter plot cannot be sorted by depth:
/// styles that are neither one for all the markers nor one (at least) per
/// marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DepthSortError {
    /// Several sizes, but fewer than the markers: matplotlib indexes the
    /// list `set_sizes` was given with the depth order, which fails.
    Sizes,
    /// Several widths (or colours, unshaded), but fewer than the markers:
    /// the depth order indexes the array beyond its end.
    OutOfBounds {
        /// The first index beyond the end, in the depth order.
        index: usize,
        /// How many values there are.
        size: usize,
    },
    /// Several colours to shade, but not one per marker: numpy cannot
    /// broadcast them to the markers.
    Colors {
        /// How many colours there are.
        given: usize,
        /// How many markers there are.
        markers: usize,
    },
}

pltrs_core::archive_enum!(DepthSortError {
    Sizes,
    OutOfBounds { index, size },
    Colors { given, markers },
});

impl DepthSortError {
    /// The exception matplotlib raises for it: a `TypeError` indexing a
    /// list, an `IndexError` indexing an array, a `ValueError`
    /// broadcasting the colours.
    pub(crate) fn kind(self) -> DrawErrorKind {
        match self {
            Self::Sizes => DrawErrorKind::InvalidType,
            Self::OutOfBounds { .. } => DrawErrorKind::OutOfRange,
            Self::Colors { .. } => DrawErrorKind::InvalidValue,
        }
    }

    /// `values` in the depth `order` when there are several of them,
    /// as they are when there is one, else what keeps them from being
    /// sorted.
    pub(crate) fn reorder<T: Copy>(values: &[T], order: &[usize]) -> Result<Vec<T>, Self> {
        if values.len() <= 1 {
            return Ok(values.to_vec());
        }
        order
            .iter()
            .map(|&i| {
                values.get(i).copied().ok_or(Self::OutOfBounds {
                    index: i,
                    size: values.len(),
                })
            })
            .collect()
    }
}

impl fmt::Display for DepthSortError {
    /// matplotlib's (numpy's) message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sizes => {
                f.write_str("only integer scalar arrays can be converted to a scalar index")
            }
            Self::OutOfBounds { index, size } => {
                write!(
                    f,
                    "index {index} is out of bounds for axis 0 with size {size}"
                )
            }
            Self::Colors { given, markers } => write!(
                f,
                "operands could not be broadcast together with remapped shapes \
                 [original->remapped]: ({given},4)  and requested shape ({markers},4)"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_numpys() {
        let colors = DepthSortError::Colors {
            given: 4,
            markers: 7,
        };
        assert!(
            colors
                .to_string()
                .ends_with("(4,4)  and requested shape (7,4)")
        );
        assert_eq!(colors.kind(), DrawErrorKind::InvalidValue);
        assert_eq!(DepthSortError::Sizes.kind(), DrawErrorKind::InvalidType);
    }

    #[test]
    fn reordering_reports_the_first_index_beyond_the_values() {
        assert_eq!(
            DepthSortError::reorder(&[1.0, 2.0], &[1, 0]),
            Ok(vec![2.0, 1.0])
        );
        assert_eq!(DepthSortError::reorder(&[5.0], &[2, 0, 1]), Ok(vec![5.0]));
        assert_eq!(DepthSortError::reorder::<f64>(&[], &[0]), Ok(vec![]));
        let short = DepthSortError::reorder(&[1.0, 2.0], &[1, 3, 2]);
        assert_eq!(
            short,
            Err(DepthSortError::OutOfBounds { index: 3, size: 2 })
        );
        assert_eq!(
            short.unwrap_err().to_string(),
            "index 3 is out of bounds for axis 0 with size 2"
        );
    }
}
