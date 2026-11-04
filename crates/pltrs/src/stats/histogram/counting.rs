//! Assigning samples to bins, reproducing numpy's two code paths.

use super::{BinSample, Uniform};
use crate::stats::summary::pairwise_sum;

/// Equal-width bins: numpy computes each sample's bin index arithmetically
/// and then corrects it by one against the actual edges. Unweighted
/// samples are counted in integers, which the float counts equal exactly
/// (below 2⁵³ samples).
pub(super) fn uniform<T: BinSample>(
    data: &[T],
    weights: Option<&[f64]>,
    edges: &[f64],
    bins: Uniform,
) -> Vec<f64> {
    let Uniform { first, last, count } = bins;
    let span = last - first;
    let scale = count as f64;
    let bin = |x: f64| {
        let mut index = ((x - first) / span * scale) as usize;
        if index >= count {
            index = count - 1;
        }
        if index > 0 && x < edges[index] {
            index -= 1;
        }
        if index != count - 1 && x >= edges[index + 1] {
            index += 1;
        }
        index
    };
    let inside = |x: f64| x >= first && x <= last;
    match weights {
        None => {
            let mut counts = vec![0_u64; count];
            for x in data.iter().map(|v| v.to_f64()).filter(|&x| inside(x)) {
                counts[bin(x)] += 1;
            }
            counts.into_iter().map(|n| n as f64).collect()
        }
        Some(weights) => {
            let mut counts = vec![0.0; count];
            for (x, &weight) in data.iter().map(|v| v.to_f64()).zip(weights) {
                if inside(x) {
                    counts[bin(x)] += weight;
                }
            }
            counts
        }
    }
}

/// Arbitrary edges: numpy sorts the samples and counts with binary
/// searches (left-closed bins, the last bin also right-closed); weighted
/// counts are differences of the cumulative weight sum.
pub(super) fn by_edges<T: BinSample>(
    data: &[T],
    weights: Option<&[f64]>,
    edges: &[f64],
) -> Vec<f64> {
    let Some((&last_edge, inner_edges)) = edges.split_last() else {
        return Vec::new();
    };
    let mut order: Vec<(f64, f64)> = data
        .iter()
        .enumerate()
        .map(|(i, v)| (v.to_f64(), weights.map_or(1.0, |w| w[i])))
        .collect();
    // numpy sorts NaN last regardless of its sign bit.
    order.sort_by(|a, b| a.0.is_nan().cmp(&b.0.is_nan()).then(a.0.total_cmp(&b.0)));
    let mut cumulative = Vec::with_capacity(order.len() + 1);
    cumulative.push(0.0);
    let mut running = 0.0;
    for &(_, w) in &order {
        running += w;
        cumulative.push(running);
    }
    let boundary = |edge: f64, inclusive: bool| {
        order.partition_point(|&(v, _)| if inclusive { v <= edge } else { v < edge })
    };
    let positions: Vec<f64> = inner_edges
        .iter()
        .map(|&edge| cumulative[boundary(edge, false)])
        .chain(std::iter::once(cumulative[boundary(last_edge, true)]))
        .collect();
    positions.windows(2).map(|w| w[1] - w[0]).collect()
}

/// numpy's `density=True`: `counts / diff(edges) / counts.sum()`.
pub(super) fn density(counts: &[f64], edges: &[f64]) -> Vec<f64> {
    let total = pairwise_sum(counts);
    counts
        .iter()
        .zip(edges.windows(2))
        .map(|(&n, w)| n / (w[1] - w[0]) / total)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_bins_close_the_last_bin() {
        let edges = [0.0, 1.0, 2.0];
        let bins = Uniform {
            first: 0.0,
            last: 2.0,
            count: 2,
        };
        let counts = uniform(&[0.0, 1.0, 2.0, 2.5, f64::NAN, -1.0], None, &edges, bins);
        assert_eq!(counts, vec![1.0, 2.0]);
        let weights = [2.0, 0.25, 0.5, 8.0];
        let weighted = uniform(&[0.5, 1.5, 2.0, 3.0], Some(&weights), &edges, bins);
        assert_eq!(weighted, vec![2.0, 0.75]);
    }

    #[test]
    fn edge_bins_skip_nan_and_outliers() {
        let counts = by_edges(&[f64::NAN, 1.0, 2.0, 3.0, -1.0], None, &[0.0, 2.0, 3.0]);
        assert_eq!(counts, vec![1.0, 2.0]);
        let counts = by_edges(&[0.5, 1.5], Some(&[2.0, 3.0]), &[0.0, 1.0, 1.0, 2.0]);
        assert_eq!(counts, vec![2.0, 0.0, 3.0]);
        assert_eq!(by_edges(&[1.0], None, &[5.0]), Vec::<f64>::new());
        assert_eq!(by_edges(&[1.0], None, &[]), Vec::<f64>::new());
    }

    #[test]
    fn density_of_empty_counts_is_nan() {
        let d = density(&[0.0, 0.0], &[0.0, 1.0, 2.0]);
        assert!(d.iter().all(|v| v.is_nan()));
    }
}
