//! The glyph path of FreeType's Adobe engine (`cf2_glyphpath_*` in
//! `pshints.c`): character-space path elements are queued one deep,
//! mapped to device space through the hint map that was current when
//! they were drawn, and handed to the [`Builder`].
//!
//! Stem darkening is off for Type 1 fonts in FreeType's default
//! configuration (and matplotlib's), so path elements carry no offsets
//! and consecutive elements always join: the intersection machinery of
//! the original never runs.

use super::blues::Blues;
use super::buffer::Buffer;
use super::builder::{Builder, DevicePoint};
use super::hintmap::HintMap;
use super::hints::{HintMask, Hints};
use crate::fixed::mul_fix;
use crate::type1::error::Type1Error;

/// A character-space point in 16.16.
pub(super) type Point = (i32, i32);

/// A queued path element.
#[derive(Clone, Copy, Debug)]
enum Element {
    Line { end: Point },
    Cube { controls: [Point; 3] },
}

/// The transform from character space to device space: horizontal
/// scale, shear and vertical scale in 16.16.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Scale {
    pub(super) x: i32,
    pub(super) c: i32,
    pub(super) y: i32,
}

impl Scale {
    /// The engine's unit scale for unhinted outlines, which the builder
    /// turns into whole font units (`0x400`, 1/64).
    pub(super) const UNHINTED: Self = Self {
        x: 0x400,
        c: 0,
        y: 0x400,
    };
}

/// The path of one charstring invocation.
#[derive(Clone, Debug)]
pub(super) struct GlyphPath {
    pub(super) hints: Hints,
    scale: Scale,
    hinted: bool,
    blues: Blues,
    /// The vertical offset of a `seac` accent, which moves its hints.
    hint_origin: i32,
    /// The map of the stems the first pass collected, which places the
    /// stems of later maps.
    initial_hint_map: HintMap,
    hint_map: HintMap,
    /// The map current at the start of the subpath, for its closing line.
    first_hint_map: HintMap,
    /// Whether a subpath is open; a move is pending while none is
    /// (FreeType's `moveIsPending` is always the opposite of `pathIsOpen`).
    open: bool,
    closing: bool,
    queued: Option<Element>,
    current_cs: Point,
    start: Point,
    current_ds: DevicePoint,
    offset_start0: Point,
    offset_start1: Point,
}

impl GlyphPath {
    pub(super) fn new(scale: Scale, hinted: bool, blues: Blues, hint_origin: i32) -> Self {
        let map = HintMap::new(hinted, scale.y);
        Self {
            hints: Hints::default(),
            scale,
            hinted,
            blues,
            hint_origin,
            initial_hint_map: map.clone(),
            hint_map: map.clone(),
            first_hint_map: map,
            open: false,
            closing: false,
            queued: None,
            current_cs: (0, 0),
            start: (0, 0),
            current_ds: (0, 0),
            offset_start0: (0, 0),
            offset_start1: (0, 0),
        }
    }

    /// `cf2_glyphpath_hintPoint`: a character-space point in device space
    /// (the outer transform is the identity and there is no fractional
    /// translation).
    fn device(&self, map: &HintMap, (x, y): Point) -> DevicePoint {
        let scaled = |a: i32, b: i32| mul_fix(i64::from(a), i64::from(b)) as i32;
        (
            scaled(self.scale.x, x).wrapping_add(scaled(self.scale.c, y)),
            map.map(y),
        )
    }

    /// Builds the initial hint map from every stem, if not done yet.
    fn build_initial_map(&mut self, origin: i32) {
        if !self.initial_hint_map.valid {
            let stems = self.hints.count();
            let mut every = HintMask::default();
            self.initial_hint_map.build(
                None,
                &mut self.hints.horizontal,
                stems,
                &mut every,
                origin,
                &self.blues,
            );
        }
    }

    /// Rebuilds the hint map from the current mask.
    fn build_hint_map(&mut self) {
        self.build_initial_map(self.hint_origin);
        let stems = self.hints.count();
        self.hint_map.build(
            Some(&self.initial_hint_map),
            &mut self.hints.horizontal,
            stems,
            &mut self.hints.mask,
            self.hint_origin,
            &self.blues,
        );
    }

    /// The `cntrmask` operator: its mask bytes, and a counter hint map
    /// built and dropped, which only places the stems it uses.
    pub(super) fn counter_mask(&mut self, code: &mut Buffer<'_>) -> Result<(), Type1Error> {
        let stems = self.hints.count();
        let mut mask = HintMask::default();
        mask.read(code, stems)?;
        self.build_initial_map(0);
        let mut counters = HintMap::new(self.hinted, self.scale.y);
        counters.build(
            Some(&self.initial_hint_map),
            &mut self.hints.horizontal,
            stems,
            &mut mask,
            0,
            &self.blues,
        );
        Ok(())
    }

    /// `cf2_glyphpath_moveTo`.
    pub(super) fn move_to(&mut self, x: i32, y: i32, out: &mut Builder) -> Result<(), Type1Error> {
        self.close_open_path(out)?;
        self.current_cs = (x, y);
        self.start = (x, y);
        if !self.hint_map.valid || self.hints.mask.new {
            self.build_hint_map();
        }
        self.first_hint_map = self.hint_map.clone();
        Ok(())
    }

    /// Starts the subpath at its first element's start point.
    fn begin(&mut self, start: Point, second: Point, out: &mut Builder) -> Result<(), Type1Error> {
        if !self.open {
            self.push_move(start, out)?;
            self.open = true;
            self.offset_start1 = second;
        }
        if let Some(element) = self.queued {
            self.push_element(element, start, false, out)?;
        }
        Ok(())
    }

    /// `cf2_glyphpath_lineTo`: zero-length lines are dropped unless a new
    /// hint mask starts with them.
    pub(super) fn line_to(&mut self, x: i32, y: i32, out: &mut Builder) -> Result<(), Type1Error> {
        let new_map = self.hints.mask.new && !self.closing;
        if self.current_cs == (x, y) && !new_map {
            return Ok(());
        }
        let start = self.current_cs;
        self.begin(start, (x, y), out)?;
        self.queued = Some(Element::Line { end: (x, y) });
        if new_map {
            self.build_hint_map();
        }
        self.current_cs = (x, y);
        Ok(())
    }

    /// `cf2_glyphpath_curveTo`.
    pub(super) fn curve_to(
        &mut self,
        controls: [Point; 3],
        out: &mut Builder,
    ) -> Result<(), Type1Error> {
        let start = self.current_cs;
        self.begin(start, controls[0], out)?;
        self.queued = Some(Element::Cube { controls });
        if self.hints.mask.new {
            self.build_hint_map();
        }
        self.current_cs = controls[2];
        Ok(())
    }

    /// `cf2_glyphpath_closeOpenPath`: a closing line back to the start,
    /// then the queued element.
    pub(super) fn close_open_path(&mut self, out: &mut Builder) -> Result<(), Type1Error> {
        if self.open {
            self.closing = true;
            self.line_to(self.start.0, self.start.1, out)?;
            if let Some(element) = self.queued {
                self.push_element(element, self.offset_start0, true, out)?;
            }
            self.open = false;
            self.closing = false;
            self.queued = None;
        }
        Ok(())
    }

    /// `cf2_glyphpath_pushMove`.
    fn push_move(&mut self, start: Point, out: &mut Builder) -> Result<(), Type1Error> {
        if !self.hint_map.valid {
            // The first subpath has no move: finish setting up the map.
            self.move_to(self.start.0, self.start.1, out)?;
        }
        let point = self.device(&self.hint_map, start);
        out.move_to();
        self.current_ds = point;
        self.offset_start0 = start;
        Ok(())
    }

    /// `cf2_glyphpath_pushPrevElem` without darkening offsets: the queued
    /// element in device space, then a line to `next` if it does not start
    /// where the element ended (only possible across hint maps).
    fn push_element(
        &mut self,
        element: Element,
        next: Point,
        close: bool,
        out: &mut Builder,
    ) -> Result<(), Type1Error> {
        let map = if close {
            &self.first_hint_map
        } else {
            &self.hint_map
        };
        let from = self.current_ds;
        match element {
            Element::Line { end } => {
                let to = self.device(map, end);
                if to != from {
                    out.line_to(from, to)?;
                    self.current_ds = to;
                }
            }
            Element::Cube { controls } => {
                let points = controls.map(|point| self.device(&self.hint_map, point));
                out.cube_to(from, points)?;
                self.current_ds = points[2];
            }
        }
        let map = if close {
            &self.first_hint_map
        } else {
            &self.hint_map
        };
        let to = self.device(map, next);
        if to != self.current_ds {
            out.line_to(self.current_ds, to)?;
            self.current_ds = to;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed(x: i32, y: i32) -> Point {
        (x << 16, y << 16)
    }

    fn unhinted() -> GlyphPath {
        GlyphPath::new(Scale::UNHINTED, false, Blues::default(), 0)
    }

    #[test]
    fn a_closed_triangle_in_font_units() {
        let mut out = Builder::default();
        let mut path = unhinted();
        path.move_to(0, 0, &mut out).unwrap();
        let (x, y) = fixed(10, 0);
        path.line_to(x, y, &mut out).unwrap();
        // A zero-length line is dropped.
        path.line_to(x, y, &mut out).unwrap();
        path.curve_to([fixed(10, 5), fixed(5, 10), fixed(0, 10)], &mut out)
            .unwrap();
        path.close_open_path(&mut out).unwrap();
        let outline = out.finish();
        let points: Vec<(f64, f64)> = outline.points.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(
            points,
            [
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 5.0),
                (5.0, 10.0),
                (0.0, 10.0)
            ]
        );
        assert_eq!(outline.contour_ends, vec![5]);
    }

    #[test]
    fn a_new_hint_mask_keeps_zero_length_lines() {
        let mut out = Builder::default();
        let mut path = unhinted();
        path.move_to(0, 0, &mut out).unwrap();
        path.hints.reset();
        path.line_to(0, 0, &mut out).unwrap();
        assert!(path.open && path.queued.is_some());
        assert!(!path.hints.mask.new);
    }
}
