//! Ancillary information carried alongside the pixels: physical resolution
//! (`pHYs`) and text (`tEXt`, `zTXt`, `iTXt`).

/// Metres per inch.
const METRES_PER_INCH: f64 = 0.0254;

/// The unit of [`PhysicalDimensions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicalUnit {
    /// Only the pixel aspect ratio is known.
    Unknown,
    /// Pixels per metre.
    Meter,
}

/// Intended pixel density, from the `pHYs` chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalDimensions {
    /// Pixels per unit along x.
    pub x: u32,
    /// Pixels per unit along y.
    pub y: u32,
    /// The unit `x` and `y` count pixels per.
    pub unit: PhysicalUnit,
}

impl PhysicalDimensions {
    /// The dimensions for `dpi` dots per inch in both directions, rounded to
    /// whole pixels per metre the way Pillow (and hence matplotlib) does.
    /// Returns `None` unless `dpi` is finite, positive and representable.
    ///
    /// ```
    /// use plt_codec::png::{PhysicalDimensions, PhysicalUnit};
    ///
    /// let dims = PhysicalDimensions::from_dpi(100.0).unwrap();
    /// assert_eq!((dims.x, dims.y, dims.unit), (3937, 3937, PhysicalUnit::Meter));
    /// assert_eq!(PhysicalDimensions::from_dpi(f64::NAN), None);
    /// ```
    pub fn from_dpi(dpi: f64) -> Option<Self> {
        let per_metre = (dpi / METRES_PER_INCH + 0.5).floor();
        (dpi.is_finite() && per_metre >= 1.0 && per_metre <= f64::from(u32::MAX)).then_some(Self {
            x: per_metre as u32,
            y: per_metre as u32,
            unit: PhysicalUnit::Meter,
        })
    }

    /// Dots per inch along x and y, if the unit is known.
    pub fn dpi(&self) -> Option<(f64, f64)> {
        (self.unit == PhysicalUnit::Meter).then(|| {
            (
                f64::from(self.x) * METRES_PER_INCH,
                f64::from(self.y) * METRES_PER_INCH,
            )
        })
    }

    /// The 9-byte `pHYs` payload.
    pub(crate) fn to_bytes(self) -> [u8; 9] {
        let mut bytes = [0; 9];
        bytes[..4].copy_from_slice(&self.x.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.y.to_be_bytes());
        bytes[8] = u8::from(self.unit == PhysicalUnit::Meter);
        bytes
    }

    /// Parses a `pHYs` payload; `None` if it is malformed.
    pub(crate) fn from_bytes(data: &[u8]) -> Option<Self> {
        let &[x0, x1, x2, x3, y0, y1, y2, y3, unit] = <&[u8; 9]>::try_from(data).ok()?;
        let unit = match unit {
            0 => PhysicalUnit::Unknown,
            1 => PhysicalUnit::Meter,
            _ => return None,
        };
        Some(Self {
            x: u32::from_be_bytes([x0, x1, x2, x3]),
            y: u32::from_be_bytes([y0, y1, y2, y3]),
            unit,
        })
    }
}

/// One textual key/value pair.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextEntry {
    /// The keyword, e.g. `"Software"` or `"Title"`.
    pub keyword: String,
    /// The text.
    pub text: String,
}

impl TextEntry {
    /// Creates an entry.
    pub fn new(keyword: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            keyword: keyword.into(),
            text: text.into(),
        }
    }
}

/// Ancillary information decoded from a PNG file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metadata {
    /// Physical pixel dimensions, from `pHYs`.
    pub physical: Option<PhysicalDimensions>,
    /// Text entries from `tEXt`, `zTXt` and `iTXt` chunks, in file order.
    pub text: Vec<TextEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_conversion_matches_pillow_rounding() {
        let cases = [(72.0, 2835), (100.0, 3937), (150.0, 5906), (300.0, 11_811)];
        for (dpi, per_metre) in cases {
            let dims = PhysicalDimensions::from_dpi(dpi).unwrap();
            assert_eq!((dims.x, dims.y), (per_metre, per_metre), "{dpi} dpi");
            let (x, y) = dims.dpi().unwrap();
            assert!((x - dpi).abs() < 0.02 && (y - dpi).abs() < 0.02);
        }
        for bad in [0.0, -5.0, f64::INFINITY, f64::NAN, 1e12, 0.001] {
            assert_eq!(PhysicalDimensions::from_dpi(bad), None, "{bad}");
        }
    }

    #[test]
    fn phys_payload_round_trips() {
        let dims = PhysicalDimensions {
            x: 3937,
            y: 1,
            unit: PhysicalUnit::Meter,
        };
        let bytes = dims.to_bytes();
        assert_eq!(bytes, [0, 0, 0x0F, 0x61, 0, 0, 0, 1, 1]);
        assert_eq!(PhysicalDimensions::from_bytes(&bytes), Some(dims));
        assert_eq!(PhysicalDimensions::from_bytes(&bytes[..8]), None);
        let mut bad_unit = bytes;
        bad_unit[8] = 2;
        assert_eq!(PhysicalDimensions::from_bytes(&bad_unit), None);
        let aspect = PhysicalDimensions {
            unit: PhysicalUnit::Unknown,
            ..dims
        };
        assert_eq!(aspect.dpi(), None);
    }
}
