//! The values of the syntax elements the encoder writes: square block
//! and transform sizes, and intra prediction modes.

use super::transform::TxType;

/// A square block or transform of `2^log2` samples a side, from 4×4
/// (`log2` 2) to 64×64 (`log2` 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Square {
    log2: u32,
}

impl Square {
    /// The 4×4 square.
    pub(crate) const S4: Self = Self { log2: 2 };
    /// The 8×8 square.
    pub(crate) const S8: Self = Self { log2: 3 };
    /// The 32×32 square.
    pub(crate) const S32: Self = Self { log2: 5 };
    /// The 64×64 square, the superblock.
    pub(crate) const S64: Self = Self { log2: 6 };

    /// The base-2 logarithm of the side in samples.
    pub(crate) const fn log2(self) -> u32 {
        self.log2
    }

    /// The side in samples.
    pub(crate) const fn samples(self) -> usize {
        1 << self.log2
    }

    /// The side in 4×4 units (`Num_4x4_Blocks_Wide`).
    pub(crate) const fn units(self) -> usize {
        1 << (self.log2 - 2)
    }

    /// `Mi_Width_Log2`: the base-2 logarithm of the side in 4×4 units.
    pub(crate) const fn units_log2(self) -> u32 {
        self.log2 - 2
    }

    /// The square transform size index (`TX_4X4` 0 to `TX_64X64` 4),
    /// which is also its `txSzCtx`.
    pub(crate) const fn tx_index(self) -> usize {
        (self.log2 - 2) as usize
    }

    /// The four squares a split makes.
    pub(crate) const fn half(self) -> Self {
        Self {
            log2: self.log2 - 1,
        }
    }

    /// The square `ss` halvings smaller (the chroma of a block).
    pub(crate) const fn subsampled(self, ss: u32) -> Self {
        Self {
            log2: self.log2 - ss,
        }
    }

    /// The largest transform of the block: the block itself up to 64×64,
    /// and the 32×32 transforms of 64×64 chroma blocks (`get_tx_size`).
    pub(crate) fn transform(self, chroma: bool) -> Self {
        Self {
            log2: self.log2.min(if chroma { 5 } else { 6 }),
        }
    }
}

/// The intra prediction modes, with their values in the specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// `DC_PRED`: the average of the edges.
    Dc = 0,
    /// `V_PRED`: the row above, copied down (90°).
    Vertical = 1,
    /// `H_PRED`: the column on the left, copied across (180°).
    Horizontal = 2,
    /// `D45_PRED`: from the top right.
    D45 = 3,
    /// `D135_PRED`: from the top left.
    D135 = 4,
    /// `D113_PRED`.
    D113 = 5,
    /// `D157_PRED`.
    D157 = 6,
    /// `D203_PRED`: from the bottom left.
    D203 = 7,
    /// `D67_PRED`.
    D67 = 8,
    /// `SMOOTH_PRED`: both edges, blended quadratically.
    Smooth = 9,
    /// `SMOOTH_V_PRED`.
    SmoothVertical = 10,
    /// `SMOOTH_H_PRED`.
    SmoothHorizontal = 11,
    /// `PAETH_PRED`: whichever of above, left and corner is nearest the
    /// gradient's prediction.
    Paeth = 12,
}

impl Mode {
    /// Every mode the encoder uses (all but chroma from luma).
    pub(crate) const ALL: [Self; 13] = [
        Self::Dc,
        Self::Vertical,
        Self::Horizontal,
        Self::D45,
        Self::D135,
        Self::D113,
        Self::D157,
        Self::D203,
        Self::D67,
        Self::Smooth,
        Self::SmoothVertical,
        Self::SmoothHorizontal,
        Self::Paeth,
    ];

    /// The mode's value, the symbol of `intra_frame_y_mode` and `uv_mode`.
    pub(crate) const fn index(self) -> usize {
        self as usize
    }

    /// The context the mode gives the luma mode of the blocks below and
    /// to the right (`Intra_Mode_Context`).
    pub(crate) const fn context(self) -> usize {
        match self {
            Self::Dc | Self::Smooth | Self::Paeth => 0,
            Self::Vertical | Self::SmoothVertical => 1,
            Self::Horizontal | Self::SmoothHorizontal => 2,
            Self::D45 | Self::D67 => 3,
            Self::D135 | Self::D113 | Self::D157 | Self::D203 => 4,
        }
    }

    /// The nominal angle of a directional mode, in degrees
    /// (`Mode_To_Angle`); `None` for the others.
    pub(crate) const fn angle(self) -> Option<i32> {
        match self {
            Self::Vertical => Some(90),
            Self::Horizontal => Some(180),
            Self::D45 => Some(45),
            Self::D135 => Some(135),
            Self::D113 => Some(113),
            Self::D157 => Some(157),
            Self::D203 => Some(203),
            Self::D67 => Some(67),
            Self::Dc
            | Self::Smooth
            | Self::SmoothVertical
            | Self::SmoothHorizontal
            | Self::Paeth => None,
        }
    }

    /// The transform type of chroma blocks predicted in this mode
    /// (`Mode_To_Txfm`).
    pub(crate) const fn chroma_tx_type(self) -> TxType {
        match self {
            Self::Dc | Self::D45 => TxType::DctDct,
            Self::Vertical | Self::SmoothVertical | Self::D113 | Self::D67 => TxType::AdstDct,
            Self::Horizontal | Self::SmoothHorizontal | Self::D157 | Self::D203 => TxType::DctAdst,
            Self::Smooth | Self::Paeth | Self::D135 => TxType::AdstAdst,
        }
    }
}

/// A prediction: a mode and, for directional modes, its angle delta in
/// steps of 3° (`AngleDeltaY`, `AngleDeltaUV`, from -3 to 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Prediction {
    /// The mode.
    pub(crate) mode: Mode,
    /// The angle delta, 0 for modes that are not directional.
    pub(crate) delta: i32,
}

impl Prediction {
    /// `MAX_ANGLE_DELTA`.
    pub(crate) const MAX_DELTA: i32 = 3;

    /// `mode` without an angle delta.
    pub(crate) const fn plain(mode: Mode) -> Self {
        Self { mode, delta: 0 }
    }

    /// The predictions of `mode`: with each angle delta for a
    /// directional mode, else the mode alone.
    pub(crate) fn with_deltas(mode: Mode) -> impl Iterator<Item = Self> {
        let deltas = if mode.angle().is_some() {
            -Self::MAX_DELTA..=Self::MAX_DELTA
        } else {
            0..=0
        };
        deltas.map(move |delta| Self { mode, delta })
    }

    /// The angle of a directional prediction, in degrees (`pAngle`).
    pub(crate) fn angle(self) -> Option<i32> {
        self.mode.angle().map(|angle| angle + 3 * self.delta)
    }
}
