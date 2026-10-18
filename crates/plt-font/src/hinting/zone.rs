//! Glyph zones: the point arrays the interpreter moves (FreeType's
//! `TT_GlyphZoneRec`).

/// A point or distance in 26.6 pixels (or font units for `orus`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct Vector {
    pub(crate) x: i64,
    pub(crate) y: i64,
}

impl Vector {
    pub(crate) const fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    /// Component-wise wrapping difference `self - other`.
    pub(crate) const fn minus(self, other: Self) -> Self {
        Self::new(self.x.wrapping_sub(other.x), self.y.wrapping_sub(other.y))
    }
}

/// The point is on the curve (`FT_CURVE_TAG_ON`).
pub(crate) const TAG_ON: u8 = 0x01;
/// The point was moved along x (`FT_CURVE_TAG_TOUCH_X`).
pub(crate) const TAG_TOUCH_X: u8 = 0x08;
/// The point was moved along y (`FT_CURVE_TAG_TOUCH_Y`).
pub(crate) const TAG_TOUCH_Y: u8 = 0x10;
/// Both touch flags.
pub(crate) const TAG_TOUCH_BOTH: u8 = TAG_TOUCH_X | TAG_TOUCH_Y;

/// The points of one zone: the twilight zone, or a glyph (with its four
/// phantom points last).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Zone {
    /// Original positions, scaled and possibly moved by `*_orig` moves.
    pub(crate) org: Vec<Vector>,
    /// Current (grid-fitted) positions.
    pub(crate) cur: Vec<Vector>,
    /// Original positions in font units.
    pub(crate) orus: Vec<Vector>,
    /// Point tags: on-curve and touch flags.
    pub(crate) tags: Vec<u8>,
    /// Index of the last point of each contour, relative to the zone.
    pub(crate) contours: Vec<usize>,
}

impl Zone {
    /// The twilight zone: `count` points at the origin, no contours.
    pub(crate) fn twilight(count: usize) -> Self {
        Self {
            org: vec![Vector::default(); count],
            cur: vec![Vector::default(); count],
            orus: vec![Vector::default(); count],
            tags: vec![0; count],
            contours: Vec::new(),
        }
    }

    /// Number of points.
    pub(crate) fn len(&self) -> usize {
        self.cur.len()
    }
}
