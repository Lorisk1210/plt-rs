//! Transforms: the types the encoder chooses from, the forward
//! transforms, and the specification's inverse ones.

mod forward;
mod inverse;

pub(crate) use forward::{Bases, forward_2d, walsh_hadamard_4x4};
pub(crate) use inverse::inverse_2d;

/// A 1D transform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// The discrete cosine transform.
    Dct,
    /// The asymmetric discrete sine transform (4 to 16 samples).
    Adst,
    /// The identity, scaled (4 to 32 samples).
    Identity,
}

/// The 2D transform types of the reduced intra set (`TX_SET_INTRA_2`),
/// named vertical then horizontal, with the specification's values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TxType {
    /// `DCT_DCT`.
    DctDct = 0,
    /// `ADST_DCT`: ADST down the columns, DCT along the rows.
    AdstDct = 1,
    /// `DCT_ADST`.
    DctAdst = 2,
    /// `ADST_ADST`.
    AdstAdst = 3,
    /// `IDTX`.
    Identity = 9,
}

impl TxType {
    /// The set in the order of its symbols (`Tx_Type_Intra_Inv_Set2`).
    pub(crate) const REDUCED_SET: [Self; 5] = [
        Self::Identity,
        Self::DctDct,
        Self::AdstAdst,
        Self::AdstDct,
        Self::DctAdst,
    ];

    /// The transform down the columns.
    pub(crate) const fn column(self) -> Kind {
        match self {
            Self::DctDct | Self::DctAdst => Kind::Dct,
            Self::AdstDct | Self::AdstAdst => Kind::Adst,
            Self::Identity => Kind::Identity,
        }
    }

    /// The transform along the rows.
    pub(crate) const fn row(self) -> Kind {
        match self {
            Self::DctDct | Self::AdstDct => Kind::Dct,
            Self::DctAdst | Self::AdstAdst => Kind::Adst,
            Self::Identity => Kind::Identity,
        }
    }

    /// The `intra_tx_type` symbol of the type.
    pub(crate) fn symbol(self) -> usize {
        Self::REDUCED_SET
            .iter()
            .position(|&t| t == self)
            .expect("every type is in the reduced set")
    }
}
