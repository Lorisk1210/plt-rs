//! Scanline filters (PNG spec §9): each byte is predicted from the
//! corresponding bytes of the pixel to its left, the pixel above and the
//! pixel above-left, and only the prediction error is stored.

use pltrs_core::alloc::{self, OutOfMemory};

use crate::deflate::input::ByteSource;

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

/// The filtered rows of an image, each prefixed with its filter type byte
/// (what the `IDAT` stream compresses), made a row at a time as they are
/// read: the compressor reads them through a window rather than from a
/// filtered copy of the whole image.
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
        let filter = match self.strategy {
            FilterStrategy::Fixed(filter) => {
                apply(
                    filter,
                    self.stride,
                    previous,
                    current,
                    &mut self.filtered[1..],
                );
                filter
            }
            FilterStrategy::Adaptive => {
                let mut best_filter = FilterType::None;
                let mut best_score = u64::MAX;
                for filter in FilterType::ALL {
                    apply(
                        filter,
                        self.stride,
                        previous,
                        current,
                        &mut self.candidate[1..],
                    );
                    let score = score(&self.candidate[1..]);
                    if score < best_score {
                        best_score = score;
                        best_filter = filter;
                        std::mem::swap(&mut self.candidate, &mut self.filtered);
                        if score == 0 {
                            break;
                        }
                    }
                }
                best_filter
            }
        };
        self.filtered[0] = filter.byte();
        self.filtered_row = Some(index);
    }
}

impl ByteSource for FilteredRows<'_> {
    fn len(&self) -> usize {
        self.data.len() / self.row_bytes * (self.row_bytes + 1)
    }

    fn append(&mut self, start: usize, count: usize, out: &mut Vec<u8>) {
        let end = (start + count).min(self.len());
        let line = self.row_bytes + 1;
        let mut position = start;
        while position < end {
            let (row, offset) = (position / line, position % line);
            self.filter_row(row);
            let taken = (line - offset).min(end - position);
            out.extend_from_slice(&self.filtered[offset..offset + taken]);
            position += taken;
        }
    }
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

    /// All the filtered rows of `data`, read in uneven pieces.
    fn filter_image(
        data: &[u8],
        row_bytes: usize,
        stride: usize,
        strategy: FilterStrategy,
    ) -> Result<Vec<u8>, OutOfMemory> {
        let mut rows = FilteredRows::new(data, row_bytes, stride, strategy)?;
        let mut out = Vec::new();
        let mut piece = 1;
        while out.len() < rows.len() {
            rows.append(out.len(), piece, &mut out);
            piece = piece * 3 + 1;
        }
        Ok(out)
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
    fn rows_are_made_again_anywhere() {
        let image = Rng::new(23).bytes(40 * 6);
        let all = filter_image(&image, 40, 4, FilterStrategy::Adaptive).unwrap();
        let mut rows = FilteredRows::new(&image, 40, 4, FilterStrategy::Adaptive).unwrap();
        for (start, count) in [(200, 30), (5, 100), (0, 1), (230, 1000)] {
            let mut out = Vec::new();
            rows.append(start, count, &mut out);
            let end = (start + count).min(all.len());
            assert_eq!(out, all[start..end], "{count} bytes from {start}");
        }
    }

    #[test]
    fn filter_bytes_round_trip() {
        for filter in FilterType::ALL {
            assert_eq!(FilterType::from_byte(filter.byte()), Some(filter));
        }
        assert_eq!(FilterType::from_byte(5), None);
    }
}
