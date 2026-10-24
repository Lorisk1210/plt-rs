//! The hinting glyph loader: FreeType's `load_truetype_glyph`,
//! `TT_Process_Simple_Glyph`, `TT_Process_Composite_Component`,
//! `TT_Process_Composite_Glyph` and `TT_Hint_Glyph`, plus the origin shift
//! and grid-fitted advance of `TT_Load_Glyph` and `FT_Load_Glyph`.

use super::error::BytecodeProgram;
use super::interp::{Definitions, Exec, Metrics, Resources, State};
use super::program::{HintingInstance, Prepared};
use super::state::GraphicsState;
use super::zone::{TAG_ON, TAG_TOUCH_BOTH, Vector, Zone};
use crate::error::{FontError, Result};
use crate::face::Face;
use crate::fixed::{mul_fix, pix_round};
use crate::outline::{Midpoint, Outline, OutlineSink, PointKind};
use crate::reader::Reader;
use crate::scale::PixelSink;
use crate::scan::{Dropouts, ScanOutline, ScanRules};
use crate::tables::glyf::{
    Component, GLYF, Glyf, MAX_COMPONENT_DEPTH, MAX_COMPONENTS, MAX_POINTS, Placement,
    ROUND_XY_TO_GRID, USE_MY_METRICS, WE_HAVE_INSTRUCTIONS, bad_anchor, parse_components,
    parse_simple,
};
use crate::types::{GlyphId, Rect};

/// The most contours a simple glyph may have (`TT_Load_Simple_Glyph`).
const MAX_CONTOURS: usize = 0xFFF;

/// The horizontal metrics FreeType gives a hinted glyph
/// (`FT_Glyph_Metrics`), in 26.6 pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct GlyphMetrics {
    /// Width of the control box, rounded out to whole pixels.
    pub width: i32,
    /// Height of the control box, rounded out to whole pixels.
    pub height: i32,
    /// Left edge of the rounded control box.
    pub hori_bearing_x: i32,
    /// Top edge of the rounded control box.
    pub hori_bearing_y: i32,
    /// The rounded advance width.
    pub hori_advance: i32,
}

/// A glyph loaded with hinting: its grid-fitted outline in 26.6 pixels,
/// with the origin at the first phantom point, its advance, and how it is
/// scan-converted without anti-aliasing.
#[derive(Clone, Debug, PartialEq)]
pub struct HintedGlyph {
    outline: Outline,
    advance: i32,
    rules: ScanRules,
}

impl HintedGlyph {
    /// A glyph from its grid-fitted outline (26.6, origin at the rounded
    /// left phantom point), rounded advance and scan-conversion rules.
    pub(crate) const fn new(outline: Outline, advance: i32, rules: ScanRules) -> Self {
        Self {
            outline,
            advance,
            rules,
        }
    }

    /// The advance width in 26.6 pixels, rounded to a whole pixel
    /// (FreeType's grid-fitted `advance.x`).
    pub const fn advance(&self) -> i32 {
        self.advance
    }

    /// The control box of the outline in 26.6 pixels, `None` for glyphs
    /// without points (`FT_Glyph_Get_CBox` with subpixel units).
    pub fn control_box(&self) -> Option<Rect> {
        self.outline.control_box().map(|(x0, y0, x1, y1)| Rect {
            x_min: x0 as i32,
            y_min: y0 as i32,
            x_max: x1 as i32,
            y_max: y1 as i32,
        })
    }

    /// The glyph's metrics as FreeType reports them after hinting: its
    /// control box rounded out to whole pixels
    /// (`ft_glyphslot_grid_fit_metrics`), and its advance; zero extents for
    /// glyphs without points.
    pub fn metrics(&self) -> GlyphMetrics {
        let floor = |v: i32| v & !63;
        let ceil = |v: i32| (v + 63) & !63;
        let cbox = self.control_box().unwrap_or_default();
        let (left, right) = (floor(cbox.x_min), ceil(cbox.x_max));
        let (bottom, top) = (floor(cbox.y_min), ceil(cbox.y_max));
        GlyphMetrics {
            width: right - left,
            height: top - bottom,
            hori_bearing_x: left,
            hori_bearing_y: top,
            hori_advance: self.advance,
        }
    }

    /// Emits the outline in pixels (26.6 values divided by 64), as
    /// matplotlib's `FT2Font.get_path` returns a hinted glyph.
    pub fn outline(&self, sink: &mut impl OutlineSink) {
        self.outline
            .decompose(&mut PixelSink(sink), Midpoint::Truncated);
    }

    /// The outline with the rules its loader set, for monochrome
    /// rendering.
    pub fn scan_outline(&self) -> ScanOutline {
        self.outline.scan_outline(self.rules)
    }
}

impl<'f> HintingInstance<'f> {
    /// Loads `glyph` of `face` (the face this instance was made for)
    /// hinted, as `FT_Load_Glyph` with hinting does.
    ///
    /// # Errors
    ///
    /// When the face has no TrueType outlines, the glyph data is
    /// malformed, or the font or control value program failed. Errors in
    /// the glyph's own instructions are ignored, as FreeType does, keeping
    /// whatever they did up to that point.
    pub fn load_glyph(&self, face: &Face<'f>, glyph: GlyphId) -> Result<HintedGlyph> {
        let prepared = self.prepared.as_ref().map_err(Clone::clone)?;
        let glyf = face.glyf().ok_or(FontError::MissingTable(GLYF))?;
        let mut load = GlyphLoad {
            face,
            glyf,
            instance: self,
            prepared,
            resources: prepared.resources.clone(),
            points: Vec::new(),
            tags: Vec::new(),
            ends: Vec::new(),
            phantoms: [Vector::default(); 4],
            components: 0,
            scan_control: prepared.graphics_state.scan_control,
            scan_type: prepared.graphics_state.scan_type,
        };
        load.load(glyph, 0)?;
        Ok(load.finish())
    }
}

/// The state of one hinted glyph load (FreeType's `TT_LoaderRec`).
struct GlyphLoad<'a, 'f> {
    face: &'a Face<'f>,
    glyf: &'a Glyf<'f>,
    instance: &'a HintingInstance<'f>,
    prepared: &'a Prepared<'f>,
    /// This load's copy of the control values, storage and twilight zone.
    resources: Resources,
    /// The outline so far, in 26.6 pixels.
    points: Vec<Vector>,
    tags: Vec<u8>,
    /// Index of the last point of each contour.
    ends: Vec<usize>,
    /// The phantom points `pp1`..`pp4` of the glyph being loaded.
    phantoms: [Vector; 4],
    /// Components visited.
    components: u32,
    /// The scan control a glyph program last left, which lasts from one
    /// program of the glyph to the next (FreeType keeps the execution
    /// context's graphics state).
    scan_control: bool,
    /// The scan type a glyph program last left.
    scan_type: i32,
}

impl GlyphLoad<'_, '_> {
    fn scale(&self, v: Vector) -> Vector {
        Vector::new(
            mul_fix(v.x, self.instance.x_scale),
            mul_fix(v.y, self.instance.y_scale),
        )
    }

    /// The phantom points in font units (`tt_loader_set_pp`, with the
    /// vertical ones centred as the version 40 interpreter places them).
    fn phantom_points(&self, glyph: GlyphId, bbox: Rect) -> [Vector; 4] {
        let lsb = i64::from(self.face.left_side_bearing(glyph));
        let advance = i64::from(self.face.glyph_advance(glyph));
        let (tsb, advance_height) = self.face.glyph_vertical_metrics(glyph, bbox.y_max);
        let pp1 = Vector::new(i64::from(bbox.x_min) - lsb, 0);
        let pp3 = Vector::new(advance / 2, i64::from(bbox.y_max) + i64::from(tsb));
        [
            pp1,
            Vector::new(pp1.x + advance, 0),
            pp3,
            Vector::new(advance / 2, pp3.y - i64::from(advance_height)),
        ]
    }

    fn load(&mut self, glyph: GlyphId, depth: u8) -> Result<()> {
        if depth > MAX_COMPONENT_DEPTH {
            return Err(FontError::malformed(
                GLYF,
                "composite glyphs nested too deeply",
            ));
        }
        self.face.check_glyph(glyph)?;
        let record = self
            .glyf
            .record(glyph)?
            .filter(|(header, _)| header.contours != 0);
        let bbox = record.map_or(Rect::default(), |(header, _)| header.bbox);
        self.phantoms = self.phantom_points(glyph, bbox);
        match record {
            None => {
                self.phantoms = self.phantoms.map(|p| self.scale(p));
                Ok(())
            }
            Some((header, body)) if header.contours > 0 => {
                self.load_simple(body, header.contours as usize)
            }
            Some((_, body)) => {
                let length = self.glyf.data(glyph)?.len();
                self.load_composite(body, length, depth)
            }
        }
    }

    fn check_points(&self, more: usize) -> Result<()> {
        if self.points.len() + more > MAX_POINTS {
            return Err(FontError::malformed(GLYF, "glyph has too many points"));
        }
        Ok(())
    }

    fn load_simple(&mut self, body: &[u8], contours: usize) -> Result<()> {
        if contours >= MAX_CONTOURS {
            return Err(FontError::malformed(GLYF, "glyph has too many contours"));
        }
        let glyph = parse_simple(body, contours)?;
        let count = glyph.points.len();
        self.check_points(count + 4)?;
        let orus: Vec<Vector> = glyph
            .points
            .iter()
            .map(|p| Vector::new(p.x, p.y))
            .chain(self.phantoms)
            .collect();
        let cur: Vec<Vector> = orus.iter().map(|&p| self.scale(p)).collect();
        let mut zone = Zone {
            org: Vec::new(),
            tags: glyph
                .points
                .iter()
                .map(|p| u8::from(p.on_curve))
                .chain([0; 4])
                .collect(),
            contours: glyph.ends.iter().map(usize::from).collect(),
            cur,
            orus,
        };
        self.phantoms = [
            zone.cur[count],
            zone.cur[count + 1],
            zone.cur[count + 2],
            zone.cur[count + 3],
        ];
        if self.prepared.instructions {
            self.hint(&mut zone, glyph.instructions, false);
        }
        let start = self.points.len();
        self.points.extend_from_slice(&zone.cur[..count]);
        self.tags.extend_from_slice(&zone.tags[..count]);
        self.ends
            .extend(zone.contours.iter().map(|end| start + end));
        Ok(())
    }

    /// Runs a glyph program over `zone` (whose last four points are the
    /// phantom points) after rounding the phantom points (`TT_Hint_Glyph`).
    /// A composite's program sees its hinted components as the original
    /// outline.
    fn hint(&mut self, zone: &mut Zone, instructions: &[u8], composite: bool) {
        let n = zone.len();
        if composite {
            zone.orus.clone_from(&zone.cur);
        }
        zone.org.clone_from(&zone.cur);
        zone.cur[n - 4].x = pix_round(zone.cur[n - 4].x);
        zone.cur[n - 3].x = pix_round(zone.cur[n - 3].x);
        zone.cur[n - 2].y = pix_round(zone.cur[n - 2].y);
        zone.cur[n - 1].y = pix_round(zone.cur[n - 1].y);
        if !instructions.is_empty() {
            let metrics = Metrics {
                orus_scale: if composite {
                    0x10000
                } else {
                    self.instance.x_scale
                },
                ..self.instance.metrics
            };
            let prepared = self.prepared;
            let state = State {
                definitions: Definitions::ReadOnly(&prepared.definitions),
                resources: &mut self.resources,
                metrics,
                num_glyphs: self.instance.num_glyphs,
                stack_size: prepared.stack_size,
            };
            let graphics_state = GraphicsState {
                scan_control: self.scan_control,
                scan_type: self.scan_type,
                ..prepared.graphics_state
            };
            let mut exec = Exec::new(
                state,
                zone,
                graphics_state,
                prepared.ranges(instructions),
                BytecodeProgram::Glyph,
                composite,
            );
            // Like FreeType without pedantic hinting, a failing glyph
            // program keeps what it did.
            let _ = exec.run();
            let left = exec.graphics_state();
            (self.scan_control, self.scan_type) = (left.scan_control, left.scan_type);
        }
        // Outside backward compatibility mode the program may move the
        // phantom points, and so the bearings and the advance.
        if self.resources.backward_compatibility == 0 {
            self.phantoms = [
                zone.cur[n - 4],
                zone.cur[n - 3],
                zone.cur[n - 2],
                zone.cur[n - 1],
            ];
        }
    }

    fn load_composite(&mut self, body: &[u8], length: usize, depth: u8) -> Result<()> {
        let (components, rest) = parse_components(body)?;
        self.phantoms = self.phantoms.map(|p| self.scale(p));
        let (start_point, start_contour) = (self.points.len(), self.ends.len());
        for component in &components {
            self.components += 1;
            if self.components > MAX_COMPONENTS {
                return Err(FontError::malformed(
                    GLYF,
                    "composite glyph has too many components",
                ));
            }
            let (saved, base) = (self.phantoms, self.points.len());
            self.load(component.glyph, depth + 1)?;
            if component.flags & USE_MY_METRICS == 0 {
                self.phantoms = saved;
            }
            if self.points.len() > base {
                self.place(component, start_point, base)?;
            }
        }
        let instructed = components
            .last()
            .is_some_and(|last| last.flags & WE_HAVE_INSTRUCTIONS != 0);
        if self.prepared.instructions && instructed && self.points.len() > start_point {
            self.hint_composite(rest, length, start_point, start_contour)?;
        }
        Ok(())
    }

    /// Transforms and positions the points from `base` on, just loaded for
    /// `component` of the composite starting at `start`.
    fn place(&mut self, component: &Component, start: usize, base: usize) -> Result<()> {
        if component.has_transform() {
            for point in &mut self.points[base..] {
                let (x, y) = component.matrix.transform_fixed(point.x, point.y);
                *point = Vector::new(x, y);
            }
        }
        let offset = match component.placement {
            Placement::Anchor(parent, child) => {
                let (parent, child) = (start + parent, base + child);
                if parent >= base || child >= self.points.len() {
                    return Err(bad_anchor());
                }
                self.points[parent].minus(self.points[child])
            }
            Placement::Offset(0, 0) => return Ok(()),
            Placement::Offset(x, y) => {
                let (x, y) = component.matrix_scaled_offset(x, y);
                let mut offset = self.scale(Vector::new(x, y));
                if component.flags & ROUND_XY_TO_GRID != 0 && self.prepared.instructions {
                    if self.resources.backward_compatibility == 0 {
                        offset.x = pix_round(offset.x);
                    }
                    offset.y = pix_round(offset.y);
                }
                offset
            }
        };
        for point in &mut self.points[base..] {
            *point = Vector::new(
                point.x.wrapping_add(offset.x),
                point.y.wrapping_add(offset.y),
            );
        }
        Ok(())
    }

    /// Runs a composite glyph's own instructions over its components
    /// (`TT_Process_Composite_Glyph`).
    fn hint_composite(
        &mut self,
        rest: &[u8],
        length: usize,
        start: usize,
        start_contour: usize,
    ) -> Result<()> {
        let mut r = Reader::new(rest, GLYF);
        let count = usize::from(r.u16()?);
        if count == 0 {
            return Ok(());
        }
        if count > length {
            return Err(FontError::malformed(
                GLYF,
                "composite glyph has more instructions than bytes",
            ));
        }
        let instructions = r.bytes(count)?;
        self.check_points(4)?;
        let mut zone = Zone {
            org: Vec::new(),
            orus: Vec::new(),
            cur: self.points[start..]
                .iter()
                .copied()
                .chain(self.phantoms)
                .collect(),
            tags: self.tags[start..]
                .iter()
                .map(|tag| tag & !TAG_TOUCH_BOTH)
                .chain([0; 4])
                .collect(),
            contours: self.ends[start_contour..]
                .iter()
                .map(|end| end - start)
                .collect(),
        };
        self.hint(&mut zone, instructions, true);
        let count = self.points.len() - start;
        self.points[start..].copy_from_slice(&zone.cur[..count]);
        self.tags[start..].copy_from_slice(&zone.tags[..count]);
        Ok(())
    }

    /// The finished glyph: moved so that the first phantom point is the
    /// origin, with the advance rounded to whole pixels.
    fn finish(self) -> HintedGlyph {
        let [pp1, pp2, ..] = self.phantoms;
        let mut outline = Outline::default();
        let mut start = 0;
        for &end in &self.ends {
            for (point, tag) in self.points[start..=end].iter().zip(&self.tags[start..=end]) {
                let kind = if tag & TAG_ON != 0 {
                    PointKind::OnCurve
                } else {
                    PointKind::Quadratic
                };
                outline.push(point.x.wrapping_sub(pp1.x) as f64, point.y as f64, kind);
            }
            outline.end_contour();
            start = end + 1;
        }
        let advance = pix_round(pp2.x.wrapping_sub(pp1.x));
        let rules = ScanRules {
            high_precision: self.instance.y_ppem < 24,
            dropouts: Dropouts::from_scan_type(self.scan_control, self.scan_type),
        };
        HintedGlyph::new(
            outline,
            advance.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            rules,
        )
    }
}
