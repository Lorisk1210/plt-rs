//! `PCLT`: the PCL 5 printer values of a font.

use crate::error::Result;
use crate::reader::Reader;
use crate::types::Tag;

const PCLT: Tag = Tag::new(b"PCLT");

/// Values from the `PCLT` table, as FreeType loads them (`TT_PCLT`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pclt {
    /// Table version as 16.16 fixed point (`0x00010000`).
    pub version: u32,
    /// The font number of the vendor's font list.
    pub font_number: u32,
    /// Width of the space in font units.
    pub pitch: u16,
    /// Height of lowercase x in font units.
    pub x_height: u16,
    /// Posture, width and structure bits.
    pub style: u16,
    /// Typeface family and vendor.
    pub type_family: u16,
    /// Height of uppercase letters in font units.
    pub cap_height: u16,
    /// The symbol set the font encodes.
    pub symbol_set: u16,
    /// The typeface name, padded.
    pub type_face: [u8; 16],
    /// Bits of the symbol collections the font covers (inverted).
    pub character_complement: [u8; 8],
    /// Stroke weight, from -7 (ultra thin) to 7 (ultra black).
    pub stroke_weight: i8,
    /// Width type, from -5 (ultra compressed) to 5 (ultra expanded).
    pub width_type: i8,
    /// Serif style bits.
    pub serif_style: u8,
}

impl Pclt {
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, PCLT);
        let version = r.u32()?;
        let font_number = r.u32()?;
        let pitch = r.u16()?;
        let x_height = r.u16()?;
        let style = r.u16()?;
        let type_family = r.u16()?;
        let cap_height = r.u16()?;
        let symbol_set = r.u16()?;
        let mut type_face = [0; 16];
        type_face.copy_from_slice(r.bytes(16)?);
        let mut character_complement = [0; 8];
        character_complement.copy_from_slice(r.bytes(8)?);
        r.skip(6)?;
        let stroke_weight = r.u8()? as i8;
        let width_type = r.u8()? as i8;
        let serif_style = r.u8()?;
        Ok(Self {
            version,
            font_number,
            pitch,
            x_height,
            style,
            type_family,
            cap_height,
            symbol_set,
            type_face,
            character_complement,
            stroke_weight,
            width_type,
            serif_style,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pclt() {
        let mut data = vec![0, 1, 0, 0, 0x80, 0, 0, 0];
        for value in [1075u16, 905, 0, 0, 1276, 0] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        data.extend_from_slice(b"cmtt10\0\0\0\0\0\0\0\0\0\0");
        data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0x37, 0xFF, 0xFF, 0xFE]);
        data.extend_from_slice(&[0; 6]);
        data.extend_from_slice(&[0, 0xFB, 0x40, 0]);
        let pclt = Pclt::parse(&data).unwrap();
        assert_eq!(pclt.font_number, 0x8000_0000);
        assert_eq!((pclt.x_height, pclt.cap_height), (905, 1276));
        assert_eq!(&pclt.type_face[..6], b"cmtt10");
        assert_eq!((pclt.width_type, pclt.serif_style), (-5, 64));
        assert!(Pclt::parse(&data[..50]).is_err());
    }
}
