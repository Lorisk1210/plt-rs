//! Bringing subsampled components to the image's size as libjpeg-turbo
//! does by default (`jdsample.c`): "fancy" triangle filtering for 2:1
//! ratios (`h2v1_fancy_upsample`, `h1v2_fancy_upsample`,
//! `h2v2_fancy_upsample`), and sample replication for other whole ratios
//! (`int_upsample`) and for components at most two samples wide.
//!
//! The main controller gives the filters the first row again above the
//! image and the last row again below it (`set_bottom_pointers`), and
//! the filters treat the first and last columns the same way, so every
//! neighbour lookup clamps to the component's samples. libjpeg's merged
//! upsampler (`jdmerge.c`) is only used without fancy upsampling, which
//! Pillow never turns off.

use super::idct::Plane;

/// The `width × height` samples of a component plane brought up by
/// `h_ratio` across and `v_ratio` down (whole numbers).
pub(super) fn upsample(
    plane: &Plane,
    h_ratio: usize,
    v_ratio: usize,
    width: usize,
    height: usize,
) -> Vec<u8> {
    let fancy_across = h_ratio == 2 && plane.width > 2;
    match (h_ratio, v_ratio) {
        (1, 1) => (0..height)
            .flat_map(|y| &plane.row(y)[..width])
            .copied()
            .collect(),
        (2, 1) if fancy_across => rows(height, |y| across(plane.row(y), width)),
        (1, 2) => rows(height, |y| {
            let (near, far, bias) = vertical_neighbours(plane, y);
            (0..width)
                .map(|x| narrow((3 * u32::from(near[x]) + u32::from(far[x]) + bias) >> 2))
                .collect()
        }),
        (2, 2) if fancy_across => rows(height, |y| {
            let (near, far, _) = vertical_neighbours(plane, y);
            let sums: Vec<u32> = near
                .iter()
                .zip(far)
                .map(|(&n, &f)| 3 * u32::from(n) + u32::from(f))
                .collect();
            (0..width)
                .map(|x| {
                    let (this, other, even) = horizontal_neighbours(&sums, x);
                    narrow((3 * this + other + if even { 8 } else { 7 }) >> 4)
                })
                .collect()
        }),
        _ => rows(height, |y| {
            let row = plane.row(y / v_ratio);
            (0..width).map(|x| row[x / h_ratio]).collect()
        }),
    }
}

/// The output rows `0..height` made by `row`.
fn rows(height: usize, row: impl Fn(usize) -> Vec<u8>) -> Vec<u8> {
    (0..height).flat_map(row).collect()
}

/// A filtered value back to a sample.
fn narrow(value: u32) -> u8 {
    u8::try_from(value).expect("a weighted average of samples")
}

/// One row filtered 2:1 across (`h2v1_fancy_upsample`): each output is
/// ¾ of the nearer input and ¼ of the further one, rounding half the
/// outputs down and half up.
fn across(row: &[u8], width: usize) -> Vec<u8> {
    let samples: Vec<u32> = row.iter().map(|&s| u32::from(s)).collect();
    (0..width)
        .map(|x| {
            let (this, other, even) = horizontal_neighbours(&samples, x);
            narrow((3 * this + other + if even { 1 } else { 2 }) >> 2)
        })
        .collect()
}

/// For output column `x` of a 2:1 filter over `values`: the nearer
/// input, the further one (clamped to the row) and whether `x` is even
/// (even columns lean left and round down, odd ones lean right and
/// round up).
fn horizontal_neighbours(values: &[u32], x: usize) -> (u32, u32, bool) {
    let column = x / 2;
    if x.is_multiple_of(2) {
        (values[column], values[column.saturating_sub(1)], true)
    } else {
        (
            values[column],
            values[(column + 1).min(values.len() - 1)],
            false,
        )
    }
}

/// For output row `y` of a 2:1 filter down a plane: the nearer input
/// row, the further one (clamped to the plane) and the rounding bias.
fn vertical_neighbours(plane: &Plane, y: usize) -> (&[u8], &[u8], u32) {
    let row = y / 2;
    if y.is_multiple_of(2) {
        (plane.row(row), plane.row(row.saturating_sub(1)), 1)
    } else {
        (
            plane.row(row),
            plane.row((row + 1).min(plane.height - 1)),
            2,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(width: usize, height: usize, samples: &[u8]) -> Plane {
        Plane {
            width,
            height,
            stride: width,
            samples: samples.to_vec(),
        }
    }

    #[test]
    fn across_follows_h2v1_fancy_upsample() {
        let p = plane(3, 1, &[0, 100, 200]);
        // First output copies; then (3·near + far + 1 or 2) / 4.
        assert_eq!(upsample(&p, 2, 1, 6, 1), [0, 25, 75, 125, 175, 200]);
    }

    #[test]
    fn down_follows_h1v2_fancy_upsample() {
        let p = plane(1, 2, &[0, 100]);
        assert_eq!(upsample(&p, 1, 2, 1, 4), [0, 25, 75, 100]);
    }

    #[test]
    fn both_ways_follow_h2v2_fancy_upsample() {
        let p = plane(3, 2, &[0, 80, 160, 40, 120, 200]);
        let out = upsample(&p, 2, 2, 6, 4);
        // Row 0: column sums are 4·row 0 (the row above repeats it).
        assert_eq!(&out[..6], [0, 20, 60, 100, 140, 160]);
        // Row 1: 3·row 0 + row 1 = 40, 360, 680 (·16: 2.5, 22.5, 42.5).
        assert_eq!(out[6], 10, "(3 · 40 + 40 + 8) / 16");
        assert_eq!(out[7], 30, "(3 · 40 + 360 + 7) / 16");
    }

    #[test]
    fn narrow_components_and_other_ratios_replicate() {
        let p = plane(2, 1, &[10, 20]);
        assert_eq!(upsample(&p, 2, 1, 3, 1), [10, 10, 20]);
        let p = plane(2, 1, &[10, 20]);
        assert_eq!(
            upsample(&p, 4, 2, 7, 2),
            [10, 10, 10, 10, 20, 20, 20].repeat(2)
        );
    }
}
