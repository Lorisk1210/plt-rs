//! Compressing rows as Pillow's PNG encoder does (`ZipEncode.c` and
//! `ImageFile._save`): each row handed to `deflate()` on its own, without
//! flushing, into output buffers of a fixed size; each buffer becomes one
//! `IDAT` chunk.

use pltrs_core::alloc::{self, OutOfMemory};

use super::{Deflater, Flush, Options, Output, Wrap, reserve_usual};

/// Rows compressed one at a time: the filtered scanlines of an image.
pub(crate) trait RowSource {
    /// The number of rows.
    fn rows(&self) -> usize;

    /// Row `index`, as it is compressed (asked for again while its bytes
    /// are being compressed). Every row has the length of the first.
    fn row(&mut self, index: usize) -> &[u8];
}

/// A zlib stream and the pieces it was delivered in.
#[derive(Debug)]
pub(crate) struct Rows {
    /// The stream.
    pub(crate) stream: Vec<u8>,
    /// Where each output buffer's bytes end in `stream`.
    pub(crate) ends: Vec<usize>,
}

/// The zlib stream of `source`'s rows as Pillow writes it with zlib-ng:
/// rows fed one at a time into output buffers of `buffer` bytes.
///
/// # Errors
///
/// When the memory for the compressor or the stream cannot be allocated.
pub(crate) fn rows(
    source: &mut impl RowSource,
    options: Options,
    buffer: usize,
) -> Result<Rows, OutOfMemory> {
    let mut deflater = Deflater::new(options, Wrap::Zlib)?;
    let mut stream = Vec::new();
    let rows = source.rows();
    if rows > 0 {
        reserve_usual(&mut stream, rows * row_len(source, 0), options);
    }
    let mut ends = Vec::new();
    let mut out = Output::new(&mut stream);
    // The row being compressed and how much of it has been taken.
    let mut current: Option<(usize, usize)> = None;
    let mut next_row = 0;
    let mut finishing = false;
    let mut ended = false;
    while !ended {
        // One call of Pillow's `ImagingZipEncode` with a fresh buffer.
        out.give_room(buffer);
        if let Some((index, taken)) =
            current.filter(|&(index, taken)| taken < row_len(source, index))
        {
            current = Some((
                index,
                taken + feed(&mut deflater, source, index, taken, &mut out),
            ));
        }
        while !finishing && out.room() > 0 {
            if next_row == source.rows() {
                finishing = true;
                break;
            }
            let taken = feed(&mut deflater, source, next_row, 0, &mut out);
            current = Some((next_row, taken));
            next_row += 1;
        }
        while finishing && out.room() > 0 {
            if deflater.deflate(&mut &[][..], &mut out, Flush::Finish) {
                ended = true;
                break;
            }
        }
        alloc::try_reserve(&mut ends, 1)?;
        ends.push(out.delivered());
    }
    out.finish()?;
    Ok(Rows { stream, ends })
}

/// The length of row `index`.
fn row_len(source: &mut impl RowSource, index: usize) -> usize {
    source.row(index).len()
}

/// Hands the bytes of row `index` from `start` on to the compressor and
/// returns how many it took.
fn feed(
    deflater: &mut Deflater,
    source: &mut impl RowSource,
    index: usize,
    start: usize,
    out: &mut Output<'_>,
) -> usize {
    let row = &source.row(index)[start..];
    if row.is_empty() {
        return 0;
    }
    let mut input = row;
    deflater.deflate(&mut input, out, Flush::None);
    debug_assert!(
        input.is_empty() || out.room() == 0,
        "input left only for want of room"
    );
    row.len() - input.len()
}
