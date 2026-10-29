//! Rows of coverage produced by sweeping the rasterizer.

/// Coverage of one horizontal run of pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Covers<'a> {
    /// One cover value per pixel.
    Varying(&'a [u8]),
    /// `len` pixels sharing a single cover value.
    Solid {
        /// Number of pixels.
        len: u32,
        /// Their common cover.
        cover: u8,
    },
}

/// A horizontal run of covered pixels within a [`Scanline`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span<'a> {
    /// Column of the first pixel.
    pub x: i32,
    /// The pixels' coverage.
    pub covers: Covers<'a>,
}

impl Span<'_> {
    /// Number of pixels in the span.
    pub fn len(&self) -> usize {
        match self.covers {
            Covers::Varying(c) => c.len(),
            Covers::Solid { len, .. } => len as usize,
        }
    }

    /// Returns `true` for a span without pixels (never produced by sweeps).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The cover of the pixel at offset `i` into the span.
    pub fn cover_at(&self, i: usize) -> u8 {
        match self.covers {
            Covers::Varying(c) => c[i],
            Covers::Solid { cover, .. } => cover,
        }
    }
}

/// Internal span record in the style of Agg's `scanline_p8`: a positive
/// `len` indexes `len` covers starting at `start`; a negative one is a
/// solid run whose single cover sits at `start`.
#[derive(Clone, Copy, Debug)]
struct SpanRecord {
    x: i32,
    len: i32,
    start: u32,
}

/// One row of anti-aliased coverage: disjoint spans in increasing `x`.
#[derive(Clone, Debug, Default)]
pub struct Scanline {
    y: i32,
    spans: Vec<SpanRecord>,
    covers: Vec<u8>,
}

impl Scanline {
    /// Creates an empty scanline.
    pub fn new() -> Self {
        Self::default()
    }

    /// The row this scanline describes.
    pub const fn y(&self) -> i32 {
        self.y
    }

    /// Returns `true` when the row has no covered pixels.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Number of spans.
    pub fn num_spans(&self) -> usize {
        self.spans.len()
    }

    /// Iterates the spans from left to right.
    pub fn spans(&self) -> impl Iterator<Item = Span<'_>> + '_ {
        self.spans.iter().map(|s| {
            let start = s.start as usize;
            let covers = if s.len > 0 {
                Covers::Varying(&self.covers[start..start + s.len as usize])
            } else {
                Covers::Solid {
                    len: s.len.unsigned_abs(),
                    cover: self.covers[start],
                }
            };
            Span { x: s.x, covers }
        })
    }

    pub(crate) fn reset(&mut self, y: i32) {
        self.y = y;
        self.spans.clear();
        self.covers.clear();
    }

    /// Appends one pixel, extending the previous per-pixel span if adjacent.
    pub(crate) fn add_cell(&mut self, x: i32, cover: u8) {
        if let Some(last) = self.spans.last_mut()
            && last.len > 0
            && last.x + last.len == x
        {
            last.len += 1;
        } else {
            self.spans.push(SpanRecord {
                x,
                len: 1,
                start: self.covers.len() as u32,
            });
        }
        self.covers.push(cover);
    }

    /// Appends a solid run of `len` pixels.
    pub(crate) fn add_span(&mut self, x: i32, len: i32, cover: u8) {
        if let Some(last) = self.spans.last_mut()
            && last.len < 0
            && last.x - last.len == x
            && self.covers[last.start as usize] == cover
        {
            last.len -= len;
            return;
        }
        self.spans.push(SpanRecord {
            x,
            len: -len,
            start: self.covers.len() as u32,
        });
        self.covers.push(cover);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_merge_and_solid_runs_stay_separate() {
        let mut sl = Scanline::new();
        sl.reset(7);
        sl.add_cell(3, 10);
        sl.add_cell(4, 20);
        sl.add_span(5, 4, 255);
        sl.add_span(9, 2, 255);
        sl.add_cell(11, 30);
        let spans: Vec<_> = sl.spans().collect();
        assert_eq!(sl.y(), 7);
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].covers, Covers::Varying(&[10, 20]));
        assert_eq!(spans[1].covers, Covers::Solid { len: 6, cover: 255 });
        assert_eq!((spans[2].x, spans[2].len()), (11, 1));
        assert_eq!(spans[1].cover_at(3), 255);
    }
}
