//! Glyphs at an instance of a variable font: the `gvar` deltas applied to
//! a glyph's points and phantom points in font units, as
//! `TT_Vary_Apply_Glyph_Deltas` applies them, giving the rounded points
//! and the unrounded ones FreeType scales from; simple glyphs drawn from
//! them, and composites' offsets varied.

use super::{Component, Phantoms, Placement, Scaling, parse_simple, push_simple};
use crate::error::Result;
use crate::fixed::mul_fix;
use crate::outline::Outline;
use crate::types::GlyphId;
use crate::var::{Gvar, fixed_to_f26dot6, fixed_to_int};

/// What varies a face's glyphs at an instance.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphVariation<'v> {
    /// The deltas.
    pub(crate) gvar: &'v Gvar<'v>,
    /// The instance's normalized coordinates.
    pub(crate) coords: &'v [i64],
    /// Whether the phantom points keep still (`HVAR` varies the advances
    /// instead).
    pub(crate) fixed_phantoms: bool,
}

/// Points and phantom points varied: rounded to font units, and in 26.6
/// font units before rounding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Varied {
    pub(crate) rounded: Vec<(i64, i64)>,
    pub(crate) unrounded: Vec<(i64, i64)>,
}

impl Varied {
    /// The linear advance of points whose phantom points start at
    /// `count`: the unrounded phantom points' distance rounded to whole
    /// units (`FT_PIX_ROUND` of the 26.6 distance).
    pub(crate) fn linear(&self, count: usize) -> i64 {
        let distance = self.unrounded[count + 1].0 - self.unrounded[count].0;
        crate::fixed::pix_round(distance) / 64
    }
}

impl GlyphVariation<'_> {
    /// `glyph`'s `points` (font units, contours ending at `ends`) and its
    /// two horizontal phantom points `pp1` and `pp2` (x only) varied; the
    /// result holds the points, then the phantom points.
    pub(crate) fn vary(
        &self,
        glyph: GlyphId,
        points: &[(i64, i64)],
        ends: &[usize],
        (pp1, pp2): (i64, i64),
    ) -> Result<Varied> {
        let mut all = points.to_vec();
        // The vertical phantom points vary nothing pltrs reads.
        all.extend([(pp1, 0), (pp2, 0), (0, 0), (0, 0)]);
        let unrounded_of = |&(x, y): &(i64, i64)| (x * 64, y * 64);
        let Some(mut deltas) = self.gvar.deltas(glyph, &all, ends, self.coords)? else {
            return Ok(Varied {
                unrounded: all.iter().map(unrounded_of).collect(),
                rounded: all,
            });
        };
        if self.fixed_phantoms {
            let count = deltas.len();
            deltas[count - 4..].fill((0, 0));
        }
        let rounded = all
            .iter()
            .zip(&deltas)
            .map(|(&(x, y), &(dx, dy))| (x + fixed_to_int(dx), y + fixed_to_int(dy)))
            .collect();
        let unrounded = all
            .iter()
            .zip(&deltas)
            .map(|(point, &(dx, dy))| {
                let (x, y) = unrounded_of(point);
                (x + fixed_to_f26dot6(dx), y + fixed_to_f26dot6(dy))
            })
            .collect();
        Ok(Varied { rounded, unrounded })
    }
}

/// Varies the offsets of `components` (those given as x and y values) and
/// the phantom points of their composite `glyph`; returns the varied
/// phantom points (font units) and the linear advance.
pub(crate) fn vary_components(
    components: &mut [Component],
    glyph: GlyphId,
    variation: &GlyphVariation<'_>,
    phantoms: (i64, i64),
) -> Result<((i64, i64), i64)> {
    let offsets: Vec<(i64, i64)> = components
        .iter()
        .map(|component| match component.placement {
            Placement::Offset(x, y) => (x, y),
            Placement::Anchor(parent, child) => (parent as i64, child as i64),
        })
        .collect();
    let ends: Vec<usize> = (0..offsets.len()).collect();
    let varied = variation.vary(glyph, &offsets, &ends, phantoms)?;
    for (component, &(x, y)) in components.iter_mut().zip(&varied.rounded) {
        if let Placement::Offset(..) = component.placement {
            component.placement = Placement::Offset(i64::from(x as i16), i64::from(y as i16));
        }
    }
    let count = offsets.len();
    let pp = (varied.rounded[count].0, varied.rounded[count + 1].0);
    Ok((pp, varied.linear(count)))
}

/// Decodes a simple glyph body into `out` at an instance: the points
/// varied in font units, then scaled from their unrounded positions as
/// FreeType scales them; returns the varied phantom points.
pub(super) fn load_simple_varied(
    body: &[u8],
    contours: usize,
    id: GlyphId,
    scaling: Scaling,
    variation: &GlyphVariation<'_>,
    phantoms: (i64, i64),
    out: &mut Outline,
) -> Result<Phantoms> {
    let glyph = parse_simple(body, contours)?;
    let points: Vec<(i64, i64)> = glyph
        .points
        .iter()
        .map(|point| (point.x, point.y))
        .collect();
    let ends: Vec<usize> = glyph.ends.iter().map(usize::from).collect();
    let varied = variation.vary(id, &points, &ends, phantoms)?;
    let place = |index: usize| -> (f64, f64) {
        match scaling {
            Scaling::Units | Scaling::Unscaled => {
                let (x, y) = varied.rounded[index];
                (x as f64, y as f64)
            }
            Scaling::Fixed { x_scale, y_scale } => {
                let (x, y) = varied.unrounded[index];
                let scaled = |value: i64, scale: i64| ((mul_fix(value, scale) + 32) >> 6) as f64;
                (scaled(x, x_scale), scaled(y, y_scale))
            }
        }
    };
    let placed: Vec<(f64, f64)> = (0..points.len()).map(place).collect();
    push_simple(&glyph, &placed, out)?;
    let count = points.len();
    Ok(Phantoms {
        origin: place(count).0,
        advance: varied.rounded[count + 1].0 - varied.rounded[count].0,
        linear: varied.linear(count),
    })
}
