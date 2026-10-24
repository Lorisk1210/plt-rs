//! Where the ticks of geographic axes go: gridlines and marks along
//! meridians and parallels, longitude labels just above the equator and
//! latitude labels beside the map's edge (matplotlib's
//! `get_xaxis_transform`, `get_xaxis_text1_transform` and their y
//! counterparts on `GeoAxes`).

use plt_core::Point;
use plt_text::{HAlign, VAlign};

use super::GeoState;
use super::transform::latitude_span;
use crate::projections::TickLabelPlacement;
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::ResolvedTransform;

/// Display pixels between the equator and the longitude labels.
const LONGITUDE_LABEL_OFFSET: f64 = 4.0;

/// Display pixels between the map's edge and the latitude labels.
const LATITUDE_LABEL_OFFSET: f64 = 8.0;

/// Vertical stretch applied to latitude label positions in projected
/// coordinates, spreading them out near the poles (matplotlib's
/// `yaxis_space`).
const LATITUDE_LABEL_SPREAD: f64 = 1.1;

impl GeoState {
    /// From the coordinates of the gridlines and marks of axis `name` to
    /// axes fractions: data along the axis, fractions of the gridline
    /// across it (longitude gridlines end at the longitude cap, latitude
    /// gridlines go all round).
    pub(super) fn gridline_transform(&self, name: AxisName) -> ResolvedTransform {
        let span = match name {
            AxisName::X => self.longitude_span(),
            AxisName::Y => latitude_span(),
        };
        ResolvedTransform::affine(span).then(self.to_axes())
    }

    /// Where the label of the tick at `loc` goes on `side`.
    pub(super) fn label_placement(&self, side: SpineSide, loc: f64) -> TickLabelPlacement {
        let first = !side.is_high();
        match side.axis() {
            AxisName::X => {
                let (dy, valign) = if first {
                    (LONGITUDE_LABEL_OFFSET, VAlign::Bottom)
                } else {
                    (-LONGITUDE_LABEL_OFFSET, VAlign::Top)
                };
                TickLabelPlacement {
                    anchor: self.to_axes().transform_point(Point::new(loc, 0.0)),
                    offset: Point::new(0.0, dy),
                    halign: HAlign::Center,
                    valign,
                }
            }
            AxisName::Y => {
                let (edge, dx, halign) = if first {
                    (0.0, -LATITUDE_LABEL_OFFSET, HAlign::Right)
                } else {
                    (1.0, LATITUDE_LABEL_OFFSET, HAlign::Left)
                };
                let lonlat = latitude_span().transform_point(Point::new(edge, loc));
                let projected = self.projection.forward(lonlat);
                let spread = Point::new(projected.x, projected.y * LATITUDE_LABEL_SPREAD);
                TickLabelPlacement {
                    anchor: self.projection.affine().transform_point(spread),
                    offset: Point::new(dx, 0.0),
                    halign,
                    valign: VAlign::Center,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;
    use crate::geo::GeoProjection;
    use crate::testing::close_points;

    #[test]
    fn labels_sit_on_the_equator_and_beside_the_edge() {
        let geo = GeoState::new(GeoProjection::Mollweide);
        let label = geo.label_placement(SpineSide::Bottom, 0.0);
        assert!(close_points(label.anchor, Point::new(0.5, 0.5)));
        assert_eq!(label.offset, Point::new(0.0, 4.0));
        assert_eq!(
            (label.halign, label.valign),
            (HAlign::Center, VAlign::Bottom)
        );
        let label = geo.label_placement(SpineSide::Top, 0.0);
        assert_eq!(label.offset, Point::new(0.0, -4.0));
        assert_eq!(label.valign, VAlign::Top);
        let label = geo.label_placement(SpineSide::Left, 0.0);
        assert!(close_points(label.anchor, Point::new(0.0, 0.5)));
        assert_eq!(label.offset, Point::new(-8.0, 0.0));
        assert_eq!(
            (label.halign, label.valign),
            (HAlign::Right, VAlign::Center)
        );
        // Labels near the poles are spread out beyond the map.
        let label = geo.label_placement(SpineSide::Right, PI / 2.0);
        assert!(close_points(label.anchor, Point::new(0.5, 1.05)));
        assert_eq!(
            (label.offset, label.halign),
            (Point::new(8.0, 0.0), HAlign::Left)
        );
    }

    #[test]
    fn gridlines_follow_their_span() {
        let geo = GeoState {
            longitude_cap: 1.0,
            ..GeoState::new(GeoProjection::Hammer)
        };
        let meridian = geo.gridline_transform(AxisName::X);
        let top = geo.to_axes().transform_point(Point::new(0.3, 1.0));
        assert!(close_points(
            meridian.transform_point(Point::new(0.3, 1.0)),
            top
        ));
        let parallel = geo.gridline_transform(AxisName::Y);
        let west = geo.to_axes().transform_point(Point::new(-PI, 0.2));
        assert!(close_points(
            parallel.transform_point(Point::new(0.0, 0.2)),
            west
        ));
    }
}
