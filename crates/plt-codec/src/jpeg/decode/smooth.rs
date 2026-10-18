//! Block smoothing of incomplete progressive images, libjpeg-turbo's
//! `decompress_smooth_data` (on by default, so Pillow applies it): when
//! some of the first nine AC coefficients of a component were never sent
//! or not refined to their last bit, they are estimated from the DC
//! values of the 5×5 neighbouring blocks. When no AC data at all was sent
//! for a component, its DC values are interpolated as well.

use super::frame::{Block, Component, Frame};

/// The coefficients libjpeg tracks for smoothing (`SAVED_COEFS`).
const SAVED_COEFS: usize = 10;

/// Natural positions of the quantization steps smoothing divides by:
/// DC, then AC01, AC10, AC20, AC11, AC02, AC03, AC12, AC21 and AC30.
const POSITIONS: [usize; SAVED_COEFS] = [0, 1, 8, 16, 9, 2, 3, 10, 17, 24];

/// An estimate of one coefficient: its index in [`POSITIONS`] (and in
/// `coef_bits`) and the weights of the 5×5 DC neighbourhood (rows from
/// two above to two below, columns from two left to two right).
struct Estimate {
    index: usize,
    weights: [[i64; 5]; 5],
}

/// Estimates when some AC data was sent (Annex K.8, on 5×5 blocks).
const ESTIMATES: [Estimate; 5] = [
    Estimate {
        index: 1,
        weights: [[0; 5], [0; 5], [-7, 50, 0, -50, 7], [0; 5], [0; 5]],
    },
    Estimate {
        index: 2,
        weights: [
            [0, 0, -7, 0, 0],
            [0, 0, 50, 0, 0],
            [0; 5],
            [0, 0, -50, 0, 0],
            [0, 0, 7, 0, 0],
        ],
    },
    Estimate {
        index: 3,
        weights: [
            [0, 0, -1, 0, 0],
            [0, 0, 13, 0, 0],
            [0, 0, -24, 0, 0],
            [0, 0, 13, 0, 0],
            [0, 0, -1, 0, 0],
        ],
    },
    Estimate {
        index: 4,
        weights: [
            [0, -1, 0, 1, 0],
            [-1, 10, 0, -10, 1],
            [0; 5],
            [1, -10, 0, 10, -1],
            [0, 1, 0, -1, 0],
        ],
    },
    Estimate {
        index: 5,
        weights: [[0; 5], [0; 5], [-1, 13, -24, 13, -1], [0; 5], [0; 5]],
    },
];

/// Estimates when no AC data was sent, the DC values interpolated too.
const INTERPOLATIONS: [Estimate; 9] = [
    Estimate {
        index: 1,
        weights: [
            [-1, -1, 0, 1, 1],
            [-3, 13, 0, -13, 3],
            [-3, 38, 0, -38, 3],
            [-3, 13, 0, -13, 3],
            [-1, -1, 0, 1, 1],
        ],
    },
    Estimate {
        index: 2,
        weights: [
            [-1, -3, -3, -3, -1],
            [-1, 13, 38, 13, -1],
            [0; 5],
            [1, -13, -38, -13, 1],
            [1, 3, 3, 3, 1],
        ],
    },
    Estimate {
        index: 3,
        weights: [
            [0, 0, 1, 0, 0],
            [0, 2, 7, 2, 0],
            [0, -5, -14, -5, 0],
            [0, 2, 7, 2, 0],
            [0, 0, 1, 0, 0],
        ],
    },
    Estimate {
        index: 4,
        weights: [
            [-1, 0, 0, 0, 1],
            [0, 9, 0, -9, 0],
            [0; 5],
            [0, -9, 0, 9, 0],
            [1, 0, 0, 0, -1],
        ],
    },
    Estimate {
        index: 5,
        weights: [
            [0; 5],
            [0, 2, -5, 2, 0],
            [1, 7, -14, 7, 1],
            [0, 2, -5, 2, 0],
            [0; 5],
        ],
    },
    Estimate {
        index: 6,
        weights: [
            [0; 5],
            [0, 1, 0, -1, 0],
            [0, 2, 0, -2, 0],
            [0, 1, 0, -1, 0],
            [0; 5],
        ],
    },
    Estimate {
        index: 7,
        weights: [[0; 5], [0, 1, -3, 1, 0], [0; 5], [0, -1, 3, -1, 0], [0; 5]],
    },
    Estimate {
        index: 8,
        weights: [
            [0; 5],
            [0, 1, 0, -1, 0],
            [0, -3, 0, 3, 0],
            [0, 1, 0, -1, 0],
            [0; 5],
        ],
    },
    Estimate {
        index: 9,
        weights: [[0; 5], [0, 1, 2, 1, 0], [0; 5], [0, -1, -2, -1, 0], [0; 5]],
    },
];

/// The interpolated DC value's weights.
const DC_WEIGHTS: [[i64; 5]; 5] = [
    [-2, -6, -8, -6, -2],
    [-6, 6, 42, 6, -6],
    [-8, 42, 152, 42, -8],
    [-6, 6, 42, 6, -6],
    [-2, -6, -8, -6, -2],
];

/// Whether libjpeg smooths the frame's blocks (`smoothing_ok`): a
/// progressive frame whose components all have their DC values and
/// nonzero quantization steps for the estimated coefficients, and where
/// some of those coefficients are not fully known.
pub(super) fn applies(frame: &Frame) -> bool {
    frame.progressive
        && frame.components.iter().all(|c| {
            c.quant
                .is_some_and(|quant| POSITIONS.iter().all(|&p| quant[p] != 0))
                && c.coef_bits[0] >= 0
        })
        && frame
            .components
            .iter()
            .any(|c| c.coef_bits[1..SAVED_COEFS].iter().any(|&bits| bits != 0))
}

/// The block at `row`, `column` of a component with its unknown low
/// coefficients estimated; `imcu_rows` is the frame's number of MCU rows.
pub(super) fn smoothed_block(
    component: &Component,
    imcu_rows: usize,
    row: usize,
    column: usize,
) -> Block {
    let quant = component.quant.unwrap_or([1; 64]);
    let bits = &component.coef_bits;
    let dc = neighbourhood(component, imcu_rows, row, column);
    let weigh = |weights: &[[i64; 5]; 5]| -> i64 {
        let sum: i64 = (0..25)
            .map(|i| weights[i / 5][i % 5] * dc[i / 5][i % 5])
            .sum();
        i64::from(quant[0]) * sum
    };
    let mut block = *component.block(row, column);
    let interpolate = bits[1..SAVED_COEFS].iter().all(|&b| b == -1);
    let estimates: &[Estimate] = if interpolate {
        &INTERPOLATIONS
    } else {
        &ESTIMATES
    };
    for estimate in estimates {
        let position = POSITIONS[estimate.index];
        let al = bits[estimate.index];
        if al != 0 && block[position] == 0 {
            let step = i64::from(quant[position]);
            block[position] = predict(weigh(&estimate.weights), step, Some(al));
        }
    }
    if interpolate {
        block[0] = predict(weigh(&DC_WEIGHTS), i64::from(quant[0]), None);
    }
    block
}

/// `num / (256 · step)` rounded half away from zero, its magnitude
/// limited below `2^al` when the coefficient is known to bit `al > 0`.
fn predict(num: i64, step: i64, al: Option<i8>) -> i16 {
    let limit = |prediction: i64| match al {
        Some(al) if al > 0 && prediction >= 1 << al => (1 << al) - 1,
        _ => prediction,
    };
    let magnitude = limit(((step << 7) + num.abs()) / (step << 8));
    (if num >= 0 { magnitude } else { -magnitude }) as i16
}

/// The DC values of the 5×5 blocks around `row`, `column`. Columns clamp
/// to the component's blocks; rows follow libjpeg's bookkeeping, which
/// counts the last MCU row's block rows by that row's height.
fn neighbourhood(
    component: &Component,
    imcu_rows: usize,
    row: usize,
    column: usize,
) -> [[i64; 5]; 5] {
    let v = component.spec.v;
    let (imcu, offset) = (row / v, row % v);
    let block_rows = if imcu + 1 < imcu_rows {
        v
    } else {
        match component.height_in_blocks % v {
            0 => v,
            rows => rows,
        }
    };
    let image_row = imcu * block_rows + offset;
    let image_rows = block_rows * imcu_rows;
    let previous = if image_row > 0 { row - 1 } else { row };
    let before_previous = if image_row > 1 { row - 2 } else { previous };
    let next = if image_row + 1 < image_rows {
        row + 1
    } else {
        row
    };
    let after_next = if image_row + 2 < image_rows {
        row + 2
    } else {
        next
    };
    let rows = [before_previous, previous, row, next, after_next];
    let last_column = component.width_in_blocks - 1;
    std::array::from_fn(|r| {
        std::array::from_fn(|c| {
            let column = (column + c).saturating_sub(2).min(last_column);
            i64::from(component.block(rows[r], column)[0])
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predictions_round_and_limit_like_libjpeg() {
        // (128 · 2 + 1000) / 512 = 2.45 → 2.
        assert_eq!(predict(1000, 2, None), 2);
        assert_eq!(predict(-1000, 2, None), -2);
        assert_eq!(predict(100_000, 1, Some(2)), 3, "limited below 2^2");
        assert_eq!(
            predict(100_000, 1, Some(-1)),
            391,
            "unknown bits are not limited"
        );
    }

    #[test]
    fn weights_are_antisymmetric_where_libjpeg_s_are() {
        // The DC interpolation keeps the average: its weights sum to 256.
        let sum: i64 = DC_WEIGHTS.iter().flatten().sum();
        assert_eq!(sum, 256);
        for estimate in ESTIMATES.iter().chain(&INTERPOLATIONS) {
            let total: i64 = estimate.weights.iter().flatten().sum();
            assert_eq!(total, 0, "estimate {}", estimate.index);
        }
    }
}
