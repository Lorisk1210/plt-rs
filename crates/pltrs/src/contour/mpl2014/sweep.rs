//! The quads a sweep has to look at.
//!
//! A sweep visits every quad of a chunk, but most quads lie wholly on one
//! side of the levels and start nothing: the per-quad checks pass over
//! them. With one bit per grid point and z-level, the quads that can start
//! a contour are found 64 at a time and visited in the order a full sweep
//! reaches them. Every quad left out is one the per-quad checks would pass
//! over, so the contours are the same.

use super::super::chunks::ChunkLimits;

/// One bit per grid point (or per quad, by its SW point), row by row.
#[derive(Clone, Debug)]
pub(super) struct Bits {
    /// Points per row.
    nx: usize,
    /// 64-bit words per row.
    words: usize,
    rows: usize,
    data: Vec<u64>,
}

impl Bits {
    /// Cleared bits for `nx × ny` points.
    pub(super) fn new(nx: usize, ny: usize) -> Self {
        let words = nx.div_ceil(64);
        Self {
            nx,
            words,
            rows: ny,
            data: vec![0; words * ny],
        }
    }

    /// The bits of the points `i + j·nx` of an `nx × ny` grid for which
    /// `set` holds.
    pub(super) fn from_fn(nx: usize, ny: usize, set: impl Fn(usize) -> bool) -> Self {
        let mut bits = Self::new(nx, ny);
        for j in 0..ny {
            for i in (0..nx).filter(|i| set(i + j * nx)) {
                bits.data[j * bits.words + i / 64] |= 1 << (i % 64);
            }
        }
        bits
    }

    /// The words of row `j`.
    fn row(&self, j: usize) -> &[u64] {
        &self.data[j * self.words..(j + 1) * self.words]
    }

    /// The words of row `j`, to write.
    pub(super) fn row_mut(&mut self, j: usize) -> &mut [u64] {
        &mut self.data[j * self.words..(j + 1) * self.words]
    }

    /// Word `k` of row `j` moved one point west: bit `i` holds point
    /// `i + 1`.
    fn east_neighbours(&self, j: usize, k: usize) -> u64 {
        let row = self.row(j);
        (row[k] >> 1) | row.get(k + 1).map_or(0, |next| next << 63)
    }

    /// The set points `i + j·nx` within `chunk`, row by row.
    pub(super) fn quads(&self, chunk: &ChunkLimits, nx: usize) -> impl Iterator<Item = usize> {
        let (istart, iend) = (chunk.istart, chunk.iend);
        (chunk.jstart..chunk.jend.min(self.rows))
            .flat_map(move |j| set_bits(self.row(j), istart, iend).map(move |i| i + j * nx))
    }
}

/// The quads a line contour can start in: those whose four corners are
/// not all on one z-level (`planes` holds the points of each level above
/// the lowest).
pub(super) fn crossed_quads(planes: [&Bits; 2]) -> Bits {
    quads_where(planes, |plane, j, k| {
        let sw = plane.row(j)[k];
        let (se, nw, ne) = (
            plane.east_neighbours(j, k),
            plane.row(j + 1)[k],
            plane.east_neighbours(j + 1, k),
        );
        (sw ^ se) | (sw ^ nw) | (sw ^ ne)
    })
}

/// The quads a filled contour can start in: those in `complex` (whose
/// topology calls for the full checks) and those whose NW, NE and SE
/// points are not all on one z-level.
pub(super) fn fill_start_quads(planes: [&Bits; 2], complex: &Bits) -> Bits {
    let mut quads = quads_where(planes, |plane, j, k| {
        let ne = plane.east_neighbours(j + 1, k);
        (plane.row(j + 1)[k] ^ ne) | (plane.east_neighbours(j, k) ^ ne)
    });
    for (word, complex) in quads.data.iter_mut().zip(&complex.data) {
        *word |= complex;
    }
    quads
}

/// The quads for which `differ` finds a difference in either plane;
/// `differ` gets the plane, the quad row `j` and the word `k` and returns
/// one bit per quad. The last row and column of points start no quads.
fn quads_where(planes: [&Bits; 2], differ: impl Fn(&Bits, usize, usize) -> u64) -> Bits {
    let [first, _] = planes;
    let mut quads = Bits::new(first.nx, first.rows);
    let last_column = (first.nx - 1) / 64;
    let last_bit = 1 << ((first.nx - 1) % 64);
    for j in 0..first.rows.saturating_sub(1) {
        let row = quads.row_mut(j);
        for (k, word) in row.iter_mut().enumerate() {
            *word = planes
                .iter()
                .fold(0, |bits, plane| bits | differ(plane, j, k));
        }
        row[last_column] &= !last_bit;
    }
    quads
}

/// The indices of the set bits of `words` in `start..end`, ascending.
fn set_bits(words: &[u64], start: usize, end: usize) -> impl Iterator<Item = usize> {
    let last = end.div_ceil(64).min(words.len());
    (start / 64..last).flat_map(move |k| {
        let base = k * 64;
        let mut word = words[k];
        if start > base {
            word &= u64::MAX << (start - base);
        }
        if end < base + 64 {
            word &= (1 << (end - base)) - 1;
        }
        std::iter::from_fn(move || {
            (word != 0).then(|| {
                let bit = word.trailing_zeros() as usize;
                word &= word - 1;
                base + bit
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(istart: usize, iend: usize, jstart: usize, jend: usize) -> ChunkLimits {
        ChunkLimits {
            ichunk: 0,
            jchunk: 0,
            istart,
            iend,
            jstart,
            jend,
        }
    }

    #[test]
    fn set_bits_respect_the_range_across_words() {
        let words = [u64::MAX, 0b1011, 1 << 63];
        let found: Vec<usize> = set_bits(&words, 62, 192).collect();
        assert_eq!(found, [62, 63, 64, 65, 67, 191]);
        assert_eq!(set_bits(&words, 65, 67).collect::<Vec<_>>(), [65]);
        assert_eq!(set_bits(&words, 0, 0).count(), 0);
    }

    #[test]
    fn quads_come_row_by_row_within_the_chunk() {
        let (nx, ny) = (70, 3);
        let bits = Bits::from_fn(nx, ny, |p| p % 7 == 0);
        let all: Vec<usize> = bits.quads(&chunk(0, nx, 0, ny), nx).collect();
        assert_eq!(all, (0..nx * ny).filter(|p| p % 7 == 0).collect::<Vec<_>>());
        let part: Vec<usize> = bits.quads(&chunk(60, 69, 1, 2), nx).collect();
        assert_eq!(part, [133]);
    }

    /// The bits of the points `levels[p] == level`.
    fn plane(levels: &[u8], nx: usize, level: u8) -> Bits {
        Bits::from_fn(nx, levels.len() / nx, |p| levels[p] == level)
    }

    #[test]
    fn crossed_quads_have_corners_on_different_levels() {
        // 66 points per row, so that quads straddle a word boundary.
        let nx = 66;
        let mut levels = vec![0_u8; nx * 3];
        levels[64] = 1;
        levels[nx + 1] = 2;
        let crossed = crossed_quads([&plane(&levels, nx, 1), &plane(&levels, nx, 2)]);
        let found: Vec<usize> = crossed.quads(&chunk(0, nx, 0, 3), nx).collect();
        assert_eq!(found, [0, 1, 63, 64, nx, nx + 1]);
    }

    #[test]
    fn fill_starts_compare_the_nw_ne_and_se_points() {
        let nx = 4;
        let mut levels = vec![1_u8; nx * 3];
        // The SW point of quad 0 does not take part.
        levels[0] = 0;
        let planes = [&plane(&levels, nx, 1), &plane(&levels, nx, 2)];
        let none = Bits::new(nx, 3);
        assert_eq!(
            fill_start_quads(planes, &none)
                .quads(&chunk(0, nx, 0, 3), nx)
                .count(),
            0
        );
        // Point 10 is the NE point of quad 5 and the NW point of quad 6.
        levels[10] = 2;
        let planes = [&plane(&levels, nx, 1), &plane(&levels, nx, 2)];
        let complex = Bits::from_fn(nx, 3, |p| p == 0);
        let found: Vec<usize> = fill_start_quads(planes, &complex)
            .quads(&chunk(0, nx, 0, 3), nx)
            .collect();
        assert_eq!(found, [0, 5, 6]);
    }
}
