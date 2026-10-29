//! Data limits of artists placed in coordinates other than the axes' own
//! data coordinates: matplotlib maps their points by `transform -
//! ax.transData` when the transform ends in the data coordinates.

use pltrs_core::Point;

use crate::artist::ArtistId;
use crate::transforms::Transform;

/// The offsets in the data coordinates of `axes` when their transform
/// passes through the axes' data transform last (matplotlib's
/// `Collection.get_datalim`, which maps them by `offset_trf -
/// transData`); `None` when they are in other coordinates.
pub(super) fn data_offsets(
    transform: &Transform,
    axes: ArtistId,
    offsets: &[Point],
) -> Option<Vec<Point>> {
    let to_data = transform.to_data(axes)?;
    Some(
        offsets
            .iter()
            .map(|&p| to_data.transform_point(p))
            .collect(),
    )
}

/// The points of a grid or mesh as its data limits count them
/// (matplotlib's `_update_pcolor_lims`): mapped into the data coordinates
/// of `axes` when `transform` ends in them, unchanged otherwise.
pub(super) fn grid_points_in_data(
    transform: Option<&Transform>,
    axes: ArtistId,
    points: Vec<Point>,
) -> Vec<Point> {
    match transform.and_then(|t| t.to_data(axes)) {
        Some(to_data) if !to_data.is_identity() => points
            .into_iter()
            .map(|p| to_data.transform_point(p))
            .collect(),
        _ => points,
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Affine2D;

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

    #[test]
    fn artists_given_a_transform_bound_the_data_where_they_land() {
        let ax = Figure::new().subplot();
        let shifted =
            Transform::Affine(Affine2D::identity().translate(2000.0, 0.0)).then(ax.trans_data());
        let props = crate::lines::LineProps::default().transform(shifted.clone());
        ax.plot_with([0.0, 10.0], [0.0, 1.0], props).unwrap();
        let bar = crate::axes::BarOptions {
            props: crate::patches::PatchProps::default().transform(shifted),
            ..crate::axes::BarOptions::default()
        };
        ax.bar(vec![20.0], [1.0], bar).unwrap();
        let limits = ax.data_lim().bbox;
        assert_eq!((limits.p0.x, limits.p1.x), (2000.0, 2020.4));
    }
}
