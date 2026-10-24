//! Annotations (`Axes::annotate`).

use std::rc::Rc;

use plt_core::Point;

use super::{Axes, Axis};
use crate::artist::{Artist, ArtistHandle, RawHandle};
use crate::coords::{CoordError, Coords};
use crate::text::{AnnotateOptions, Annotation, AnnotationState, TextArtist};
use crate::transforms::Transform;
use crate::units::{AxisValue, DataPoint, UnitSources};

impl Axes {
    /// Annotates the point `xy` with `text`, placed and optionally joined
    /// to the point by an arrow as `options` say (matplotlib's `annotate`).
    /// Fails when `options.xycoords` is an offset.
    ///
    /// The coordinates of the point, and of `options.xytext`, may be dates,
    /// categories or user values where they are data coordinates,
    /// converted on their axis as matplotlib's annotations convert them
    /// when drawn: without giving the axis a converter, tick helpers or a
    /// unit (which plotting data does), and again, for user values, when
    /// the axis changes unit. As for [`Axes::text`], a category name
    /// needs an axis with categories.
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
        let xy = xy.into();
        let xytext = options.xytext.clone().unwrap_or_else(|| xy.clone());
        let textcoords = options
            .textcoords
            .clone()
            .unwrap_or_else(|| options.xycoords.clone());
        let (point, point_sources) = self.place(&xy, &options.xycoords);
        let (position, position_sources) = self.place(&xytext, &textcoords);
        let rc = crate::rc::params();
        let clip_on = options.clip_on;
        let mut state = AnnotationState::new(point, options, &rc)?;
        state.unit_sources = point_sources;
        let mut artist = TextArtist::new(position, text, rc);
        artist.unit_sources = position_sources;
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

    /// `point` in `coords` as numbers, with the user values they came
    /// from.
    fn place(&self, point: &DataPoint, coords: &Coords) -> (Point, UnitSources) {
        let (x_data, y_data) = data_components(coords);
        let numbers = Point::new(
            component(&self.xaxis(), &point.x, x_data),
            component(&self.yaxis(), &point.y, y_data),
        );
        (numbers, UnitSources::of_point(point))
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

/// `value` as a coordinate along `axis`: converted on the axis as
/// annotations convert their values when drawn when it is a data
/// coordinate (matplotlib's `convert_xunits` in `_get_xy`), a plain number
/// otherwise; dates, categories and user values always go through the
/// axis.
fn component(axis: &Axis, value: &AxisValue, data: bool) -> f64 {
    match value {
        &AxisValue::Number(number) if !data => number,
        value => axis.convert_when_drawn(value),
    }
}
