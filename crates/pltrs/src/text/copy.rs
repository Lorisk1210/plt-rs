//! Copying the look of one text to another (matplotlib's
//! `Text.update_from`).

use super::{Text, TextArtist};
use crate::artist::ArtistHandle;

impl Text {
    /// Gives the text the common artist properties of `other`
    /// ([`RawHandle::update_from`](crate::artist::RawHandle::update_from))
    /// and its look: colour, alignments, font, TeX, rotation and whether
    /// its transform turns it, picker, line spacing and anti-aliasing
    /// (matplotlib's `Text.update_from`); whether the transform and the
    /// clip regions were copied too. The string and the position stay.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let title = fig.text(0.5, 0.9, "title");
    /// title.set_fontsize(20.0).set_rotation(30.0);
    /// let note = fig.text(0.1, 0.1, "note");
    /// assert!(note.update_from(&title));
    /// assert_eq!((note.font().size, note.rotation()), (20.0, 30.0));
    /// assert_eq!(note.text(), "note");
    /// ```
    pub fn update_from(&self, other: &Text) -> bool {
        let references = self.raw().update_from(other.raw());
        let picker = other.picker();
        let look = other.raw().with::<TextArtist, _>(|t| {
            (
                t.color.clone(),
                t.multialign,
                t.valign,
                t.halign,
                t.font.clone(),
                t.usetex,
                (t.rotation, t.rotation_reduction),
                t.transform_rotates_text,
                t.linespacing,
                t.antialiased,
            )
        });
        self.raw().with_mut::<TextArtist, _>(|t| {
            (
                t.color,
                t.multialign,
                t.valign,
                t.halign,
                t.font,
                t.usetex,
                (t.rotation, t.rotation_reduction),
                t.transform_rotates_text,
                t.linespacing,
                t.antialiased,
            ) = look;
        });
        self.set_picker(picker);
        references
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::{ArtistHandle, Picker};
    use crate::figure::Figure;
    use crate::text::{HAlign, VAlign};

    #[test]
    fn the_look_and_the_picker_are_copied() {
        let fig = Figure::new();
        let source = fig.text(0.5, 0.5, "source");
        source
            .set_halign(HAlign::Right)
            .set_valign(VAlign::Top)
            .set_usetex(false)
            .set_picker(Some(Picker::Radius(3.0)));
        let other = Figure::new();
        let target = other.text(0.0, 0.0, "target");
        assert!(!target.update_from(&source));
        assert_eq!(
            (target.halign(), target.valign()),
            (HAlign::Right, VAlign::Top)
        );
        assert!(matches!(target.picker(), Some(Picker::Radius(r)) if (r - 3.0).abs() < 1e-12));
        assert_eq!(target.text(), "target");
    }
}
