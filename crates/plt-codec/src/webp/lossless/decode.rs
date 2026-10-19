//! Decoding VP8L bitstreams (libwebp's `vp8l_dec.c`): the header, the
//! transforms, and the entropy-coded images with their groups of prefix
//! codes, colour cache and backward references.

use super::bit_reader::BitReader;
use super::prefix::{PrefixCode, read_prefix_code};
use super::transform::{Palette, TileImage, Transform, subsample};
use crate::raster::MAX_PIXELS;
use crate::webp::DecodeError;

/// The first byte of a VP8L bitstream.
const SIGNATURE: u8 = 0x2F;
/// Green values, and literal symbols of the other channels.
const LITERALS: usize = 256;
/// Symbols of backward-reference lengths, after the green literals.
const LENGTH_CODES: usize = 24;
/// Symbols of backward-reference distances.
const DISTANCE_CODES: usize = 40;
/// The largest colour cache, in bits of its index.
const MAX_CACHE_BITS: u32 = 11;

/// The short distances: for distance codes 1–120, the offset to the
/// referenced pixel as `(rows up) << 4 | (8 - columns left)`.
const CODE_TO_PLANE: [u8; 120] = [
    0x18, 0x07, 0x17, 0x19, 0x28, 0x06, 0x27, 0x29, 0x16, 0x1a, 0x26, 0x2a, 0x38, 0x05, 0x37, 0x39,
    0x15, 0x1b, 0x36, 0x3a, 0x25, 0x2b, 0x48, 0x04, 0x47, 0x49, 0x14, 0x1c, 0x35, 0x3b, 0x46, 0x4a,
    0x24, 0x2c, 0x58, 0x45, 0x4b, 0x34, 0x3c, 0x03, 0x57, 0x59, 0x13, 0x1d, 0x56, 0x5a, 0x23, 0x2d,
    0x44, 0x4c, 0x55, 0x5b, 0x33, 0x3d, 0x68, 0x02, 0x67, 0x69, 0x12, 0x1e, 0x66, 0x6a, 0x22, 0x2e,
    0x54, 0x5c, 0x43, 0x4d, 0x65, 0x6b, 0x32, 0x3e, 0x78, 0x01, 0x77, 0x79, 0x53, 0x5d, 0x11, 0x1f,
    0x64, 0x6c, 0x42, 0x4e, 0x76, 0x7a, 0x21, 0x2f, 0x75, 0x7b, 0x31, 0x3f, 0x63, 0x6d, 0x52, 0x5e,
    0x00, 0x74, 0x7c, 0x41, 0x4f, 0x10, 0x20, 0x62, 0x6e, 0x30, 0x73, 0x7d, 0x51, 0x5f, 0x40, 0x72,
    0x7e, 0x61, 0x6f, 0x50, 0x71, 0x7f, 0x60, 0x70,
];

/// The size and alpha hint of a VP8L header (libwebp's `VP8LGetInfo`).
pub(crate) fn read_info(data: &[u8]) -> Result<(u32, u32, bool), DecodeError> {
    read_header(&mut BitReader::new(data))
}

/// Reads the header: signature, size, alpha hint and version 0.
fn read_header(reader: &mut BitReader<'_>) -> Result<(u32, u32, bool), DecodeError> {
    if reader.read(8) != u32::from(SIGNATURE) {
        return Err(DecodeError::LosslessHeader);
    }
    let width = reader.read(14) + 1;
    let height = reader.read(14) + 1;
    let alpha_hint = reader.flag();
    if reader.read(3) != 0 {
        return Err(DecodeError::LosslessHeader);
    }
    if reader.is_overrun() {
        return Err(DecodeError::Truncated);
    }
    Ok((width, height, alpha_hint))
}

/// Decodes a VP8L bitstream into ARGB words, row by row.
pub(crate) fn decode(data: &[u8]) -> Result<Vec<u32>, DecodeError> {
    let mut reader = BitReader::new(data);
    let (width, height, _) = read_header(&mut reader)?;
    decode_stream_with(&mut reader, width, height)
}

/// Decodes a headerless image stream of `width × height` pixels, as
/// `ALPH` chunks hold them.
pub(crate) fn decode_stream(data: &[u8], width: u32, height: u32) -> Result<Vec<u32>, DecodeError> {
    decode_stream_with(&mut BitReader::new(data), width, height)
}

/// Reads the transforms and the coded image, then undoes the transforms.
fn decode_stream_with(
    reader: &mut BitReader<'_>,
    width: u32,
    height: u32,
) -> Result<Vec<u32>, DecodeError> {
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(DecodeError::TooLarge { width, height });
    }
    let (width, height) = (width as usize, height as usize);
    let mut transforms: Vec<Transform> = Vec::new();
    let mut coded_width = width;
    while reader.flag() {
        let transform = read_transform(reader, coded_width, height, &transforms)?;
        if let Transform::ColorIndexing(palette) = &transform {
            coded_width = subsample(coded_width, palette.bits);
        }
        transforms.push(transform);
    }
    let mut pixels = read_entropy_image(reader, coded_width, height, true)?;
    let mut current_width = coded_width;
    for transform in transforms.iter().rev() {
        (pixels, current_width) = transform.invert(pixels, current_width);
    }
    Ok(pixels)
}

/// Reads a transform of an image `width` pixels wide; `seen` holds the
/// transforms before it, none of which may be of the same kind.
fn read_transform(
    reader: &mut BitReader<'_>,
    width: usize,
    height: usize,
    seen: &[Transform],
) -> Result<Transform, DecodeError> {
    let kind = reader.read(2);
    let repeated = seen.iter().any(|transform| {
        kind == match transform {
            Transform::Predictor(_) => 0,
            Transform::CrossColor(_) => 1,
            Transform::SubtractGreen => 2,
            Transform::ColorIndexing(_) => 3,
        }
    });
    if repeated {
        return Err(DecodeError::RepeatedTransform);
    }
    Ok(match kind {
        0 | 1 => {
            let bits = 2 + reader.read(3);
            let tiles = read_entropy_image(
                reader,
                subsample(width, bits),
                subsample(height, bits),
                false,
            )?;
            let image = TileImage { width, bits, tiles };
            if kind == 0 {
                Transform::Predictor(image)
            } else {
                Transform::CrossColor(image)
            }
        }
        2 => Transform::SubtractGreen,
        _ => {
            let colors = reader.read_usize(8) + 1;
            let bits = match colors {
                1..=2 => 3,
                3..=4 => 2,
                5..=16 => 1,
                _ => 0,
            };
            let deltas = read_entropy_image(reader, colors, 1, false)?;
            Transform::ColorIndexing(Palette::from_deltas(width, bits, &deltas))
        }
    })
}

/// The five prefix codes of a group: green with lengths and cache
/// indices, red, blue, alpha, and distances.
#[derive(Debug)]
struct CodeGroup {
    green: PrefixCode,
    red: PrefixCode,
    blue: PrefixCode,
    alpha: PrefixCode,
    distance: PrefixCode,
}

impl CodeGroup {
    /// Reads a group whose green alphabet ends with `cache_size` cache
    /// indices.
    fn read(reader: &mut BitReader<'_>, cache_size: usize) -> Result<Self, DecodeError> {
        Ok(Self {
            green: read_prefix_code(reader, LITERALS + LENGTH_CODES + cache_size)?,
            red: read_prefix_code(reader, LITERALS)?,
            blue: read_prefix_code(reader, LITERALS)?,
            alpha: read_prefix_code(reader, LITERALS)?,
            distance: read_prefix_code(reader, DISTANCE_CODES)?,
        })
    }
}

/// Which group codes each tile of an image coded with several.
#[derive(Debug)]
struct GroupMap {
    bits: u32,
    tiles_per_row: usize,
    groups: Vec<usize>,
}

/// The codes of an entropy-coded image.
#[derive(Debug)]
struct EntropyCodes {
    groups: Vec<CodeGroup>,
    map: Option<GroupMap>,
}

impl EntropyCodes {
    /// The group coding pixel `(x, y)`.
    fn group_at(&self, x: usize, y: usize) -> &CodeGroup {
        let index = self.map.as_ref().map_or(0, |map| {
            map.groups[(y >> map.bits) * map.tiles_per_row + (x >> map.bits)]
        });
        &self.groups[index]
    }
}

/// Reads the codes of a `width × height` image; only the main image
/// (`allow_map`) may use several groups. Groups no tile uses are read and
/// checked but not kept, as in libwebp.
fn read_codes(
    reader: &mut BitReader<'_>,
    width: usize,
    height: usize,
    cache_size: usize,
    allow_map: bool,
) -> Result<EntropyCodes, DecodeError> {
    let mut map = if allow_map && reader.flag() {
        let bits = 2 + reader.read(3);
        let tiles_per_row = subsample(width, bits);
        let image = read_entropy_image(reader, tiles_per_row, subsample(height, bits), false)?;
        let groups = image
            .iter()
            .map(|&pixel| ((pixel >> 8) & 0xFFFF) as usize)
            .collect();
        Some(GroupMap {
            bits,
            tiles_per_row,
            groups,
        })
    } else {
        None
    };
    if reader.is_overrun() {
        return Err(DecodeError::Truncated);
    }
    let count = map
        .as_ref()
        .and_then(|map| map.groups.iter().max())
        .map_or(1, |&max| max + 1);
    let mut kept_as = vec![None; count];
    if let Some(map) = &map {
        for &group in &map.groups {
            kept_as[group] = Some(0);
        }
    } else {
        kept_as[0] = Some(0);
    }
    let mut groups = Vec::new();
    for kept in &mut kept_as {
        let group = CodeGroup::read(reader, cache_size)?;
        if kept.is_some() {
            *kept = Some(groups.len());
            groups.push(group);
        }
    }
    if let Some(map) = &mut map {
        for group in &mut map.groups {
            *group = kept_as[*group].expect("every mapped group is kept");
        }
    }
    Ok(EntropyCodes { groups, map })
}

/// A colour cache: recently decoded colours, indexed by a hash.
#[derive(Debug)]
struct ColorCache {
    shift: u32,
    colors: Vec<u32>,
}

impl ColorCache {
    /// A cache of `1 << bits` colours.
    fn new(bits: u32) -> Self {
        Self {
            shift: 32 - bits,
            colors: vec![0; 1 << bits],
        }
    }

    fn insert(&mut self, argb: u32) {
        self.colors[(0x1E35_A7BD_u32.wrapping_mul(argb) >> self.shift) as usize] = argb;
    }
}

/// Reads an entropy-coded image of `width × height` pixels (both at least
/// one): its colour cache, its codes, then its pixels.
fn read_entropy_image(
    reader: &mut BitReader<'_>,
    width: usize,
    height: usize,
    allow_map: bool,
) -> Result<Vec<u32>, DecodeError> {
    let mut cache = if reader.flag() {
        let bits = reader.read(4);
        if !(1..=MAX_CACHE_BITS).contains(&bits) {
            return Err(DecodeError::InvalidColorCache);
        }
        Some(ColorCache::new(bits))
    } else {
        None
    };
    let cache_size = cache.as_ref().map_or(0, |cache| cache.colors.len());
    let codes = read_codes(reader, width, height, cache_size, allow_map)?;
    let total = width * height;
    let mut pixels = vec![0u32; total];
    let (mut position, mut x, mut y) = (0, 0, 0);
    while position < total {
        let group = codes.group_at(x, y);
        let code = usize::from(group.green.read_symbol(reader));
        let run = if code < LITERALS {
            let red = u32::from(group.red.read_symbol(reader));
            let blue = u32::from(group.blue.read_symbol(reader));
            let alpha = u32::from(group.alpha.read_symbol(reader));
            pixels[position] = (alpha << 24) | (red << 16) | ((code as u32) << 8) | blue;
            1
        } else if code < LITERALS + LENGTH_CODES {
            let length = copy_amount(code - LITERALS, reader);
            let distance_code =
                copy_amount(usize::from(group.distance.read_symbol(reader)), reader);
            let distance = plane_distance(width, distance_code);
            if reader.is_overrun() {
                return Err(DecodeError::Truncated);
            }
            if distance > position || length > total - position {
                return Err(DecodeError::InvalidBackwardReference);
            }
            for index in position..position + length {
                pixels[index] = pixels[index - distance];
            }
            length
        } else {
            let colors = &cache
                .as_ref()
                .expect("cache symbols exist only with a cache")
                .colors;
            pixels[position] = colors[code - LITERALS - LENGTH_CODES];
            1
        };
        if reader.is_overrun() {
            return Err(DecodeError::Truncated);
        }
        if let Some(cache) = &mut cache {
            pixels[position..position + run]
                .iter()
                .for_each(|&argb| cache.insert(argb));
        }
        position += run;
        x += run;
        y += x / width;
        x %= width;
    }
    Ok(pixels)
}

/// A length or distance code's value: codes below 4 stand for 1–4, the
/// others for a range whose offset the extra bits give.
fn copy_amount(symbol: usize, reader: &mut BitReader<'_>) -> usize {
    if symbol < 4 {
        return symbol + 1;
    }
    let extra = ((symbol - 2) >> 1) as u32;
    let offset = (2 + (symbol & 1)) << extra;
    offset + reader.read_usize(extra) + 1
}

/// The distance in pixels a distance code stands for in rows of `width`:
/// codes up to 120 name short two-dimensional offsets.
fn plane_distance(width: usize, code: usize) -> usize {
    if code > CODE_TO_PLANE.len() {
        return code - CODE_TO_PLANE.len();
    }
    let offset = CODE_TO_PLANE[code - 1];
    let rows = usize::from(offset >> 4);
    let columns = 8 - i64::from(offset & 0xF);
    // Small widths can make the offset point at or after the pixel.
    usize::try_from(rows as i64 * width as i64 + columns)
        .unwrap_or(0)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_distances_are_two_dimensional() {
        // Code 1: straight up; code 2: one to the left.
        assert_eq!(plane_distance(10, 1), 10);
        assert_eq!(plane_distance(10, 2), 1);
        // Code 3: one row up, one column left.
        assert_eq!(plane_distance(10, 3), 11);
        // Past the table, distances are plain.
        assert_eq!(plane_distance(10, 121), 1);
        // Code 4, a row up and a column right, is the pixel itself in a
        // one-pixel row: libwebp's floor of 1.
        assert_eq!(plane_distance(1, 4), 1);
    }

    #[test]
    fn copy_amounts_grow_with_extra_bits() {
        let mut reader = BitReader::new(&[0b11]);
        assert_eq!(copy_amount(3, &mut reader), 4);
        // Symbol 4: one extra bit → 5 or 6.
        assert_eq!(copy_amount(4, &mut reader), 6);
        // Symbol 5: one extra bit → 7 or 8.
        assert_eq!(copy_amount(5, &mut reader), 8);
    }

    #[test]
    fn headers_are_checked() {
        // 0x2F, then width − 1 = 0 and height − 1 = 0 in 14 bits each,
        // alpha 1, version 0.
        let header = [0x2F, 0, 0, 0, 0x10];
        assert_eq!(read_info(&header), Ok((1, 1, true)));
        assert_eq!(
            read_info(&[0x2E, 0, 0, 0, 0]),
            Err(DecodeError::LosslessHeader)
        );
        assert_eq!(
            read_info(&[0x2F, 0, 0, 0, 0x20]),
            Err(DecodeError::LosslessHeader)
        );
        assert_eq!(read_info(&[0x2F, 0]), Err(DecodeError::Truncated));
    }
}
