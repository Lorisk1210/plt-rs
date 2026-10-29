//! The RIFF container of WebP files, read as libwebp's demuxer reads it
//! for `WebPAnimDecoder`: simple files holding one bitstream, extended
//! files (`VP8X`) with alpha and metadata chunks, and animations, of which
//! the first frame is wanted.

use super::{DecodeError, lossless, lossy};

/// The `VP8X` flag of images with alpha.
const ALPHA_FLAG: u8 = 0x10;
/// The `VP8X` flag of animations.
const ANIMATION_FLAG: u8 = 0x02;
/// The size of a `VP8X` payload.
const VP8X_SIZE: usize = 10;
/// The size of the fields of an `ANMF` payload, before its frame.
const ANMF_HEADER_SIZE: usize = 16;

/// A chunk: its four-character code and payload.
#[derive(Clone, Copy, Debug)]
struct Chunk<'a> {
    fourcc: [u8; 4],
    payload: &'a [u8],
}

/// The chunks of a RIFF body, each padded to an even size.
#[derive(Debug)]
struct Chunks<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for Chunks<'a> {
    type Item = Result<Chunk<'a>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let Some((header, rest)) = self.rest.split_first_chunk::<8>() else {
            self.rest = &[];
            return Some(Err(DecodeError::Truncated));
        };
        let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        let padded = size + (size & 1);
        if padded > rest.len() {
            self.rest = &[];
            return Some(Err(DecodeError::Truncated));
        }
        self.rest = &rest[padded..];
        Some(Ok(Chunk {
            fourcc: [header[0], header[1], header[2], header[3]],
            payload: &rest[..size],
        }))
    }
}

/// The bitstream of a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bitstream<'a> {
    /// A VP8 key frame, with the payload of its `ALPH` chunk if any.
    Lossy {
        vp8: &'a [u8],
        alpha: Option<&'a [u8]>,
    },
    /// A VP8L image.
    Lossless(&'a [u8]),
}

/// A frame placed on the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Frame<'a> {
    /// The frame's offset on the canvas.
    pub(super) x: u32,
    pub(super) y: u32,
    /// The frame's size.
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) bitstream: Bitstream<'a>,
}

/// What decoding a file needs: the canvas, whether Pillow opens it with
/// alpha, and the first frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Container<'a> {
    pub(super) width: u32,
    pub(super) height: u32,
    /// Whether the file declares alpha (libwebp's `WebPGetFeatures`).
    pub(super) has_alpha: bool,
    pub(super) frame: Frame<'a>,
}

/// Appends a chunk with its padding.
fn push_chunk(out: &mut Vec<u8>, fourcc: [u8; 4], payload: &[u8]) {
    out.extend(fourcc);
    out.extend((payload.len() as u32).to_le_bytes());
    out.extend(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
}

/// A RIFF `WEBP` file around `body`.
fn riff(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + body.len());
    out.extend(b"RIFF");
    out.extend((4 + body.len() as u32).to_le_bytes());
    out.extend(b"WEBP");
    out.extend(body);
    out
}

/// The low three bytes of `value`, little-endian.
fn le24(value: u32) -> [u8; 3] {
    let [a, b, c, _] = value.to_le_bytes();
    [a, b, c]
}

/// A simple WebP file holding the bitstream `payload` in a `fourcc` chunk.
pub(super) fn write(fourcc: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(9 + payload.len());
    push_chunk(&mut body, fourcc, payload);
    riff(&body)
}

/// An animation of full-canvas VP8L `frames` (bitstream and duration in
/// milliseconds) on a `width × height` canvas, played `loops` times
/// (0: forever); frames replace the canvas without blending.
pub(super) fn write_animation(
    width: u32,
    height: u32,
    uses_alpha: bool,
    loops: u16,
    frames: &[(Vec<u8>, u32)],
) -> Vec<u8> {
    const DO_NOT_BLEND: u8 = 0x02;
    let mut body = Vec::new();
    let flags = ANIMATION_FLAG | if uses_alpha { ALPHA_FLAG } else { 0 };
    let mut vp8x = vec![flags, 0, 0, 0];
    vp8x.extend(le24(width - 1));
    vp8x.extend(le24(height - 1));
    push_chunk(&mut body, *b"VP8X", &vp8x);
    // A transparent background, and the loop count.
    let mut anim = vec![0; 4];
    anim.extend(loops.to_le_bytes());
    push_chunk(&mut body, *b"ANIM", &anim);
    for (bitstream, duration) in frames {
        let mut anmf = Vec::with_capacity(24 + bitstream.len());
        anmf.extend(le24(0));
        anmf.extend(le24(0));
        anmf.extend(le24(width - 1));
        anmf.extend(le24(height - 1));
        anmf.extend(le24((*duration).min(0xFF_FFFF)));
        anmf.push(DO_NOT_BLEND);
        push_chunk(&mut anmf, *b"VP8L", bitstream);
        push_chunk(&mut body, *b"ANMF", &anmf);
    }
    riff(&body)
}

/// Parses a WebP file.
pub(super) fn parse(data: &[u8]) -> Result<Container<'_>, DecodeError> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return Err(DecodeError::NotWebp);
    }
    let riff_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    if riff_size < 12 {
        return Err(DecodeError::MalformedChunk { fourcc: *b"RIFF" });
    }
    // Data after the RIFF chunk is ignored.
    let body = data.get(12..8 + riff_size).ok_or(DecodeError::Truncated)?;
    let mut chunks = Chunks { rest: body };
    let first = chunks.next().ok_or(DecodeError::NoImage)??;
    match &first.fourcc {
        b"VP8 " | b"VP8L" => {
            let bitstream = image_bitstream(first, None)?.ok_or(DecodeError::NoImage)?;
            let (width, height, has_alpha) = bitstream_info(bitstream)?;
            Ok(Container {
                width,
                height,
                has_alpha,
                frame: whole_canvas(width, height, bitstream),
            })
        }
        b"VP8X" => extended(first.payload, chunks),
        _ => Err(DecodeError::NoImage),
    }
}

/// A frame covering a `width × height` canvas.
fn whole_canvas(width: u32, height: u32, bitstream: Bitstream<'_>) -> Frame<'_> {
    Frame {
        x: 0,
        y: 0,
        width,
        height,
        bitstream,
    }
}

/// The size and alpha of a bitstream: VP8L headers say whether alpha is
/// used, VP8 frames have alpha when an `ALPH` chunk comes with them.
fn bitstream_info(bitstream: Bitstream<'_>) -> Result<(u32, u32, bool), DecodeError> {
    match bitstream {
        Bitstream::Lossless(data) => lossless::read_info(data),
        Bitstream::Lossy { vp8, alpha } => {
            let (width, height) = lossy::read_info(vp8)?;
            Ok((width, height, alpha.is_some()))
        }
    }
}

/// The bitstream of an image chunk (`alpha` being the `ALPH` payload met
/// before it), or `None` for any other chunk.
fn image_bitstream<'a>(
    chunk: Chunk<'a>,
    alpha: Option<&'a [u8]>,
) -> Result<Option<Bitstream<'a>>, DecodeError> {
    Ok(match &chunk.fourcc {
        b"VP8 " => Some(Bitstream::Lossy {
            vp8: chunk.payload,
            alpha,
        }),
        // VP8L carries its own alpha: an ALPH chunk before it is invalid.
        b"VP8L" if alpha.is_some() => return Err(DecodeError::MalformedChunk { fourcc: *b"ALPH" }),
        b"VP8L" => Some(Bitstream::Lossless(chunk.payload)),
        _ => None,
    })
}

/// The first image among `chunks`, with the `ALPH` chunk before it, if
/// any; unknown chunks are skipped.
fn find_image<'a>(
    chunks: impl Iterator<Item = Result<Chunk<'a>, DecodeError>>,
) -> Result<Bitstream<'a>, DecodeError> {
    let mut alpha = None;
    for chunk in chunks {
        let chunk = chunk?;
        if &chunk.fourcc == b"ALPH" {
            alpha = Some(chunk.payload);
        } else if let Some(bitstream) = image_bitstream(chunk, alpha)? {
            return Ok(bitstream);
        }
    }
    Err(DecodeError::NoImage)
}

/// A 24-bit little-endian field.
fn u24(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0])
}

/// Parses an extended file from its `VP8X` payload and the chunks after.
fn extended<'a>(header: &[u8], mut chunks: Chunks<'a>) -> Result<Container<'a>, DecodeError> {
    let malformed = |fourcc: &[u8; 4]| DecodeError::MalformedChunk { fourcc: *fourcc };
    if header.len() != VP8X_SIZE {
        return Err(malformed(b"VP8X"));
    }
    let flags = header[0];
    let width = 1 + u24(&header[4..7]);
    let height = 1 + u24(&header[7..10]);
    if flags & ANIMATION_FLAG != 0 {
        let frame = loop {
            let chunk = chunks.next().ok_or(DecodeError::NoImage)??;
            if &chunk.fourcc == b"ANMF" {
                break animation_frame(chunk.payload)?;
            }
        };
        let fits = u64::from(frame.x) + u64::from(frame.width) <= u64::from(width)
            && u64::from(frame.y) + u64::from(frame.height) <= u64::from(height);
        if !fits {
            return Err(malformed(b"ANMF"));
        }
        return Ok(Container {
            width,
            height,
            has_alpha: flags & ALPHA_FLAG != 0,
            frame,
        });
    }
    let bitstream = find_image(chunks)?;
    let (image_width, image_height, bitstream_alpha) = bitstream_info(bitstream)?;
    if (image_width, image_height) != (width, height) {
        return Err(malformed(b"VP8X"));
    }
    let has_alpha = match bitstream {
        Bitstream::Lossless(_) => bitstream_alpha,
        Bitstream::Lossy { .. } => bitstream_alpha || flags & ALPHA_FLAG != 0,
    };
    Ok(Container {
        width,
        height,
        has_alpha,
        frame: whole_canvas(width, height, bitstream),
    })
}

/// Parses an `ANMF` payload: the frame's offset (in units of two pixels)
/// and size, its duration and flags, then its image chunks.
fn animation_frame(payload: &[u8]) -> Result<Frame<'_>, DecodeError> {
    let malformed = DecodeError::MalformedChunk { fourcc: *b"ANMF" };
    let (fields, image) = payload
        .split_at_checked(ANMF_HEADER_SIZE)
        .ok_or(malformed.clone())?;
    let bitstream = find_image(Chunks { rest: image })?;
    let (width, height, _) = bitstream_info(bitstream)?;
    if (width, height) != (1 + u24(&fields[6..9]), 1 + u24(&fields[9..12])) {
        return Err(malformed);
    }
    Ok(Frame {
        x: 2 * u24(&fields[0..3]),
        y: 2 * u24(&fields[3..6]),
        width,
        height,
        bitstream,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A chunk with its padding.
    fn chunk(fourcc: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = fourcc.to_vec();
        out.extend((payload.len() as u32).to_le_bytes());
        out.extend(payload);
        if payload.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    /// A RIFF file of `chunks`.
    fn riff(chunks: &[Vec<u8>]) -> Vec<u8> {
        let body: Vec<u8> = chunks.concat();
        let mut out = b"RIFF".to_vec();
        out.extend((body.len() as u32 + 4).to_le_bytes());
        out.extend(b"WEBP");
        out.extend(body);
        out
    }

    /// A VP8L header of a `width × height` image with the alpha hint.
    fn vp8l(width: u32, height: u32, alpha: bool) -> Vec<u8> {
        let bits = u64::from(width - 1) | (u64::from(height - 1) << 14) | (u64::from(alpha) << 28);
        let mut out = vec![0x2F];
        out.extend(&bits.to_le_bytes()[..4]);
        out
    }

    fn vp8x(flags: u8, width: u32, height: u32) -> Vec<u8> {
        let mut out = vec![flags, 0, 0, 0];
        out.extend(&(width - 1).to_le_bytes()[..3]);
        out.extend(&(height - 1).to_le_bytes()[..3]);
        out
    }

    #[test]
    fn simple_lossless_files_take_their_alpha_hint() {
        let file = riff(&[chunk(*b"VP8L", &vp8l(3, 2, true))]);
        let container = parse(&file).unwrap();
        assert_eq!(
            (container.width, container.height, container.has_alpha),
            (3, 2, true)
        );
        let mut trailing = file.clone();
        trailing.extend(b"ignored");
        assert_eq!(parse(&trailing), Ok(container));
        assert_eq!(parse(&file[..file.len() - 1]), Err(DecodeError::Truncated));
        assert_eq!(parse(b"RIFF\x04\0\0\0WEBQ"), Err(DecodeError::NotWebp));
    }

    #[test]
    fn extended_files_need_matching_sizes_and_no_alpha_chunk_with_vp8l() {
        let image = chunk(*b"VP8L", &vp8l(3, 2, false));
        let file = riff(&[
            chunk(*b"VP8X", &vp8x(ALPHA_FLAG, 3, 2)),
            chunk(*b"ICCP", b"x"),
            image.clone(),
        ]);
        let container = parse(&file).unwrap();
        assert!(
            !container.has_alpha,
            "the VP8L hint overrides the VP8X flag"
        );
        let wrong = riff(&[chunk(*b"VP8X", &vp8x(0, 4, 2)), image.clone()]);
        assert_eq!(
            parse(&wrong),
            Err(DecodeError::MalformedChunk { fourcc: *b"VP8X" })
        );
        let with_alpha = riff(&[
            chunk(*b"VP8X", &vp8x(0, 3, 2)),
            chunk(*b"ALPH", b"\0"),
            image,
        ]);
        assert_eq!(
            parse(&with_alpha),
            Err(DecodeError::MalformedChunk { fourcc: *b"ALPH" })
        );
    }

    #[test]
    fn animations_yield_their_first_frame_in_place() {
        let mut anmf = vec![1, 0, 0, 2, 0, 0];
        anmf.extend(&[2, 0, 0, 1, 0, 0]);
        anmf.extend([0; 4]);
        anmf.extend(chunk(*b"VP8L", &vp8l(3, 2, true)));
        let file = riff(&[
            chunk(*b"VP8X", &vp8x(ANIMATION_FLAG | ALPHA_FLAG, 8, 8)),
            chunk(*b"ANIM", &[0; 6]),
            chunk(*b"ANMF", &anmf),
        ]);
        let container = parse(&file).unwrap();
        assert!(container.has_alpha);
        let frame = container.frame;
        assert_eq!((frame.x, frame.y, frame.width, frame.height), (2, 4, 3, 2));
        let small = riff(&[
            chunk(*b"VP8X", &vp8x(ANIMATION_FLAG, 4, 4)),
            chunk(*b"ANMF", &anmf),
        ]);
        assert_eq!(
            parse(&small),
            Err(DecodeError::MalformedChunk { fourcc: *b"ANMF" })
        );
    }
}
