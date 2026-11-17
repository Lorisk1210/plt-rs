//! Whether a collection holds exactly what an array of the program's
//! holds: the comparison pltrs (Python) makes of an array a collection
//! handed out (`_own_state`), with the numbers pltrs (Rust) holds, both
//! read where they are. Nothing is copied, and nothing is kept beside
//! the array to compare it with later.

use pltrs::collections::Collection;
use pltrs_core::Point;

use crate::binding::prelude::*;
use crate::binding::with_item_bytes;

/// How many numbers are compared before the comparison may stop: within
/// a block every number is compared, which lets the processor compare
/// several at a time.
const BLOCK: usize = 64;

/// What differs between the number in `bytes` and `number`, bit by bit:
/// zero for the same number (the same NaN; 0.0 is not -0.0).
fn difference(bytes: [u8; 8], number: f64) -> u64 {
    u64::from_le_bytes(bytes) ^ number.to_bits()
}

/// Whether `bytes` hold exactly `numbers`, eight bytes each.
fn same_numbers(bytes: &[u8], numbers: &[f64]) -> bool {
    let (items, rest) = bytes.as_chunks::<8>();
    rest.is_empty()
        && items.len() == numbers.len()
        && items
            .chunks(BLOCK)
            .zip(numbers.chunks(BLOCK))
            .all(|(items, numbers)| {
                let differing = items
                    .iter()
                    .zip(numbers)
                    .fold(0, |differing, (&item, &number)| {
                        differing | difference(item, number)
                    });
                differing == 0
            })
}

/// Whether `bytes` hold the x and y of each of `points` in turn, eight
/// bytes each: exactly, or, where a byte of `masked` (one per number, if
/// given) is set, any number while pltrs (Rust) holds NaN there, as it
/// does for what a masked array masks.
fn same_points(bytes: &[u8], masked: Option<&[u8]>, points: &[Point]) -> bool {
    let (items, rest) = bytes.as_chunks::<8>();
    let (pairs, odd) = items.as_chunks::<2>();
    if !rest.is_empty() || !odd.is_empty() || pairs.len() != points.len() {
        return false;
    }
    if masked.is_some_and(|masked| masked.len() != items.len()) {
        return false;
    }
    // (A mask that masks nothing, the usual one, compares as none.)
    let masks = masked
        .filter(|masked| !nothing_masked(masked))
        .map(|masked| masked.as_chunks::<2>().0);
    let Some(masks) = masks else {
        return pairs
            .chunks(BLOCK)
            .zip(points.chunks(BLOCK))
            .all(|(pairs, points)| {
                let differing = pairs
                    .iter()
                    .zip(points)
                    .fold(0, |differing, (&[x, y], point)| {
                        differing | difference(x, point.x) | difference(y, point.y)
                    });
                differing == 0
            });
    };
    pairs
        .chunks(BLOCK)
        .zip(masks.chunks(BLOCK))
        .zip(points.chunks(BLOCK))
        .all(|((pairs, masks), points)| {
            let differing = pairs.iter().zip(masks).zip(points).fold(
                0,
                |differing, ((&[x, y], &[x_masked, y_masked]), point)| {
                    differing
                        | masked_difference(x, x_masked, point.x)
                        | masked_difference(y, y_masked, point.y)
                },
            );
            differing == 0
        })
}

/// Whether no byte of `masked` is set.
fn nothing_masked(masked: &[u8]) -> bool {
    masked
        .chunks(BLOCK)
        .all(|block| block.iter().fold(0, |set, &byte| set | byte) == 0)
}

/// What differs between the number in `bytes` and `number` ([`difference`])
/// where `masked` is not set; where it is, zero while `number` is NaN,
/// whatever the bytes.
fn masked_difference(bytes: [u8; 8], masked: u8, number: f64) -> u64 {
    if masked == 0 {
        difference(bytes, number)
    } else {
        u64::from(!number.is_nan())
    }
}

/// Whether the offsets of `collection` are what the rows of `offsets`
/// hold, and NaN where `mask` (the mask of a masked array, if given: a
/// bool for each number) masks them; `None` when `offsets` is no array of
/// float64 of shape `(n, 2)` in row-major order, or `mask` no such array
/// of bool, which the caller compares another way.
fn holds_offsets(
    collection: Collection,
    offsets: Owned,
    mask: Option<Owned>,
) -> PyResult<Option<bool>> {
    let same = Python::with_gil(|py| {
        with_item_bytes::<f64, _>(&offsets.bind(py), |bytes, shape| {
            if !matches!(shape, [_, 2]) {
                return None;
            }
            let compare = |masked: Option<&[u8]>| {
                collection.with_offsets(|points| same_points(bytes, masked, points))
            };
            match &mask {
                None => Some(compare(None)),
                Some(mask) => with_item_bytes::<bool, _>(&mask.bind(py), |masked, mask_shape| {
                    (mask_shape == shape).then(|| compare(Some(masked)))
                })
                .flatten(),
            }
        })
    });
    Ok(same.flatten())
}

/// Whether the marker areas of `collection` are exactly `sizes` (none set
/// being none); `None` when `sizes` is no one-dimensional array of
/// float64, which the caller compares another way.
fn holds_sizes(collection: Collection, sizes: Owned) -> PyResult<Option<bool>> {
    let same = Python::with_gil(|py| {
        with_item_bytes::<f64, _>(&sizes.bind(py), |bytes, shape| {
            matches!(shape, [_])
                .then(|| collection.with_sizes(|own| same_numbers(bytes, own.unwrap_or_default())))
        })
    });
    Ok(same.flatten())
}

/// Registers the comparisons.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("holds_offsets", holds_offsets)?;
    module.add("holds_sizes", holds_sizes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, extension, with_python};
    use pltrs::figure::Figure;

    /// The bytes of `numbers`, as an array of float64 keeps them.
    fn bytes_of(numbers: &[f64]) -> Vec<u8> {
        numbers.iter().flat_map(|n| n.to_le_bytes()).collect()
    }

    #[test]
    fn numbers_are_the_same_bit_for_bit() {
        let numbers: Vec<f64> = (0..200).map(f64::from).collect();
        assert!(same_numbers(&bytes_of(&numbers), &numbers));
        assert!(same_numbers(&[], &[]));
        assert!(!same_numbers(&bytes_of(&numbers[1..]), &numbers));
        for at in [0, 63, 64, 199] {
            let mut changed = numbers.clone();
            changed[at] += 0.5;
            assert!(!same_numbers(&bytes_of(&changed), &numbers), "at {at}");
        }
        assert!(same_numbers(&bytes_of(&[f64::NAN]), &[f64::NAN]));
        assert!(!same_numbers(&bytes_of(&[0.0]), &[-0.0]));
        assert!(!same_numbers(&[0; 7], &[0.0]), "no whole number");
    }

    #[test]
    fn points_are_the_same_by_rows() {
        let points: Vec<Point> = (0..150)
            .map(|i| Point::new(f64::from(i), f64::from(i) * 0.5))
            .collect();
        let rows = |points: &[Point]| -> Vec<u8> {
            bytes_of(&points.iter().flat_map(|p| [p.x, p.y]).collect::<Vec<_>>())
        };
        assert!(same_points(&rows(&points), None, &points));
        assert!(!same_points(&rows(&points[..149]), None, &points));
        for at in [0, 64, 149] {
            let mut changed = points.clone();
            changed[at].y = f64::NAN;
            assert!(!same_points(&rows(&changed), None, &points), "at {at}");
        }
        assert!(!same_points(
            &bytes_of(&[1.0, 2.0, 3.0]),
            None,
            &points[..1]
        ));
        // What a mask masks is NaN in pltrs (Rust), whatever the array
        // holds there.
        let held = [Point::new(1.0, f64::NAN), Point::new(3.0, 4.0)];
        let given = bytes_of(&[1.0, 7.0, 3.0, 4.0]);
        assert!(same_points(&given, Some(&[0, 1, 0, 0]), &held));
        assert!(!same_points(&given, Some(&[0, 0, 0, 0]), &held), "unmasked");
        assert!(
            !same_points(&given, Some(&[0, 1, 0, 1]), &held),
            "masked anew"
        );
        assert!(
            !same_points(&given, Some(&[0, 1, 0]), &held),
            "a mask of another size"
        );
        assert!(same_points(
            &given,
            None,
            &[Point::new(1.0, 7.0), Point::new(3.0, 4.0)]
        ));
    }

    #[test]
    fn arrays_are_compared_where_they_are() {
        with_python(|py| {
            extension(py);
            let dots = Figure::new()
                .subplot()
                .scatter(vec![1.0, 2.0], vec![3.0, 4.0])
                .unwrap();
            // Buffers of doubles as numpy's arrays of float64 export them.
            let rows = |numbers: &str, shape: &str| {
                eval(
                    py,
                    "import array",
                    &format!(
                        "memoryview(array.array('d', {numbers})).cast('B').cast('d', {shape})"
                    ),
                )
                .unbind()
            };
            let other = |expression: &str| eval(py, "import array", expression).unbind();
            let held = |array: Owned| holds_offsets(dots.clone(), array, None).unwrap();
            assert_eq!(held(rows("[1, 3, 2, 4]", "[2, 2]")), Some(true));
            assert_eq!(held(rows("[1, 3, 2, 5]", "[2, 2]")), Some(false));
            assert_eq!(held(rows("[1, 3]", "[1, 2]")), Some(false));
            // What is no buffer of doubles in rows of two, in order, is
            // not compared.
            assert_eq!(held(rows("[1, 3, 2, 4]", "[4]")), None);
            assert_eq!(held(rows("[1, 3, 2, 4]", "[4, 1]")), None);
            assert_eq!(held(other("array.array('q', [1, 3, 2, 4])")), None);
            assert_eq!(held(other("[[1.0, 3.0], [2.0, 4.0]]")), None);
            let mask = |bools: &str, shape: &str| {
                eval(
                    py,
                    "import array",
                    &format!("memoryview(array.array('B', {bools})).cast('B').cast('?', {shape})"),
                )
                .unbind()
            };
            let masked =
                |array: Owned, mask: Owned| holds_offsets(dots.clone(), array, Some(mask)).unwrap();
            let given = || rows("[1, 3, 2, 4]", "[2, 2]");
            assert_eq!(masked(given(), mask("[0, 0, 0, 0]", "[2, 2]")), Some(true));
            assert_eq!(masked(given(), mask("[0, 0, 0, 1]", "[2, 2]")), Some(false));
            assert_eq!(masked(given(), mask("[0, 0, 0, 0]", "[4]")), None);
            assert_eq!(masked(given(), rows("[0, 0, 0, 0]", "[2, 2]")), None);
            let sized = |array: Owned| holds_sizes(dots.clone(), array).unwrap();
            dots.set_sizes(vec![9.0, 16.0]);
            assert_eq!(sized(other("array.array('d', [9, 16])")), Some(true));
            assert_eq!(sized(other("array.array('d', [9, 17])")), Some(false));
            assert_eq!(sized(rows("[9, 16]", "[1, 2]")), None);
            assert_eq!(
                sized(other("memoryview(array.array('d', [9, 0, 16, 0]))[::2]")),
                None,
                "strided"
            );
        });
    }
}
