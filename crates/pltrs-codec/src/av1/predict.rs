//! Intra prediction (section 7.11.2) from the reconstructed samples above
//! and to the left of a transform block, exactly as a decoder predicts:
//! DC, smooth and Paeth prediction, and the directional modes with their
//! angle deltas (the intra edge filter and upsampling being off).

use super::plane::Plane;
use super::syntax::{Mode, Prediction, Square};
use super::tables::lookup::{
    SM_WEIGHTS_4, SM_WEIGHTS_8, SM_WEIGHTS_16, SM_WEIGHTS_32, SM_WEIGHTS_64,
};

/// `Dr_Intra_Derivative`: the step along an edge per sample, in 64ths,
/// of the angles directional prediction uses.
#[rustfmt::skip]
const DERIVATIVES: [i32; 90] = [
    0, 0, 0, 1023, 0, 0, 547, 0, 0, 372, 0, 0, 0, 0,
    273, 0, 0, 215, 0, 0, 178, 0, 0, 151, 0, 0, 132, 0, 0,
    116, 0, 0, 102, 0, 0, 0, 90, 0, 0, 80, 0, 0, 71, 0, 0,
    64, 0, 0, 57, 0, 0, 51, 0, 0, 45, 0, 0, 0, 40, 0, 0,
    35, 0, 0, 31, 0, 0, 27, 0, 0, 23, 0, 0, 19, 0, 0,
    15, 0, 0, 0, 0, 11, 0, 0, 7, 0, 0, 3, 0, 0,
];

/// Where a transform block sits in its plane, and which of its edges
/// hold decoded samples.
#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the four edge flags of the specification's predict_intra"
)]
pub(crate) struct Position {
    /// The column of the top-left sample.
    pub(crate) x: usize,
    /// The row of the top-left sample.
    pub(crate) y: usize,
    /// Whether the samples to the left are decoded (`haveLeft`).
    pub(crate) have_left: bool,
    /// Whether the samples above are decoded (`haveAbove`).
    pub(crate) have_above: bool,
    /// Whether the samples above and to the right are decoded
    /// (`haveAboveRight`).
    pub(crate) have_above_right: bool,
    /// Whether the samples to the left and below are decoded
    /// (`haveBelowLeft`).
    pub(crate) have_below_left: bool,
    /// The last column of the plane's 4×4 grid (`maxX`).
    pub(crate) max_x: usize,
    /// The last row of the plane's 4×4 grid (`maxY`).
    pub(crate) max_y: usize,
}

/// The smooth weights of an edge of `2^log2` samples.
fn smooth_weights(log2: u32) -> &'static [u16] {
    match log2 {
        2 => &SM_WEIGHTS_4,
        3 => &SM_WEIGHTS_8,
        4 => &SM_WEIGHTS_16,
        5 => &SM_WEIGHTS_32,
        _ => &SM_WEIGHTS_64,
    }
}

/// The edges a prediction reads, each `2 × size` long: `AboveRow` and
/// `LeftCol` from index -1 (the corner) at position 0.
struct Edges {
    above: Vec<i32>,
    left: Vec<i32>,
}

impl Edges {
    /// The edges of the block at `at`, `size` a side, in `recon`.
    fn new(recon: &Plane, at: Position, size: usize) -> Self {
        let Position { x, y, .. } = at;
        let sample = |x: usize, y: usize| i32::from(recon.at(x, y));
        let corner = match (at.have_above, at.have_left) {
            (true, true) => sample(x - 1, y - 1),
            (true, false) => sample(x, y - 1),
            (false, true) => sample(x - 1, y),
            (false, false) => 128,
        };
        let length = 2 * size;
        let mut above = vec![corner];
        match (at.have_above, at.have_left) {
            (false, true) => above.resize(length + 1, sample(x - 1, y)),
            (false, false) => above.resize(length + 1, 127),
            (true, _) => {
                let reach = if at.have_above_right { 2 * size } else { size };
                let limit = at.max_x.min(x + reach - 1);
                above.extend((0..length).map(|i| sample(limit.min(x + i), y - 1)));
            }
        }
        let mut left = vec![corner];
        match (at.have_left, at.have_above) {
            (false, true) => left.resize(length + 1, sample(x, y - 1)),
            (false, false) => left.resize(length + 1, 129),
            (true, _) => {
                let reach = if at.have_below_left { 2 * size } else { size };
                let limit = at.max_y.min(y + reach - 1);
                left.extend((0..length).map(|i| sample(x - 1, limit.min(y + i))));
            }
        }
        Self { above, left }
    }

    /// `AboveRow[i]`, from -1.
    fn above(&self, i: i32) -> i32 {
        self.above[(i + 1) as usize]
    }

    /// `LeftCol[i]`, from -1.
    fn left(&self, i: i32) -> i32 {
        self.left[(i + 1) as usize]
    }
}

/// The DC prediction: the rounded average of the available edges.
fn dc(edges: &Edges, at: Position, log2: u32) -> i32 {
    let size = 1 << log2;
    let sum = |edge: &[i32]| edge[1..=size as usize].iter().sum::<i32>();
    match (at.have_left, at.have_above) {
        (true, true) => (sum(&edges.left) + sum(&edges.above) + size) >> (log2 + 1),
        (true, false) => (sum(&edges.left) + (size >> 1)) >> log2,
        (false, true) => (sum(&edges.above) + (size >> 1)) >> log2,
        (false, false) => 128,
    }
}

/// The `PAETH_PRED` sample of a row's left sample and a column's above
/// sample.
fn paeth(left: i32, above: i32, corner: i32) -> i32 {
    let base = above + left - corner;
    let (p_left, p_top, p_corner) = (
        (base - left).abs(),
        (base - above).abs(),
        (base - corner).abs(),
    );
    if p_left <= p_top && p_left <= p_corner {
        left
    } else if p_top <= p_corner {
        above
    } else {
        corner
    }
}

/// The two-tap interpolation of directional prediction between `a` and
/// `b`, `shift` 32nds of the way.
fn interpolate(a: i32, b: i32, shift: i32) -> i32 {
    (a * (32 - shift) + b * shift + 16) >> 5
}

/// The directional prediction at row `i`, column `j` of a block of
/// `size` (section 7.11.2.4, steps 5 to 11, without upsampling).
fn directional(edges: &Edges, angle: i32, size: i32, (i, j): (i32, i32)) -> i32 {
    match angle {
        90 => edges.above(j),
        180 => edges.left(i),
        _ if angle < 90 => {
            let dx = DERIVATIVES[angle as usize];
            let idx = (i + 1) * dx;
            let base = (idx >> 6) + j;
            let max_base = 2 * size - 1;
            if base < max_base {
                interpolate(edges.above(base), edges.above(base + 1), (idx >> 1) & 0x1F)
            } else {
                edges.above(max_base)
            }
        }
        _ if angle < 180 => {
            let dx = DERIVATIVES[(180 - angle) as usize];
            let idx = (j << 6) - (i + 1) * dx;
            let base = idx >> 6;
            if base >= -1 {
                interpolate(edges.above(base), edges.above(base + 1), (idx >> 1) & 0x1F)
            } else {
                let dy = DERIVATIVES[(angle - 90) as usize];
                let idx = (i << 6) - (j + 1) * dy;
                let base = idx >> 6;
                interpolate(edges.left(base), edges.left(base + 1), (idx >> 1) & 0x1F)
            }
        }
        _ => {
            let dy = DERIVATIVES[(270 - angle) as usize];
            let idx = (j + 1) * dy;
            let base = (idx >> 6) + i;
            interpolate(edges.left(base), edges.left(base + 1), (idx >> 1) & 0x1F)
        }
    }
}

/// The prediction of a `tx`-sized block at `at`, row by row.
pub(crate) fn predict(recon: &Plane, at: Position, tx: Square, prediction: Prediction) -> Vec<u8> {
    let size = tx.samples();
    let log2 = tx.log2();
    let edges = Edges::new(recon, at, size);
    let weights = smooth_weights(log2);
    let average = dc(&edges, at, log2);
    let last = size as i32 - 1;
    let mut out = Vec::with_capacity(size * size);
    for i in 0..size as i32 {
        let wy = i32::from(weights[i as usize]);
        for j in 0..size as i32 {
            let wx = i32::from(weights[j as usize]);
            let (above, left) = (edges.above(j), edges.left(i));
            let value = match prediction.mode {
                Mode::Dc => average,
                Mode::Smooth => {
                    let sum = wy * above
                        + (256 - wy) * edges.left(last)
                        + wx * left
                        + (256 - wx) * edges.above(last);
                    (sum + 256) >> 9
                }
                Mode::SmoothVertical => (wy * above + (256 - wy) * edges.left(last) + 128) >> 8,
                Mode::SmoothHorizontal => (wx * left + (256 - wx) * edges.above(last) + 128) >> 8,
                Mode::Paeth => paeth(left, above, edges.above(-1)),
                Mode::Vertical
                | Mode::Horizontal
                | Mode::D45
                | Mode::D135
                | Mode::D113
                | Mode::D157
                | Mode::D203
                | Mode::D67 => {
                    let angle = prediction.angle().expect("a directional mode has an angle");
                    directional(&edges, angle, size as i32, (i, j))
                }
            };
            out.push(value as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An 8×8 plane whose row 0 and column 0 are decoded, the rest blank.
    fn plane() -> Plane {
        let mut samples = vec![0u8; 64];
        for i in 0..8 {
            samples[i] = 10 * i as u8 + 10;
            samples[i * 8] = 100 + i as u8;
        }
        Plane::padded(&samples, (8, 8), (8, 8))
    }

    fn at(have_left: bool, have_above: bool) -> Position {
        Position {
            x: 1,
            y: 1,
            have_left,
            have_above,
            have_above_right: false,
            have_below_left: false,
            max_x: 7,
            max_y: 7,
        }
    }

    fn plain(mode: Mode) -> Prediction {
        Prediction::plain(mode)
    }

    #[test]
    fn missing_edges_use_the_specification_constants() {
        let plane = plane();
        let none = predict(&plane, at(false, false), Square::S4, plain(Mode::Dc));
        assert!(none.iter().all(|&v| v == 128));
        let vertical = predict(&plane, at(false, false), Square::S4, plain(Mode::Vertical));
        assert!(vertical.iter().all(|&v| v == 127));
        let horizontal = predict(
            &plane,
            at(false, false),
            Square::S4,
            plain(Mode::Horizontal),
        );
        assert!(horizontal.iter().all(|&v| v == 129));
        // Without the left column, the corner sample stands in for it.
        let horizontal = predict(&plane, at(false, true), Square::S4, plain(Mode::Horizontal));
        assert!(horizontal.iter().all(|&v| v == 20));
    }

    #[test]
    fn directional_copies_and_averages() {
        let plane = plane();
        let vertical = predict(&plane, at(true, true), Square::S4, plain(Mode::Vertical));
        assert_eq!(&vertical[..4], [20, 30, 40, 50]);
        assert_eq!(&vertical[12..], [20, 30, 40, 50]);
        let horizontal = predict(&plane, at(true, true), Square::S4, plain(Mode::Horizontal));
        assert_eq!(
            horizontal.chunks(4).map(|r| r[3]).collect::<Vec<_>>(),
            [101, 102, 103, 104]
        );
        // (20 + 30 + 40 + 50 + 101 + 102 + 103 + 104 + 4) / 8.
        let dc = predict(&plane, at(true, true), Square::S4, plain(Mode::Dc));
        assert!(dc.iter().all(|&v| v == 69));
        let above_only = predict(&plane, at(false, true), Square::S4, plain(Mode::Dc));
        assert!(above_only.iter().all(|&v| v == 35));
    }

    #[test]
    fn smooth_and_paeth() {
        let plane = plane();
        let smooth = predict(&plane, at(true, true), Square::S4, plain(Mode::Smooth));
        // Row 0, column 0: (255·20 + 1·104 + 255·101 + 1·50 + 256) >> 9.
        assert_eq!(smooth[0], 61);
        let paeth = predict(&plane, at(true, true), Square::S4, plain(Mode::Paeth));
        // Base 20 + 101 - 100 = 21: nearest the sample above.
        assert_eq!(paeth[0], 20);
        // Row 3, column 3: base 50 + 104 - 100 = 54, nearest above again.
        assert_eq!(paeth[15], 50);
    }

    #[test]
    fn edges_stop_at_the_grid() {
        let plane = plane();
        let mut position = at(true, true);
        position.max_x = 2;
        let vertical = predict(&plane, position, Square::S4, plain(Mode::Vertical));
        assert_eq!(&vertical[..4], [20, 30, 30, 30]);
    }

    #[test]
    fn diagonals_follow_their_angles() {
        let plane = plane();
        let mut position = at(true, true);
        position.have_above_right = true;
        // 45°: row i, column j reads AboveRow[i + j + 1] (dx = 64), here
        // up to x + 7, which is the plane's column 7 at most.
        let d45 = predict(&plane, position, Square::S4, plain(Mode::D45));
        assert_eq!(&d45[..4], [30, 40, 50, 60]);
        assert_eq!(d45[15], 80);
        // 135°: the diagonal from the top left; row 0 reads the corner
        // then the row above, column 0 the left column.
        let d135 = predict(&plane, position, Square::S4, plain(Mode::D135));
        assert_eq!(&d135[..4], [100, 20, 30, 40]);
        assert_eq!(d135[4], 101);
        // An angle delta of +3 turns D45 to 54°, between the samples.
        let tilted = Prediction {
            mode: Mode::D45,
            delta: 3,
        };
        let d54 = predict(&plane, position, Square::S4, tilted);
        assert_ne!(d54, d45);
    }

    #[test]
    fn every_prediction_stays_on_its_edges() {
        let plane = Plane::padded(&[90; 128 * 128], (128, 128), (128, 128));
        let squares =
            std::iter::successors(Some(Square::S64), |s| (*s > Square::S4).then(|| s.half()));
        for prediction in Mode::ALL.into_iter().flat_map(Prediction::with_deltas) {
            for tx in squares.clone() {
                for flags in 0..16 {
                    let flag = |bit: u32| flags & (1 << bit) != 0;
                    let size = tx.samples();
                    let position = Position {
                        x: 64,
                        y: 64,
                        have_left: flag(0),
                        have_above: flag(1),
                        have_above_right: flag(2),
                        have_below_left: flag(3),
                        max_x: 127,
                        max_y: 127,
                    };
                    let samples = predict(&plane, position, tx, prediction);
                    assert_eq!(samples.len(), size * size, "{prediction:?}");
                }
            }
        }
    }
}
