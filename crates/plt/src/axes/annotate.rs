//! Annotations (`Axes::annotate`).

use std::rc::Rc;

use plt_core::Point;

use super::Axes;
use crate::artist::{Artist, ArtistHandle, RawHandle};
use crate::coords::CoordError;
use crate::text::{AnnotateOptions, Annotation, AnnotationState, TextArtist};
use crate::transforms::Transform;

impl Axes {
    /// Annotates the point `xy` with `text`, placed and optionally joined
    /// to the point by an arrow as `options` say (matplotlib's `annotate`).
    /// Fails when `options.xycoords` is an offset.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::patches::ArrowStyle;
    /// use plt::text::AnnotateOptions;
    /// let ax = Figure::new().subplot();
    /// let note = ax.annotate("peak", (1.0, 1.0), AnnotateOptions {
    ///     xytext: Some((0.2, 0.5).into()),
    ///     textcoords: Some("axes fraction".parse()?),
    ///     arrowprops: Some("->".parse::<ArrowStyle>()?.into()),
    ///     ..AnnotateOptions::default()
    /// })?;
    /// assert_eq!(note.text(), "peak");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn annotate(
        &self,
        text: impl Into<String>,
        xy: impl Into<Point>,
        options: AnnotateOptions,
    ) -> Result<Annotation, CoordError> {
        let xy = xy.into();
        let rc = crate::rc::params();
        let xytext = options.xytext.unwrap_or(xy);
        let clip_on = options.clip_on;
        let state = AnnotationState::new(xy, options, &rc)?;
        let mut artist = TextArtist::new(xytext, text, rc);
        artist.annotation = Some(Box::new(state));
        let base = artist.base_mut();
        base.transform = Some(Transform::Display);
        if clip_on {
            base.clip_box = Some(self.id());
        }
        let figure = self.raw().figure_state();
        let id = figure.borrow_mut().adopt(self.id(), Box::new(artist));
        Ok(Annotation::from_raw(RawHandle::new(Rc::clone(figure), id)))
    }
}
