//! Tracking by the `trak` table (HarfBuzz's
//! `hb-aat-layout-trak-table.hh`): an advance added to the first glyph of
//! every grapheme, read for the size of the text from the font's tracks.
//!
//! HarfBuzz tracks only fonts that also have a `STAT` table, at the
//! normal track, and (as matplotlib's fonts never give it a point size)
//! with the values for CoreText's default size.

use pltrs_core::numeric::fused_f32;

use crate::face::Face;
use crate::layout::buffer::Buffer;
use crate::reader::peek;
use crate::scale::Scaler;
use crate::types::Tag;

/// The point size HarfBuzz tracks at when the font has none (CoreText's
/// default font size).
const DEFAULT_POINT_SIZE: f32 = 12.0;

/// A 16.16 fixed-point value as a float.
fn fixed(value: i32) -> f32 {
    value as f32 / 65536.0
}

/// The `trak` table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Trak<'a> {
    data: &'a [u8],
}

/// The horizontal tracks of a [`Trak`] table: tracks with values for
/// each of a list of sizes.
#[derive(Clone, Copy, Debug)]
struct TrackData<'a> {
    /// The `trak` table, from which offsets count.
    table: &'a [u8],
    tracks: usize,
    sizes: usize,
    /// Offset of the size list.
    size_table: usize,
    /// Offset of the first track record.
    records: usize,
}

impl<'a> Trak<'a> {
    /// The face's `trak` table, when HarfBuzz applies it: when the table
    /// has data and the face has a `STAT` table.
    pub(crate) fn of(face: &Face<'a>) -> Option<Self> {
        let has_data = |tag: &[u8; 4]| {
            face.table(Tag::new(tag))
                .and_then(|data| peek::<u32>(data, 0))
                .is_some_and(|version| version != 0)
        };
        if !has_data(b"STAT") || !has_data(b"trak") {
            return None;
        }
        face.table(Tag::new(b"trak")).map(|data| Self { data })
    }

    /// The tracks for horizontal text.
    fn horizontal(&self) -> Option<TrackData<'a>> {
        let offset = usize::from(peek::<u16>(self.data, 6)?);
        if offset == 0 {
            return None;
        }
        Some(TrackData {
            table: self.data,
            tracks: usize::from(peek::<u16>(self.data, offset)?),
            sizes: usize::from(peek::<u16>(self.data, offset + 2)?),
            size_table: usize::try_from(peek::<u32>(self.data, offset + 4)?).ok()?,
            records: offset + 8,
        })
    }

    /// Adds the tracking to the first glyph of each grapheme (HarfBuzz's
    /// `trak::apply` for horizontal text).
    pub(crate) fn apply(&self, buffer: &mut Buffer, scaler: &Scaler<'_>) {
        let tracking = self
            .horizontal()
            .map_or(0.0, |data| data.tracking(DEFAULT_POINT_SIZE, 0.0));
        let advance = scaler.scale_float_x(tracking);
        for (index, (info, pos)) in buffer.info.iter().zip(&mut buffer.pos).enumerate() {
            if index == 0 || !info.chars.continuation {
                pos.x_advance += advance;
            }
        }
    }
}

impl TrackData<'_> {
    /// The track value (16.16) of record `index`.
    fn track(&self, index: usize) -> f32 {
        fixed(peek::<i32>(self.table, self.records + 8 * index).unwrap_or(0))
    }

    /// The size (16.16 points) at `index` of the size list.
    fn size(&self, index: usize) -> f32 {
        fixed(peek::<i32>(self.table, self.size_table + 4 * index).unwrap_or(0))
    }

    /// The value of record `record` for the size at `index`.
    fn value(&self, record: usize, index: usize) -> f32 {
        let values = peek::<u16>(self.table, self.records + 8 * record + 6).map_or(0, usize::from);
        f32::from(peek::<i16>(self.table, values + 2 * index).unwrap_or(0))
    }

    /// The tracking of `track` at `point_size`, interpolated between the
    /// tracks and sizes listed (HarfBuzz's `TrackData::get_tracking`).
    fn tracking(&self, point_size: f32, track: f32) -> f32 {
        match self.tracks {
            0 => 0.0,
            1 => self.value_at(0, point_size),
            count => {
                let mut i = 0;
                while i + 1 < count && self.track(i + 1) <= track {
                    i += 1;
                }
                let mut j = count - 1;
                while j > 0 && self.track(j - 1) >= track {
                    j -= 1;
                }
                if i == j {
                    return self.value_at(i, point_size);
                }
                let (t0, t1) = (self.track(i), self.track(j));
                let t = (track - t0) / (t1 - t0);
                let (a, b) = (self.value_at(i, point_size), self.value_at(j, point_size));
                fused_f32(t, b - a, a)
            }
        }
    }

    /// The value of record `record` at `point_size` (HarfBuzz's
    /// `TrackTableEntry::get_value`).
    #[expect(clippy::float_cmp, reason = "sizes match exactly, as in HarfBuzz")]
    fn value_at(&self, record: usize, point_size: f32) -> f32 {
        let sizes = self.sizes;
        match sizes {
            0 => return 0.0,
            1 => return self.value(record, 0),
            _ => {}
        }
        let index = (0..sizes)
            .find(|&i| self.size(i) >= point_size)
            .unwrap_or(sizes);
        if index == 0 {
            return self.value(record, 0);
        }
        if index == sizes {
            return self.value(record, sizes - 1);
        }
        if self.size(index) == point_size {
            return self.value(record, index);
        }
        let (mut s0, mut s1) = (self.size(index - 1), self.size(index));
        let (mut v0, mut v1) = (self.value(record, index - 1), self.value(record, index));
        if s1 < s0 {
            std::mem::swap(&mut s0, &mut s1);
            std::mem::swap(&mut v0, &mut v1);
        }
        if point_size < s0 {
            return v0;
        }
        if point_size > s1 {
            return v1;
        }
        if s0 == s1 {
            return f32::midpoint(v0, v1);
        }
        let t = (point_size - s0) / (s1 - s0);
        fused_f32(t, v1 - v0, v0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Track data with two tracks (-1 and 0) for sizes 9 and 18 points.
    fn track_data() -> Vec<u8> {
        let mut data = vec![0; 12];
        for word in [2_u16, 2] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        data.extend_from_slice(&36_u32.to_be_bytes());
        for (track, values) in [(-65536_i32, 44_u16), (0, 48)] {
            data.extend_from_slice(&track.to_be_bytes());
            data.extend_from_slice(&[0, 0]);
            data.extend_from_slice(&values.to_be_bytes());
        }
        for size in [9_i32 * 65536, 18 * 65536] {
            data.extend_from_slice(&size.to_be_bytes());
        }
        for value in [-30_i16, -10, 20, 50] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        data
    }

    #[test]
    fn values_interpolate_between_sizes_and_tracks() {
        let table = track_data();
        let data = TrackData {
            table: &table,
            tracks: 2,
            sizes: 2,
            size_table: 36,
            records: 20,
        };
        assert_eq!(data.tracking(9.0, 0.0), 20.0);
        assert_eq!(data.tracking(12.0, 0.0), 30.0);
        assert_eq!(data.tracking(40.0, 0.0), 50.0);
        assert_eq!(data.tracking(4.0, -1.0), -30.0);
        assert_eq!(data.tracking(18.0, -0.5), 20.0);
    }
}
