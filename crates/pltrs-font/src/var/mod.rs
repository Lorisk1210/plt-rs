//! Font variations, as FreeType 2.14's `ttgxvar.c` applies them to a face
//! at an instance of a variable font: the axes and named instances
//! (`fvar`), the normalization of design coordinates with `avar`
//! (versions 1 and 2), the glyph deltas of TrueType outlines (`gvar`, with
//! the points without deltas interpolated), the advance widths (`HVAR`)
//! and the face's line metrics (`MVAR`). CFF2 outlines blend at the
//! instance's coordinates in [`crate::cff`].
//!
//! FreeType varies a face differently for a named instance
//! (`FT_Set_Named_Instance`) and for design coordinates
//! (`FT_Set_Var_Design_Coordinates`), on a face opened anew: both move the
//! glyph points and the line metrics, but only design coordinates vary
//! the advance widths through `HVAR` (named instances keep the `hmtx`
//! advances of CFF2 fonts and move the phantom points of TrueType ones).
//! [`InstanceKind`] keeps which it is.
//!
//! The bytecode hints glyphs at an instance with its control values varied
//! by `cvar`. Left out: vertical metrics (`VVAR`).

mod avar;
mod cvar;
mod fvar;
mod gvar;
mod metrics;
mod store;

pub use fvar::{NamedInstance, VariationAxis};

pub(crate) use avar::Avar;
pub(crate) use cvar::vary_cvt;
pub(crate) use fvar::Fvar;
pub(crate) use gvar::{Gvar, fixed_to_f26dot6, fixed_to_int};
pub(crate) use metrics::{Hvar, LineMetricDeltas, Mvar};
pub(crate) use store::{AxisRange, axis_range, region_scalar};

use std::cmp::Ordering;

use crate::fixed::div_fix;
use crate::tables::glyf::GlyphVariation;
use crate::types::GlyphId;

/// How a face's instance was chosen, which decides what FreeType varies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum InstanceKind {
    /// The default instance: nothing varies.
    #[default]
    Default,
    /// A named instance.
    Named,
    /// Design coordinates.
    Design,
}

/// The variation tables of a face and the instance it is at.
#[derive(Clone, Debug)]
pub(crate) struct Variations<'a> {
    pub(crate) fvar: Fvar,
    pub(crate) kind: InstanceKind,
    /// The normalized coordinates, in 16.16, one per axis.
    pub(crate) coords: Vec<i64>,
    pub(crate) gvar: Option<Gvar<'a>>,
    pub(crate) hvar: Option<Hvar<'a>>,
    pub(crate) mvar: Option<Mvar<'a>>,
}

impl Variations<'_> {
    /// A face of the axes of `fvar` at its default instance.
    pub(crate) fn new(fvar: Fvar) -> Self {
        let coords = vec![0; fvar.axes.len()];
        Self {
            fvar,
            kind: InstanceKind::Default,
            coords,
            gvar: None,
            hvar: None,
            mvar: None,
        }
    }

    /// `ft_var_to_normalized`: design coordinates (16.16, one per axis)
    /// normalized to -1, 0 and 1 at the axes' minimum, default and
    /// maximum, then remapped by `avar`.
    pub(crate) fn normalize(&self, avar: Option<&Avar<'_>>, design: &[i64]) -> Vec<i64> {
        let mut normalized: Vec<i64> = self
            .fvar
            .axes
            .iter()
            .zip(design)
            .map(
                |(&(_, [minimum, default, maximum], _, _), &coord)| match coord.cmp(&default) {
                    Ordering::Greater if coord >= maximum => 0x10000,
                    Ordering::Greater => div_fix(coord - default, maximum - default),
                    Ordering::Less if coord <= minimum => -0x10000,
                    Ordering::Less => div_fix(coord - default, default - minimum),
                    Ordering::Equal => 0,
                },
            )
            .collect();
        normalized.resize(self.fvar.axes.len(), 0);
        if let Some(avar) = avar {
            avar.apply(&mut normalized);
        }
        normalized
    }

    /// The deltas `MVAR` gives the line metrics at the instance.
    pub(crate) fn line_metrics(&self) -> LineMetricDeltas {
        match (&self.mvar, self.kind) {
            (Some(mvar), InstanceKind::Named | InstanceKind::Design) => {
                mvar.line_metrics(&self.coords)
            }
            _ => LineMetricDeltas::default(),
        }
    }

    /// The delta `HVAR` gives `glyph`'s advance width, for design
    /// coordinates.
    pub(crate) fn advance_delta(&self, glyph: GlyphId) -> Option<i64> {
        let hvar = self.hvar.as_ref()?;
        (self.kind == InstanceKind::Design).then(|| hvar.advance_delta(glyph, &self.coords))
    }

    /// What varies the TrueType glyphs at the instance, if anything does.
    pub(crate) fn glyph_variation(&self) -> Option<GlyphVariation<'_>> {
        let gvar = self.gvar.as_ref()?;
        (self.kind != InstanceKind::Default).then(|| GlyphVariation {
            gvar,
            coords: &self.coords,
            fixed_phantoms: self.kind == InstanceKind::Design && self.hvar.is_some(),
        })
    }
}
