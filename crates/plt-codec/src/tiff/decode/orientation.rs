//! The `Orientation` tag, which Pillow applies to TIFF images as it loads
//! them (`ImageOps.exif_transpose`).

/// Maps an output pixel `(x, y)` to its source pixel, given the source's
/// width and height.
type Source = fn(usize, usize, usize, usize) -> (usize, usize);

/// An image of `width × height` pixels of `channels` bytes each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Canvas {
    /// Pixels across.
    pub(super) width: usize,
    /// Pixels down.
    pub(super) height: usize,
    /// Bytes per pixel.
    pub(super) channels: usize,
    /// The pixels, row by row.
    pub(super) bytes: Vec<u8>,
}

impl Canvas {
    /// The canvas turned upright for an `Orientation` value: 2 mirrors
    /// it, 3 turns it half round, 4 flips it, 5 transposes it, 6 turns it
    /// a quarter clockwise, 7 transposes it along the other diagonal and
    /// 8 turns it a quarter anticlockwise. Other values leave it alone.
    pub(super) fn oriented(self, orientation: u64) -> Self {
        let (w, h) = (self.width, self.height);
        // The source pixel of each output pixel, with the output's size.
        let (out_w, out_h, source): (usize, usize, Source) = match orientation {
            2 => (w, h, |x, y, w, _| (w - 1 - x, y)),
            3 => (w, h, |x, y, w, h| (w - 1 - x, h - 1 - y)),
            4 => (w, h, |x, y, _, h| (x, h - 1 - y)),
            5 => (h, w, |x, y, _, _| (y, x)),
            6 => (h, w, |x, y, _, h| (y, h - 1 - x)),
            7 => (h, w, |x, y, w, h| (w - 1 - y, h - 1 - x)),
            8 => (h, w, |x, y, w, _| (w - 1 - y, x)),
            _ => return self,
        };
        let channels = self.channels;
        let mut bytes = Vec::with_capacity(self.bytes.len());
        for y in 0..out_h {
            for x in 0..out_w {
                let (sx, sy) = source(x, y, w, h);
                let at = (sy * w + sx) * channels;
                bytes.extend_from_slice(&self.bytes[at..at + channels]);
            }
        }
        Self {
            width: out_w,
            height: out_h,
            channels,
            bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3 × 2 canvas of one-byte pixels:
    /// ```text
    /// 1 2 3
    /// 4 5 6
    /// ```
    fn canvas() -> Canvas {
        Canvas {
            width: 3,
            height: 2,
            channels: 1,
            bytes: vec![1, 2, 3, 4, 5, 6],
        }
    }

    #[test]
    fn every_orientation() {
        let cases: [(u64, (usize, usize), [u8; 6]); 8] = [
            (1, (3, 2), [1, 2, 3, 4, 5, 6]),
            (2, (3, 2), [3, 2, 1, 6, 5, 4]),
            (3, (3, 2), [6, 5, 4, 3, 2, 1]),
            (4, (3, 2), [4, 5, 6, 1, 2, 3]),
            (5, (2, 3), [1, 4, 2, 5, 3, 6]),
            (6, (2, 3), [4, 1, 5, 2, 6, 3]),
            (7, (2, 3), [6, 3, 5, 2, 4, 1]),
            (8, (2, 3), [3, 6, 2, 5, 1, 4]),
        ];
        for (orientation, size, bytes) in cases {
            let turned = canvas().oriented(orientation);
            assert_eq!((turned.width, turned.height), size, "{orientation}");
            assert_eq!(turned.bytes, bytes, "{orientation}");
        }
    }
}
