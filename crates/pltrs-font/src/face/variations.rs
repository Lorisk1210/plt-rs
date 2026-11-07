//! The instance of a variable face: its axes and named instances, and the
//! instance its outlines and metrics are at (see [`crate::var`]).

use super::Face;
use crate::error::{FontError, Result};
use crate::types::Tag;
use crate::var::{Avar, Gvar, Hvar, InstanceKind, Mvar, NamedInstance, VariationAxis, Variations};

/// The error for a face that has no variations.
const fn not_variable() -> FontError {
    FontError::MissingTable(Tag::new(b"fvar"))
}

impl<'a> Face<'a> {
    /// The face's variations when it is at an instance other than the
    /// default.
    pub(crate) fn varied(&self) -> Option<&Variations<'a>> {
        self.variations
            .as_deref()
            .filter(|variations| variations.kind != InstanceKind::Default)
    }

    /// The variation axes of a variable font, in `fvar` order; none for
    /// other fonts.
    pub fn variation_axes(&self) -> Vec<VariationAxis> {
        self.variations
            .as_ref()
            .map_or_else(Vec::new, |variations| variations.fvar.variation_axes())
    }

    /// The named instances of a variable font, in `fvar` order.
    pub fn named_instances(&self) -> Vec<NamedInstance> {
        self.variations
            .as_ref()
            .map_or_else(Vec::new, |variations| variations.fvar.named_instances())
    }

    /// The face's normalized coordinates (-1 to 1 per axis, 0 at the
    /// default instance), after `avar`.
    pub fn normalized_coordinates(&self) -> Vec<f64> {
        self.variations
            .as_ref()
            .map_or_else(Vec::new, |variations| {
                variations
                    .coords
                    .iter()
                    .map(|&coord| coord as f64 / 65536.0)
                    .collect()
            })
    }

    /// Sets the instance by design coordinates: `values` gives axes by tag
    /// (the others keep their defaults), as FreeType's
    /// `FT_Set_Var_Design_Coordinates` sets them on a face opened anew.
    /// Values beyond an axis' range are clamped to it.
    ///
    /// # Errors
    ///
    /// When the face is no variable font, an axis tag is not the face's,
    /// or its glyph variations (`gvar`) are malformed.
    pub fn set_variations(&mut self, values: &[(Tag, f64)]) -> Result<()> {
        let variations = self.variations.as_ref().ok_or_else(not_variable)?;
        let mut design: Vec<i64> = variations
            .fvar
            .axes
            .iter()
            .map(|&(_, [_, default, _], _, _)| default)
            .collect();
        for &(tag, value) in values {
            let axis = variations
                .fvar
                .axes
                .iter()
                .position(|&(axis, ..)| axis == tag)
                .ok_or(FontError::UnknownName {
                    kind: "variation axis",
                    name: tag.to_string(),
                })?;
            design[axis] = (value * 65536.0).round() as i64;
        }
        self.set_instance(InstanceKind::Design, &design)
    }

    /// Sets named instance `index` (in [`named_instances`] order), as
    /// FreeType's `FT_Set_Named_Instance` sets it on a face opened anew.
    ///
    /// # Errors
    ///
    /// When the face is no variable font or has no such instance, or its
    /// glyph variations are malformed.
    ///
    /// [`named_instances`]: Self::named_instances
    pub fn set_named_instance(&mut self, index: usize) -> Result<()> {
        let variations = self.variations.as_ref().ok_or_else(not_variable)?;
        let design = variations
            .fvar
            .instances
            .get(index)
            .map(|(_, coords, _)| coords.clone())
            .ok_or(FontError::UnknownName {
                kind: "named instance",
                name: index.to_string(),
            })?;
        self.set_instance(InstanceKind::Named, &design)
    }

    /// Puts the face at its default instance.
    pub fn reset_variations(&mut self) {
        if let Some(variations) = &mut self.variations {
            variations.kind = InstanceKind::Default;
            variations.coords.fill(0);
        }
        // The default instance of a CFF2 table parsed before parses again.
        let _ = self.reparse_outlines();
    }

    /// Normalizes `design` and puts the face at the instance, reading the
    /// variation tables it needs.
    fn set_instance(&mut self, kind: InstanceKind, design: &[i64]) -> Result<()> {
        let table = |tag: &[u8; 4]| self.table(Tag::new(tag));
        let (avar, gvar, hvar, mvar) = (
            table(b"avar"),
            table(b"gvar"),
            table(b"HVAR"),
            table(b"MVAR"),
        );
        let variations = self.variations.as_mut().ok_or_else(not_variable)?;
        let axes = variations.fvar.axes.len();
        let avar = avar.and_then(|data| Avar::parse(data, axes).ok());
        variations.coords = variations.normalize(avar.as_ref(), design);
        variations.kind = kind;
        variations.gvar = gvar.map(|data| Gvar::parse(data, axes)).transpose()?;
        variations.hvar = hvar.and_then(|data| Hvar::parse(data, axes).ok());
        variations.mvar = mvar.and_then(|data| Mvar::parse(data, axes).ok());
        self.reparse_outlines()
    }

    /// Parses the outlines again at the face's instance (CFF2 tables blend
    /// their Private DICTs when they are read).
    fn reparse_outlines(&mut self) -> Result<()> {
        if !self.is_cff() {
            return Ok(());
        }
        let coords = self
            .variations
            .as_ref()
            .map_or_else(Vec::new, |variations| variations.coords.clone());
        self.outlines =
            Self::outlines(&self.directory, &self.head, self.index, self.file, &coords)?;
        Ok(())
    }
}
