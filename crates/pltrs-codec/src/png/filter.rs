//! Scanline filters (PNG spec §9): each byte is predicted from the
//! corresponding bytes of the pixel to its left, the pixel above and the
//! pixel above-left, and only the prediction error is stored.

use pltrs_core::alloc::{self, OutOfMemory};

/// A scanline filter type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilterType {
    /// Bytes are stored unchanged.
    None,
    /// Predict from the pixel to the left.
    Sub,
    /// Predict from the pixel above.
    Up,
    /// Predict from the mean of the left and upper pixels.
    Average,
    /// Predict with the Paeth predictor (left, above or above-left).
    Paeth,
}

impl FilterType {
    const ALL: [Self; 5] = [Self::None, Self::Sub, Self::Up, Self::Average, Self::Paeth];

    /// The filter with type byte `byte`.
    pub(crate) const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::None),
            1 => Some(Self::Sub),
            2 => Some(Self::Up),
            3 => Some(Self::Average),
            4 => Some(Self::Paeth),
            _ => None,
        }
    }

    /// The type byte that precedes each filtered row.
    pub(crate) const fn byte(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Sub => 1,
            Self::Up => 2,
            Self::Average => 3,
            Self::Paeth => 4,
        }
    }
}

/// How the encoder chooses a filter for each row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FilterStrategy {
    /// Try every filter on each row and keep the one whose output has the
    /// smallest sum of absolute values (bytes read as signed), libpng's
    /// heuristic.
    #[default]
    Adaptive,
    /// Use the same filter for every row.
    Fixed(FilterType),
}

/// Filters the rows of `data` (each `row_bytes` long, pixels `stride` bytes
/// apart), prefixing every row with its filter type byte.
///
/// # Errors
///
/// When the memory for the filtered rows cannot be allocated.
pub(crate) fn filter_image(
    data: &[u8],
    row_bytes: usize,
    stride: usize,
    strategy: FilterStrategy,
) -> Result<Vec<u8>, OutOfMemory> {
    let mut out = alloc::try_with_capacity(data.len() + data.len() / row_bytes)?;
    let zero_row = vec![0; row_bytes];
    let mut candidate = vec![0; row_bytes];
    let mut best = vec![0; row_bytes];
    let mut previous = zero_row.as_slice();
    for row in data.chunks_exact(row_bytes) {
        let filter = match strategy {
            FilterStrategy::Fixed(filter) => {
                apply(filter, stride, previous, row, &mut best);
                filter
            }
            FilterStrategy::Adaptive => {
                let mut best_filter = FilterType::None;
                let mut best_score = u64::MAX;
                for filter in FilterType::ALL {
                    apply(filter, stride, previous, row, &mut candidate);
                    let score = score(&candidate);
                    if score < best_score {
                        best_score = score;
                        best_filter = filter;
                        std::mem::swap(&mut candidate, &mut best);
                        if score == 0 {
                            break;
                        }
                    }
                }
                best_filter
            }
        };
        out.push(filter.byte());
        out.extend_from_slice(&best);
        previous = row;
    }
    Ok(out)
}

/// Sum of the filtered bytes' magnitudes when read as signed values.
fn score(filtered: &[u8]) -> u64 {
    filtered
        .iter()
        .map(|&byte| u64::from((byte as i8).unsigned_abs()))
        .sum()
}

/// Writes `current` filtered with `filter` into `out`.
fn apply(filter: FilterType, stride: usize, previous: &[u8], current: &[u8], out: &mut [u8]) {
    let (head, tail) = out.split_at_mut(stride);
    let lefts = current;
    let upper_lefts = previous;
    let (current_head, current_tail) = current.split_at(stride);
    let (previous_head, previous_tail) = previous.split_at(stride);
    match filter {
        FilterType::None => out.copy_from_slice(current),
        FilterType::Sub => {
            head.copy_from_slice(current_head);
            for ((o, &c), &left) in tail.iter_mut().zip(current_tail).zip(lefts) {
                *o = c.wrapping_sub(left);
            }
        }
        FilterType::Up => {
            for ((o, &c), &up) in out.iter_mut().zip(current).zip(previous) {
                *o = c.wrapping_sub(up);
            }
        }
        FilterType::Average => {
            for ((o, &c), &up) in head.iter_mut().zip(current_head).zip(previous_head) {
                *o = c.wrapping_sub(up / 2);
            }
            let neighbours = lefts.iter().zip(previous_tail);
            for ((o, &c), (&left, &up)) in tail.iter_mut().zip(current_tail).zip(neighbours) {
                *o = c.wrapping_sub(average(left, up));
            }
        }
        FilterType::Paeth => {
            for ((o, &c), &up) in head.iter_mut().zip(current_head).zip(previous_head) {
                *o = c.wrapping_sub(up);
            }
            let neighbours = lefts.iter().zip(previous_tail).zip(upper_lefts);
            for ((o, &c), ((&left, &up), &upper_left)) in
                tail.iter_mut().zip(current_tail).zip(neighbours)
            {
                *o = c.wrapping_sub(paeth(left, up, upper_left));
            }
        }
    }
}

/// Reverses `filter` on `row` in place, given the already reconstructed
/// `previous` row (all zeros for the first row).
pub(crate) fn unfilter(filter: FilterType, stride: usize, previous: &[u8], row: &mut [u8]) {
    let len = row.len();
    match filter {
        FilterType::None => {}
        FilterType::Sub => {
            for i in stride..len {
                row[i] = row[i].wrapping_add(row[i - stride]);
            }
        }
        FilterType::Up => {
            for (byte, &up) in row.iter_mut().zip(previous) {
                *byte = byte.wrapping_add(up);
            }
        }
        FilterType::Average => {
            for (byte, &up) in row[..stride].iter_mut().zip(previous) {
                *byte = byte.wrapping_add(up / 2);
            }
            for i in stride..len {
                row[i] = row[i].wrapping_add(average(row[i - stride], previous[i]));
            }
        }
        FilterType::Paeth => {
            for (byte, &up) in row[..stride].iter_mut().zip(previous) {
                *byte = byte.wrapping_add(up);
            }
            for i in stride..len {
                let predicted = paeth(row[i - stride], previous[i], previous[i - stride]);
                row[i] = row[i].wrapping_add(predicted);
            }
        }
    }
}

/// The Average filter's prediction: the mean of `left` and `up`, rounded
/// down.
fn average(left: u8, up: u8) -> u8 {
    left.midpoint(up)
}

/// The Paeth predictor: whichever neighbour is closest to
/// `left + up - upper_left`, preferring left, then up.
fn paeth(left: u8, up: u8, upper_left: u8) -> u8 {
    let (a, b, c) = (i16::from(left), i16::from(up), i16::from(upper_left));
    let distance_left = (b - c).abs();
    let distance_up = (a - c).abs();
    let distance_upper_left = (a + b - 2 * c).abs();
    if distance_left <= distance_up && distance_left <= distance_upper_left {
        left
    } else if distance_up <= distance_upper_left {
        up
    } else {
        upper_left
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Rng;

    #[test]
    fn paeth_prefers_left_then_up() {
        assert_eq!(paeth(10, 20, 15), 15);
        assert_eq!(paeth(10, 20, 10), 20);
        assert_eq!(paeth(10, 20, 30), 10);
        assert_eq!(paeth(100, 0, 90), 0);
        assert_eq!(paeth(50, 60, 55), 55);
        assert_eq!(paeth(0, 0, 255), 0);
        assert_eq!(paeth(9, 2, 7), 2);
        assert_eq!(paeth(200, 100, 160), 160);
    }

    #[test]
    fn every_filter_round_trips() {
        let mut rng = Rng::new(21);
        for stride in [1, 2, 3, 4, 6, 8] {
            let row_bytes = stride * (1 + rng.below(20));
            let image = rng.bytes(row_bytes * 5);
            for filter in FilterType::ALL {
                let filtered =
                    filter_image(&image, row_bytes, stride, FilterStrategy::Fixed(filter)).unwrap();
                assert_eq!(filtered.len(), image.len() + 5);
                let mut restored = vec![0; row_bytes];
                let mut previous = vec![0; row_bytes];
                for (index, line) in filtered.chunks_exact(row_bytes + 1).enumerate() {
                    assert_eq!(FilterType::from_byte(line[0]), Some(filter));
                    restored.copy_from_slice(&line[1..]);
                    unfilter(filter, stride, &previous, &mut restored);
                    let original = &image[index * row_bytes..(index + 1) * row_bytes];
                    assert_eq!(
                        restored, original,
                        "{filter:?}, stride {stride}, row {index}"
                    );
                    previous.copy_from_slice(&restored);
                }
            }
        }
    }

    #[test]
    fn adaptive_choice_follows_content() {
        // A horizontal gradient favours Sub; identical rows favour Up.
        let gradient: Vec<u8> = (0..64).collect();
        let filtered = filter_image(&gradient, 64, 1, FilterStrategy::Adaptive).unwrap();
        assert_eq!(filtered[0], FilterType::Sub.byte());
        let noise = Rng::new(22).bytes(64);
        let filtered = filter_image(&noise.repeat(2), 64, 1, FilterStrategy::Adaptive).unwrap();
        assert_eq!(filtered[65], FilterType::Up.byte());
        assert!(filtered[66..].iter().all(|&b| b == 0));
    }

    #[test]
    fn filter_bytes_round_trip() {
        for filter in FilterType::ALL {
            assert_eq!(FilterType::from_byte(filter.byte()), Some(filter));
        }
        assert_eq!(FilterType::from_byte(5), None);
    }
}
