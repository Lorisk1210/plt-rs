//! What Pillow reports about an animated GIF or PNG: the frame count, the
//! size, each frame's duration and the loop count (`n_frames`, `size`,
//! `info["duration"]` and `info["loop"]`).

use crate::json::Json;
use crate::probes::{ProbeError, encode};

/// An animation file as Pillow describes it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Movie {
    pub(super) size: (u32, u32),
    /// Each frame's duration in milliseconds.
    pub(super) durations: Vec<f64>,
    /// How often the animation repeats (0: forever), if the file says.
    pub(super) loops: Option<u32>,
}

impl Movie {
    /// `{frames, size, durations, loop}`.
    pub(super) fn to_json(&self) -> Json {
        let number = |n: u32| Json::from(f64::from(n));
        encode::object([
            ("frames", Json::from(self.durations.len() as f64)),
            (
                "size",
                Json::Array(vec![number(self.size.0), number(self.size.1)]),
            ),
            (
                "durations",
                Json::Array(self.durations.iter().map(|&d| Json::from(d)).collect()),
            ),
            ("loop", self.loops.map_or(Json::Null, number)),
        ])
    }
}

fn malformed(what: &str) -> ProbeError {
    ProbeError::BadArgs(format!("malformed {what} file"))
}

/// Reads bytes from the front of a file.
struct Reader<'a> {
    data: &'a [u8],
    what: &'static str,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ProbeError> {
        if self.data.len() < n {
            return Err(malformed(self.what));
        }
        let (head, rest) = self.data.split_at(n);
        self.data = rest;
        Ok(head)
    }

    fn byte(&mut self) -> Result<u8, ProbeError> {
        Ok(self.take(1)?[0])
    }

    /// The data sub-blocks of a GIF block, up to the terminator.
    fn sub_blocks(&mut self) -> Result<Vec<&'a [u8]>, ProbeError> {
        let mut blocks = Vec::new();
        loop {
            let len = usize::from(self.byte()?);
            if len == 0 {
                return Ok(blocks);
            }
            blocks.push(self.take(len)?);
        }
    }
}

fn u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn u32_be(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// A GIF's frames (image descriptors), the delays of their graphic
/// control extensions and the loop count of its NETSCAPE2.0 extension.
pub(super) fn gif(data: &[u8]) -> Result<Movie, ProbeError> {
    const GLOBAL_TABLE: u8 = 0x80;
    let mut reader = Reader { data, what: "GIF" };
    if !reader.take(6)?.starts_with(b"GIF8") {
        return Err(malformed("GIF"));
    }
    let screen = reader.take(7)?;
    let size = (
        u32::from(u16_le(&screen[0..2])),
        u32::from(u16_le(&screen[2..4])),
    );
    if screen[4] & GLOBAL_TABLE != 0 {
        reader.take(3 << ((screen[4] & 7) + 1))?;
    }
    let (mut durations, mut loops, mut delay) = (Vec::new(), None, 0.0);
    loop {
        match reader.byte()? {
            0x21 => {
                let label = reader.byte()?;
                let blocks = reader.sub_blocks()?;
                match (label, blocks.as_slice()) {
                    (0xF9, [control, ..]) if control.len() >= 3 => {
                        delay = f64::from(u16_le(&control[1..3])) * 10.0;
                    }
                    (0xFF, [name, settings, ..])
                        if *name == b"NETSCAPE2.0" && settings.len() >= 3 =>
                    {
                        loops = Some(u32::from(u16_le(&settings[1..3])));
                    }
                    _ => {}
                }
            }
            0x2C => {
                let descriptor = reader.take(9)?;
                if descriptor[8] & GLOBAL_TABLE != 0 {
                    reader.take(3 << ((descriptor[8] & 7) + 1))?;
                }
                reader.byte()?;
                reader.sub_blocks()?;
                durations.push(delay);
            }
            0x3B => break,
            _ => return Err(malformed("GIF")),
        }
    }
    Ok(Movie {
        size,
        durations,
        loops,
    })
}

/// An animated PNG's size, the delays of its frame controls and the play
/// count of its animation control.
pub(super) fn apng(data: &[u8]) -> Result<Movie, ProbeError> {
    let mut reader = Reader { data, what: "PNG" };
    if reader.take(8)? != b"\x89PNG\r\n\x1a\n" {
        return Err(malformed("PNG"));
    }
    let (mut size, mut durations, mut loops) = ((0, 0), Vec::new(), None);
    while !reader.data.is_empty() {
        let len = usize::try_from(u32_be(reader.take(4)?)).map_err(|_| malformed("PNG"))?;
        let kind = reader.take(4)?;
        let body = reader.take(len)?;
        reader.take(4)?;
        match kind {
            b"IHDR" if body.len() >= 8 => size = (u32_be(&body[0..4]), u32_be(&body[4..8])),
            b"acTL" if body.len() >= 8 => loops = Some(u32_be(&body[4..8])),
            b"fcTL" if body.len() >= 24 => {
                let numerator = f64::from(u16::from_be_bytes([body[20], body[21]]));
                let denominator = match u16::from_be_bytes([body[22], body[23]]) {
                    0 => 100.0,
                    d => f64::from(d),
                };
                durations.push(numerator / denominator * 1000.0);
            }
            _ => {}
        }
    }
    Ok(Movie {
        size,
        durations,
        loops,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gifs_and_animated_pngs_are_described_like_pillow() {
        let pixels = [0_u8, 0, 0, 255].repeat(6);
        let white = vec![255_u8; 24];
        let frames = [
            pltrs_codec::gif::GifFrame {
                pixels: &pixels,
                delay: 20,
            },
            pltrs_codec::gif::GifFrame {
                pixels: &white,
                delay: 20,
            },
        ];
        let bytes = pltrs_codec::gif::GifEncoder::new()
            .encode(3, 2, &frames)
            .unwrap();
        let movie = gif(&bytes).unwrap();
        assert_eq!(movie.size, (3, 2));
        assert_eq!(movie.durations, [200.0, 200.0]);
        assert_eq!(movie.loops, Some(0));
        assert!(gif(b"GIF89a").is_err());
        assert!(apng(b"not a png").is_err());
    }
}
