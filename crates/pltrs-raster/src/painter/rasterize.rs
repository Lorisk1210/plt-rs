//! The painter's operations that rasterize into a fresh buffer rather
//! than a target: hatch tiles, marker stamps and coverage masks.

use pltrs_core::{Path, Rgba};

use super::Painter;
use crate::clip::ClipBox;
use crate::color::Rgba8;
use crate::error::RasterError;
use crate::geometry::feed_path;
use crate::mask::Mask;
use crate::pixmap::Pixmap;
use crate::render::marker::MarkerStamp;
use crate::style::{Antialias, FillRule, LineCap, StrokeStyle};

impl Painter {
    /// Renders a `size × size` hatch tile the way matplotlib does: the
    /// tile is cleared to `background` (matplotlib uses
    /// [`Rgba8::TRANSPARENT_WHITE`]), then `hatch` (tile pixel coordinates)
    /// is filled and stroked in `color` with square caps and Agg's default
    /// joins, without clipping.
    pub fn hatch_tile(
        &mut self,
        size: u32,
        hatch: &Path,
        color: Rgba,
        linewidth: f64,
        background: Rgba8,
    ) -> Result<Pixmap, RasterError> {
        let mut tile = Pixmap::filled(size, size, background)?;
        let color = Rgba8::from(color);
        self.rasterizer.reset_clipping();
        self.rasterizer.set_fill_rule(FillRule::NonZero);
        feed_path(hatch, &mut self.rasterizer);
        self.render_solid(&mut tile, color, Antialias::On, None)?;
        self.rasterizer.reset();
        self.add_stroke(
            hatch,
            &StrokeStyle::new(linewidth).with_cap(LineCap::Square),
        );
        self.render_solid(&mut tile, color, Antialias::On, None)?;
        Ok(tile)
    }

    /// Rasterizes a marker once: the fill of `path` (if `fill`) and its
    /// stroke (if `stroke` is given), both anti-aliased and unclipped, as
    /// matplotlib's `draw_markers` does. Stamp it with [`MarkerStamp::draw`].
    pub fn marker_stamp(
        &mut self,
        path: &Path,
        fill: bool,
        stroke: Option<&StrokeStyle>,
    ) -> Result<MarkerStamp, RasterError> {
        self.rasterizer.reset_clipping();
        self.rasterizer.set_fill_rule(FillRule::NonZero);
        let mut fill_rows = Vec::new();
        if fill {
            feed_path(path, &mut self.rasterizer);
            self.rasterizer
                .sweep(&mut self.scanline, |sl| fill_rows.push(sl.clone()))?;
            self.rasterizer.reset();
        }
        let mut stroke_rows = Vec::new();
        if let Some(style) = stroke {
            self.add_stroke(path, style);
            if self.rasterizer.covers_cells() {
                self.rasterizer
                    .sweep(&mut self.scanline, |sl| stroke_rows.push(sl.clone()))?;
            } else {
                // Agg's scanline storage keeps the fill when the stroke
                // covers no cell (a zero-width outline inside one pixel):
                // matplotlib's `draw_markers` stamps the fill again, in the
                // edge colour.
                stroke_rows.clone_from(&fill_rows);
            }
        }
        Ok(MarkerStamp::new(fill_rows, stroke_rows))
    }

    /// Rasterizes `path` into a `width × height` coverage mask — for clip
    /// paths (matplotlib's `render_clippath`) and glyph bitmaps. The
    /// geometry is clipped to `rect` if given, otherwise not at all:
    /// matplotlib rasterizes clip paths with the current clip box (or the
    /// canvas bounds) for paths, images and triangles, but unclipped for
    /// markers. With [`Antialias::Off`] every touched pixel becomes 255.
    pub fn coverage_mask(
        &mut self,
        path: &Path,
        width: u32,
        height: u32,
        rule: FillRule,
        aa: Antialias,
        rect: Option<ClipBox>,
    ) -> Result<Mask, RasterError> {
        let mut mask = Mask::new(width, height)?;
        if let Some(rect) = rect {
            self.set_clip(rect, rule);
        } else {
            self.rasterizer.reset_clipping();
            self.rasterizer.set_fill_rule(rule);
        }
        feed_path(path, &mut self.rasterizer);
        let full = ClipBox::full(width, height);
        let Some(bounds) = rect.map_or(Some(full), |r| r.within(width, height)) else {
            return Ok(mask);
        };
        self.rasterizer.sweep(&mut self.scanline, |sl| {
            let Some(row) = u32::try_from(sl.y()).ok().and_then(|y| mask.row_mut(y)) else {
                return;
            };
            for span in sl.spans() {
                for i in 0..span.len() {
                    let x = span.x + i as i32;
                    if x >= bounds.x0() && x < bounds.x1() {
                        row[x as usize] = if aa == Antialias::Off {
                            255
                        } else {
                            span.cover_at(i)
                        };
                    }
                }
            }
        })?;
        Ok(mask)
    }
}
