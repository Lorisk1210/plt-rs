//! Type 42 font programs (`_font_to_ps_type42`, `_serialize_type42`): a
//! TrueType subset of the font as `sfnts` strings, its glyphs reachable by
//! name through `CharStrings`.

use std::fmt::Write as _;

use plt_font::{Face, NameKey, OutlineSink, Tag};

use crate::vector::{FontInfo, Subset, VectorError, repr};

/// The longest `sfnts` string (PostScript strings hold at most 65535
/// bytes; one is kept for the terminating NUL).
const MAX_STRING: usize = 65534;

/// `name.getDebugName`: the Macintosh or Windows English record of
/// `name_id`.
fn debug_name(face: &Face<'_>, name_id: u16) -> String {
    let windows = NameKey {
        platform_id: 3,
        encoding_id: 1,
        language_id: 0x409,
    };
    face.name_string(NameKey::MAC_ENGLISH, name_id)
        .or_else(|| face.name_string(windows, name_id))
        .unwrap_or_default()
}

/// The exact bounds of every glyph outline of a face (fontTools'
/// `BoundsPen` over the whole glyph set, matplotlib's `_bounds`).
#[derive(Debug, Default)]
struct Bounds {
    current: (f64, f64),
    bounds: Option<[f64; 4]>,
}

impl Bounds {
    fn add(&mut self, x: f64, y: f64) {
        self.bounds = Some(match self.bounds {
            None => [x, y, x, y],
            Some([x0, y0, x1, y1]) => [x0.min(x), y0.min(y), x1.max(x), y1.max(y)],
        });
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        self.bounds
            .is_some_and(|[x0, y0, x1, y1]| x0 <= x && x <= x1 && y0 <= y && y <= y1)
    }

    /// Adds the extremes of a curve with polynomial coefficients
    /// (highest power first) per axis, at the roots of the derivative in
    /// `[0, 1)`.
    fn add_extremes(&mut self, roots: &[f64], at: impl Fn(f64) -> (f64, f64)) {
        for &t in roots {
            if (0.0..1.0).contains(&t) {
                let (x, y) = at(t);
                self.add(x, y);
            }
        }
    }
}

/// The roots of `a t² + b t + c` (fontTools' `solveQuadratic`).
fn quadratic_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    if a.abs() < 1e-12 {
        return if b.abs() < 1e-12 {
            Vec::new()
        } else {
            vec![-c / b]
        };
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return Vec::new();
    }
    let root = discriminant.sqrt();
    vec![f64::midpoint(-b, root) / a, f64::midpoint(-b, -root) / a]
}

impl OutlineSink for Bounds {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = (f64::from(x), f64::from(y));
        self.add(x, y);
        self.current = (x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = (f64::from(x), f64::from(y));
        self.add(x, y);
        self.current = (x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let ((x0, y0), (x1, y1), (x2, y2)) = (
            self.current,
            (f64::from(x1), f64::from(y1)),
            (f64::from(x), f64::from(y)),
        );
        self.add(x2, y2);
        if !self.contains(x1, y1) {
            let (ax, ay) = (x0 - 2.0 * x1 + x2, y0 - 2.0 * y1 + y2);
            let (bx, by) = (2.0 * (x1 - x0), 2.0 * (y1 - y0));
            let roots: Vec<f64> = [(ax, bx), (ay, by)]
                .into_iter()
                .filter(|&(a, _)| a != 0.0)
                .map(|(a, b)| -b / (2.0 * a))
                .collect();
            self.add_extremes(&roots, |t| {
                (ax * t * t + bx * t + x0, ay * t * t + by * t + y0)
            });
        }
        self.current = (x2, y2);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p = [
            self.current,
            (f64::from(x1), f64::from(y1)),
            (f64::from(x2), f64::from(y2)),
            (f64::from(x), f64::from(y)),
        ];
        self.add(p[3].0, p[3].1);
        if !self.contains(p[1].0, p[1].1) || !self.contains(p[2].0, p[2].1) {
            let coefficients = |v: [f64; 4]| {
                let c = 3.0 * (v[1] - v[0]);
                let b = 3.0 * (v[2] - v[1]) - c;
                (v[3] - v[0] - c - b, b, c, v[0])
            };
            let (ax, bx, cx, dx) = coefficients(p.map(|q| q.0));
            let (ay, by, cy, dy) = coefficients(p.map(|q| q.1));
            let mut roots = quadratic_roots(3.0 * ax, 2.0 * bx, cx);
            roots.extend(quadratic_roots(3.0 * ay, 2.0 * by, cy));
            self.add_extremes(&roots, |t| {
                (
                    ((ax * t + bx) * t + cx) * t + dx,
                    ((ay * t + by) * t + cy) * t + dy,
                )
            });
        }
        self.current = p[3];
    }

    fn close(&mut self) {}
}

/// `_nums_to_str` for one number: integers without a fraction, others
/// with up to three decimals.
fn num_to_str(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        let text = format!("{value:.3}");
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

/// The offsets an `sfnts` string may end at: table and glyph starts.
fn breakpoints(data: &[u8]) -> Vec<usize> {
    let read32 = |at: usize| {
        data.get(at..at + 4)
            .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let count = data
        .get(4..6)
        .map_or(0, |b| usize::from(u16::from_be_bytes([b[0], b[1]])));
    let mut points: Vec<usize> = (0..count).map(|i| read32(12 + 16 * i + 8)).collect();
    if let Ok(face) = Face::parse(data, 0) {
        let glyf = (0..count)
            .find(|&i| data.get(12 + 16 * i..16 + 16 * i) == Some(b"glyf".as_slice()))
            .map(|i| read32(12 + 16 * i + 8));
        if let (Some(glyf), Some(loca)) = (glyf, face.table(Tag::new(b"loca"))) {
            points.extend(
                loca.as_chunks::<4>()
                    .0
                    .iter()
                    .map(|&offset| glyf + u32::from_be_bytes(offset) as usize),
            );
        }
    }
    points.push(data.len());
    points.sort_unstable();
    points.dedup();
    points
}

/// `_generate_sfnts`: the font data as NUL-terminated hex strings split at
/// breakpoints, in lines of 100 characters.
fn sfnts(data: &[u8]) -> String {
    let points = breakpoints(data);
    let mut text = String::from("/sfnts[");
    let mut pos = 0;
    while pos < data.len() {
        let limit = points.partition_point(|&p| p < pos + MAX_STRING);
        let mut end = points[limit.saturating_sub(1)];
        if end <= pos {
            end = data.len();
        }
        text.push('<');
        for byte in &data[pos..end] {
            let _ = write!(text, "{byte:02x}");
        }
        text.push_str("00>");
        pos = end;
    }
    text.push_str("]def");
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(100)
        .map(|line| line.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The PostScript resource defining subset `index` of `font` as a Type 42
/// font.
pub(super) fn type42_font(
    font: &FontInfo,
    index: usize,
    subset: &Subset,
) -> Result<String, VectorError> {
    let face = font.face();
    let glyphs = subset.entries().iter().map(|&(_, glyph)| glyph);
    let program = plt_font::subset(face, glyphs).map_err(|err| VectorError::Font(err.into()))?;
    let mut bounds = Bounds::default();
    for glyph in 0..face.num_glyphs() {
        // Glyphs that cannot be read add nothing.
        let _ = face.outline(plt_font::GlyphId(glyph), &mut bounds);
    }
    let bbox = bounds.bounds.unwrap_or_default().map(num_to_str).join(" ");
    let head = face.table(Tag::new(b"head")).unwrap_or_default();
    let revision = head
        .get(4..8)
        .map_or(0, |b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let version = program.data.get(..4).map_or((0, 0), |b| {
        (
            i16::from_be_bytes([b[0], b[1]]),
            i16::from_be_bytes([b[2], b[3]]),
        )
    });
    let (underline_position, underline_thickness) = face.underline_metrics().unwrap_or((0, 0));
    let mut chars = format!("/CharStrings {} dict dup begin\n", program.glyphs.len());
    for (new, &glyph) in program.glyphs.iter().enumerate() {
        let _ = writeln!(chars, "/{} {new} def", font.glyph_name(glyph));
    }
    chars.push_str("end readonly def");
    let indent = "        ";
    let lines = [
        format!(
            "%%!PS-TrueTypeFont-{}.{}-{:.7}",
            version.0,
            version.1,
            f64::from(revision) / 65536.0
        ),
        "10 dict begin".to_owned(),
        "/FontType 42 def".to_owned(),
        "/FontMatrix [1 0 0 1 0 0] def".to_owned(),
        format!("/FontName /{}-{index} def", debug_name(face, 6)),
        "/FontInfo 7 dict dup begin".to_owned(),
        format!("/FullName ({}) def", debug_name(face, 4)),
        format!("/FamilyName ({}) def", debug_name(face, 1)),
        format!("/Version ({}) def", debug_name(face, 5)),
        format!("/ItalicAngle {} def", repr(face.italic_angle())),
        format!("/isFixedPitch {} def", font.is_fixed_pitch()),
        format!("/UnderlinePosition {underline_position} def"),
        format!("/UnderlineThickness {underline_thickness} def"),
        "end readonly def".to_owned(),
        "/Encoding StandardEncoding def".to_owned(),
        format!("/FontBBox [{bbox}] def"),
        "/PaintType 0 def".to_owned(),
        "/CIDMap 0 def".to_owned(),
        chars,
        sfnts(&program.data),
        "FontName currentdict end definefont pop".to_owned(),
    ];
    let mut out = String::from("\n");
    for line in lines {
        let _ = writeln!(out, "{indent}{line}");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_like_matplotlib() {
        assert_eq!(num_to_str(-2090.0), "-2090");
        assert_eq!(num_to_str(12.5), "12.5");
        assert_eq!(num_to_str(1.23456), "1.235");
    }

    #[test]
    fn quadratic_roots_and_bounds() {
        assert_eq!(quadratic_roots(0.0, 2.0, -1.0), [0.5]);
        assert_eq!(quadratic_roots(1.0, 0.0, 1.0), Vec::<f64>::new());
        let mut bounds = Bounds::default();
        bounds.move_to(0.0, 0.0);
        bounds.quad_to(5.0, 10.0, 10.0, 0.0);
        assert_eq!(bounds.bounds, Some([0.0, 0.0, 10.0, 5.0]));
    }

    #[test]
    fn short_programs_are_one_string() {
        let text = sfnts(&[0, 1, 0, 0, 0, 0]);
        assert_eq!(text, "/sfnts[<00010000000000>]def");
    }
}
