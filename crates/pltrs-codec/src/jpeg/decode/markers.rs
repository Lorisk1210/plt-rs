//! The marker structure of a JPEG file: finding markers as libjpeg's
//! `next_marker` does and parsing the segments that describe the image
//! (`jdmarker.c`).

use super::error::{DecodeError, TableKind};
use crate::jpeg::marker::{DQT, DRI, SOF2, SOI, SOS};
use crate::jpeg::tables::ZIGZAG;

/// Most components a frame may have (Pillow reads 1, 3 or 4).
const MAX_COMPONENTS: usize = 4;
/// Most components a scan may interleave.
const MAX_SCAN_COMPONENTS: usize = 4;

/// A position in the file, reading markers and their segments.
#[derive(Debug)]
pub(super) struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    /// A cursor after the start-of-image marker that must open `data`.
    pub(super) fn new(data: &'a [u8]) -> Result<Self, DecodeError> {
        match data {
            [0xFF, SOI, 0xFF, ..] => Ok(Self { data, pos: 2 }),
            _ => Err(DecodeError::NotJpeg),
        }
    }

    /// A cursor at `pos` of `data`, which has been checked before.
    pub(super) fn at(data: &'a [u8], pos: usize) -> Self {
        Self {
            data,
            pos: pos.min(data.len()),
        }
    }

    /// The offset of the next unread byte.
    pub(super) fn position(&self) -> usize {
        self.pos
    }

    /// Continues reading at `pos`.
    pub(super) fn seek(&mut self, pos: usize) {
        self.pos = pos.min(self.data.len());
    }

    fn byte(&mut self) -> Result<u8, DecodeError> {
        let byte = *self.data.get(self.pos).ok_or(DecodeError::Truncated)?;
        self.pos += 1;
        Ok(byte)
    }

    /// The next marker code: bytes before an `0xFF` are skipped, as are
    /// fill bytes (repeated `0xFF`) and stuffed `0xFF 0x00` pairs.
    pub(super) fn next_marker(&mut self) -> Result<u8, DecodeError> {
        loop {
            while self.byte()? != 0xFF {}
            let mut code = self.byte()?;
            while code == 0xFF {
                code = self.byte()?;
            }
            if code != 0 {
                return Ok(code);
            }
        }
    }

    /// The payload of the segment that follows a marker (after its 16-bit
    /// length, which counts itself).
    pub(super) fn segment(&mut self, marker: u8) -> Result<&'a [u8], DecodeError> {
        let length = usize::from(u16::from_be_bytes([self.byte()?, self.byte()?]));
        let payload_len = length
            .checked_sub(2)
            .ok_or(DecodeError::MalformedSegment { marker })?;
        let payload = self
            .data
            .get(self.pos..self.pos + payload_len)
            .ok_or(DecodeError::Truncated)?;
        self.pos += payload_len;
        Ok(payload)
    }
}

/// A component as the frame header declares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ComponentSpec {
    /// The identifier scans refer to it by.
    pub(super) id: u8,
    /// Horizontal sampling factor, 1–4.
    pub(super) h: usize,
    /// Vertical sampling factor, 1–4.
    pub(super) v: usize,
    /// The quantization table number.
    pub(super) quant_table: u8,
}

/// The start-of-frame segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FrameHeader {
    /// Whether the frame is progressive (`SOF2`) rather than sequential.
    pub(super) progressive: bool,
    /// The image width.
    pub(super) width: u16,
    /// The image height.
    pub(super) height: u16,
    /// The components, in frame order.
    pub(super) components: Vec<ComponentSpec>,
}

impl FrameHeader {
    /// Parses the segment of a supported start-of-frame marker.
    pub(super) fn parse(marker: u8, payload: &[u8]) -> Result<Self, DecodeError> {
        let malformed = DecodeError::MalformedSegment { marker };
        let [precision, h0, h1, w0, w1, count, specs @ ..] = payload else {
            return Err(malformed);
        };
        if *precision != 8 {
            return Err(DecodeError::UnsupportedPrecision(*precision));
        }
        if !matches!(count, 1 | 3 | 4) {
            return Err(DecodeError::UnsupportedComponents(*count));
        }
        let (width, height) = (
            u16::from_be_bytes([*w0, *w1]),
            u16::from_be_bytes([*h0, *h1]),
        );
        if width == 0 || height == 0 {
            return Err(DecodeError::EmptyImage);
        }
        if specs.len() != usize::from(*count) * 3 {
            return Err(malformed);
        }
        let components = specs
            .as_chunks::<3>()
            .0
            .iter()
            .map(|spec| ComponentSpec {
                id: spec[0],
                h: usize::from(spec[1] >> 4),
                v: usize::from(spec[1] & 15),
                quant_table: spec[2],
            })
            .collect::<Vec<_>>();
        if components
            .iter()
            .any(|c| !(1..=4).contains(&c.h) || !(1..=4).contains(&c.v))
        {
            return Err(DecodeError::InvalidSampling);
        }
        debug_assert!(components.len() <= MAX_COMPONENTS);
        Ok(Self {
            progressive: marker == SOF2,
            width,
            height,
            components,
        })
    }
}

/// A component of a scan: its index in the frame and its tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ScanComponent {
    /// The component's position in the frame.
    pub(super) index: usize,
    /// The DC Huffman table number (0–15 as written).
    pub(super) dc_table: u8,
    /// The AC Huffman table number (0–15 as written).
    pub(super) ac_table: u8,
}

/// The start-of-scan segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ScanHeader {
    /// The components coded in the scan, in scan order.
    pub(super) components: Vec<ScanComponent>,
    /// The first coefficient of the spectral band (zigzag index).
    pub(super) ss: u8,
    /// The last coefficient of the spectral band.
    pub(super) se: u8,
    /// The previous successive approximation bit position (0 for a first
    /// scan).
    pub(super) ah: u8,
    /// The successive approximation bit position.
    pub(super) al: u8,
}

impl ScanHeader {
    /// Parses a scan header against the frame's components, matching
    /// libjpeg's `get_sos`: each selector names the first not yet chosen
    /// component of the frame (among its first four) with that identifier.
    pub(super) fn parse(payload: &[u8], frame: &[ComponentSpec]) -> Result<Self, DecodeError> {
        let malformed = DecodeError::MalformedSegment { marker: SOS };
        let [count, rest @ ..] = payload else {
            return Err(malformed);
        };
        let count = usize::from(*count);
        if !(1..=MAX_SCAN_COMPONENTS).contains(&count) || rest.len() != count * 2 + 3 {
            return Err(malformed);
        }
        let (selectors, params) = rest.split_at(count * 2);
        let mut components: Vec<ScanComponent> = Vec::with_capacity(count);
        for selector in selectors.as_chunks::<2>().0 {
            let index = frame
                .iter()
                .take(MAX_SCAN_COMPONENTS)
                .enumerate()
                .position(|(index, spec)| {
                    spec.id == selector[0] && components.iter().all(|c| c.index != index)
                })
                .ok_or(DecodeError::InvalidScan)?;
            components.push(ScanComponent {
                index,
                dc_table: selector[1] >> 4,
                ac_table: selector[1] & 15,
            });
        }
        Ok(Self {
            components,
            ss: params[0],
            se: params[1],
            ah: params[2] >> 4,
            al: params[2] & 15,
        })
    }
}

/// The four quantization table slots, in natural (row-major) order.
#[derive(Clone, Debug, Default)]
pub(super) struct QuantTables([Option<[u16; 64]>; 4]);

impl QuantTables {
    /// Stores every table of a DQT segment. Tables must be complete, as
    /// Pillow requires.
    pub(super) fn read(&mut self, payload: &[u8]) -> Result<(), DecodeError> {
        let malformed = || DecodeError::MalformedSegment { marker: DQT };
        let mut rest = payload;
        while let [spec, tail @ ..] = rest {
            let (precision, slot) = (spec >> 4, usize::from(spec & 15));
            let width = if precision == 0 { 1 } else { 2 };
            let table = self.0.get_mut(slot).ok_or_else(malformed)?;
            let values = tail.get(..64 * width).ok_or_else(malformed)?;
            let mut natural = [0_u16; 64];
            for (k, value) in values.chunks_exact(width).enumerate() {
                natural[ZIGZAG[k]] = match *value {
                    [byte] => u16::from(byte),
                    [high, low] => u16::from_be_bytes([high, low]),
                    _ => unreachable!("chunks of one or two bytes"),
                };
            }
            *table = Some(natural);
            rest = &tail[64 * width..];
        }
        Ok(())
    }

    /// Table `index`, which must have been defined.
    pub(super) fn get(&self, index: u8) -> Result<[u16; 64], DecodeError> {
        self.0
            .get(usize::from(index))
            .copied()
            .flatten()
            .ok_or(DecodeError::MissingTable {
                kind: TableKind::Quantization,
                index,
            })
    }
}

/// The restart interval of a DRI segment, in MCUs (0 disables restarts).
pub(super) fn restart_interval(payload: &[u8]) -> Result<u16, DecodeError> {
    match payload {
        [high, low] => Ok(u16::from_be_bytes([*high, *low])),
        _ => Err(DecodeError::MalformedSegment { marker: DRI }),
    }
}

/// Whether an APP0 payload is a JFIF header (libjpeg's `examine_app0`).
pub(super) fn is_jfif(payload: &[u8]) -> bool {
    payload.len() >= 14 && payload.starts_with(b"JFIF\0")
}

/// The colour transform of an Adobe APP14 payload (libjpeg's
/// `examine_app14`), if it is one.
pub(super) fn adobe_transform(payload: &[u8]) -> Option<u8> {
    (payload.len() >= 12 && payload.starts_with(b"Adobe")).then(|| payload[11])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpeg::marker::{EOI, SOF0};

    #[test]
    fn markers_skip_garbage_fill_and_stuffing() {
        let data = [
            0xFF, SOI, 0xFF, 0xFF, 0xE0, 1, 2, 0xFF, 0x00, 0xFF, 0xFF, EOI,
        ];
        let mut cursor = Cursor::new(&data).unwrap();
        assert_eq!(cursor.next_marker(), Ok(0xE0));
        assert_eq!(cursor.next_marker(), Ok(EOI));
        assert_eq!(cursor.next_marker(), Err(DecodeError::Truncated));
        assert_eq!(Cursor::new(b"\xff\xd8").err(), Some(DecodeError::NotJpeg));
    }

    #[test]
    fn segments_are_length_checked() {
        let data = [0xFF, SOI, 0xFF, 0x00, 0x04, 9, 9, 0x00, 0x01];
        let mut cursor = Cursor::new(&data).unwrap();
        cursor.seek(3);
        assert_eq!(cursor.segment(DQT), Ok(&[9, 9][..]));
        assert_eq!(
            cursor.segment(DQT),
            Err(DecodeError::MalformedSegment { marker: DQT })
        );
        let truncated = [0xFF, SOI, 0xFF, 0x00, 0x09, 1];
        let mut short = Cursor::new(&truncated).unwrap();
        short.seek(3);
        assert_eq!(short.segment(DQT), Err(DecodeError::Truncated));
    }

    #[test]
    fn frames_are_validated() {
        let frame = [8, 0, 9, 0, 17, 3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1];
        let header = FrameHeader::parse(SOF0, &frame).unwrap();
        assert_eq!((header.width, header.height), (17, 9));
        assert_eq!(header.components[0].h, 2);
        let mut bad = frame;
        bad[0] = 12;
        assert_eq!(
            FrameHeader::parse(SOF0, &bad),
            Err(DecodeError::UnsupportedPrecision(12))
        );
        bad = frame;
        bad[7] = 0x51;
        assert_eq!(
            FrameHeader::parse(SOF0, &bad),
            Err(DecodeError::InvalidSampling)
        );
        bad = frame;
        bad[3] = 0;
        bad[4] = 0;
        assert_eq!(FrameHeader::parse(SOF0, &bad), Err(DecodeError::EmptyImage));
        assert!(FrameHeader::parse(SOF0, &frame[..14]).is_err());
    }

    #[test]
    fn scans_pick_unused_components() {
        let frame = FrameHeader::parse(
            SOF0,
            &[8, 0, 1, 0, 1, 3, 1, 0x11, 0, 1, 0x11, 1, 3, 0x11, 1],
        )
        .unwrap();
        let scan = ScanHeader::parse(&[2, 1, 0x00, 1, 0x11, 0, 63, 0], &frame.components).unwrap();
        assert_eq!(scan.components[0].index, 0);
        assert_eq!(
            scan.components[1].index, 1,
            "the second component with id 1"
        );
        assert_eq!(
            ScanHeader::parse(&[1, 9, 0x00, 0, 63, 0], &frame.components),
            Err(DecodeError::InvalidScan)
        );
    }

    #[test]
    fn quantization_tables_are_dezigzagged() {
        let mut payload = vec![0x01];
        payload.extend(1..=64_u8);
        let mut tables = QuantTables::default();
        tables.read(&payload).unwrap();
        let table = tables.get(1).unwrap();
        assert_eq!((table[0], table[1], table[8]), (1, 2, 3));
        assert!(tables.get(0).is_err());
        assert!(tables.read(&payload[..40]).is_err());
        assert!(tables.read(&[0x05]).is_err());
    }
}
