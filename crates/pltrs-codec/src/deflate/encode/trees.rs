//! Blocks: zlib-ng's `trees.c` (by Jean-loup Gailly) without the code
//! construction (in [`codes`](super::codes)): the symbol buffer, the choice
//! between stored, fixed and dynamic blocks, and their output.

use pltrs_core::alloc::{self, OutOfMemory};

use super::Output;
use super::codes::{
    BL_CODES, Builder, Code, D_CODES, DISTANCE_TREE, END_BLOCK, HEAP_SIZE, L_CODES, LENGTH_TREE,
    LITERAL_TREE, LITERALS, Node, STATIC_DTREE, STATIC_LTREE,
};
use crate::deflate::tables::{
    CODE_LENGTH_ORDER, DIST_BASE, DIST_EXTRA, LENGTH_BASE, LENGTH_EXTRA, dist_index, length_index,
};

/// Repeat the previous length 3–6 times (`REP_3_6`).
const REP_3_6: usize = 16;
/// Repeat a zero length 3–10 times (`REPZ_3_10`).
const REPZ_3_10: usize = 17;
/// Repeat a zero length 11–138 times (`REPZ_11_138`).
const REPZ_11_138: usize = 18;

/// The block types of the three-bit header.
const STORED_BLOCK: u64 = 0;
const STATIC_TREES: u64 = 1;
const DYN_TREES: u64 = 2;

/// The symbols of the block being gathered (zlib-ng's `d_buf` and
/// `l_buf`): a distance (0 for a literal) and a literal or a match length
/// less three.
#[derive(Debug)]
pub(super) struct Symbols {
    dist: Vec<u16>,
    lit: Vec<u8>,
    /// The symbols gathered (`sym_next`).
    len: usize,
    /// A block holds this many (`sym_end`).
    end: usize,
}

impl Symbols {
    /// Room for this many symbols, one more than a block holds.
    pub(super) fn capacity(&self) -> usize {
        self.end + 1
    }

    /// These buffers, emptied.
    pub(super) fn cleared(mut self) -> Self {
        self.len = 0;
        self
    }

    /// Room for `capacity - 1` symbols (zlib's `lit_bufsize`).
    ///
    /// # Errors
    ///
    /// When the memory cannot be allocated.
    pub(super) fn new(capacity: usize) -> Result<Self, OutOfMemory> {
        let mut dist = alloc::try_with_capacity(capacity)?;
        dist.resize(capacity, 0);
        let mut lit = alloc::try_with_capacity(capacity)?;
        lit.resize(capacity, 0);
        Ok(Self {
            dist,
            lit,
            len: 0,
            end: capacity - 1,
        })
    }
}

/// A run of equal code lengths as `scan_tree` and `send_tree` send it.
#[derive(Clone, Copy, Debug)]
enum Run {
    /// `count` copies of `len`, each sent on its own.
    Each { len: u16, count: u32 },
    /// `count` copies of a non-zero `len`: the length itself first unless
    /// the previous run had it, then a repeat of the rest.
    Repeat { len: u16, count: u32, first: bool },
    /// `count` zero lengths.
    Zeros { count: u32 },
}

/// The runs of the lengths of `tree`'s first `max_code + 1` symbols as
/// zlib groups them (the loop of `scan_tree` and `send_tree`).
fn runs(tree: &[Node], max_code: usize, mut visit: impl FnMut(Run)) {
    let mut prevlen = None;
    let mut nextlen = tree[0].len;
    let mut count = 0;
    let (mut max_count, mut min_count) = if nextlen == 0 { (138, 3) } else { (7, 4) };
    for n in 0..=max_code {
        let curlen = nextlen;
        nextlen = if n == max_code {
            u16::MAX
        } else {
            tree[n + 1].len
        };
        count += 1;
        if count < max_count && curlen == nextlen {
            continue;
        }
        visit(if count < min_count {
            Run::Each { len: curlen, count }
        } else if curlen != 0 {
            Run::Repeat {
                len: curlen,
                count,
                first: prevlen != Some(curlen),
            }
        } else {
            Run::Zeros { count }
        });
        count = 0;
        prevlen = Some(curlen);
        (max_count, min_count) = if nextlen == 0 {
            (138, 3)
        } else if curlen == nextlen {
            (6, 3)
        } else {
            (7, 4)
        };
    }
}

/// The trees of the block being gathered (the tree parts of zlib's
/// `deflate_state`).
#[derive(Debug)]
pub(super) struct Trees {
    ltree: [Node; HEAP_SIZE],
    dtree: [Node; 2 * D_CODES + 1],
    bltree: [Node; 2 * BL_CODES + 1],
    lcodes: [Code; L_CODES],
    dcodes: [Code; D_CODES],
    blcodes: [Code; BL_CODES],
    builder: Builder,
}

impl Trees {
    pub(super) fn new() -> Self {
        let mut trees = Self {
            ltree: [Node::default(); HEAP_SIZE],
            dtree: [Node::default(); 2 * D_CODES + 1],
            bltree: [Node::default(); 2 * BL_CODES + 1],
            lcodes: [Code::default(); L_CODES],
            dcodes: [Code::default(); D_CODES],
            blcodes: [Code::default(); BL_CODES],
            builder: Builder::new(),
        };
        trees.init_block();
        trees
    }

    /// Starts a new block (`init_block`).
    fn init_block(&mut self) {
        for node in &mut self.ltree[..L_CODES] {
            node.freq = 0;
        }
        for node in &mut self.dtree[..D_CODES] {
            node.freq = 0;
        }
        for node in &mut self.bltree[..BL_CODES] {
            node.freq = 0;
        }
        self.ltree[END_BLOCK].freq = 1;
        self.builder.opt_len = 0;
        self.builder.static_len = 0;
    }

    /// Records a literal and returns whether the block is full
    /// (`zng_tr_tally_lit`).
    #[inline]
    pub(super) fn tally_lit(&mut self, syms: &mut Symbols, byte: u8) -> bool {
        syms.dist[syms.len] = 0;
        syms.lit[syms.len] = byte;
        syms.len += 1;
        self.ltree[usize::from(byte)].freq += 1;
        syms.len == syms.end
    }

    /// Records a match `dist` back of `len + 3` bytes and returns whether
    /// the block is full (`zng_tr_tally_dist`).
    #[inline]
    pub(super) fn tally_dist(&mut self, syms: &mut Symbols, dist: u32, len: u32) -> bool {
        syms.dist[syms.len] = dist as u16;
        syms.lit[syms.len] = len as u8;
        syms.len += 1;
        self.ltree[length_index(len as usize + 3) + LITERALS + 1].freq += 1;
        self.dtree[dist_index(dist as usize)].freq += 1;
        syms.len == syms.end
    }

    /// Writes the gathered symbols as a block — stored if `stored` (the
    /// bytes they code, while the window still holds them) is smallest,
    /// else fixed or dynamic, fixed always when `fixed_only` — and starts
    /// the next (`zng_tr_flush_block`).
    pub(super) fn flush_block(
        &mut self,
        syms: &mut Symbols,
        out: &mut Output<'_>,
        stored: Option<&[u8]>,
        last: bool,
        fixed_only: bool,
    ) {
        let mut dynamic = None;
        let (mut opt_lenb, static_lenb) = if syms.len == 0 {
            (0, 0)
        } else {
            let l_max_code =
                self.builder
                    .build_tree(&mut self.ltree, &mut self.lcodes, &LITERAL_TREE);
            let d_max_code =
                self.builder
                    .build_tree(&mut self.dtree, &mut self.dcodes, &DISTANCE_TREE);
            let max_blindex = self.build_bl_tree(l_max_code, d_max_code);
            dynamic = Some((l_max_code, d_max_code, max_blindex));
            (
                self.builder.opt_len.wrapping_add(3 + 7) >> 3,
                self.builder.static_len.wrapping_add(3 + 7) >> 3,
            )
        };
        if static_lenb <= opt_lenb || fixed_only {
            opt_lenb = static_lenb;
        }
        match (stored, dynamic) {
            (Some(bytes), _) if bytes.len() as u64 + 4 <= opt_lenb => {
                stored_block(out, bytes, last);
            }
            (_, Some((l_max_code, d_max_code, max_blindex))) if static_lenb != opt_lenb => {
                send_block_header(out, DYN_TREES, last);
                self.send_all_trees(out, l_max_code + 1, d_max_code + 1, max_blindex + 1);
                compress_block(syms, out, &self.lcodes, &self.dcodes);
            }
            _ => {
                send_block_header(out, STATIC_TREES, last);
                compress_block(syms, out, &STATIC_LTREE, &STATIC_DTREE);
            }
        }
        self.init_block();
        syms.len = 0;
        if last {
            out.align();
        }
    }

    /// Builds the code-length code of the two trees and returns the index
    /// in [`CODE_LENGTH_ORDER`] of the last length to send
    /// (`build_bl_tree`).
    fn build_bl_tree(&mut self, l_max_code: usize, d_max_code: usize) -> usize {
        scan_tree(&mut self.bltree, &self.ltree, l_max_code);
        scan_tree(&mut self.bltree, &self.dtree, d_max_code);
        self.builder
            .build_tree(&mut self.bltree, &mut self.blcodes, &LENGTH_TREE);
        let max_blindex = (3..BL_CODES)
            .rev()
            .find(|&index| self.bltree[CODE_LENGTH_ORDER[index]].len != 0)
            .expect("a non-zero length is sent as itself");
        self.builder.opt_len = self
            .builder
            .opt_len
            .wrapping_add(3 * (max_blindex as u64 + 1) + 5 + 5 + 4);
        max_blindex
    }

    /// Sends a dynamic block's code counts, code-length code and the two
    /// trees (`send_all_trees`).
    fn send_all_trees(&self, out: &mut Output<'_>, lcodes: usize, dcodes: usize, blcodes: usize) {
        out.send_bits((lcodes - 257) as u64, 5);
        out.send_bits((dcodes - 1) as u64, 5);
        out.send_bits((blcodes - 4) as u64, 4);
        for &symbol in &CODE_LENGTH_ORDER[..blcodes] {
            out.send_bits(u64::from(self.bltree[symbol].len), 3);
        }
        self.send_tree(out, &self.ltree, lcodes - 1);
        self.send_tree(out, &self.dtree, dcodes - 1);
    }

    /// Sends `tree`'s lengths with the code-length code (`send_tree`).
    fn send_tree(&self, out: &mut Output<'_>, tree: &[Node], max_code: usize) {
        let send = |out: &mut Output<'_>, symbol: usize| {
            let code = self.blcodes[symbol];
            out.send_bits(u64::from(code.code), u32::from(code.len));
        };
        runs(tree, max_code, |run| match run {
            Run::Each { len, count } => {
                for _ in 0..count {
                    send(out, usize::from(len));
                }
            }
            Run::Repeat { len, count, first } => {
                let mut count = count;
                if first {
                    send(out, usize::from(len));
                    count -= 1;
                }
                send(out, REP_3_6);
                out.send_bits(u64::from(count - 3), 2);
            }
            Run::Zeros { count } if count <= 10 => {
                send(out, REPZ_3_10);
                out.send_bits(u64::from(count - 3), 3);
            }
            Run::Zeros { count } => {
                send(out, REPZ_11_138);
                out.send_bits(u64::from(count - 11), 7);
            }
        });
    }
}

/// Counts the code-length symbols that send `tree`'s lengths into
/// `bltree` (`scan_tree`).
fn scan_tree(bltree: &mut [Node], tree: &[Node], max_code: usize) {
    runs(tree, max_code, |run| match run {
        Run::Each { len, count } => bltree[usize::from(len)].freq += count,
        Run::Repeat { len, first, .. } => {
            if first {
                bltree[usize::from(len)].freq += 1;
            }
            bltree[REP_3_6].freq += 1;
        }
        Run::Zeros { count } if count <= 10 => bltree[REPZ_3_10].freq += 1,
        Run::Zeros { .. } => bltree[REPZ_11_138].freq += 1,
    });
}

/// Sends a block's three-bit header (`zng_tr_emit_tree`).
#[inline]
pub(super) fn send_block_header(out: &mut Output<'_>, kind: u64, last: bool) {
    out.send_bits(kind << 1 | u64::from(last), 3);
}

/// Opens a fixed-code block (`QUICK_START_BLOCK`'s header).
pub(super) fn send_fixed_header(out: &mut Output<'_>, last: bool) {
    send_block_header(out, STATIC_TREES, last);
}

/// Sends the header of a stored block of `len` bytes: the block type,
/// padding to a byte, the length and its complement.
pub(super) fn send_stored_header(out: &mut Output<'_>, len: u16, last: bool) {
    send_block_header(out, STORED_BLOCK, last);
    out.align();
    out.put_bytes(&len.to_le_bytes());
    out.put_bytes(&(!len).to_le_bytes());
}

/// Writes `bytes` as a stored block (`zng_tr_stored_block`).
fn stored_block(out: &mut Output<'_>, bytes: &[u8], last: bool) {
    send_stored_header(out, bytes.len() as u16, last);
    out.put_bytes(bytes);
}

/// Sends a literal with the fixed code (`zng_tr_emit_lit`).
#[inline]
pub(super) fn send_fixed_literal(out: &mut Output<'_>, byte: u8) {
    let code = STATIC_LTREE[usize::from(byte)];
    out.send_bits(u64::from(code.code), u32::from(code.len));
}

/// Sends a match with the fixed codes (`zng_tr_emit_dist`).
#[inline]
pub(super) fn send_fixed_match(out: &mut Output<'_>, len: u32, dist: u32) {
    send_match(out, &STATIC_LTREE, &STATIC_DTREE, len, dist);
}

/// Ends a fixed-code block (`zng_tr_emit_end_block`'s code).
pub(super) fn send_fixed_end(out: &mut Output<'_>) {
    let code = STATIC_LTREE[END_BLOCK];
    out.send_bits(u64::from(code.code), u32::from(code.len));
}

/// Sends a match `dist` back of `len + 3` bytes: the length's code and
/// extra bits, then the distance's, at once (`zng_emit_dist`).
#[inline]
fn send_match(out: &mut Output<'_>, lcodes: &[Code], dcodes: &[Code], len: u32, dist: u32) {
    let length = len as usize + 3;
    let index = length_index(length);
    let code = lcodes[LITERALS + 1 + index];
    let mut bits = u64::from(code.code);
    let mut count = u32::from(code.len);
    bits |= ((length - usize::from(LENGTH_BASE[index])) as u64) << count;
    count += u32::from(LENGTH_EXTRA[index]);
    let index = dist_index(dist as usize);
    let code = dcodes[index];
    bits |= u64::from(code.code) << count;
    count += u32::from(code.len);
    bits |= u64::from(dist - u32::from(DIST_BASE[index])) << count;
    count += u32::from(DIST_EXTRA[index]);
    out.send_bits(bits, count);
}

/// Sends the gathered symbols with the given codes, then the end of the
/// block (`compress_block`).
fn compress_block(syms: &Symbols, out: &mut Output<'_>, lcodes: &[Code], dcodes: &[Code]) {
    for (&dist, &lit) in syms.dist[..syms.len].iter().zip(&syms.lit[..syms.len]) {
        if dist == 0 {
            let code = lcodes[usize::from(lit)];
            out.send_bits(u64::from(code.code), u32::from(code.len));
        } else {
            send_match(out, lcodes, dcodes, u32::from(lit), u32::from(dist));
        }
    }
    let code = lcodes[END_BLOCK];
    out.send_bits(u64::from(code.code), u32::from(code.len));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_group_lengths_as_zlib_does() {
        let lengths = [0, 0, 0, 0, 5, 5, 5, 5, 5, 5, 5, 5, 3, 0, 0];
        let mut tree = [Node::default(); 16];
        for (node, &len) in tree.iter_mut().zip(&lengths) {
            node.len = len;
        }
        let mut seen = Vec::new();
        runs(&tree, lengths.len() - 1, |run| {
            seen.push(format!("{run:?}"));
        });
        assert_eq!(
            seen,
            [
                "Zeros { count: 4 }",
                "Repeat { len: 5, count: 7, first: true }",
                "Each { len: 5, count: 1 }",
                "Each { len: 3, count: 1 }",
                "Each { len: 0, count: 2 }",
            ]
        );
    }
}
