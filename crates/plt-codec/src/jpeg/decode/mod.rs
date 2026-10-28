//! JPEG decoding as Pillow does it with libjpeg-turbo's defaults.
//!
//! The file is read marker by marker (`markers`); each scan's
//! entropy-coded data (`bits`, `huffman`) adds to whole-image buffers of
//! quantized coefficients (`frame`), sequentially (`sequential`) or
//! progressively (`progressive`). Once all scans are in, the blocks of a
//! progressive image that lacks refinements are smoothed (`smooth`), every
//! block goes through the accurate integer inverse DCT (`idct`),
//! subsampled components are brought to full size (`upsample`) and the
//! colours are converted (`color`). Each step is a port of the
//! libjpeg-turbo routine Pillow runs, so the pixels are identical.

mod bits;
mod color;
mod error;
mod frame;
mod huffman;
mod idct;
mod markers;
mod progressive;
mod scan;
mod sequential;
mod smooth;
mod upsample;

#[cfg(test)]
mod tests;

pub use error::{DecodeError, TableKind};

use self::color::{ColorHints, ColorSpace};
use self::frame::Frame;
use self::huffman::HuffmanTables;
use self::markers::{Cursor, FrameHeader, QuantTables, ScanHeader};
use self::scan::{Tables, decode_scan};
use crate::jpeg::marker::{
    APP0, APP14, COM, DHT, DNL, DQT, DRI, EOI, RST0, SOF0, SOF1, SOF2, SOI, SOS, TEM,
};
use crate::raster::{Pixels, Raster};

/// Decodes a JPEG file into the image Pillow opens: mode `L` for one
/// component, `RGB` for three (converted from YCbCr unless the file is
/// coded in RGB) and `CMYK` for four (Adobe's inverted inks un-inverted,
/// as Pillow does).
///
/// Baseline, extended and progressive Huffman-coded files are read, with
/// any sampling factors and restart intervals.
///
/// # Errors
///
/// When the data is not a complete, valid JPEG file of a supported kind,
/// or claims more pixels than its size allows.
pub fn decode(data: &[u8]) -> Result<Raster, DecodeError> {
    let (frame, space) = read(data)?;
    Ok(raster(&frame, space))
}

/// The tables and settings defined so far.
#[derive(Debug, Default)]
struct Definitions {
    huffman: HuffmanTables,
    quant: QuantTables,
    restart_interval: u16,
    hints: ColorHints,
}

/// Reads every marker and scan: the coefficients of the frame and its
/// colour space.
fn read(data: &[u8]) -> Result<(Frame, ColorSpace), DecodeError> {
    let mut cursor = Cursor::new(data)?;
    let mut definitions = Definitions::default();
    let mut frame: Option<Frame> = None;
    let mut space = None;
    loop {
        let marker = cursor.next_marker()?;
        match marker {
            SOF0 | SOF1 | SOF2 => {
                if frame.is_some() {
                    return Err(DecodeError::DuplicateMarker(marker));
                }
                let header = FrameHeader::parse(marker, cursor.segment(marker)?)?;
                frame = Some(Frame::new(&header)?);
            }
            0xC3 | 0xC5..=0xCB | 0xCD..=0xCF => {
                return Err(DecodeError::UnsupportedProcess { marker });
            }
            SOS => {
                let frame = frame.as_mut().ok_or(DecodeError::MissingFrame)?;
                let specs: Vec<_> = frame.components.iter().map(|c| c.spec).collect();
                let header = ScanHeader::parse(cursor.segment(SOS)?, &specs)?;
                let first = space.is_none();
                let ids: Vec<u8> = specs.iter().map(|spec| spec.id).collect();
                space.get_or_insert(ColorSpace::guess(&ids, definitions.hints));
                let tables = Tables {
                    huffman: &definitions.huffman,
                    quant: &definitions.quant,
                    restart_interval: definitions.restart_interval,
                };
                let end = decode_scan(frame, &header, tables, data, cursor.position())?;
                cursor.seek(end);
                // libjpeg reads a sequential file whose first scan has
                // every component as a single pass, ignoring what follows.
                if first && !frame.progressive && header.components.len() == specs.len() {
                    break;
                }
            }
            EOI if space.is_some() => break,
            EOI => return Err(DecodeError::MissingFrame),
            DHT => definitions.huffman.read(cursor.segment(marker)?)?,
            DQT => definitions.quant.read(cursor.segment(marker)?)?,
            DRI => definitions.restart_interval = markers::restart_interval(cursor.segment(DRI)?)?,
            APP0 => {
                let payload = cursor.segment(marker)?;
                definitions.hints.jfif |= markers::is_jfif(payload);
            }
            APP14 => {
                let payload = cursor.segment(marker)?;
                if let Some(transform) = markers::adobe_transform(payload) {
                    definitions.hints.adobe = Some(transform);
                }
            }
            0xE1..=0xEF | COM | DNL | 0xCC => {
                cursor.segment(marker)?;
            }
            TEM => {}
            _ if (RST0..RST0 + 8).contains(&marker) => {}
            SOI => return Err(DecodeError::DuplicateMarker(SOI)),
            _ => return Err(DecodeError::UnknownMarker(marker)),
        }
    }
    let frame = frame.ok_or(DecodeError::MissingFrame)?;
    Ok((frame, space.ok_or(DecodeError::MissingFrame)?))
}

/// The pixels of a fully read frame.
fn raster(frame: &Frame, space: ColorSpace) -> Raster {
    let smoothing = smooth::applies(frame);
    let planes: Vec<Vec<u8>> = frame
        .components
        .iter()
        .map(|component| {
            let plane = idct::plane(component, |row, column| {
                if smoothing {
                    smooth::smoothed_block(component, frame.mcus_down, row, column)
                } else {
                    *component.block(row, column)
                }
            });
            upsample::upsample(
                &plane,
                frame.max_h / component.spec.h,
                frame.max_v / component.spec.v,
                usize::from(frame.width),
                usize::from(frame.height),
            )
        })
        .collect();
    let pixels = match (space, &planes[..]) {
        (ColorSpace::Gray, [gray, ..]) => Pixels::Gray(gray.clone()),
        (ColorSpace::YCbCr, [y, cb, cr, ..]) => Pixels::Rgb(color::ycc_to_rgb(y, cb, cr)),
        (ColorSpace::Rgb, [r, g, b, ..]) => Pixels::Rgb(
            r.iter()
                .zip(g)
                .zip(b)
                .map(|((&r, &g), &b)| [r, g, b])
                .collect(),
        ),
        (ColorSpace::Cmyk, [c, m, y, k, ..]) => Pixels::Cmyk(
            (0..c.len())
                .map(|i| [c[i], m[i], y[i], k[i]].map(|ink| 255 - ink))
                .collect(),
        ),
        (ColorSpace::Ycck, [y, cb, cr, k, ..]) => Pixels::Cmyk(
            color::ycck_to_cmyk([y, cb, cr, k])
                .into_iter()
                .map(|inks| inks.map(|ink| 255 - ink))
                .collect(),
        ),
        _ => unreachable!("the colour space matches the number of components"),
    };
    Raster::new(u32::from(frame.width), u32::from(frame.height), pixels)
}
