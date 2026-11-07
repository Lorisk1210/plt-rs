//! Private DICTs (`cff_load_private_dict`): the alignment zones and stem
//! data the Adobe engine hints with, the local subroutines, and, in CFF2,
//! values blended between the masters of a variable font.

use super::bytes::CffBytes;
use super::dict::{DictKind, Operand, Token, tokens};
use super::vstore::VariationStore;
use crate::error::{FontError, Result};
use crate::psaux::PrivateDict;
use crate::types::Tag;

/// The Private DICT operators the engine reads.
const BLUE_VALUES: u16 = 6;
const OTHER_BLUES: u16 = 7;
const FAMILY_BLUES: u16 = 8;
const FAMILY_OTHER_BLUES: u16 = 9;
const SUBRS: u16 = 19;
const VSINDEX: u16 = 22;
const BLEND: u16 = 23;
const BLUE_SCALE: u16 = 0x109;
const BLUE_SHIFT: u16 = 0x10A;
const BLUE_FUZZ: u16 = 0x10B;
const LANGUAGE_GROUP: u16 = 0x111;

/// The blending a CFF2 Private DICT is read with: the variation store and
/// the instance's normalized coordinates (none for the default).
#[derive(Clone, Copy, Debug)]
pub(super) struct Instance<'v> {
    pub(super) store: &'v VariationStore,
    pub(super) coords: &'v [i64],
}

/// A Private DICT as read: its hinting entries and where its local
/// subroutines are, relative to the DICT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Private {
    pub(super) dict: PrivateDict,
    pub(super) subrs: Option<usize>,
}

impl Private {
    /// A Font DICT without a Private DICT: FreeType's fields stay zero.
    pub(super) fn absent() -> Self {
        Self {
            dict: PrivateDict {
                blue_scale: 0,
                blue_shift: 0,
                blue_fuzz: 0,
                ..PrivateDict::default()
            },
            subrs: None,
        }
    }

    /// Reads the Private DICT of `size` bytes at `offset` of `table`.
    /// `stack` is the most operands it may hold; `instance` blends a
    /// CFF2 DICT's values (none: a CFF DICT, without blends).
    pub(super) fn read(
        table: CffBytes<'_>,
        (size, offset): (usize, usize),
        stack: usize,
        instance: Option<Instance<'_>>,
        tag: Tag,
    ) -> Result<Self> {
        if size == 0 || offset == 0 {
            return Ok(Self::absent());
        }
        let data = table
            .get(offset..offset.saturating_add(size))
            .ok_or(FontError::Truncated(tag))?;
        let mut private = Self {
            dict: PrivateDict::default(),
            subrs: None,
        };
        let kind = if instance.is_some() {
            DictKind::Cff2Private
        } else {
            DictKind::CffPrivate
        };
        let mut operands: Vec<Operand<'_>> = Vec::new();
        let mut blended = false;
        for token in tokens(data, tag) {
            let operator = match token? {
                Token::Operand(operand) => {
                    if operands.len() >= stack {
                        return Err(FontError::malformed(tag, "too many DICT operands"));
                    }
                    operands.push(operand);
                    continue;
                }
                Token::Operator(operator) => operator,
            };
            if operands.len() >= stack {
                return Err(FontError::malformed(tag, "too many DICT operands"));
            }
            kind.check(operator, operands.len(), tag)?;
            match (operator, instance) {
                (BLEND, Some(instance)) => {
                    blend(&mut operands, instance, private.dict.vsindex, stack, tag)?;
                    blended = true;
                    // The blended values stay for the next operator.
                    continue;
                }
                (VSINDEX, Some(_)) => {
                    if blended {
                        return Err(FontError::malformed(tag, "vsindex after blend"));
                    }
                    let first = operands.first().ok_or(FontError::Truncated(tag))?;
                    private.dict.vsindex = first.num() as u32;
                }
                _ => private.store(operator, &operands),
            }
            operands.clear();
        }
        let blues = &mut private.dict.blue_values;
        blues.truncate(blues.len() & !1);
        let dict = &mut private.dict;
        if !(0..=1000).contains(&dict.blue_shift) {
            dict.blue_shift = 7;
        }
        if !(0..=1000).contains(&dict.blue_fuzz) {
            dict.blue_fuzz = 1;
        }
        Ok(private)
    }

    /// Stores the value of an entry the engine reads (`CFF_PrivateRec`).
    fn store(&mut self, operator: u16, operands: &[Operand<'_>]) {
        let dict = &mut self.dict;
        let first = operands.first();
        match operator {
            BLUE_VALUES => dict.blue_values = deltas(operands, 14),
            OTHER_BLUES => dict.other_blues = deltas(operands, 10),
            FAMILY_BLUES => dict.family_blues = deltas(operands, 14),
            FAMILY_OTHER_BLUES => dict.family_other_blues = deltas(operands, 10),
            BLUE_SCALE => {
                if let Some(value) = first {
                    dict.blue_scale = value.fixed_scaled(3) as i32;
                }
            }
            BLUE_SHIFT => {
                if let Some(value) = first {
                    dict.blue_shift = value.num() as i32;
                }
            }
            BLUE_FUZZ => {
                if let Some(value) = first {
                    dict.blue_fuzz = value.num() as i32;
                }
            }
            LANGUAGE_GROUP => {
                if let Some(value) = first {
                    dict.language_group = value.num() as i32;
                }
            }
            SUBRS => {
                if let Some(value) = first {
                    self.subrs = usize::try_from(value.num()).ok().filter(|&at| at != 0);
                }
            }
            _ => {}
        }
    }
}

/// A delta-encoded array of at most `max` values in 16.16
/// (`cff_kind_delta_fixed`): each the sum of those before.
fn deltas(operands: &[Operand<'_>], max: usize) -> Vec<i32> {
    let mut sum = 0i64;
    operands
        .iter()
        .take(max)
        .map(|operand| {
            sum = sum.wrapping_add(operand.fixed());
            sum as i32
        })
        .collect()
}

/// `cff_blend_doBlend`: the last operand counts the values to blend; each
/// default value is replaced by itself plus its deltas weighted by the
/// blend vector, and the deltas are dropped.
fn blend(
    operands: &mut Vec<Operand<'_>>,
    instance: Instance<'_>,
    vsindex: u32,
    stack: usize,
    tag: Tag,
) -> Result<()> {
    let invalid = || FontError::malformed(tag, "invalid blend");
    let vector = instance
        .store
        .blend_vector(vsindex, instance.coords)
        .map_err(|_| invalid())?;
    let count = operands.pop().ok_or_else(invalid)?.num();
    let blends = usize::try_from(count).map_err(|_| invalid())?;
    if blends > stack {
        return Err(invalid());
    }
    let needed = blends * vector.len();
    let base = operands.len().checked_sub(needed).ok_or_else(invalid)?;
    let mut delta = base + blends;
    for i in 0..blends {
        let mut sum = operands[base + i].fixed();
        for &weight in &vector[1..] {
            sum += crate::fixed::mul_fix(operands[delta].fixed(), i64::from(weight));
            delta += 1;
        }
        operands[base + i] = Operand::Blended(sum as i32);
    }
    operands.truncate(base + blends);
    Ok(())
}
