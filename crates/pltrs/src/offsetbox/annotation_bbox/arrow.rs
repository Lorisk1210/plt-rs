//! The arrow of an annotation box as a patch (matplotlib's
//! `AnnotationBbox.arrow_patch`), as an annotation's arrow is
//! ([`Annotation::arrow_patch`](crate::text::Annotation::arrow_patch)):
//! the box draws it as a part of its own, from the frame to the annotated
//! point, and hands out a loose patch of its look, whose changes it takes
//! back.

use pltrs_core::{Bbox, Path};

use super::{AnnotationBbox, AnnotationBboxArtist};
use crate::artist::{ArtistHandle, LayoutContext, RawHandle};
use crate::coords::annotation_shown;
use crate::patches::{FancyArrowPatch, Patch};
use crate::text::AnnotationArrow;

impl AnnotationBboxArtist {
    /// The outline of the arrow as drawn in `layout`, in display pixels;
    /// `None` without an arrow, while the annotated point is hidden or
    /// when the arrow cannot be placed.
    fn shown_arrow_path(&self, layout: LayoutContext<'_>) -> Option<Path> {
        let geometry = self.geometry(layout).ok()?;
        annotation_shown(
            layout,
            self.annotation_clip,
            &self.xycoords,
            self.axes,
            geometry.point,
        )
        .then(|| self.arrow_path(layout, &geometry))
        .flatten()
    }
}

impl AnnotationBbox {
    /// A loose patch with the look of the box's arrow, `None` without an
    /// arrow (matplotlib's `arrow_patch`); its changes reach the arrow
    /// through [`AnnotationBbox::restyle_arrow`].
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::offsetbox::{AnnotationBboxOptions, TextArea};
    /// use pltrs::text::TextProps;
    /// let ax = Figure::new().subplot();
    /// let options = AnnotationBboxOptions {
    ///     xybox: Some((0.2, 0.2).into()),
    ///     arrowprops: Some("->".parse()?),
    ///     ..AnnotationBboxOptions::default()
    /// };
    /// let note = ax.add_annotation_bbox(
    ///     TextArea::from_text("peak", TextProps::default()),
    ///     (0.5, 0.7),
    ///     options,
    /// )?;
    /// let arrow = note.arrow_patch().expect("an arrow");
    /// arrow.set_alpha(Some(0.5));
    /// note.restyle_arrow(&arrow);
    /// assert_eq!(note.arrow_patch().map(|a| a.alpha()), Some(Some(0.5)));
    /// assert!(note.arrow_extent().is_some_and(|extent| extent.width() > 0.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn arrow_patch(&self) -> Option<Patch<FancyArrowPatch>> {
        self.raw()
            .with::<AnnotationBboxArtist, _>(|ab| ab.arrow.as_ref().map(AnnotationArrow::to_patch))
    }

    /// Gives the arrow the look of `patch` (see
    /// [`AnnotationBbox::arrow_patch`]); without an arrow, nothing
    /// changes.
    pub fn restyle_arrow(&self, patch: &Patch<FancyArrowPatch>) -> &Self {
        self.raw().with_mut::<AnnotationBboxArtist, _>(|ab| {
            if let Some(arrow) = &mut ab.arrow {
                arrow.restyle_from(patch);
            }
        });
        self
    }

    /// The display box of the arrow in the figure's current layout (laid
    /// out first), `None` without an arrow or while the annotated point is
    /// hidden (matplotlib's `arrow_patch.get_window_extent()`).
    pub fn arrow_extent(&self) -> Option<Bbox> {
        self.arrow_display_path().map(|path| path.extents())
    }

    /// The outline of the arrow in display pixels in the figure's current
    /// layout (laid out first), `None` without an arrow, while the
    /// annotated point is hidden or when the arrow cannot be placed
    /// (matplotlib's `arrow_patch.get_path()` once the box placed its
    /// arrow).
    pub fn arrow_display_path(&self) -> Option<Path> {
        let raw: &RawHandle = self.raw();
        let mut state = raw.figure_state().borrow_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        state
            .store
            .get_as::<AnnotationBboxArtist>(raw.id())?
            .shown_arrow_path(layout)
    }
}
