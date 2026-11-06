//! Gaussian blur of coverage masks, as SVG's `feGaussianBlur` computes
//! it (Filter Effects 1, §9.2): for a standard deviation of 2 pixels or
//! more, three successive box blurs; below, a sampled Gaussian kernel.
//! Pixels outside the mask count as transparent, so coverage near the
//! edges fades out.

use crate::mask::Mask;

/// From this standard deviation on, three box blurs stand in for the
/// Gaussian, as the specification allows.
const BOX_THRESHOLD: f64 = 2.0;

/// One pass of a separable blur along a line of pixels.
#[derive(Debug, Clone, PartialEq)]
enum Pass {
    /// The mean of the `len` pixels starting `offset` pixels before the
    /// output pixel (the sum divided by `len`, pixels outside counting as
    /// zero).
    Box {
        /// How far before the output pixel the box starts.
        offset: i64,
        /// The box's width.
        len: i64,
    },
    /// A normalised kernel centred on the output pixel, of odd length.
    Kernel(Vec<f32>),
}

impl Pass {
    /// The passes of a blur of standard deviation `sigma` (positive).
    fn for_sigma(sigma: f64) -> Vec<Self> {
        if sigma >= BOX_THRESHOLD {
            // d = ⌊s · 3√(2π)/4 + 0.5⌋; the saturating cast keeps absurd
            // deviations finite (a box wider than the mask clears it).
            let d = (sigma * 3.0 * (2.0 * std::f64::consts::PI).sqrt() / 4.0 + 0.5).floor();
            let d = (d as i64).clamp(1, 1 << 40);
            let half = d / 2;
            if d % 2 == 1 {
                vec![
                    Self::Box {
                        offset: half,
                        len: d
                    };
                    3
                ]
            } else {
                // Two boxes centred on the pixel's left and right edges,
                // then one of d + 1 centred on the pixel.
                vec![
                    Self::Box {
                        offset: half,
                        len: d,
                    },
                    Self::Box {
                        offset: half - 1,
                        len: d,
                    },
                    Self::Box {
                        offset: half,
                        len: d + 1,
                    },
                ]
            }
        } else {
            let radius = (3.0 * sigma).ceil() as i32;
            let weights: Vec<f64> = (-radius..=radius)
                .map(|k| (-f64::from(k * k) / (2.0 * sigma * sigma)).exp())
                .collect();
            let total: f64 = weights.iter().sum();
            vec![Self::Kernel(
                weights.iter().map(|w| (w / total) as f32).collect(),
            )]
        }
    }

    /// Blurs `line` into `out` (of the same length).
    fn apply(&self, line: &[f32], out: &mut [f32], prefix: &mut Vec<f64>) {
        let n = line.len() as i64;
        match self {
            Self::Box { offset, len } => {
                prefix.clear();
                prefix.push(0.0);
                let mut sum = 0.0;
                for &value in line {
                    sum += f64::from(value);
                    prefix.push(sum);
                }
                let at = |i: i64| prefix[i.clamp(0, n) as usize];
                let scale = 1.0 / *len as f64;
                for (x, value) in (0..n).zip(out.iter_mut()) {
                    let start = x.saturating_sub(*offset);
                    let end = start.saturating_add(*len);
                    *value = ((at(end) - at(start)) * scale) as f32;
                }
            }
            Self::Kernel(weights) => {
                let radius = (weights.len() / 2) as i64;
                for (x, value) in (0..n).zip(out.iter_mut()) {
                    *value = weights
                        .iter()
                        .zip(x - radius..)
                        .filter(|&(_, i)| (0..n).contains(&i))
                        .map(|(w, i)| w * line[i as usize])
                        .sum();
                }
            }
        }
    }
}

impl Mask {
    /// This mask blurred by a Gaussian of standard deviation `sigma`
    /// pixels, as SVG's `feGaussianBlur` blurs an alpha channel: three
    /// box blurs from 2 pixels on, a sampled kernel below, pixels outside
    /// the mask counting as transparent. A `sigma` that is not positive
    /// leaves the mask as it is.
    ///
    /// ```
    /// use pltrs_raster::Mask;
    /// let mut dot = Mask::new(9, 9)?;
    /// dot.data_mut()[4 * 9 + 4] = 255;
    /// let soft = dot.blurred(1.0);
    /// assert!(soft.get(4, 4) < Some(255));
    /// assert!(soft.get(5, 4) > Some(0));
    /// # Ok::<(), pltrs_raster::RasterError>(())
    /// ```
    #[must_use]
    pub fn blurred(&self, sigma: f64) -> Self {
        if sigma.is_nan() || sigma <= 0.0 || self.data().is_empty() {
            return self.clone();
        }
        let passes = Pass::for_sigma(sigma);
        let (width, height) = (self.width() as usize, self.height() as usize);
        let mut pixels: Vec<f32> = self.data().iter().map(|&v| f32::from(v)).collect();
        let mut prefix = Vec::new();
        let mut line = Vec::with_capacity(width.max(height));
        let mut out = Vec::with_capacity(width.max(height));
        let mut blur_line = |line: &mut Vec<f32>, out: &mut Vec<f32>| {
            out.resize(line.len(), 0.0);
            for pass in &passes {
                pass.apply(line, out, &mut prefix);
                std::mem::swap(line, out);
            }
        };
        for row in pixels.chunks_exact_mut(width) {
            line.clear();
            line.extend_from_slice(row);
            blur_line(&mut line, &mut out);
            row.copy_from_slice(&line);
        }
        for x in 0..width {
            line.clear();
            line.extend(pixels.iter().skip(x).step_by(width));
            blur_line(&mut line, &mut out);
            for (pixel, &value) in pixels.iter_mut().skip(x).step_by(width).zip(&line) {
                *pixel = value;
            }
        }
        let data = pixels
            .iter()
            .map(|&v| v.round().clamp(0.0, 255.0) as u8)
            .collect();
        Self::from_vec(self.width(), self.height(), data)
            .expect("the blurred mask has the size of the original")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(side: u32, from: u32, to: u32) -> Mask {
        let mut mask = Mask::new(side, side).unwrap();
        for y in from..to {
            for x in from..to {
                mask.data_mut()[(y * side + x) as usize] = 255;
            }
        }
        mask
    }

    fn total(mask: &Mask) -> f64 {
        mask.data().iter().map(|&v| f64::from(v)).sum()
    }

    #[test]
    fn box_sizes_follow_the_specification() {
        // σ = 14: d = ⌊14 · 1.8799 + 0.5⌋ = 26, even.
        assert_eq!(
            Pass::for_sigma(14.0),
            [
                Pass::Box {
                    offset: 13,
                    len: 26
                },
                Pass::Box {
                    offset: 12,
                    len: 26
                },
                Pass::Box {
                    offset: 13,
                    len: 27
                },
            ]
        );
        // σ = 2.5: d = ⌊5.2⌋ = 5, odd: three centred boxes.
        assert_eq!(
            Pass::for_sigma(2.5),
            vec![Pass::Box { offset: 2, len: 5 }; 3]
        );
        let Pass::Kernel(weights) = &Pass::for_sigma(0.5)[0] else {
            panic!("small deviations sample the Gaussian");
        };
        assert_eq!(weights.len(), 5);
        assert!((weights.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!(weights[2] > weights[1] && weights[1] > weights[0]);
    }

    #[test]
    fn blurring_keeps_coverage_and_symmetry() {
        for sigma in [0.7, 3.0, 4.0] {
            let mask = square(64, 24, 40);
            let blurred = mask.blurred(sigma);
            // Far from the edges nothing is lost (rounding aside).
            assert!(
                (total(&blurred) - total(&mask)).abs() < 64.0 * 64.0 * 0.5,
                "σ = {sigma}"
            );
            assert!(blurred.get(32, 32).unwrap() > 200, "σ = {sigma}");
            assert!(blurred.get(23, 32).unwrap() > 0, "spreads, σ = {sigma}");
            assert_eq!(blurred.get(0, 0), Some(0), "σ = {sigma}");
            // Symmetric about the square's centre (31.5), up to the half
            // pixel even boxes shift.
            for i in 0..20 {
                let (a, b) = (
                    i32::from(blurred.get(31 - i, 32).unwrap()),
                    i32::from(blurred.get(32 + i, 32).unwrap()),
                );
                assert!((a - b).abs() <= 3, "σ = {sigma}, {i}: {a} vs {b}");
            }
        }
    }

    #[test]
    fn edges_fade_into_transparency() {
        let full = Mask::from_vec(32, 32, vec![255; 32 * 32]).unwrap();
        let blurred = full.blurred(3.0);
        assert!(blurred.get(0, 0).unwrap() < 128, "corner");
        assert_eq!(blurred.get(16, 16), Some(255), "centre");
    }

    #[test]
    fn degenerate_deviations_and_sizes() {
        let mask = square(8, 2, 6);
        assert_eq!(mask.blurred(0.0), mask);
        assert_eq!(mask.blurred(-1.0), mask);
        assert_eq!(mask.blurred(f64::NAN), mask);
        assert_eq!(Mask::new(0, 0).unwrap().blurred(2.0).width(), 0);
        let line = Mask::from_vec(5, 1, vec![0, 0, 255, 0, 0]).unwrap();
        assert!(line.blurred(1.0).get(1, 0).unwrap() > 0);
        // An absurd deviation spreads the coverage to nothing.
        assert!(mask.blurred(f64::INFINITY).data().iter().all(|&v| v == 0));
        assert!(mask.blurred(1e300).data().iter().all(|&v| v == 0));
    }
}
