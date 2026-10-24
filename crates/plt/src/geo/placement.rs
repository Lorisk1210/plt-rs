//! Where the ticks of geographic axes go: gridlines and marks along
//! meridians and parallels, longitude labels just above the equator and
//! latitude labels beside the map's edge (matplotlib's
//! `get_xaxis_transform`, `get_xaxis_text1_transform` and their y
//! counterparts on `GeoAxes`). The axis painter calls these hooks.

use plt_core::{Bbox, Point};
use plt_text::{HAlign, VAlign};

use super::GeoState;
use super::transform::latitude_span;
use crate::artist::ArtistId;
use crate::ticker::AxisName;
use crate::transforms::Transform;

/// Display pixels between the equator and the longitude labels.
const LONGITUDE_LABEL_OFFSET: f64 = 4.0;

/// Display pixels between the map's edge and the latitude labels.
const LATITUDE_LABEL_OFFSET: f64 = 8.0;

/// Vertical stretch applied to latitude label positions in projected
/// coordinates, spreading them out near the poles (matplotlib's
/// `yaxis_space`).
const LATITUDE_LABEL_SPREAD: f64 = 1.1;

/// The coordinates of the gridlines and marks of axis `name` of `axes`:
/// data along the axis, fractions of the gridline across it (longitude
/// gridlines end at the longitude cap, latitude gridlines go all round).
pub(crate) fn tick_transform(axes: ArtistId, geo: &GeoState, name: AxisName) -> Transform {
    let span = match name {
        AxisName::X => geo.longitude_span(),
        AxisName::Y => latitude_span(),
    };
    Transform::Affine(span).then(Transform::Data(axes))
}

/// Where the first or second label of the tick at `loc` of axis `name`
/// goes for axes placed at `display`, with its alignment.
pub(crate) fn tick_label_anchor(
    geo: &GeoState,
    display: &Bbox,
    name: AxisName,
    loc: f64,
    first: bool,
) -> (Point, (HAlign, VAlign)) {
    match name {
        AxisName::X => {
            let equator = geo
                .data_transform(display)
                .transform_point(Point::new(loc, 0.0));
            let (dy, valign) = if first {
                (LONGITUDE_LABEL_OFFSET, VAlign::Bottom)
            } else {
                (-LONGITUDE_LABEL_OFFSET, VAlign::Top)
            };
            (equator + Point::new(0.0, dy), (HAlign::Center, valign))
        }
        AxisName::Y => {
            let (edge, dx, halign) = if first {
                (0.0, -LATITUDE_LABEL_OFFSET, HAlign::Right)
            } else {
                (1.0, LATITUDE_LABEL_OFFSET, HAlign::Left)
            };
            let lonlat = latitude_span().transform_point(Point::new(edge, loc));
            let projected = geo.projection.forward(lonlat);
            let spread = Point::new(projected.x, projected.y * LATITUDE_LABEL_SPREAD);
            let anchor = geo.projected_to_display(display).transform_point(spread);
            (anchor + Point::new(dx, 0.0), (halign, VAlign::Center))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;
    use crate::artist::ArtistHandle;
    use crate::geo::GeoProjection;
    use crate::testing::close_points;

    #[test]
    fn labels_sit_on_the_equator_and_beside_the_edge() {
        let geo = GeoState::new(GeoProjection::Mollweide);
        let display = Bbox::from_extents(0.0, 0.0, 400.0, 200.0);
        let (anchor, align) = tick_label_anchor(&geo, &display, AxisName::X, 0.0, true);
        assert!(close_points(anchor, Point::new(200.0, 104.0)));
        assert_eq!(align, (HAlign::Center, VAlign::Bottom));
        let (anchor, align) = tick_label_anchor(&geo, &display, AxisName::X, 0.0, false);
        assert!(close_points(anchor, Point::new(200.0, 96.0)));
        assert_eq!(align, (HAlign::Center, VAlign::Top));
        let (anchor, align) = tick_label_anchor(&geo, &display, AxisName::Y, 0.0, true);
        assert!(close_points(anchor, Point::new(-8.0, 100.0)));
        assert_eq!(align, (HAlign::Right, VAlign::Center));
        // Labels near the poles are spread out beyond the map.
        let (anchor, align) = tick_label_anchor(&geo, &display, AxisName::Y, PI / 2.0, false);
        assert!(close_points(anchor, Point::new(208.0, 210.0)));
        assert_eq!(align, (HAlign::Left, VAlign::Center));
    }

    #[test]
    fn gridlines_follow_their_span() {
        let geo = GeoState::new(GeoProjection::Hammer);
        let axes = crate::figure::Figure::new()
            .geo_subplot(GeoProjection::Hammer)
            .id();
        let Transform::Composite(span, data) = tick_transform(axes, &geo, AxisName::X) else {
            panic!("a span followed by data coordinates");
        };
        assert_eq!(*span, Transform::Affine(geo.longitude_span()));
        assert_eq!(*data, Transform::Data(axes));
    }
}
