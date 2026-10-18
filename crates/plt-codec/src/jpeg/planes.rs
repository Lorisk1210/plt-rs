//! From RGB pixels to the three padded component planes of a 4:2:0 JPEG,
//! as libjpeg prepares them: fixed-point YCbCr conversion (`jccolor.c`),
//! edge replication (`jcprepct.c`, `jcsample.c`) and 2×2 chroma averaging
//! with alternating rounding bias (`h2v2_downsample`).

/// Fractional bits of the colour conversion constants.
const SCALE_BITS: u32 = 16;
const ONE_HALF: i32 = 1 << (SCALE_BITS - 1);
const CBCR_OFFSET: i32 = 128 << SCALE_BITS;

/// The weights of R, G and B in Y, Cb and Cr (`FIX(x)` of `jccolor.c`).
const Y_WEIGHTS: [i32; 3] = [19595, 38470, 7471];
const CB_WEIGHTS: [i32; 3] = [-11059, -21709, 32768];
const CR_WEIGHTS: [i32; 3] = [32768, -27439, -5329];

/// One component's samples, `width × height`, row by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Plane {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) samples: Vec<u8>,
}

impl Plane {
    /// The level-shifted 8×8 block whose top-left sample is
    /// `(8 × column, 8 × row)`.
    pub(super) fn block(&self, column: usize, row: usize) -> [i32; 64] {
        std::array::from_fn(|i| {
            let (x, y) = (column * 8 + i % 8, row * 8 + i / 8);
            i32::from(self.samples[y * self.width + x]) - 128
        })
    }

    /// The plane widened to `width` by repeating its last column and
    /// lengthened to `height` by repeating its last row.
    fn extended(&self, width: usize, height: usize) -> Self {
        let mut samples = Vec::with_capacity(width * height);
        for row in self.samples.chunks_exact(self.width) {
            samples.extend_from_slice(row);
            let last = row[self.width - 1];
            samples.extend(std::iter::repeat_n(last, width - self.width));
        }
        let last_row = samples[samples.len() - width..].to_vec();
        for _ in self.height..height {
            samples.extend_from_slice(&last_row);
        }
        Self {
            width,
            height,
            samples,
        }
    }

    /// The plane halved both ways, each sample the average of a 2×2 square
    /// rounded with a bias alternating 1, 2, 1, … along the row.
    fn halved(&self) -> Self {
        let (width, height) = (self.width / 2, self.height / 2);
        let mut samples = Vec::with_capacity(width * height);
        for pair in self.samples.chunks_exact(2 * self.width) {
            let (upper, lower) = pair.split_at(self.width);
            for (x, bias) in (0..width).zip([1_u16, 2].into_iter().cycle()) {
                let sum: u16 = [
                    upper[2 * x],
                    upper[2 * x + 1],
                    lower[2 * x],
                    lower[2 * x + 1],
                ]
                .into_iter()
                .map(u16::from)
                .sum();
                samples.push(u8::try_from((sum + bias) >> 2).expect("an average of bytes"));
            }
        }
        Self {
            width,
            height,
            samples,
        }
    }
}

/// The number of `block`-long pieces covering `length`.
fn blocks(length: usize, block: usize) -> usize {
    length.div_ceil(block)
}

/// The Y, Cb and Cr planes of `width × height` RGB pixels (both sides
/// non-zero), padded as libjpeg pads them for 4:2:0 sampling: Y to whole
/// 8×8 blocks across and whole 16-row MCUs down, Cb and Cr to whole
/// blocks of half the size.
pub(super) fn planes(width: usize, height: usize, rgb: &[u8]) -> [Plane; 3] {
    let convert = |weights: [i32; 3], bias: i32| Plane {
        width,
        height,
        samples: rgb
            .as_chunks::<3>()
            .0
            .iter()
            .map(|pixel| {
                let sum: i32 = pixel
                    .iter()
                    .zip(weights)
                    .map(|(&channel, weight)| weight * i32::from(channel))
                    .sum();
                u8::try_from((sum + bias) >> SCALE_BITS).expect("YCbCr samples are bytes")
            })
            .collect(),
    };
    let luma = convert(Y_WEIGHTS, ONE_HALF);
    let mcu_rows = blocks(height, 16);
    let luma = luma.extended(blocks(width, 8) * 8, mcu_rows * 16);
    let chroma = |weights| {
        // Rows are converted in pairs, the last repeated for an odd
        // height; the halved plane is then padded to whole MCUs.
        let full = convert(weights, CBCR_OFFSET + ONE_HALF - 1);
        let half = full
            .extended(blocks(width, 16) * 16, blocks(height, 2) * 2)
            .halved();
        half.extended(half.width, mcu_rows * 8)
    };
    [luma, chroma(CB_WEIGHTS), chroma(CR_WEIGHTS)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grey_has_neutral_chroma() {
        let [y, cb, cr] = planes(1, 1, &[100, 100, 100]);
        assert_eq!((y.width, y.height), (8, 16));
        assert!(y.samples.iter().all(|&s| s == 100));
        assert_eq!((cb.width, cb.height), (8, 8));
        assert!(cb.samples.iter().chain(&cr.samples).all(|&s| s == 128));
    }

    #[test]
    fn primaries_convert_like_libjpeg() {
        let [y, cb, cr] = planes(1, 1, &[255, 0, 0]);
        assert_eq!((y.samples[0], cb.samples[0], cr.samples[0]), (76, 85, 255));
        let [y, cb, cr] = planes(1, 1, &[0, 0, 255]);
        assert_eq!((y.samples[0], cb.samples[0], cr.samples[0]), (29, 255, 107));
    }

    #[test]
    fn chroma_averages_with_alternating_bias() {
        // Cb is 255 for blue and 128 for black: each 2×2 square averages
        // 191.5, rounded down, then up.
        let blue = [0, 0, 255];
        let black = [0, 0, 0];
        let row: Vec<u8> = [blue, black, blue, black].concat();
        let rgb = [row.clone(), row].concat();
        let [_, cb, _] = planes(4, 2, &rgb);
        assert_eq!(
            &cb.samples[..3],
            [191, 192, 128],
            "then the padding's black"
        );
    }

    #[test]
    fn edges_repeat_the_last_row_and_column() {
        let plane = Plane {
            width: 2,
            height: 2,
            samples: vec![1, 2, 3, 4],
        };
        let wide = plane.extended(3, 3);
        assert_eq!(wide.samples, [1, 2, 2, 3, 4, 4, 3, 4, 4]);
    }
}
