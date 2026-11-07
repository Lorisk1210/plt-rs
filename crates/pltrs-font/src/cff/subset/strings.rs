//! The strings a CFF subset keeps: those its glyph names and DICTs name,
//! renumbered after the standard strings, as fontTools rebuilds the
//! String INDEX of a subset from what it refers to.

use std::collections::BTreeMap;

use super::super::dict::{DictKind, Entries, Operator, entries, raw_entries};
use super::super::index::{CFF, Index};
use super::super::{CFF_DICT_STACK, CountSize, STANDARD_STRINGS};
use super::write::{entry, index};
use crate::error::{FontError, Result};

/// How many of `operator`'s first operands are string ids.
const fn string_operands(operator: Operator) -> usize {
    match operator {
        // version, Notice, FullName, FamilyName, Weight, Copyright,
        // PostScript, BaseFontName and FontName.
        0..=4 | 0x100 | 0x115 | 0x116 | 0x126 => 1,
        // ROS: Registry and Ordering.
        0x11E => 2,
        _ => 0,
    }
}

/// The entries of a `CFF ` Top or Font DICT, parsed.
fn parsed(dict: &[u8]) -> Result<Entries<'_>> {
    entries(dict, DictKind::CffTop, CFF_DICT_STACK, CFF)
}

/// The custom strings a subset keeps.
pub(super) struct Strings<'a> {
    index: Index<'a>,
    /// Each kept custom string's id in the font and in the subset.
    renumbered: BTreeMap<u16, u16>,
}

impl<'a> Strings<'a> {
    /// The strings of the String INDEX `index` that the string ids `used`
    /// name.
    pub(super) fn new(index: Index<'a>, used: impl IntoIterator<Item = u16>) -> Self {
        let standard = STANDARD_STRINGS.len() as u16;
        let mut renumbered: BTreeMap<u16, u16> = used
            .into_iter()
            .filter(|&sid| sid >= standard)
            .map(|sid| (sid, 0))
            .collect();
        for (new, sid) in (standard..).zip(renumbered.values_mut()) {
            *sid = new;
        }
        Self { index, renumbered }
    }

    /// The string ids the DICT `dict` names.
    pub(super) fn used_by(dict: &[u8]) -> Result<Vec<u16>> {
        Ok(parsed(dict)?
            .iter()
            .flat_map(|(operator, operands)| operands.iter().take(string_operands(*operator)))
            .filter_map(|operand| u16::try_from(operand.num()).ok())
            .collect())
    }

    /// The subset's id of the font's string `sid`.
    pub(super) fn id(&self, sid: u16) -> u16 {
        self.renumbered.get(&sid).copied().unwrap_or(sid)
    }

    /// The subset's String INDEX.
    pub(super) fn string_index(&self) -> Result<Vec<u8>> {
        let standard = STANDARD_STRINGS.len();
        let strings = self
            .renumbered
            .keys()
            .map(|&sid| {
                self.index
                    .get(usize::from(sid) - standard)
                    .ok_or(FontError::malformed(CFF, "string id beyond the strings"))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(index(strings, CountSize::Short))
    }

    /// The DICT `dict` without the entries of `dropped` operators, the
    /// string ids it names renumbered.
    pub(super) fn rewrite(&self, dict: &[u8], dropped: &[Operator]) -> Result<Vec<u8>> {
        let raw = raw_entries(dict, CFF)?;
        let parsed = parsed(dict)?;
        let mut out = Vec::with_capacity(dict.len());
        for ((operator, bytes), (_, operands)) in raw.into_iter().zip(&parsed) {
            if dropped.contains(&operator) {
                continue;
            }
            let strings = string_operands(operator);
            if strings == 0 {
                out.extend(bytes);
                continue;
            }
            let values = operands
                .iter()
                .enumerate()
                .map(|(i, operand)| {
                    let value = operand.num();
                    let value = match u16::try_from(value) {
                        Ok(sid) if i < strings => i64::from(self.id(sid)),
                        _ => value,
                    };
                    usize::try_from(value)
                        .map_err(|_| FontError::malformed(CFF, "negative string id"))
                })
                .collect::<Result<Vec<_>>>()?;
            out.extend(entry(operator, &values));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kept_strings_are_renumbered() {
        let strings: [&[u8]; 3] = [b"Zero", b"One", b"Two"];
        let encoded = index(strings, CountSize::Short);
        let font_strings = Index::read(&mut crate::reader::Reader::new(&encoded, CFF)).unwrap();
        // FullName "Two" (393), FamilyName "Times" (a standard string),
        // UniqueID 0.
        let top = [28, 1, 0x89, 2, 28, 0, 52, 3, 139, 13];
        assert_eq!(Strings::used_by(&top).unwrap(), [393, 52]);
        let kept = Strings::new(font_strings, [393, 52, 391, 393]);
        assert_eq!((kept.id(391), kept.id(393), kept.id(52)), (391, 392, 52));
        let expected = index([b"Zero".as_slice(), b"Two"], CountSize::Short);
        assert_eq!(kept.string_index().unwrap(), expected);
        let rewritten = kept.rewrite(&top, &[13]).unwrap();
        assert_eq!(rewritten, [29, 0, 0, 1, 0x88, 2, 29, 0, 0, 0, 52, 3]);
    }
}
