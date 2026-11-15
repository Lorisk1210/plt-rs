//! The compressor: zlib-ng 2.3's deflate, ported step for step so that it
//! writes the same bytes as zlib-ng for the same input, options and calls.
//!
//! zlib-ng's parse depends not only on the data but on how it arrives:
//! the parsers stop when the input runs out or the caller's output buffer
//! fills up, and some of them forget a look-ahead match when they do. So
//! the state machine is kept as zlib-ng has it — [`Deflater::deflate`] is
//! its `deflate()`, fed and drained by a driver that calls it as the
//! program being matched does: all at once as `compress2` does
//! ([`one_shot`]), or a scanline at a time into 64 KiB buffers as Pillow
//! writes PNG images ([`rows`]).
//!
//! The modules follow zlib-ng's files: [`window`] (the sliding window,
//! `fill_window` and the hash chains of `insert_string`), [`matcher`]
//! (`longest_match`), one module per parsing (`deflate_quick.c`, ...),
//! [`trees`] (`trees.c`'s blocks) and [`codes`] (its Huffman codes).

mod codes;
mod fast;
mod matcher;
mod medium;
mod options;
mod output;
mod plain;
mod quick;
mod rows;
mod slow;
mod stored;
mod trees;
mod window;

pub use options::{InvalidMemoryLevel, MemoryLevel, Options, Strategy};
pub(crate) use output::Output;
pub(crate) use rows::{RowSource, Rows, rows};

use pltrs_core::alloc::OutOfMemory;

use self::options::{Config, Method};
use self::trees::{Symbols, Trees};
use self::window::Window;
use crate::checksum::Adler32;
use crate::deflate::CompressionLevel;

/// The shortest match DEFLATE codes (`STD_MIN_MATCH`).
const MIN_MATCH: u32 = 3;
/// The longest match DEFLATE codes (`STD_MAX_MATCH`).
const MAX_MATCH: u32 = 258;
/// The shortest match zlib-ng's parsers emit (`WANT_MIN_MATCH`).
const WANT_MIN_MATCH: u32 = 4;
/// The lookahead the parsers want before they parse on
/// (`MIN_LOOKAHEAD`).
const MIN_LOOKAHEAD: u32 = MAX_MATCH + MIN_MATCH + 1;
/// The window's size, the farthest a match reaches (`w_size`).
const W_SIZE: u32 = 1 << 15;
/// Positions modulo the window (`w_mask`).
const W_MASK: u32 = W_SIZE - 1;
/// The farthest back the parsers look, so that the lookahead is never
/// cut by a slide (`MAX_DIST`).
const MAX_DIST: u32 = W_SIZE - MIN_LOOKAHEAD;
/// The window buffer, twice the window (`window_size`).
const WINDOW_SIZE: u32 = 2 * W_SIZE;
/// The largest stored block (`MAX_STORED`).
const MAX_STORED: u32 = 65_535;

/// How a stream is wrapped (zlib's `wrap`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wrap {
    /// Raw DEFLATE.
    Raw,
    /// The zlib format: a header, then an Adler-32 trailer.
    Zlib,
}

/// What a call of [`Deflater::deflate`] is asked to do (zlib's `flush`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flush {
    /// Compress what can be compressed now (`Z_NO_FLUSH`).
    None,
    /// The input is complete: compress all of it and end the stream
    /// (`Z_FINISH`).
    Finish,
}

/// What a parser returns (zlib's `block_state`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockState {
    /// More input or more output room is needed (`need_more`).
    NeedMore,
    /// The final block is begun but not all written out
    /// (`finish_started`).
    FinishStarted,
    /// The final block is written (`finish_done`).
    FinishDone,
}

/// Where the stream is (zlib's `status`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    /// The header is still to be written (`INIT_STATE`).
    Init,
    /// Compressing (`BUSY_STATE`).
    Busy,
    /// The final block is written; only the trailer may be left
    /// (`FINISH_STATE`).
    Finished,
}

/// A deflate stream in progress: zlib-ng's `deflate_state`.
#[derive(Debug)]
pub(crate) struct Deflater {
    level: u8,
    strategy: Strategy,
    config: Config,
    wrap: Wrap,
    status: Status,
    /// Whether the trailer has been written (zlib's negated `wrap`).
    trailer_written: bool,
    win: Window,
    /// The start of the string being parsed (`strstart`).
    strstart: u32,
    /// Where the current block began; negative once slid out of the
    /// window (`block_start`).
    block_start: i64,
    /// The valid bytes from `strstart` on (`lookahead`).
    lookahead: u32,
    /// The start of the match `longest_match` found last
    /// (`match_start`).
    match_start: u32,
    /// The length of the match at the previous position, for lazy
    /// parsing (`prev_length`).
    prev_length: u32,
    /// The start of that match (`prev_match`).
    prev_match: u32,
    /// Whether the previous position awaits a decision (`match_available`).
    match_available: bool,
    /// The rolling hash of level 9 (`ins_h`).
    ins_h: u32,
    syms: Symbols,
    trees: Trees,
    /// Whether a fixed-code block of level 1 is open: 0 no, 1 yes, 2 the
    /// final one (`block_open`).
    block_open: u8,
    /// The size of zlib's pending buffer (`pending_buf_size`).
    pending_buf_size: usize,
    /// The Adler-32 of the input read so far.
    adler: Adler32,
}

impl Deflater {
    /// A stream compressing with `options` (zlib-ng's `deflateInit2` with
    /// a 32 KiB window).
    ///
    /// # Errors
    ///
    /// When the memory for the window, the hash chains or the symbol
    /// buffer cannot be allocated.
    pub(crate) fn new(options: Options, wrap: Wrap) -> Result<Self, OutOfMemory> {
        let syms = Symbols::new(options.memory_level.symbol_buffer())?;
        Ok(Self::with_tables(
            options,
            wrap,
            Window::new()?,
            syms,
            Trees::new(),
        ))
    }

    /// A stream compressing at most `len` bytes with `options`, made of
    /// this one's tables: the stream [`new`](Self::new) makes, without its
    /// allocations.
    ///
    /// # Errors
    ///
    /// When `options` asks for another memory level and the memory for
    /// its symbol buffer cannot be allocated.
    pub(crate) fn restart(
        self,
        options: Options,
        wrap: Wrap,
        len: usize,
    ) -> Result<Self, OutOfMemory> {
        let Self {
            mut win,
            syms,
            trees,
            ..
        } = self;
        win.restart(len);
        let symbol_buffer = options.memory_level.symbol_buffer();
        let syms = if syms.capacity() == symbol_buffer {
            syms.cleared()
        } else {
            Symbols::new(symbol_buffer)?
        };
        Ok(Self::with_tables(options, wrap, win, syms, trees.cleared()))
    }

    /// A stream at its start with these tables, which hold what a new
    /// stream's hold as far as it reads them.
    fn with_tables(options: Options, wrap: Wrap, win: Window, syms: Symbols, trees: Trees) -> Self {
        let symbol_buffer = syms.capacity();
        Self {
            level: options.level.get(),
            strategy: options.strategy,
            config: Config::of(options.level),
            wrap,
            status: Status::Init,
            trailer_written: false,
            win,
            strstart: 0,
            block_start: 0,
            lookahead: 0,
            match_start: 0,
            prev_length: 0,
            prev_match: 0,
            match_available: false,
            ins_h: 0,
            syms,
            trees,
            block_open: 0,
            pending_buf_size: 4 * symbol_buffer,
            adler: Adler32::new(),
        }
    }

    /// One call of zlib-ng's `deflate()`: compresses what it can of
    /// `input` (advancing it past what it took) into `out`, and returns
    /// whether the stream has ended (`Z_STREAM_END`).
    ///
    /// The caller gives `out` room (at least one byte) before each call,
    /// and passes either input or [`Flush::Finish`].
    pub(crate) fn deflate(
        &mut self,
        input: &mut &[u8],
        out: &mut Output<'_>,
        flush: Flush,
    ) -> bool {
        debug_assert!(out.room() > 0 && (!input.is_empty() || flush == Flush::Finish));
        if out.pending() != 0 {
            out.flush_pending();
            if out.room() == 0 {
                return false;
            }
        }
        if self.status == Status::Init {
            self.status = Status::Busy;
            if self.wrap == Wrap::Zlib {
                out.put_bytes(&self.header());
                out.flush_pending();
                if out.pending() != 0 {
                    return false;
                }
            }
        }
        let finishing = flush == Flush::Finish && self.status != Status::Finished;
        if !input.is_empty() || self.lookahead != 0 || finishing {
            let state = self.parse(input, out, flush);
            if state != BlockState::NeedMore {
                self.status = Status::Finished;
            }
            if state != BlockState::FinishDone {
                return false;
            }
        }
        if flush != Flush::Finish {
            return false;
        }
        if self.wrap == Wrap::Zlib && !self.trailer_written {
            out.put_bytes(&self.adler.finish().to_be_bytes());
        }
        self.trailer_written = true;
        out.flush_pending();
        out.pending() == 0
    }

    /// Runs the parser of the level and strategy (the dispatch of
    /// `deflate()`).
    fn parse(&mut self, input: &mut &[u8], out: &mut Output<'_>, flush: Flush) -> BlockState {
        match (self.config.method, self.strategy) {
            (Method::Stored, _) => self.deflate_stored(input, out, flush),
            (_, Strategy::HuffmanOnly) => self.deflate_huff(input, out, flush),
            (_, Strategy::Rle) => self.deflate_rle(input, out, flush),
            (Method::Quick, _) => self.deflate_quick(input, out, flush),
            (Method::Fast, _) => self.deflate_fast(input, out, flush),
            (Method::Medium, _) => self.deflate_medium(input, out, flush),
            (Method::Slow, _) => self.deflate_slow(input, out, flush),
        }
    }

    /// The two bytes of the zlib header: the method and window, then the
    /// level's `FLEVEL` with the check bits.
    fn header(&self) -> [u8; 2] {
        let level_flags: u16 = if self.strategy.is_fastest_kind() || self.level < 2 {
            0
        } else if self.level < 6 {
            1
        } else if self.level == 6 {
            2
        } else {
            3
        };
        let mut header = (8 + (7 << 4)) << 8 | level_flags << 6;
        header += 31 - header % 31;
        header.to_be_bytes()
    }

    /// Copies at most `room` bytes of `input` to the window at `at`,
    /// adding them to the checksum (`read_buf`), and returns how many.
    fn read_buf(&mut self, input: &mut &[u8], at: u32, room: u32) -> u32 {
        let count = input.len().min(room as usize);
        let (taken, rest) = input.split_at(count);
        self.adler.update(taken);
        self.win.bytes[at as usize..at as usize + count].copy_from_slice(taken);
        *input = rest;
        count as u32
    }

    /// Ends the current block as the cheapest of stored, fixed and
    /// dynamic, and hands the caller what it takes (`FLUSH_BLOCK_ONLY`).
    fn flush_block_only(&mut self, out: &mut Output<'_>, last: bool) {
        // The block's bytes, unless the window has moved past them.
        let stored = usize::try_from(self.block_start)
            .ok()
            .map(|start| &self.win.bytes[start..self.strstart as usize]);
        self.trees.flush_block(
            &mut self.syms,
            out,
            stored,
            last,
            self.strategy == Strategy::Fixed,
        );
        self.block_start = i64::from(self.strstart);
        out.flush_pending();
    }

    /// [`flush_block_only`](Self::flush_block_only), then whether the
    /// parser must stop because the caller's buffer is full
    /// (`FLUSH_BLOCK`).
    fn flush_block(&mut self, out: &mut Output<'_>, last: bool) -> Option<BlockState> {
        self.flush_block_only(out, last);
        (out.room() == 0).then_some(if last {
            BlockState::FinishStarted
        } else {
            BlockState::NeedMore
        })
    }

    /// The end of a parser's loop, which it leaves only when finishing:
    /// the final block (the common ending of `deflate_fast`,
    /// `deflate_medium`, ...).
    ///
    /// zlib-ng also records here how many bytes before `strstart` the
    /// next `fill_window` should hash (`s->insert`); a finished stream
    /// reads no more input, so that is left out, as everywhere else: the
    /// streams here never change level or take a dictionary, the only
    /// other ways to leave bytes unhashed.
    fn finish_blocks(&mut self, out: &mut Output<'_>, flush: Flush) -> BlockState {
        debug_assert_eq!(flush, Flush::Finish, "parsers stop early otherwise");
        self.flush_block(out, true)
            .unwrap_or(BlockState::FinishDone)
    }
}

/// The raw DEFLATE (or zlib) stream of `data`, written as zlib-ng's
/// `compress2` writes it: all the input at once, with all the room it
/// needs. The compressor is made of the tables `tables` holds, if any,
/// which hold its tables afterwards.
///
/// # Errors
///
/// When the memory for the compressor or the output cannot be allocated
/// (what was written of the output stays in `out`).
pub(crate) fn one_shot(
    data: &[u8],
    options: Options,
    wrap: Wrap,
    tables: &mut Option<Deflater>,
    out: &mut Vec<u8>,
) -> Result<(), OutOfMemory> {
    let mut deflater = match tables.take() {
        Some(used) => used.restart(options, wrap, data.len())?,
        None => Deflater::new(options, wrap)?,
    };
    reserve_usual(out, data.len(), options);
    let mut output = Output::new(out);
    let mut input = data;
    while !deflater.deflate(&mut input, &mut output, Flush::Finish) {}
    *tables = Some(deflater);
    output.finish()
}

/// Reserves room in `out` for the usual compression of `len` bytes with
/// `options`: all of them and the stored blocks' headers at level 0, a
/// quarter of them otherwise. The output grows further if the data
/// compresses less, so a refusal here is not yet a failure.
fn reserve_usual(out: &mut Vec<u8>, len: usize, options: Options) {
    let _ = out.try_reserve(if options.level == CompressionLevel::NONE {
        len + 5 * (len / 65_535 + 1) + 6
    } else {
        len / 4 + 64
    });
}
