//! Scanline filters (PNG spec §9): each byte is predicted from the
//! corresponding bytes of the pixel to its left, the pixel above and the
//! pixel above-left, and only the prediction error is stored.

use std::ops::Range;

use pltrs_core::alloc::{self, OutOfMemory};

use crate::deflate::encode::RowSource;

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
    /// The filters [`FilterStrategy::Adaptive`] tries after none, in
    /// Pillow's order.
    const TRIED: [Self; 3] = [Self::Up, Self::Sub, Self::Paeth];

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
    /// Pillow's choice for each row: no filter, unless Up, then Sub, then
    /// Paeth gives a smaller sum of absolute values (bytes read as signed),
    /// a heuristic taken from libpng.
    #[default]
    Adaptive,
    /// Use the same filter for every row.
    Fixed(FilterType),
}

/// The filtered rows of an image, each prefixed with its filter type byte
/// (what the `IDAT` stream compresses), made a row at a time as the
/// compressor takes them rather than filtered into a copy of the whole
/// image.
#[derive(Debug)]
pub(crate) struct FilteredRows<'a> {
    data: &'a [u8],
    row_bytes: usize,
    stride: usize,
    strategy: FilterStrategy,
    /// The row last filtered, if any.
    filtered_row: Option<usize>,
    /// That row: its type byte, then its filtered bytes.
    filtered: Vec<u8>,
    /// Room for the other filters an adaptive choice tries.
    candidate: Vec<u8>,
    /// The row above the first.
    zero_row: Vec<u8>,
}

impl<'a> FilteredRows<'a> {
    /// The rows of `data` (each `row_bytes` long, pixels `stride` bytes
    /// apart) filtered as `strategy` chooses.
    ///
    /// # Errors
    ///
    /// When the memory for a row cannot be allocated.
    pub(crate) fn new(
        data: &'a [u8],
        row_bytes: usize,
        stride: usize,
        strategy: FilterStrategy,
    ) -> Result<Self, OutOfMemory> {
        let zeros = |len: usize| -> Result<Vec<u8>, OutOfMemory> {
            let mut row = alloc::try_with_capacity(len)?;
            row.resize(len, 0);
            Ok(row)
        };
        Ok(Self {
            data,
            row_bytes,
            stride,
            strategy,
            filtered_row: None,
            filtered: zeros(row_bytes + 1)?,
            candidate: zeros(row_bytes + 1)?,
            zero_row: zeros(row_bytes)?,
        })
    }

    /// Filters row `index` into `filtered`, unless it is there already.
    fn filter_row(&mut self, index: usize) {
        if self.filtered_row == Some(index) {
            return;
        }
        let row_bytes = self.row_bytes;
        let current = &self.data[index * row_bytes..(index + 1) * row_bytes];
        let previous = match index.checked_sub(1) {
            Some(above) => &self.data[above * row_bytes..index * row_bytes],
            None => &self.zero_row,
        };
        let row = Row {
            stride: self.stride,
            previous,
            current,
        };
        let filter = match self.strategy {
            FilterStrategy::Fixed(filter) => {
                row.apply(filter, &mut self.filtered[1..], 0..row_bytes);
                filter
            }
            FilterStrategy::Adaptive => {
                let mut best_filter = FilterType::None;
                let mut best_score = row
                    .scored(FilterType::None, &mut self.filtered[1..], u64::MAX)
                    .expect("no row scores u64::MAX");
                for filter in FilterType::TRIED {
                    if best_score == 0 {
                        break;
                    }
                    if let Some(score) = row.scored(filter, &mut self.candidate[1..], best_score) {
                        best_score = score;
                        best_filter = filter;
                        std::mem::swap(&mut self.candidate, &mut self.filtered);
                    }
                }
                best_filter
            }
        };
        self.filtered[0] = filter.byte();
        self.filtered_row = Some(index);
    }
}

impl RowSource for FilteredRows<'_> {
    fn rows(&self) -> usize {
        self.data.len() / self.row_bytes
    }

    fn row(&mut self, index: usize) -> &[u8] {
        self.filter_row(index);
        &self.filtered
    }
}

/// The bytes filtered and scored at a time. An adaptive choice stops
/// trying a filter on a row once the sum so far is no better than the
/// best filter's, and the sum of this many bytes fits a `u32`, which the
/// compiler sums several bytes at a time.
const CHUNK: usize = 1024;

/// A scanline and the one above it, to be filtered.
#[derive(Clone, Copy, Debug)]
struct Row<'a> {
    /// The distance from a byte to the corresponding one of the pixel to
    /// its left.
    stride: usize,
    /// The row above (zeros above the first).
    previous: &'a [u8],
    /// The row filtered.
    current: &'a [u8],
}

impl Row<'_> {
    /// Writes the bytes `range` of the row filtered with `filter` into
    /// the same bytes of `out`.
    fn apply(self, filter: FilterType, out: &mut [u8], range: Range<usize>) {
        let Self {
            stride,
            previous,
            current,
        } = self;
        // The first pixel's bytes have no left neighbours, which count
        // as zeros.
        for i in range.start.min(stride)..range.end.min(stride) {
            out[i] = current[i].wrapping_sub(predict(filter, 0, previous[i], 0));
        }
        let tail = range.start.max(stride)..range.end.max(stride);
        let left = tail.start - stride..tail.end - stride;
        let (out, current, up) = (
            &mut out[tail.clone()],
            &current[tail.clone()],
            &previous[tail],
        );
        let (left, upper_left) = (&self.current[left.clone()], &previous[left]);
        match filter {
            FilterType::None => out.copy_from_slice(current),
            FilterType::Sub => {
                for ((o, &c), &left) in out.iter_mut().zip(current).zip(left) {
                    *o = c.wrapping_sub(left);
                }
            }
            FilterType::Up => {
                for ((o, &c), &up) in out.iter_mut().zip(current).zip(up) {
                    *o = c.wrapping_sub(up);
                }
            }
            FilterType::Average => {
                for ((o, &c), (&left, &up)) in out.iter_mut().zip(current).zip(left.iter().zip(up))
                {
                    *o = c.wrapping_sub(average(left, up));
                }
            }
            FilterType::Paeth => {
                let neighbours = left.iter().zip(up).zip(upper_left);
                for ((o, &c), ((&left, &up), &upper_left)) in
                    out.iter_mut().zip(current).zip(neighbours)
                {
                    *o = c.wrapping_sub(paeth(left, up, upper_left));
                }
            }
        }
    }

    /// Writes the row filtered with `filter` into `out` and returns its
    /// score (the sum of the filtered bytes' magnitudes, read as signed
    /// values), unless the score reaches `bound`: then `out` holds part of
    /// the filtered row only.
    fn scored(self, filter: FilterType, out: &mut [u8], bound: u64) -> Option<u64> {
        let mut score = 0;
        let len = self.current.len();
        for start in (0..len).step_by(CHUNK) {
            let range = start..(start + CHUNK).min(len);
            self.apply(filter, out, range.clone());
            score += u64::from(chunk_score(&out[range]));
            if score >= bound {
                return None;
            }
        }
        Some(score)
    }
}

/// The sum of the magnitudes of at most [`CHUNK`] filtered bytes, read as
/// signed values.
fn chunk_score(filtered: &[u8]) -> u32 {
    filtered
        .iter()
        .map(|&byte| u32::from((byte as i8).unsigned_abs()))
        .sum()
}

/// What `filter` predicts a byte from: the corresponding bytes of the
/// pixels to its left, above it and above-left.
fn predict(filter: FilterType, left: u8, up: u8, upper_left: u8) -> u8 {
    match filter {
        FilterType::None => 0,
        FilterType::Sub => left,
        FilterType::Up => up,
        FilterType::Average => average(left, up),
        FilterType::Paeth => paeth(left, up, upper_left),
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

    /// Every filter, in the order of their type bytes.
    const ALL: [FilterType; 5] = [
        FilterType::None,
        FilterType::Sub,
        FilterType::Up,
        FilterType::Average,
        FilterType::Paeth,
    ];

    /// All the filtered rows of `data`.
    fn filter_image(
        data: &[u8],
        row_bytes: usize,
        stride: usize,
        strategy: FilterStrategy,
    ) -> Result<Vec<u8>, OutOfMemory> {
        let mut rows = FilteredRows::new(data, row_bytes, stride, strategy)?;
        Ok((0..rows.rows())
            .flat_map(|index| rows.row(index).to_vec())
            .collect())
    }

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
            for filter in ALL {
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
    fn adaptive_choice_is_pillows_first_filter_of_smallest_score() {
        // Rows of several chunks (filters are dropped part-way), of noise,
        // a gradient, a repeated row and a constant one (scores tie).
        let mut rng = Rng::new(24);
        for stride in [1, 3, 4, 8] {
            let row_bytes = stride * (2 * CHUNK / stride + 1 + rng.below(50));
            let mut image = rng.bytes(row_bytes * 6);
            for (i, byte) in image[row_bytes..2 * row_bytes].iter_mut().enumerate() {
                *byte = (i / 7) as u8;
            }
            image.copy_within(row_bytes..2 * row_bytes, 2 * row_bytes);
            image[4 * row_bytes..5 * row_bytes].fill(255);
            let adaptive =
                filter_image(&image, row_bytes, stride, FilterStrategy::Adaptive).unwrap();
            let tried = [
                FilterType::None,
                FilterType::Up,
                FilterType::Sub,
                FilterType::Paeth,
            ];
            let fixed: Vec<Vec<u8>> = tried
                .iter()
                .map(|&f| {
                    filter_image(&image, row_bytes, stride, FilterStrategy::Fixed(f)).unwrap()
                })
                .collect();
            for line in (0..adaptive.len()).step_by(row_bytes + 1) {
                let bytes = line..line + row_bytes + 1;
                let score = |filtered: &Vec<u8>| -> u64 {
                    filtered[line + 1..bytes.end]
                        .iter()
                        .map(|&b| u64::from((b as i8).unsigned_abs()))
                        .sum()
                };
                let (first_smallest, _) = fixed
                    .iter()
                    .enumerate()
                    .min_by_key(|&(index, filtered)| (score(filtered), index))
                    .expect("four filters");
                assert_eq!(
                    adaptive[bytes.clone()],
                    fixed[first_smallest][bytes],
                    "stride {stride}, line at {line}"
                );
            }
        }
    }

    #[test]
    fn rows_are_made_again_anywhere() {
        let image = Rng::new(23).bytes(40 * 6);
        let all = filter_image(&image, 40, 4, FilterStrategy::Adaptive).unwrap();
        let mut rows = FilteredRows::new(&image, 40, 4, FilterStrategy::Adaptive).unwrap();
        for index in [5, 1, 1, 0, 3] {
            assert_eq!(
                rows.row(index),
                &all[index * 41..(index + 1) * 41],
                "row {index}"
            );
        }
    }

    #[test]
    fn adaptive_choice_breaks_ties_as_pillow_does() {
        // Up, Sub and Paeth score 6 on the second row, no filter 7 and
        // Average 5: Pillow tries Up first, keeps it, and never tries
        // Average.
        let image = [2, 0, 3, 0, 3, 1, 1, 2];
        let filtered = filter_image(&image, 4, 1, FilterStrategy::Adaptive).unwrap();
        assert_eq!(filtered[0], FilterType::None.byte());
        assert_eq!(filtered[5..], [FilterType::Up.byte(), 1, 1, 254, 2]);
    }

    #[test]
    fn filter_bytes_round_trip() {
        for filter in ALL {
            assert_eq!(FilterType::from_byte(filter.byte()), Some(filter));
        }
        assert_eq!(FilterType::from_byte(5), None);
    }
}
