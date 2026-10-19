//! Arrow plots of vector fields and their keys (matplotlib's `quiver` and
//! `quiverkey`).

use std::rc::Rc;

use plt_core::Point;

use super::children::DataUpdate;
use super::data_coords::data_offsets;
use super::{Axes, DataLimits, PlotError};
use crate::artist::{Artist, ArtistHandle, ArtistId};
use crate::collections::{Collection, CollectionArtist, CollectionKind};
use crate::quiver::{
    Angles, ArrowSizing, KeyCoordinates, KeyVector, Quiver, QuiverGeometry, QuiverKey,
    QuiverKeyArtist, QuiverKeyOptions, QuiverOptions, VectorField,
};
use crate::transforms::Transform;

impl Axes {
    /// Draws an arrow for every vector of `field` (matplotlib's `quiver`).
    /// The arrows are sized when the figure is drawn, from the axes
    /// geometry at that time.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::quiver::{QuiverOptions, VectorField};
    /// let ax = Figure::new().subplot();
    /// let field = VectorField::new([0.0, 1.0], [0.0, 0.0], [1.0, 0.5], [0.0, 1.0]);
    /// let q = ax.quiver(field, QuiverOptions::default())?;
    /// assert_eq!(q.len(), 2);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the positions, components, colour values or angles disagree
    /// in size, or the colours cannot be mapped.
    pub fn quiver(&self, field: VectorField, options: QuiverOptions) -> Result<Quiver, PlotError> {
        let rc = crate::rc::params();
        let scalar_components = field.u.len() == 1 && field.v.len() == 1;
        let mut points = field.into_points(&rc)?;
        let n = points.xy.len();
        if let Angles::Degrees(degrees) = &options.angles
            && degrees.len() != 1
            && degrees.len() != n
        {
            return Err(PlotError::Broadcast {
                argument: "angles",
                len: degrees.len(),
                points: n,
            });
        }
        let mask: Vec<bool> = (0..n).map(|i| points.masked(i)).collect();
        let mapping = points.colors.take().map(|mut mapping| {
            for (value, &masked) in mapping.values.iter_mut().zip(&mask) {
                if masked {
                    *value = None;
                }
            }
            mapping
        });
        let mut collection = CollectionArtist::new(CollectionKind::Poly, Vec::new(), rc);
        collection.facecolors = Some(vec![options.color]);
        collection.linewidths = Some(vec![0.0]);
        options.props.apply(&mut collection);
        collection.base_mut().transform = Some(self.trans_data());
        collection.offsets.clone_from(&points.xy);
        let positions = options.transform.unwrap_or_else(|| self.trans_data());
        let data_positions = data_offsets(&positions, self.id(), &points.xy);
        collection.offset_transform = Some(positions);
        let sizing = ArrowSizing {
            scale: options.scale,
            scale_units: options.scale_units,
            units: options.units,
            angles: options.angles,
            width: options.width,
            shape: options.shape,
            pivot: options.pivot,
        };
        let mut limits = DataLimits::NULL;
        limits.update_from_points(&data_positions.unwrap_or_default(), true, true);
        let geometry =
            QuiverGeometry::new(self.id(), points.xy, (points.u, points.v), mask, sizing)
                .with_scalar_components(scalar_components);
        collection.draw_time = Some(Box::new(geometry));
        let quiver = Collection::from_raw(self.adopt(collection, &DataUpdate::limits(limits)));
        if let Some(mapping) = mapping {
            quiver.set_mapping(mapping);
        }
        Ok(Quiver::new(quiver))
    }

    /// Adds a key to `quiver`: an arrow of `length` data units at
    /// `position` with a label (matplotlib's `quiverkey`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::quiver::{QuiverKeyOptions, QuiverOptions, VectorField};
    /// let ax = Figure::new().subplot();
    /// let q = ax.quiver(VectorField::new([0.0], [0.0], [1.0], [1.0]), QuiverOptions::default())?;
    /// let key = ax.quiverkey(&q, (0.9, 1.05), 1.0, "1 m/s", &QuiverKeyOptions::default());
    /// assert_eq!(key.text().text(), "1 m/s");
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn quiverkey(
        &self,
        quiver: &Quiver,
        position: (f64, f64),
        length: f64,
        label: impl Into<String>,
        options: &QuiverKeyOptions,
    ) -> QuiverKey {
        let quiver_axes = quiver
            .raw()
            .with_base(|base| base.parent)
            .unwrap_or(self.id());
        let transform = self.key_transform(quiver_axes, options.coordinates);
        let vector = KeyVector {
            position: Point::new(position.0, position.1),
            length,
            angle: options.angle,
        };
        let text = QuiverKeyArtist::label_artist(&vector, &transform, label.into(), options);
        let figure = Rc::clone(self.raw().figure_state());
        let text = figure.borrow_mut().store.insert(Box::new(text));
        let mut key = QuiverKeyArtist::new(quiver.id(), vector, transform, options, text);
        key.base_mut().zorder = options.zorder.unwrap_or(quiver.zorder() + 0.1);
        let raw = self.adopt(key, &DataUpdate::default());
        if let Some(text) = figure.borrow_mut().store.get_mut(text) {
            text.base_mut().parent = Some(raw.id());
        }
        QuiverKey::from_raw(raw)
    }

    /// The coordinate system of a key position, relative to the axes of
    /// its quiver (matplotlib's `QuiverKey._set_transform`).
    fn key_transform(&self, quiver_axes: ArtistId, coordinates: KeyCoordinates) -> Transform {
        match coordinates {
            KeyCoordinates::Data => Transform::Data(quiver_axes),
            KeyCoordinates::Axes => Transform::Axes(quiver_axes),
            KeyCoordinates::Figure => {
                let figure = self.raw().figure_state().borrow();
                let frame = figure
                    .store
                    .get(quiver_axes)
                    .and_then(|axes| axes.base().parent);
                frame.map_or(Transform::Display, Transform::Figure)
            }
            KeyCoordinates::Inches => Transform::Inches,
        }
    }
}

#[cfg(test)]
mod tests {
    use plt_core::Rgba;

    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::quiver::{Angles, QuiverKeyOptions, QuiverOptions, VectorField};
    use crate::testing::{Call, RecordingRenderer};

    #[test]
    fn data_limits_cover_the_positions() {
        let ax = Figure::new().subplot();
        let field = VectorField::new([-2.0, 2.0], [-1.0, 3.0], [10.0], [10.0]);
        ax.quiver(field, QuiverOptions::default()).unwrap();
        let lim = ax.data_lim().bbox;
        assert_eq!((lim.p0.x, lim.p1.y), (-2.0, 3.0));
    }

    #[test]
    fn empty_fields_draw_nothing() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let empty = VectorField::new(Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let q = ax.quiver(empty, QuiverOptions::default()).unwrap();
        assert!(q.is_empty());
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).unwrap();
    }

    #[test]
    fn angle_counts_are_checked() {
        let ax = Figure::new().subplot();
        let options = QuiverOptions {
            angles: Angles::Degrees(vec![0.0, 90.0]),
            ..QuiverOptions::default()
        };
        let field = VectorField::new([0.0; 3], [0.0; 3], [1.0], [1.0]);
        assert!(ax.quiver(field, options).is_err());
    }

    #[test]
    fn quivers_and_keys_draw() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_axis_off();
        let field = VectorField::new([0.0, 1.0], [0.0, 1.0], [1.0, f64::NAN], [0.5, 1.0]);
        let q = ax.quiver(field, QuiverOptions::default()).unwrap();
        let options = QuiverKeyOptions {
            color: Some("r".into()),
            ..QuiverKeyOptions::default()
        };
        // An empty label draws no glyphs, leaving only arrows to count.
        let key = ax.quiverkey(&q, (0.5, 1.02), 1.0, "", &options);
        let label = key.text();
        assert_eq!(label.text(), "");
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).unwrap();
        let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
        let arrows: Vec<_> = renderer
            .calls
            .iter()
            .filter_map(|call| match call {
                Call::Path {
                    points,
                    fill: Some(fill),
                    ..
                } if points.len() >= 8 => Some(*fill),
                _ => None,
            })
            .collect();
        assert_eq!(arrows.len(), 2, "one unmasked arrow and the key");
        assert_eq!(arrows[1], red);
        key.remove();
        assert!(!label.raw().is_alive());
    }
}
