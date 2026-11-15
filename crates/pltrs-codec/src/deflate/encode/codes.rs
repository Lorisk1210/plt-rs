//! The Huffman codes of a block: zlib's `build_tree`, `gen_bitlen` and
//! `gen_codes` (trees.c, by Jean-loup Gailly), codes as long as the
//! symbols' frequencies call for, limited to 15 bits (7 for the
//! code-length code), with zlib's order among equal frequencies; and the
//! fixed codes of RFC 1951.

use crate::deflate::tables::{DIST_EXTRA, LENGTH_EXTRA, fixed_litlen_lengths};

/// Literal bytes (`LITERALS`).
pub(super) const LITERALS: usize = 256;
/// The end-of-block symbol (`END_BLOCK`).
pub(super) const END_BLOCK: usize = 256;
/// Literal/length symbols, with the end of block (`L_CODES`).
pub(super) const L_CODES: usize = 286;
/// Distance symbols (`D_CODES`).
pub(super) const D_CODES: usize = 30;
/// Code-length symbols (`BL_CODES`).
pub(super) const BL_CODES: usize = 19;
/// The nodes of the largest tree (`HEAP_SIZE`).
pub(super) const HEAP_SIZE: usize = 2 * L_CODES + 1;
/// The longest literal/length and distance code (`MAX_BITS`).
const MAX_BITS: usize = 15;
/// The longest code-length code (`MAX_BL_BITS`).
const MAX_BL_BITS: usize = 7;
/// Extra bits of the code-length symbols (`extra_blbits`).
const EXTRA_BL: [u8; BL_CODES] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 7];

/// A node of a Huffman tree being built (zlib's `ct_data`, its unions
/// unfolded): leaves are symbols, the nodes above them follow.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Node {
    pub(super) freq: u32,
    dad: u16,
    pub(super) len: u16,
}

/// A symbol's code, bit-reversed for LSB-first output, and its length.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Code {
    pub(super) code: u16,
    pub(super) len: u16,
}

/// What `build_tree` needs to know of a tree (`static_tree_desc`).
pub(super) struct TreeKind {
    /// The fixed code, to count the block's size with it.
    fixed: Option<&'static [Code]>,
    /// The extra bits of the symbols from `extra_base` on.
    extra: &'static [u8],
    extra_base: usize,
    /// The symbols.
    elems: usize,
    /// The longest code.
    max_length: usize,
}

/// The fixed literal/length code (`static_ltree`).
pub(super) static STATIC_LTREE: [Code; L_CODES + 2] = canonical(&fixed_litlen_lengths());
/// The fixed distance code (`static_dtree`).
pub(super) static STATIC_DTREE: [Code; D_CODES] = canonical(&[5; D_CODES]);

pub(super) const LITERAL_TREE: TreeKind = TreeKind {
    fixed: Some(&STATIC_LTREE),
    extra: &LENGTH_EXTRA,
    extra_base: LITERALS + 1,
    elems: L_CODES,
    max_length: MAX_BITS,
};
pub(super) const DISTANCE_TREE: TreeKind = TreeKind {
    fixed: Some(&STATIC_DTREE),
    extra: &DIST_EXTRA,
    extra_base: 0,
    elems: D_CODES,
    max_length: MAX_BITS,
};
pub(super) const LENGTH_TREE: TreeKind = TreeKind {
    fixed: None,
    extra: &EXTRA_BL,
    extra_base: 0,
    elems: BL_CODES,
    max_length: MAX_BL_BITS,
};

/// `code`'s low `len` bits reversed (`bi_reverse`).
const fn reverse(code: u16, len: u16) -> u16 {
    code.reverse_bits() >> (16 - len)
}

/// The canonical codes of `lengths` (`gen_codes`, at compile time).
const fn canonical<const N: usize>(lengths: &[u8; N]) -> [Code; N] {
    let mut count = [0_u16; MAX_BITS + 1];
    let mut i = 0;
    while i < N {
        count[lengths[i] as usize] += 1;
        i += 1;
    }
    let mut next = first_codes(&count);
    let mut codes = [Code { code: 0, len: 0 }; N];
    let mut n = 0;
    while n < N {
        let len = lengths[n] as u16;
        if len != 0 {
            codes[n] = Code {
                code: reverse(next[len as usize], len),
                len,
            };
            next[len as usize] += 1;
        }
        n += 1;
    }
    codes
}

/// The first code of each length, given how many codes each length has
/// (the first loop of `gen_codes`).
const fn first_codes(count: &[u16; MAX_BITS + 1]) -> [u16; MAX_BITS + 1] {
    let mut next = [0_u16; MAX_BITS + 1];
    let mut code = 0_u32;
    let mut bits = 1;
    while bits <= MAX_BITS {
        code = (code + count[bits - 1] as u32) << 1;
        next[bits] = code as u16;
        bits += 1;
    }
    next
}

/// Whether `n` sorts before `m` in the heap: less frequent, or as
/// frequent and no deeper (`smaller`).
#[inline]
fn smaller(tree: &[Node], depth: &[u8], n: usize, m: usize) -> bool {
    tree[n].freq < tree[m].freq || (tree[n].freq == tree[m].freq && depth[n] <= depth[m])
}

/// The heap and counts the trees are built with, and the block's sizes.
#[derive(Debug)]
pub(super) struct Builder {
    /// The heap from 1 to `heap_len`; from `heap_max` up, the nodes in
    /// the order they left it (`heap`).
    heap: Box<[usize; HEAP_SIZE]>,
    heap_len: usize,
    heap_max: usize,
    /// Each node's height, the tie-breaker (`depth`).
    depth: Box<[u8; HEAP_SIZE]>,
    /// The codes of each length of the tree last built (`bl_count`).
    bl_count: [u16; MAX_BITS + 1],
    /// The block's bits with the dynamic codes (`opt_len`), modulo 2^64
    /// as zlib's sums are.
    pub(super) opt_len: u64,
    /// The block's bits with the fixed codes (`static_len`).
    pub(super) static_len: u64,
}

/// `N` copies of `value` in an array on the heap, never on the stack: a
/// stream's trees and heaps are kilobytes, more than a thread with a small
/// stack (musl's 128 KiB) should spend on each frame holding one. An
/// array, not a slice, so that indexing it knows its length.
pub(super) fn on_heap<T: Copy, const N: usize>(value: T) -> Box<[T; N]> {
    vec![value; N]
        .into_boxed_slice()
        .try_into()
        .unwrap_or_else(|_| unreachable!("a vector of N elements is an array of N"))
}

impl Builder {
    pub(super) fn new() -> Self {
        Self {
            heap: on_heap(0),
            heap_len: 0,
            heap_max: 0,
            depth: on_heap(0),
            bl_count: [0; MAX_BITS + 1],
            opt_len: 0,
            static_len: 0,
        }
    }

    /// Back to what [`new`](Self::new) makes, without allocating.
    pub(super) fn clear(&mut self) {
        self.heap.fill(0);
        self.heap_len = 0;
        self.heap_max = 0;
        self.depth.fill(0);
        self.bl_count = [0; MAX_BITS + 1];
        self.opt_len = 0;
        self.static_len = 0;
    }

    /// Restores the heap below position `k` (`pqdownheap`).
    fn pqdownheap(&mut self, tree: &[Node], mut k: usize) {
        let v = self.heap[k];
        let mut j = k << 1;
        while j <= self.heap_len {
            if j < self.heap_len && smaller(tree, &self.depth[..], self.heap[j + 1], self.heap[j]) {
                j += 1;
            }
            if smaller(tree, &self.depth[..], v, self.heap[j]) {
                break;
            }
            self.heap[k] = self.heap[j];
            k = j;
            j <<= 1;
        }
        self.heap[k] = v;
    }

    /// Builds the length-limited Huffman code of `tree`'s frequencies into
    /// `codes`, adds the block's cost with it (and with the fixed code) and
    /// returns the largest symbol used (`build_tree`).
    pub(super) fn build_tree(
        &mut self,
        tree: &mut [Node],
        codes: &mut [Code],
        kind: &TreeKind,
    ) -> usize {
        self.heap_len = 0;
        self.heap_max = HEAP_SIZE;
        let mut max_code: Option<usize> = None;
        for (n, node) in tree[..kind.elems].iter_mut().enumerate() {
            if node.freq == 0 {
                node.len = 0;
            } else {
                self.heap_len += 1;
                self.heap[self.heap_len] = n;
                max_code = Some(n);
                self.depth[n] = 0;
            }
        }
        // At least two codes, so that each has at least one bit.
        while self.heap_len < 2 {
            let node = match max_code {
                Some(code) if code >= 2 => 0,
                other => {
                    let next = other.map_or(0, |code| code + 1);
                    max_code = Some(next);
                    next
                }
            };
            self.heap_len += 1;
            self.heap[self.heap_len] = node;
            tree[node].freq = 1;
            self.depth[node] = 0;
            self.opt_len = self.opt_len.wrapping_sub(1);
            if let Some(fixed) = kind.fixed {
                self.static_len = self.static_len.wrapping_sub(u64::from(fixed[node].len));
            }
        }
        let max_code = max_code.expect("the heap holds two codes");
        for n in (1..=self.heap_len / 2).rev() {
            self.pqdownheap(tree, n);
        }
        let mut node = kind.elems;
        loop {
            let n = self.heap[1];
            self.heap[1] = self.heap[self.heap_len];
            self.heap_len -= 1;
            self.pqdownheap(tree, 1);
            let m = self.heap[1];
            self.heap_max -= 2;
            self.heap[self.heap_max + 1] = n;
            self.heap[self.heap_max] = m;
            tree[node].freq = tree[n].freq + tree[m].freq;
            self.depth[node] = self.depth[n].max(self.depth[m]) + 1;
            tree[n].dad = node as u16;
            tree[m].dad = node as u16;
            self.heap[1] = node;
            node += 1;
            self.pqdownheap(tree, 1);
            if self.heap_len < 2 {
                break;
            }
        }
        self.heap_max -= 1;
        self.heap[self.heap_max] = self.heap[1];
        self.gen_bitlen(tree, kind, max_code);
        let mut next = first_codes(&self.bl_count);
        for (code, node) in codes.iter_mut().zip(&tree[..=max_code]) {
            *code = Code {
                code: if node.len == 0 {
                    0
                } else {
                    let len = usize::from(node.len);
                    next[len] += 1;
                    reverse(next[len] - 1, node.len)
                },
                len: node.len,
            };
        }
        for code in &mut codes[max_code + 1..] {
            *code = Code::default();
        }
        max_code
    }

    /// Sets the code lengths from the tree, at most the kind's longest,
    /// and counts the block's bits (`gen_bitlen`).
    fn gen_bitlen(&mut self, tree: &mut [Node], kind: &TreeKind, max_code: usize) {
        let max_length = kind.max_length;
        self.bl_count = [0; MAX_BITS + 1];
        let mut overflow = 0_i32;
        tree[self.heap[self.heap_max]].len = 0;
        for h in self.heap_max + 1..HEAP_SIZE {
            let n = self.heap[h];
            let mut bits = usize::from(tree[usize::from(tree[n].dad)].len) + 1;
            if bits > max_length {
                bits = max_length;
                overflow += 1;
            }
            tree[n].len = bits as u16;
            if n > max_code {
                continue;
            }
            self.bl_count[bits] += 1;
            let extra = n
                .checked_sub(kind.extra_base)
                .map_or(0, |index| u64::from(kind.extra[index]));
            let freq = u64::from(tree[n].freq);
            self.opt_len = self.opt_len.wrapping_add(freq * (bits as u64 + extra));
            if let Some(fixed) = kind.fixed {
                self.static_len = self
                    .static_len
                    .wrapping_add(freq * (u64::from(fixed[n].len) + extra));
            }
        }
        if overflow == 0 {
            return;
        }
        // Move leaves down from shorter lengths until the overflowing ones
        // fit, then hand the lengths out again by frequency.
        while overflow > 0 {
            let mut bits = max_length - 1;
            while self.bl_count[bits] == 0 {
                bits -= 1;
            }
            self.bl_count[bits] -= 1;
            self.bl_count[bits + 1] += 2;
            self.bl_count[max_length] -= 1;
            overflow -= 2;
        }
        let mut h = HEAP_SIZE;
        for bits in (1..=max_length).rev() {
            let mut n = self.bl_count[bits];
            while n != 0 {
                h -= 1;
                let m = self.heap[h];
                if m > max_code {
                    continue;
                }
                if usize::from(tree[m].len) != bits {
                    let freq = u64::from(tree[m].freq);
                    self.opt_len = self
                        .opt_len
                        .wrapping_add(bits as u64 * freq)
                        .wrapping_sub(u64::from(tree[m].len) * freq);
                    tree[m].len = bits as u16;
                }
                n -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_codes_are_rfc_1951s() {
        // 0 is 00110000 and 256 is 0000000, both sent most significant
        // bit first.
        assert_eq!(
            (STATIC_LTREE[0].code, STATIC_LTREE[0].len),
            (0b0000_1100, 8)
        );
        assert_eq!(
            (STATIC_LTREE[144].code, STATIC_LTREE[144].len),
            (0b1_0011, 9)
        );
        assert_eq!((STATIC_LTREE[256].code, STATIC_LTREE[256].len), (0, 7));
        assert_eq!((STATIC_DTREE[1].code, STATIC_DTREE[1].len), (0b1_0000, 5));
    }

    #[test]
    fn lengths_are_limited_and_complete() {
        // Fibonacci frequencies make the deepest tree.
        let mut tree = [Node::default(); HEAP_SIZE];
        let (mut a, mut b) = (1, 1);
        for node in &mut tree[..L_CODES] {
            node.freq = a;
            (a, b) = (b, (a + b).min(1 << 20));
        }
        let mut builder = Builder::new();
        let mut codes = [Code::default(); L_CODES];
        let max_code = builder.build_tree(&mut tree, &mut codes, &LITERAL_TREE);
        assert_eq!(max_code, L_CODES - 1);
        assert!(codes.iter().all(|code| (1..=15).contains(&code.len)));
        let kraft: u32 = codes.iter().map(|code| 1 << (15 - code.len)).sum();
        assert_eq!(kraft, 1 << 15, "a complete code");
    }
}
