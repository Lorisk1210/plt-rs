//! Data limits of artists placed in coordinates other than the axes' own
//! data coordinates: matplotlib maps their points by `transform -
//! ax.transData` when the transform ends in the data coordinates.

use plt_core::{Affine2D, Point};

use crate::artist::ArtistId;
use crate::transforms::Transform;

/// The fixed affine map `transform` applies before the data transform of
/// `axes` it ends with, or `None` when it ends elsewhere.
pub(super) fn affine_before_data(transform: &Transform, axes: ArtistId) -> Option<Affine2D> {
    match transform {
        Transform::Data(id) => (*id == axes).then_some(Affine2D::IDENTITY),
        Transform::Composite(first, last) => {
            let rest = affine_before_data(last, axes)?;
            match first.as_ref() {
                Transform::Affine(affine) => Some(affine.then(rest)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The offsets in the data coordinates of `axes` when their transform
/// passes through the axes' data transform last, after a fixed affine map
/// (matplotlib's `Collection.get_datalim`, which maps them by
/// `offset_trf - transData`); `None` when they are in other coordinates.
pub(super) fn data_offsets(
    transform: &Transform,
    axes: ArtistId,
    offsets: &[Point],
) -> Option<Vec<Point>> {
    let to_data = affine_before_data(transform, axes)?;
    Some(
        offsets
            .iter()
            .map(|&p| to_data.transform_point(p))
            .collect(),
    )
}

/// The points of a grid or mesh as its data limits count them
/// (matplotlib's `_update_pcolor_lims` and `QuadContourSet`): mapped into
/// the data coordinates of `axes` when `transform` ends in them after a
/// fixed affine map, unchanged otherwise.
pub(super) fn grid_points_in_data(
    transform: Option<&Transform>,
    axes: ArtistId,
    points: Vec<Point>,
) -> Vec<Point> {
    match transform.and_then(|t| affine_before_data(t, axes)) {
        Some(to_data) if to_data != Affine2D::IDENTITY => points
            .into_iter()
            .map(|p| to_data.transform_point(p))
            .collect(),
        _ => points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    #[test]
    fn points_map_through_the_affine_before_the_data_transform() {
        let ax = Figure::new().subplot();
        let times10 =
            Transform::Affine(Affine2D::identity().scale(10.0, 10.0)).then(ax.trans_data());
        let point = vec![Point::new(1.0, 2.0)];
        assert_eq!(
            grid_points_in_data(Some(&times10), ax.id(), point.clone()),
            vec![Point::new(10.0, 20.0)]
        );
        assert_eq!(grid_points_in_data(None, ax.id(), point.clone()), point);
        let axes = ax.trans_axes();
        assert_eq!(
            grid_points_in_data(Some(&axes), ax.id(), point.clone()),
            point
        );
        assert_eq!(data_offsets(&axes, ax.id(), &point), None);
    }
}
