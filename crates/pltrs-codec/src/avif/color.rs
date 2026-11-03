//! RGB to YUV, as libavif converts without libyuv: BT.601 luma and chroma
//! in full range from normalized floats, chroma averaged over each 2×2
//! square for 4:2:0, and the identity (G, B, R) for lossless pictures.

use crate::av1::ChromaSampling;

/// `Kr` of BT.601.
const KR: f32 = 0.299;
/// `Kb` of BT.601.
const KB: f32 = 0.114;
/// `Kg`, the rest.
const KG: f32 = 1.0 - KR - KB;

/// The planes of a picture, row by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Planes {
    /// Y (or G).
    pub(crate) y: Vec<u8>,
    /// U (or B); empty for monochrome.
    pub(crate) u: Vec<u8>,
    /// V (or R); empty for monochrome.
    pub(crate) v: Vec<u8>,
}

/// A normalized value as an 8-bit sample: rounded half away from zero,
/// clamped.
fn to_byte(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// The Y, U and V of an RGB pixel, normalized: Y in `[0, 1]`, U and V in
/// `[-0.5, 0.5]`.
fn yuv([r, g, b]: [u8; 3]) -> [f32; 3] {
    let [r, g, b] = [r, g, b].map(|c| f32::from(c) / 255.0);
    let y = KR * r + KG * g + KB * b;
    [
        y,
        (b - y) / (2.0 * (1.0 - KB)),
        (r - y) / (2.0 * (1.0 - KR)),
    ]
}

/// The BT.601 full-range planes of `width × height` RGBA pixels in
/// `sampling` (alpha is ignored).
pub(crate) fn bt601(width: usize, height: usize, rgba: &[u8], sampling: ChromaSampling) -> Planes {
    let pixel = |x: usize, y: usize| {
        let i = (y * width + x) * 4;
        yuv([rgba[i], rgba[i + 1], rgba[i + 2]])
    };
    let y_plane = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| to_byte(pixel(x, y)[0] * 255.0))
        .collect();
    let chroma = |channel: usize| -> Vec<u8> {
        match sampling {
            ChromaSampling::Monochrome => Vec::new(),
            ChromaSampling::Yuv444 => (0..height)
                .flat_map(|y| (0..width).map(move |x| (x, y)))
                .map(|(x, y)| to_byte(pixel(x, y)[channel] * 255.0 + 128.0))
                .collect(),
            ChromaSampling::Yuv420 => {
                let (cw, ch) = (width.div_ceil(2), height.div_ceil(2));
                (0..ch)
                    .flat_map(|cy| (0..cw).map(move |cx| (cx, cy)))
                    .map(|(cx, cy)| {
                        let xs = 2 * cx..(2 * cx + 2).min(width);
                        let ys = 2 * cy..(2 * cy + 2).min(height);
                        let count = (xs.len() * ys.len()) as f32;
                        let sum: f32 = ys
                            .flat_map(|y| xs.clone().map(move |x| (x, y)))
                            .map(|(x, y)| pixel(x, y)[channel])
                            .sum();
                        to_byte(sum / count * 255.0 + 128.0)
                    })
                    .collect()
            }
        }
    };
    Planes {
        y: y_plane,
        u: chroma(1),
        v: chroma(2),
    }
}

/// The identity planes of RGBA pixels: G, B and R as they are.
pub(crate) fn identity(rgba: &[u8]) -> Planes {
    let channel = |c: usize| rgba.as_chunks::<4>().0.iter().map(|p| p[c]).collect();
    Planes {
        y: channel(1),
        u: channel(2),
        v: channel(0),
    }
}

/// The alpha plane of RGBA pixels.
pub(crate) fn alpha(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>().0.iter().map(|p| p[3]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primaries_and_greys() {
        let rgba = [
            255, 255, 255, 255, 0, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255,
        ];
        let planes = bt601(4, 1, &rgba, ChromaSampling::Yuv444);
        assert_eq!(planes.y, [255, 0, 76, 29]);
        assert_eq!(planes.u, [128, 128, 85, 255]);
        assert_eq!(planes.v, [128, 128, 255, 107]);
    }

    #[test]
    fn chroma_averages_over_squares() {
        // 3×1: one full pair and a lone pixel.
        let rgba = [255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255];
        let planes = bt601(3, 1, &rgba, ChromaSampling::Yuv420);
        assert_eq!(planes.y.len(), 3);
        assert_eq!(planes.u, [170, 255]);
        assert_eq!(planes.v, [181, 107]);
        let grey = bt601(3, 1, &rgba, ChromaSampling::Monochrome);
        assert!(grey.u.is_empty() && grey.v.is_empty());
    }

    #[test]
    fn identity_reorders_channels() {
        let planes = identity(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(
            (planes.y, planes.u, planes.v),
            (vec![2, 6], vec![3, 7], vec![1, 5])
        );
        assert_eq!(alpha(&[1, 2, 3, 4, 5, 6, 7, 8]), [4, 8]);
    }
}
