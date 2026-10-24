//! Annotations (`Axes::annotate`).

use std::rc::Rc;

use plt_core::Point;

use super::{Axes, Axis};
use crate::artist::{Artist, ArtistHandle, RawHandle};
use crate::coords::{CoordError, Coords};
use crate::text::{AnnotateOptions, Annotation, AnnotationState, TextArtist};
use crate::transforms::Transform;
use crate::units::{AxisValue, DataPoint};

impl Axes {
    /// Annotates the point `xy` with `text`, placed and optionally joined
    /// to the point by an arrow as `options` say (matplotlib's `annotate`).
    /// Fails when `options.xycoords` is an offset.
    ///
    /// The point's coordinates may be dates or categories where they are
    /// data coordinates, converted on their axis as matplotlib converts
    /// them; `options.xytext` holds axis numbers (see
    /// [`Axis::convert_value`]).
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
        xy: impl Into<DataPoint>,
        options: AnnotateOptions,
    ) -> Result<Annotation, CoordError> {
        let DataPoint { x, y } = xy.into();
        let (x_data, y_data) = data_components(&options.xycoords);
        let xy = Point::new(
            component(&self.xaxis(), x, x_data),
            component(&self.yaxis(), y, y_data),
        );
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

/// Whether the x and the y of points in `coords` are data coordinates.
fn data_components(coords: &Coords) -> (bool, bool) {
    match coords {
        Coords::Data => (true, true),
        Coords::Blended(x, y) => (matches!(**x, Coords::Data), matches!(**y, Coords::Data)),
        _ => (false, false),
    }
}

/// `value` as a coordinate along `axis`: converted on the axis when it is
/// a data coordinate (matplotlib's `convert_xunits` in `_get_xy`), a plain
/// number otherwise; dates and categories always go through the axis.
fn component(axis: &Axis, value: AxisValue, data: bool) -> f64 {
    match value {
        AxisValue::Number(number) if !data => number,
        value => axis.convert_value(value),
    }
}
