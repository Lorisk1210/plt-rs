//! Wind barb plots (matplotlib's `barbs`).

use pltrs_core::Path;

use super::children::DataUpdate;
use super::data_coords::data_offsets;
use super::{Axes, DataLimits, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionArtist, CollectionKind, EdgeColors};
use crate::quiver::{BarbFlip, BarbsOptions, FieldPoints, VectorField, barb_outlines, retained};
use crate::transforms::Transform;

/// The vectors of `field` left once the masked ones are dropped, and the
/// outline of the barb of each, as `options` draw them.
fn barb_items(
    field: VectorField,
    options: &BarbsOptions,
) -> Result<(FieldPoints, Vec<Path>), PlotError> {
    let rc = crate::rc::params();
    let points = field.into_points(&rc)?;
    let n = points.xy.len();
    let flip = match options.flip.len() {
        1 => vec![options.flip[0]; n],
        len if len == n => options.flip.clone(),
        len => {
            return Err(PlotError::Broadcast {
                argument: "flip_barb",
                len,
                points: n,
            });
        }
    };
    let keep = points.unmasked();
    let points = points.retain(&keep);
    let flip = retained(flip, &keep);
    let paths = barb_outlines((&points.u, &points.v), options, BarbFlip(&flip))
        .into_iter()
        .map(Path::polygon)
        .collect();
    Ok((points, paths))
}

impl Collection {
    /// Draws the barbs of `field` in place of those of a collection made
    /// by [`Axes::barbs`], as `options` draw them (matplotlib's
    /// `Barbs.set_UVC`): masked vectors are left out, the offsets follow
    /// the vectors left and their colour values are mapped again.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::quiver::{BarbsOptions, VectorField};
    /// let ax = Figure::new().subplot();
    /// let field = VectorField::new([0.0, 1.0], [0.0, 0.0], [15.0, 65.0], [0.0, 10.0]);
    /// let barbs = ax.barbs(field, BarbsOptions::default())?;
    /// let calm = VectorField::new([0.0, 1.0], [0.0, 0.0], [1.0, f64::NAN], [0.0, 0.0]);
    /// barbs.set_barbs(calm, &BarbsOptions::default())?;
    /// assert_eq!(barbs.paths().len(), 1);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Axes::barbs`].
    pub fn set_barbs(
        &self,
        field: VectorField,
        options: &BarbsOptions,
    ) -> Result<&Self, PlotError> {
        let (mut points, paths) = barb_items(field, options)?;
        self.set_paths(paths).set_offsets(points.xy.clone());
        if let Some(mapping) = points.colors.take() {
            self.set_mapping(mapping);
        }
        Ok(self)
    }
}

impl Axes {
    /// Draws a wind barb for every vector of `field` (matplotlib's
    /// `barbs`): flags, full and half barbs on a staff pointing into the
    /// wind, or a circle for magnitudes below a half barb. Masked vectors
    /// are left out.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::quiver::{BarbsOptions, VectorField};
    /// let ax = Figure::new().subplot();
    /// let field = VectorField::new([0.0, 1.0], [0.0, 0.0], [15.0, 65.0], [0.0, 10.0]);
    /// let barbs = ax.barbs(field, BarbsOptions::default())?;
    /// assert_eq!(barbs.paths().len(), 2);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the positions, components, colour values or flip flags
    /// disagree in size, or the colours cannot be mapped.
    pub fn barbs(
        &self,
        field: VectorField,
        options: BarbsOptions,
    ) -> Result<Collection, PlotError> {
        let (mut points, paths) = barb_items(field, &options)?;
        let mut collection =
            CollectionArtist::new(CollectionKind::Poly, paths, crate::rc::params());
        // `barbcolor` colours the edges and `flagcolor` the faces; with
        // only one of them, the whole barb takes it.
        let (faces, edges) = match (options.barbcolor, options.flagcolor) {
            (Some(barb), Some(flag)) => (flag, EdgeColors::Colors(vec![barb])),
            (barb, flag) => (
                flag.or(barb)
                    .unwrap_or_else(|| collection.rc().pltrs_ink.clone()),
                EdgeColors::Face,
            ),
        };
        collection.facecolors = Some(vec![faces]);
        collection.edgecolors = edges;
        // The outlines must be stroked for the staffs and barbs to show.
        collection.linewidths = Some(vec![1.0]);
        options.props.apply(&mut collection);
        // The outlines are in points, scaled by the staff length (an
        // empirically chosen marker area of `length² / 4`).
        collection.sizes = Some(vec![options.length * options.length / 4.0]);
        collection.base_mut().transform = Some(Transform::Display);
        collection.offsets.clone_from(&points.xy);
        let positions = options.transform.unwrap_or_else(|| self.trans_data());
        let data_positions = data_offsets(&positions, self.id(), &points.xy);
        collection.offset_transform = Some(positions);
        let mut limits = DataLimits::NULL;
        limits.update_from_points(&data_positions.unwrap_or_default(), true, true);
        let barbs =
            Collection::from_raw(self.adopt_collection(collection, &DataUpdate::limits(limits)));
        if let Some(mapping) = points.colors.take() {
            barbs.set_mapping(mapping);
        }
        Ok(barbs)
    }
}

#[cfg(test)]
mod tests {
    use crate::axes::MappedColors;
    use crate::figure::Figure;
    use crate::quiver::{BarbsOptions, QuiverOptions, VectorField};
    use crate::transforms::Transform;

    #[test]
    fn masked_barbs_are_dropped_with_their_flags() {
        let ax = Figure::new().subplot();
        let field = VectorField::new(
            [0.0, 1.0, 2.0],
            [0.0, 0.0, 5.0],
            [10.0, f64::NAN, 20.0],
            [0.0; 3],
        )
        .with_colors(MappedColors::new(vec![1.0, 2.0, 3.0]));
        let options = BarbsOptions {
            flip: vec![true, false, false],
            ..BarbsOptions::default()
        };
        let barbs = ax.barbs(field, options).unwrap();
        assert_eq!(barbs.offsets().len(), 2);
        assert_eq!(ax.data_lim().bbox.p1.y, 5.0);
        // The first barb is flipped: its full barb points to -x before
        // the rotation into a westerly wind turns it to -y.
        let first = &barbs.paths()[0];
        assert!(first.vertices().iter().any(|p| p.y < -1.0));
    }

    #[test]
    fn positions_count_where_their_transform_puts_them() {
        let ax = Figure::new().subplot();
        let shifted =
            Transform::Affine(pltrs_core::Affine2D::translation(25.0, 32.0)).then(ax.trans_data());
        let options = BarbsOptions {
            transform: Some(shifted),
            ..BarbsOptions::default()
        };
        let field = VectorField::new([-5.0, 10.0], [-2.0, 4.0], [1.0, 1.0], [1.0, 1.0]);
        ax.barbs(field, options).unwrap();
        let limits = ax.data_lim().bbox;
        assert_eq!((limits.p0.x, limits.p0.y), (20.0, 30.0));
        assert_eq!((limits.p1.x, limits.p1.y), (35.0, 36.0));
        let options = QuiverOptions {
            transform: Some(ax.trans_axes()),
            ..QuiverOptions::default()
        };
        let field = VectorField::new([100.0], [100.0], [1.0], [1.0]);
        ax.quiver(field, options).unwrap();
        assert_eq!(ax.data_lim().bbox, limits, "axes positions do not count");
    }

    #[test]
    fn flip_counts_are_checked() {
        let ax = Figure::new().subplot();
        let options = BarbsOptions {
            flip: vec![true, false],
            ..BarbsOptions::default()
        };
        let field = VectorField::new([0.0; 3], [0.0; 3], [1.0], [1.0]);
        assert!(ax.barbs(field, options).is_err());
    }
}
