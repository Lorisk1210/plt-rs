//! What differs between x86-simd-sort's AVX-512 and AVX2 sorts of 64-bit
//! keys: how many keys a register holds, the networks that order the keys
//! of one register, and the order in which a partition stores a register's
//! upper keys.
//!
//! Everything else is one algorithm, written once in x86-simd-sort over a
//! vector type and here over a [`Width`]: the same 256 keys go to the
//! bitonic networks (32 registers of 8 or 64 of 4), a partition takes four
//! registers a step (32 or 16 keys), and the pivot is the upper median of
//! as many samples as a register has lanes.

/// One step of an in-register network: each lane whose bit is clear in
/// the mask takes the minimum of itself and the lane at its position XOR
/// the distance.
pub(super) type Step = (usize, u8);

/// The most lanes a register has.
pub(super) const MAX_LANES: usize = 8;

/// The registers x86-simd-sort sorts indices with.
pub(super) trait Width {
    /// Keys per register.
    const LANES: usize;
    /// `sort_reg_Nlanes`: sorts one register.
    const SORT_REGISTER: &'static [Step];
    /// `bitonic_merge_reg_Nlanes`: sorts one bitonic register.
    const MERGE_REGISTER: &'static [Step];
    /// Whether a partition stores a register's keys of at least the pivot
    /// last lane first.
    const REVERSES_UPPER_KEYS: bool;
}

/// AVX-512: 8 keys a register (`zmm_vector<double>`), compressed to
/// either side of a partition in lane order (`vcompresspd`).
pub(super) struct Avx512;

impl Width for Avx512 {
    const LANES: usize = 8;
    const SORT_REGISTER: &'static [Step] = &[
        (1, 0xAA),
        (3, 0xCC),
        (1, 0xAA),
        (7, 0xF0),
        (2, 0xCC),
        (1, 0xAA),
    ];
    const MERGE_REGISTER: &'static [Step] = &[(4, 0xF0), (2, 0xCC), (1, 0xAA)];
    const REVERSES_UPPER_KEYS: bool = false;
}

/// AVX2: 4 keys a register (`avx2_vector<double>`). AVX2 has no compress
/// instruction, so a partition permutes the register by a table
/// (`avx2_compressstore_lut64_gen`) that packs the lower keys to the front
/// in lane order and the upper ones to the back from the last lane down,
/// and stores it whole on both sides.
pub(super) struct Avx2;

impl Width for Avx2 {
    const LANES: usize = 4;
    const SORT_REGISTER: &'static [Step] = &[(1, 0xA), (3, 0xC), (1, 0xA)];
    const MERGE_REGISTER: &'static [Step] = &[(2, 0xC), (1, 0xA)];
    const REVERSES_UPPER_KEYS: bool = true;
}
