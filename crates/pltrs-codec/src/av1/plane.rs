//! Planes of 8-bit samples, padded right and down to whole superblocks
//! so that blocks reaching past the picture's edge stay in bounds.

/// One plane of samples.
#[derive(Clone, Debug)]
pub(crate) struct Plane {
    /// The visible width.
    pub(crate) width: usize,
    /// The visible height.
    pub(crate) height: usize,
    /// The padded width, the distance between rows.
    pub(crate) stride: usize,
    /// The samples of the padded rows.
    pub(crate) data: Vec<u8>,
}

impl Plane {
    /// `width × height` samples padded to `stride × rows` by repeating
    /// the last column and row, as the source of a picture.
    pub(crate) fn padded(
        samples: &[u8],
        (width, height): (usize, usize),
        (stride, rows): (usize, usize),
    ) -> Self {
        let mut data = Vec::with_capacity(stride * rows);
        for y in 0..rows {
            let row = &samples[y.min(height - 1) * width..][..width];
            data.extend_from_slice(row);
            data.resize((y + 1) * stride, row[width - 1]);
        }
        Self {
            width,
            height,
            stride,
            data,
        }
    }

    /// A blank plane of the same size, for reconstructed samples.
    pub(crate) fn blank_like(&self) -> Self {
        Self {
            data: vec![0; self.data.len()],
            ..*self
        }
    }

    /// The sample at column `x`, row `y`.
    pub(crate) fn at(&self, x: usize, y: usize) -> u8 {
        self.data[y * self.stride + x]
    }

    /// The `size × size` square at `(x, y)` copied row by row.
    pub(crate) fn square(&self, x: usize, y: usize, size: usize) -> Vec<u8> {
        (y..y + size)
            .flat_map(|row| &self.data[row * self.stride + x..][..size])
            .copied()
            .collect()
    }

    /// Writes a `size × size` square, row by row, at `(x, y)`.
    pub(crate) fn put_square(&mut self, x: usize, y: usize, size: usize, samples: &[u8]) {
        for (row, line) in samples.chunks_exact(size).enumerate() {
            let start = (y + row) * self.stride + x;
            self.data[start..start + size].copy_from_slice(line);
        }
    }

    /// The columns and rows of the `size × size` square at `(x, y)` that
    /// are visible.
    fn visible(&self, x: usize, y: usize, size: usize) -> (usize, usize) {
        (
            size.min(self.width.saturating_sub(x)),
            size.min(self.height.saturating_sub(y)),
        )
    }

    /// The squared error between two planes over the visible part of the
    /// `size × size` square at `(x, y)`.
    pub(crate) fn squared_error(&self, other: &Self, x: usize, y: usize, size: usize) -> u64 {
        let (columns, rows) = self.visible(x, y, size);
        (y..y + rows)
            .map(|row| {
                let start = row * self.stride + x;
                squared_difference(
                    &self.data[start..][..columns],
                    &other.data[start..][..columns],
                )
            })
            .sum()
    }

    /// The squared error of `samples`, a `size × size` square row by row,
    /// against the visible part of the square at `(x, y)`.
    pub(crate) fn error_against(&self, x: usize, y: usize, size: usize, samples: &[u8]) -> u64 {
        let (columns, rows) = self.visible(x, y, size);
        (0..rows)
            .map(|row| {
                let start = (y + row) * self.stride + x;
                squared_difference(
                    &self.data[start..][..columns],
                    &samples[row * size..][..columns],
                )
            })
            .sum()
    }
}

/// The sum of squared differences of two runs of samples.
fn squared_difference(a: &[u8], b: &[u8]) -> u64 {
    a.iter()
        .zip(b)
        .map(|(&a, &b)| {
            let d = u64::from(a.abs_diff(b));
            d * d
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_repeats_the_edges() {
        let plane = Plane::padded(&[1, 2, 3, 4, 5, 6], (3, 2), (4, 3));
        assert_eq!(plane.data, [1, 2, 3, 3, 4, 5, 6, 6, 4, 5, 6, 6]);
        assert_eq!(plane.at(3, 2), 6);
        assert_eq!(plane.square(2, 1, 2), [6, 6, 6, 6]);
    }

    #[test]
    fn errors_count_only_visible_samples() {
        let source = Plane::padded(&[10, 20, 30, 40], (2, 2), (4, 4));
        let mut recon = source.blank_like();
        recon.put_square(0, 0, 4, &[0; 16]);
        assert_eq!(
            source.squared_error(&recon, 0, 0, 4),
            100 + 400 + 900 + 1600
        );
        assert_eq!(source.squared_error(&recon, 1, 1, 2), 1600);
        assert_eq!(source.squared_error(&recon, 2, 0, 2), 0);
        assert_eq!(source.error_against(1, 0, 2, &[18, 0, 39, 0]), 4 + 1);
    }
}
