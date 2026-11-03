//! Choosing palettes for a block: its exact colours when it has at most
//! eight, and the colours k-means finds, for the search to try. Blocks of
//! more than 64 distinct colours (photographs, gradients) get none.

use super::{MAX_COLORS, Palette};
use crate::av1::plane::Plane;

/// The most distinct colours of a block worth a palette.
const MAX_DISTINCT: usize = 64;
/// The rounds of k-means.
const ROUNDS: usize = 5;
/// A guess of the bits a colour index costs per bit of entropy of the
/// map: neighbours predict most indices.
const MAP_BITS_PER_ENTROPY: f64 = 0.5;

/// Where the block lies in a plane.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Area {
    /// The top-left sample.
    pub(crate) at: (usize, usize),
    /// The block's side.
    pub(crate) side: usize,
    /// The columns and rows on screen.
    pub(crate) onscreen: (usize, usize),
}

/// The distinct colours of the block's samples on screen and how often
/// each occurs, ascending.
fn histogram<const D: usize>(samples: &[[u8; D]]) -> Vec<([u8; D], u32)> {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let mut counts: Vec<([u8; D], u32)> = Vec::new();
    for point in sorted {
        match counts.last_mut() {
            Some((last, count)) if *last == point => *count += 1,
            _ => counts.push((point, 1)),
        }
    }
    counts
}

/// The squared distance of two colours.
fn distance<const D: usize>(a: [u8; D], b: [u8; D]) -> u32 {
    a.iter()
        .zip(&b)
        .map(|(&x, &y)| {
            let d = u32::from(x.abs_diff(y));
            d * d
        })
        .sum()
}

/// The index of the colour of `centers` nearest `point`.
fn nearest<const D: usize>(centers: &[[u8; D]], point: [u8; D]) -> usize {
    (0..centers.len())
        .min_by_key(|&i| distance(centers[i], point))
        .expect("a palette has colours")
}

/// `k` colours for `counts` by k-means, from the `k` most frequent:
/// ascending and distinct.
fn kmeans<const D: usize>(counts: &[([u8; D], u32)], k: usize) -> Vec<[u8; D]> {
    let mut by_frequency: Vec<&([u8; D], u32)> = counts.iter().collect();
    by_frequency.sort_by_key(|&&(color, count)| (std::cmp::Reverse(count), color));
    let mut centers: Vec<[u8; D]> = by_frequency.iter().take(k).map(|&&(c, _)| c).collect();
    for _ in 0..ROUNDS {
        let mut sums = vec![([0u64; D], 0u64); centers.len()];
        for &(color, count) in counts {
            let (sum, total) = &mut sums[nearest(&centers, color)];
            for (s, &c) in sum.iter_mut().zip(&color) {
                *s += u64::from(c) * u64::from(count);
            }
            *total += u64::from(count);
        }
        for (center, (sum, total)) in centers.iter_mut().zip(&sums) {
            if *total > 0 {
                *center = sum.map(|s| ((s + total / 2) / total) as u8);
            }
        }
    }
    centers.sort_unstable();
    centers.dedup();
    centers
}

/// The squared error of mapping `counts` to `centers`, and the entropy
/// of the indices, in bits.
fn proxy<const D: usize>(counts: &[([u8; D], u32)], centers: &[[u8; D]]) -> (f64, f64) {
    let mut uses = vec![0u32; centers.len()];
    let mut error = 0u64;
    for &(color, count) in counts {
        let index = nearest(centers, color);
        uses[index] += count;
        error += u64::from(distance(centers[index], color)) * u64::from(count);
    }
    let total: u32 = uses.iter().sum();
    let entropy = uses
        .iter()
        .filter(|&&n| n > 0)
        .map(|&n| {
            let p = f64::from(n) / f64::from(total);
            -p * p.log2() * f64::from(n)
        })
        .sum();
    (error as f64, entropy)
}

/// The palette of `centers` for the samples of `area`, read by `sample`.
fn palette<const D: usize>(
    centers: &[[u8; D]],
    area: Area,
    sample: impl Fn(usize, usize) -> [u8; D],
) -> Palette {
    let (width, height) = area.onscreen;
    let side = area.side;
    let mut map = vec![0u8; side * side];
    for row in 0..side {
        for col in 0..side {
            map[row * side + col] = if row < height && col < width {
                nearest(centers, sample(area.at.0 + col, area.at.1 + row)) as u8
            } else {
                map[row.min(height - 1) * side + col.min(width - 1)]
            };
        }
    }
    let colors = (0..D)
        .map(|d| centers.iter().map(|c| c[d]).collect())
        .collect();
    Palette {
        colors,
        map,
        side,
        onscreen: area.onscreen,
    }
}

/// The colour sets worth trying for the samples of `area`: the exact
/// colours when there are at most eight, and the k-means colours whose
/// error and guessed rate (weighed by `lambda`) are least.
fn candidates<const D: usize>(
    area: Area,
    sample: impl Fn(usize, usize) -> [u8; D],
    lambda: f64,
) -> Vec<Palette> {
    let (width, height) = area.onscreen;
    let samples: Vec<[u8; D]> = (0..height)
        .flat_map(|row| (0..width).map(move |col| (col, row)))
        .map(|(col, row)| sample(area.at.0 + col, area.at.1 + row))
        .collect();
    let counts = histogram(&samples);
    if counts.len() < 2 || counts.len() > MAX_DISTINCT {
        return Vec::new();
    }
    let mut sets: Vec<Vec<[u8; D]>> = Vec::new();
    if counts.len() <= MAX_COLORS {
        sets.push(counts.iter().map(|&(color, _)| color).collect());
    }
    let best = (2..counts.len().min(MAX_COLORS + 1))
        .map(|k| kmeans(&counts, k))
        .filter(|centers| centers.len() >= 2)
        .map(|centers| {
            let (error, entropy) = proxy(&counts, &centers);
            let bits = (8 * D * centers.len()) as f64 + MAP_BITS_PER_ENTROPY * entropy;
            (error + lambda * bits, centers)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, centers)) = best {
        sets.push(centers);
    }
    sets.iter()
        .map(|centers| palette(centers, area, &sample))
        .collect()
}

/// The luma palettes worth trying for the block at `area` of `luma`.
pub(crate) fn luma_candidates(luma: &Plane, area: Area, lambda: f64) -> Vec<Palette> {
    candidates(area, |x, y| [luma.at(x, y)], lambda)
}

/// The chroma palettes worth trying for the block at `area` of the U and
/// V planes: pairs ascending by U.
pub(crate) fn chroma_candidates(u: &Plane, v: &Plane, area: Area, lambda: f64) -> Vec<Palette> {
    candidates(area, |x, y| [u.at(x, y), v.at(x, y)], lambda)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn few_colours_are_taken_exactly() {
        let samples: Vec<u8> = (0..64).map(|i| [255, 0, 128][i % 3]).collect();
        let plane = Plane::padded(&samples, (8, 8), (8, 8));
        let area = Area {
            at: (0, 0),
            side: 8,
            onscreen: (8, 8),
        };
        let found = luma_candidates(&plane, area, 10.0);
        assert_eq!(found[0].colors, [vec![0, 128, 255]]);
        let index = |v: u8| [0, 128, 255].iter().position(|&c| c == v).unwrap() as u8;
        assert!(
            found[0]
                .map
                .iter()
                .zip(&samples)
                .all(|(&m, &s)| m == index(s))
        );
        // A flat block has nothing to gain.
        let flat = Plane::padded(&[9; 64], (8, 8), (8, 8));
        assert_eq!(luma_candidates(&flat, area, 10.0), []);
    }

    #[test]
    fn many_colours_are_clustered() {
        // Two clusters around 20 and 220, ten levels in all.
        let samples: Vec<u8> = (0..256u32)
            .map(|i| {
                if i % 2 == 0 {
                    18 + (i / 2 % 5) as u8
                } else {
                    218 + (i / 2 % 5) as u8
                }
            })
            .collect();
        let plane = Plane::padded(&samples, (16, 16), (16, 16));
        let area = Area {
            at: (0, 0),
            side: 16,
            onscreen: (16, 16),
        };
        let found = luma_candidates(&plane, area, 1000.0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].len(), 2);
        assert!(
            found[0].colors[0][0].abs_diff(20) <= 1 && found[0].colors[0][1].abs_diff(220) <= 1
        );
    }

    #[test]
    fn maps_repeat_past_the_screen() {
        let samples: Vec<u8> = (0..16).map(|i| if i % 4 < 2 { 0 } else { 99 }).collect();
        let plane = Plane::padded(&samples, (4, 4), (8, 8));
        let area = Area {
            at: (0, 0),
            side: 8,
            onscreen: (3, 4),
        };
        let found = luma_candidates(&plane, area, 1.0);
        let map = &found[0].map;
        assert_eq!(&map[..8], [0, 0, 1, 1, 1, 1, 1, 1]);
        assert_eq!(&map[56..], &map[24..32]);
    }

    #[test]
    fn chroma_pairs_sort_by_u() {
        let u = Plane::padded(&[200, 10, 200, 10], (2, 2), (4, 4));
        let v = Plane::padded(&[1, 2, 1, 2], (2, 2), (4, 4));
        let area = Area {
            at: (0, 0),
            side: 4,
            onscreen: (2, 2),
        };
        let found = chroma_candidates(&u, &v, area, 1.0);
        assert_eq!(found[0].colors, [vec![10, 200], vec![2, 1]]);
    }
}
