//! Encoding the structures of a CFF or CFF2 table: INDEXes, DICT entries
//! with offset operands, charsets and `FDSelect`s.

use super::super::CountSize;
use super::super::dict::Operator;

/// The length of a DICT entry of `operator` with `operands` offsets.
pub(super) const fn entry_length(operator: Operator, operands: usize) -> usize {
    operands * 5 + if operator >= 0x100 { 2 } else { 1 }
}

/// A DICT entry of `operator` whose operands are offsets or sizes, each
/// written as a 32-bit integer (operator 29) so that its length is known
/// before the offsets are.
pub(super) fn entry(operator: Operator, operands: &[usize]) -> Vec<u8> {
    let mut out = Vec::with_capacity(entry_length(operator, operands.len()));
    for &operand in operands {
        out.push(29);
        out.extend((operand as u32).to_be_bytes());
    }
    if operator >= 0x100 {
        out.extend([12, (operator & 0xFF) as u8]);
    } else {
        out.push(operator as u8);
    }
    out
}

/// The bytes of an offset into data of `total` bytes (`offSize`).
fn offset_size(total: usize) -> usize {
    match total + 1 {
        0..=0xFF => 1,
        0x100..=0xFFFF => 2,
        0x1_0000..=0xFF_FFFF => 3,
        _ => 4,
    }
}

/// The length of an INDEX of `lengths`-long items with counts of `count`.
pub(super) fn index_length(lengths: impl IntoIterator<Item = usize>, count: CountSize) -> usize {
    let lengths: Vec<usize> = lengths.into_iter().collect();
    let head = match count {
        CountSize::Short => 2,
        CountSize::Long => 4,
    };
    if lengths.is_empty() {
        return head;
    }
    let total: usize = lengths.iter().sum();
    head + 1 + (lengths.len() + 1) * offset_size(total) + total
}

/// An INDEX of `items` with counts of `count`.
pub(super) fn index<'i>(items: impl IntoIterator<Item = &'i [u8]>, count: CountSize) -> Vec<u8> {
    let items: Vec<&[u8]> = items.into_iter().collect();
    let mut out = Vec::with_capacity(index_length(items.iter().map(|i| i.len()), count));
    match count {
        CountSize::Short => out.extend((items.len() as u16).to_be_bytes()),
        CountSize::Long => out.extend((items.len() as u32).to_be_bytes()),
    }
    if items.is_empty() {
        return out;
    }
    let size = offset_size(items.iter().map(|i| i.len()).sum());
    out.push(size as u8);
    let mut offset = 1usize;
    for length in std::iter::once(0).chain(items.iter().map(|i| i.len())) {
        offset += length;
        out.extend(&(offset as u32).to_be_bytes()[4 - size..]);
    }
    for item in items {
        out.extend(item);
    }
    out
}

/// A format 0 charset of the strings (or CIDs) of glyphs 1 onwards.
pub(super) fn charset(sids: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + sids.len() * 2);
    out.push(0);
    for sid in sids {
        out.extend(sid.to_be_bytes());
    }
    out
}

/// A format 3 `FDSelect` giving glyph `i` the Font DICT `fds[i]`.
pub(super) fn fd_select(fds: &[u8]) -> Vec<u8> {
    let mut ranges: Vec<(u16, u8)> = Vec::new();
    for (glyph, &fd) in fds.iter().enumerate() {
        if ranges.last().is_none_or(|&(_, last)| last != fd) {
            ranges.push((glyph as u16, fd));
        }
    }
    let mut out = Vec::with_capacity(5 + ranges.len() * 3);
    out.push(3);
    out.extend((ranges.len() as u16).to_be_bytes());
    for (first, fd) in ranges {
        out.extend(first.to_be_bytes());
        out.push(fd);
    }
    out.extend((fds.len() as u16).to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_and_entries() {
        assert_eq!(index([], CountSize::Short), [0, 0]);
        assert_eq!(index([], CountSize::Long), [0, 0, 0, 0]);
        let items: [&[u8]; 2] = [b"ab", b"c"];
        let encoded = index(items, CountSize::Short);
        assert_eq!(encoded, [0, 2, 1, 1, 3, 4, b'a', b'b', b'c']);
        assert_eq!(index_length([2, 1], CountSize::Short), encoded.len());
        let long = vec![0u8; 300];
        let encoded = index([long.as_slice()], CountSize::Long);
        assert_eq!(&encoded[..9], [0, 0, 0, 1, 2, 0, 1, 1, 45]);
        assert_eq!(index_length([300], CountSize::Long), encoded.len());
        assert_eq!(entry(17, &[0x1234]), [29, 0, 0, 0x12, 0x34, 17]);
        assert_eq!(entry(0x124, &[1]), [29, 0, 0, 0, 1, 12, 0x24]);
        assert_eq!(entry_length(18, 2), 11);
    }

    #[test]
    fn charsets_and_fd_selects() {
        assert_eq!(charset(&[5, 0x102]), [0, 0, 5, 1, 2]);
        assert_eq!(
            fd_select(&[0, 0, 2, 2, 1]),
            [3, 0, 3, 0, 0, 0, 0, 2, 2, 0, 4, 1, 0, 5]
        );
    }
}
