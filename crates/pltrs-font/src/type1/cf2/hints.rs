//! Stem hints and hint masks of FreeType's Adobe engine (`psintrp.c`,
//! `pshints.c`): the stems a charstring declares, where hint maps placed
//! them, and which of them are active.

use super::buffer::Buffer;
use crate::type1::error::Type1Error;

/// `CF2_MAX_HINTS`: horizontal and vertical stems together.
pub(super) const MAX_HINTS: usize = 96;

/// One stem hint: its edges in character space and, once a hint map has
/// used it, in device space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct StemHint {
    pub(super) min: i32,
    pub(super) max: i32,
    pub(super) used: bool,
    pub(super) min_ds: i32,
    pub(super) max_ds: i32,
}

impl StemHint {
    pub(super) fn new(min: i32, max: i32) -> Self {
        Self {
            min,
            max,
            ..Self::default()
        }
    }
}

/// A hint mask: one bit per stem, horizontal stems first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct HintMask {
    pub(super) valid: bool,
    pub(super) new: bool,
    pub(super) bits: usize,
    pub(super) bytes: [u8; MAX_HINTS.div_ceil(8)],
}

impl HintMask {
    /// A mask that is neither valid nor new (`cf2_hintmask_init`).
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    /// `cf2_hintmask_setCounts`: a valid, new mask for `bits` stems, unless
    /// there are too many.
    fn set_counts(&mut self, bits: usize) -> bool {
        if bits > MAX_HINTS {
            return false;
        }
        self.bits = bits;
        self.valid = true;
        self.new = true;
        true
    }

    /// `cf2_hintmask_setAll`: every stem active.
    pub(super) fn set_all(&mut self, bits: usize) -> bool {
        if !self.set_counts(bits) {
            return false;
        }
        let count = bits.div_ceil(8);
        self.bytes = [0; MAX_HINTS.div_ceil(8)];
        self.bytes[..count].fill(0xFF);
        if !bits.is_multiple_of(8) {
            self.bytes[count - 1] &= 0xFF << (8 - bits % 8);
        }
        true
    }

    /// `cf2_hintmask_read`: the mask bytes following a `hintmask` or
    /// `cntrmask` operator.
    pub(super) fn read(&mut self, code: &mut Buffer<'_>, bits: usize) -> Result<(), Type1Error> {
        if !self.set_counts(bits) {
            return Err(Type1Error::BadCharstring("too many stem hints"));
        }
        for byte in &mut self.bytes[..bits.div_ceil(8)] {
            *byte = code.read();
        }
        Ok(())
    }

    /// Whether stem `index` is active.
    pub(super) fn is_set(&self, index: usize) -> bool {
        self.bytes[index / 8] & (0x80 >> (index % 8)) != 0
    }

    /// Deactivates stem `index`.
    pub(super) fn clear(&mut self, index: usize) {
        self.bytes[index / 8] &= !(0x80 >> (index % 8));
    }
}

/// The stems of a charstring and its current hint mask.
#[derive(Clone, Debug, Default)]
pub(super) struct Hints {
    pub(super) horizontal: Vec<StemHint>,
    pub(super) vertical: Vec<StemHint>,
    pub(super) mask: HintMask,
}

impl Hints {
    /// The number of stems.
    pub(super) fn count(&self) -> usize {
        self.horizontal.len() + self.vertical.len()
    }

    /// Forgets the stems and starts a new mask (Type 1 hint replacement
    /// and the rewind after the first pass).
    pub(super) fn reset(&mut self) {
        self.horizontal.clear();
        self.vertical.clear();
        self.mask.reset();
        self.mask.new = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks() {
        let mut mask = HintMask::default();
        assert!(mask.set_all(10));
        assert_eq!(&mask.bytes[..3], &[0xFF, 0xC0, 0]);
        assert!(mask.valid && mask.new && mask.is_set(9) && !mask.is_set(10));
        mask.clear(9);
        assert!(!mask.is_set(9));
        assert!(!mask.set_all(97));
        let mut code = Buffer::new(&[0xA5, 0x80, 7]);
        mask.read(&mut code, 9).unwrap();
        assert_eq!(code.position(), 2);
        assert!(mask.is_set(0) && !mask.is_set(1) && mask.is_set(8));
        assert!(mask.read(&mut code, 97).is_err());
        let mut hints = Hints::default();
        hints.horizontal.push(StemHint::new(1, 2));
        hints.reset();
        assert!(hints.mask.new && !hints.mask.valid && hints.count() == 0);
    }
}
