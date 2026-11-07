//! `fvar`: the axes of a variable font and its named instances.

use crate::error::{FontError, Result};
use crate::reader::Reader;
use crate::types::Tag;

const FVAR: Tag = Tag::new(b"fvar");

/// A variation axis, its values in design units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VariationAxis {
    /// The axis tag (`wght`, `wdth`, `opsz`, ...).
    pub tag: Tag,
    /// The smallest value.
    pub minimum: f64,
    /// The default instance's value.
    pub default: f64,
    /// The largest value.
    pub maximum: f64,
    /// Whether the axis is meant to be hidden from users.
    pub hidden: bool,
    /// The `name` table id of the axis' name.
    pub name_id: u16,
}

/// A named instance: its name and coordinates in design units.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedInstance {
    /// The `name` table id of the instance's subfamily name.
    pub subfamily_name_id: u16,
    /// The `name` table id of its PostScript name, if the font has one.
    pub postscript_name_id: Option<u16>,
    /// Its coordinates, one per axis.
    pub coordinates: Vec<f64>,
}

/// An `fvar` table, its values in 16.16 as FreeType keeps them.
#[derive(Clone, Debug, Default)]
pub(crate) struct Fvar {
    /// Each axis: tag, minimum, default and maximum (16.16), flags and
    /// name id.
    pub(crate) axes: Vec<(Tag, [i64; 3], u16, u16)>,
    /// Each named instance: subfamily name id, coordinates (16.16) and
    /// PostScript name id.
    pub(crate) instances: Vec<(u16, Vec<i64>, Option<u16>)>,
}

impl Fvar {
    /// Parses an `fvar` table as `sfnt_init_face` checks it; an axis whose
    /// minimum exceeds its default or whose default exceeds its maximum is
    /// fixed at its default, as FreeType does.
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, FVAR);
        if r.u32()? != 0x0001_0000 {
            return Err(FontError::malformed(FVAR, "unknown fvar version"));
        }
        let axes_at = usize::from(r.u16()?);
        r.skip(2)?;
        let axis_count = usize::from(r.u16()?);
        let axis_size = usize::from(r.u16()?);
        let instance_count = usize::from(r.u16()?);
        let instance_size = usize::from(r.u16()?);
        let sizes_known =
            instance_size == 4 + 4 * axis_count || instance_size == 6 + 4 * axis_count;
        if axis_size != 20 || axis_count == 0 || axis_count > 0x3FFE || !sizes_known {
            return Err(FontError::malformed(FVAR, "bad fvar record sizes"));
        }
        if instance_count > 0x7EFF {
            return Err(FontError::malformed(FVAR, "too many named instances"));
        }
        let mut r = Reader::at(data, axes_at, FVAR)?;
        let axes = (0..axis_count)
            .map(|_| {
                let tag = r.tag()?;
                let (minimum, default, maximum) = (
                    i64::from(r.i32()?),
                    i64::from(r.i32()?),
                    i64::from(r.i32()?),
                );
                let flags = r.u16()?;
                let name_id = r.u16()?;
                let range = if minimum > default || default > maximum {
                    [default; 3]
                } else {
                    [minimum, default, maximum]
                };
                Ok((tag, range, flags, name_id))
            })
            .collect::<Result<Vec<_>>>()?;
        let instances = (0..instance_count)
            .map(|_| {
                let record = r.bytes(instance_size)?;
                let mut fields = Reader::new(record, FVAR);
                let name_id = fields.u16()?;
                fields.skip(2)?;
                let coords = (0..axis_count)
                    .map(|_| fields.i32().map(i64::from))
                    .collect::<Result<Vec<_>>>()?;
                let postscript = if instance_size == 6 + 4 * axis_count {
                    Some(fields.u16()?).filter(|&id| id != 0xFFFF)
                } else {
                    None
                };
                Ok((name_id, coords, postscript))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { axes, instances })
    }

    /// The axes in design units.
    pub(crate) fn variation_axes(&self) -> Vec<VariationAxis> {
        let design = |value: i64| value as f64 / 65536.0;
        self.axes
            .iter()
            .map(
                |&(tag, [minimum, default, maximum], flags, name_id)| VariationAxis {
                    tag,
                    minimum: design(minimum),
                    default: design(default),
                    maximum: design(maximum),
                    hidden: flags & 1 != 0,
                    name_id,
                },
            )
            .collect()
    }

    /// The named instances in design units.
    pub(crate) fn named_instances(&self) -> Vec<NamedInstance> {
        self.instances
            .iter()
            .map(|(name_id, coords, postscript)| NamedInstance {
                subfamily_name_id: *name_id,
                postscript_name_id: *postscript,
                coordinates: coords.iter().map(|&v| v as f64 / 65536.0).collect(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_and_instances() {
        let mut data = vec![0, 1, 0, 0, 0, 16, 0, 2, 0, 1, 0, 20, 0, 1, 0, 10];
        data.extend(b"wght");
        for value in [100i32, 400, 900] {
            data.extend((value << 16).to_be_bytes());
        }
        data.extend([0, 0, 1, 0]);
        data.extend([1, 2, 0, 0]);
        data.extend((700i32 << 16).to_be_bytes());
        data.extend([1, 3]);
        let fvar = Fvar::parse(&data).unwrap();
        let axis = fvar.variation_axes()[0];
        assert_eq!(
            (axis.tag, axis.minimum, axis.default, axis.maximum),
            (Tag::new(b"wght"), 100.0, 400.0, 900.0)
        );
        assert_eq!((axis.hidden, axis.name_id), (false, 256));
        let instance = &fvar.named_instances()[0];
        assert_eq!(instance.coordinates, [700.0]);
        assert_eq!(
            (instance.subfamily_name_id, instance.postscript_name_id),
            (258, Some(259))
        );
        assert!(Fvar::parse(&data[..data.len() - 3]).is_err());
        assert!(Fvar::parse(&[0, 2, 0, 0]).is_err());
    }
}
