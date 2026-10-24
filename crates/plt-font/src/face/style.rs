//! The face's style descriptors: FreeType's style flags, italic angle,
//! weight and width classes and underline metrics.

use super::Face;

/// FreeType's style flags.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct StyleFlags {
    /// The face is italic or oblique.
    pub italic: bool,
    /// The face is bold.
    pub bold: bool,
}

impl Face<'_> {
    /// FreeType's style flags: from `OS/2` `fsSelection` when present, else
    /// from `head.macStyle`.
    pub fn style_flags(&self) -> StyleFlags {
        match &self.os2 {
            Some(os2) => StyleFlags {
                italic: os2.is_italic(),
                bold: os2.is_bold(),
            },
            None => StyleFlags {
                italic: self.head.mac_style & 2 != 0,
                bold: self.head.mac_style & 1 != 0,
            },
        }
    }

    /// Italic angle in degrees (0 when there is no `post` table).
    pub fn italic_angle(&self) -> f64 {
        self.post().map_or(0.0, |post| post.italic_angle)
    }

    /// Visual weight from `OS/2` (400 regular, 700 bold), if present.
    pub fn weight_class(&self) -> Option<u16> {
        self.os2.as_ref().map(|os2| os2.weight_class)
    }

    /// Width class from `OS/2` (5 normal, lower is condensed), if present.
    pub fn width_class(&self) -> Option<u16> {
        self.os2.as_ref().map(|os2| os2.width_class)
    }

    /// Underline position (top edge, as stored in `post`) and thickness in
    /// font units. FreeType reports the centre instead:
    /// `position - thickness / 2`.
    pub fn underline_metrics(&self) -> Option<(i16, i16)> {
        self.post()
            .map(|post| (post.underline_position, post.underline_thickness))
    }
}
