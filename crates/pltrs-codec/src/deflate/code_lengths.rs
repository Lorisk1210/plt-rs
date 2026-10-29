//! Run-length coding of Huffman code lengths with the 19-symbol alphabet
//! DEFLATE (RFC 1951 §3.2.7) and WebP's lossless format share: lengths
//! 0–15, 16 repeating the previous length 3–6 times, 17 and 18 for runs
//! of 3–10 and 11–138 zeros. Symbol 16 only ever follows the non-zero
//! length it repeats, which both formats read the same way.

/// One instruction of the run-length encoded code lengths: a code-length
/// symbol (0–18) and the value of its extra bits.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CodeLengthOp {
    /// The code-length symbol.
    pub(crate) symbol: u8,
    /// The value of its extra bits.
    pub(crate) extra: u8,
}

impl CodeLengthOp {
    const fn length(value: u8) -> Self {
        Self {
            symbol: value,
            extra: 0,
        }
    }

    /// Repeat the previous length `count` (3–6) times.
    const fn repeat(count: usize) -> Self {
        Self {
            symbol: 16,
            extra: (count - 3) as u8,
        }
    }

    /// `count` zero lengths: 3–10 with symbol 17, 11–138 with symbol 18.
    const fn zeros(count: usize) -> Self {
        if count <= 10 {
            Self {
                symbol: 17,
                extra: (count - 3) as u8,
            }
        } else {
            Self {
                symbol: 18,
                extra: (count - 11) as u8,
            }
        }
    }

    /// How many extra bits the symbol carries.
    pub(crate) const fn extra_bits(self) -> u32 {
        match self.symbol {
            16 => 2,
            17 => 3,
            18 => 7,
            _ => 0,
        }
    }
}

/// The instructions coding `lengths`.
pub(crate) fn run_length_ops(lengths: &[u8]) -> Vec<CodeLengthOp> {
    let mut ops = Vec::new();
    for run in lengths.chunk_by(|a, b| a == b) {
        encode_run(run[0], run.len(), &mut ops);
    }
    ops
}

/// Appends the instructions for `count` repetitions of code length `value`.
fn encode_run(value: u8, count: usize, ops: &mut Vec<CodeLengthOp>) {
    let mut left = count;
    if value == 0 {
        while left >= 3 {
            let run = left.min(138);
            ops.push(CodeLengthOp::zeros(run));
            left -= run;
        }
    } else {
        ops.push(CodeLengthOp::length(value));
        left -= 1;
        while left >= 3 {
            let run = left.min(6);
            ops.push(CodeLengthOp::repeat(run));
            left -= run;
        }
    }
    ops.extend(std::iter::repeat_n(CodeLengthOp::length(value), left));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(ops: &[CodeLengthOp]) -> Vec<u8> {
        let mut lengths: Vec<u8> = Vec::new();
        for op in ops {
            match op.symbol {
                16 => {
                    let last = *lengths.last().unwrap();
                    lengths.extend(std::iter::repeat_n(last, usize::from(op.extra) + 3));
                }
                17 => lengths.extend(std::iter::repeat_n(0, usize::from(op.extra) + 3)),
                18 => lengths.extend(std::iter::repeat_n(0, usize::from(op.extra) + 11)),
                value => lengths.push(value),
            }
        }
        lengths
    }

    #[test]
    fn run_length_encoding_round_trips() {
        for value in [0u8, 1, 8, 15] {
            for count in 1..300 {
                let mut ops = Vec::new();
                encode_run(value, count, &mut ops);
                assert_eq!(expand(&ops), vec![value; count], "value {value} x {count}");
                assert!(ops.iter().all(|op| op.extra < 128));
            }
        }
    }
}
