//! The tick marks of an axis artist (matplotlib's `axis_artist.Ticks`):
//! markers at the tick locations, turned to point away from the tick
//! labels.

use pltrs_core::{Affine2D, Path, Point};
use pltrs_render::GraphicsContext;

use super::helper::TickInfo;
use super::labels::TickLabel;
use crate::artist::DrawContext;
use crate::axes::TickDirection;
use crate::colors::Color;
use crate::transforms::ResolvedTransform;

/// Tick marks of one level (matplotlib's `Ticks`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TicksState {
    /// Length in points.
    pub(crate) size: f64,
    pub(crate) direction: TickDirection,
    pub(crate) visible: bool,
    /// The colour; `None` follows the major ticks of the reference axis.
    pub(crate) color: Option<Color>,
    /// The line width in points; `None` follows the major ticks of the
    /// reference axis.
    pub(crate) width: Option<f64>,
    /// The opacity of the marks.
    pub(crate) alpha: Option<f64>,
}

pltrs_core::archive_struct!(TicksState {
    size,
    direction,
    visible,
    color,
    width,
    alpha
});

impl TicksState {
    /// Visible marks of `size` points pointing `direction`.
    pub(crate) fn new(size: f64, direction: TickDirection) -> Self {
        Self {
            size,
            direction,
            visible: true,
            color: None,
            width: None,
            alpha: None,
        }
    }

    /// The mark from the tick location, before scaling to the tick length
    /// and turning to the tick angle (`Ticks._tick_paths`).
    fn path(&self) -> Path {
        let (start, end) = match self.direction {
            TickDirection::In => (0.0, 1.0),
            TickDirection::InOut => (-0.5, 0.5),
            TickDirection::Out => (-1.0, 0.0),
        };
        Path::polyline(vec![Point::new(start, 0.0), Point::new(end, 0.0)])
    }

    /// How far the marks reach out past the line in points, for the tick
    /// labels (zero when hidden).
    pub(crate) fn outward(&self) -> f64 {
        self.size * self.direction.outward_fraction() * f64::from(u8::from(self.visible))
    }

    /// How far the marks reach in from the line in points, for an axis
    /// label on the other side than the tick labels (zero when hidden;
    /// matplotlib's `_update_label`).
    pub(crate) fn inward(&self) -> f64 {
        let fraction = match self.direction {
            TickDirection::In => 1.0,
            TickDirection::InOut => 0.5,
            TickDirection::Out => 0.0,
        };
        self.size * fraction * f64::from(u8::from(self.visible))
    }

    /// Draws the marks at `marks` (locations in `transform`'s coordinates
    /// with angles) with the pen `gc` (matplotlib's `Ticks.draw`; the
    /// marks belong to no axes, so none is left out of the view).
    pub(crate) fn draw(
        &self,
        ctx: &mut DrawContext<'_>,
        marks: &[(Point, f64)],
        transform: &ResolvedTransform,
        gc: &GraphicsContext,
    ) {
        let size = ctx.points_to_pixels(self.size);
        let path = self.path();
        let affine = transform.affine_part();
        for &(loc, angle) in marks {
            let loc = transform.transform_point_non_affine(loc);
            let marker = Affine2D::scaling(size, size).rotate_deg(angle);
            let points = Path::polyline(vec![loc]);
            ctx.renderer()
                .draw_markers(gc, &path, &marker, &points, &affine, None);
        }
    }
}

/// The marks and labels of `ticks`: each mark points away from its label,
/// which sits in the direction 90° clockwise from the line turned by
/// `add_angle` (matplotlib's `AxisArtist._get_tick_info`).
pub(crate) fn tick_info(ticks: &[TickInfo], add_angle: f64) -> (Vec<(Point, f64)>, Vec<TickLabel>) {
    ticks
        .iter()
        .map(|tick| {
            let label_angle = tick.angle_tangent - 90.0 + add_angle;
            let turn = (label_angle - tick.angle_normal).rem_euclid(360.0);
            let mark_angle = if (90.0..=270.0).contains(&turn) {
                tick.angle_normal
            } else {
                tick.angle_normal + 180.0
            };
            (
                (tick.loc, mark_angle),
                TickLabel {
                    loc: tick.loc,
                    angle: label_angle,
                    text: tick.label.clone(),
                },
            )
        })
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_paths_point_away_from_the_line() {
        let out = TicksState::new(3.5, TickDirection::Out);
        assert_eq!(out.path().vertices()[0], Point::new(-1.0, 0.0));
        assert_eq!((out.outward(), out.inward()), (3.5, 0.0));
        let inside = TicksState {
            direction: TickDirection::In,
            ..out
        };
        assert_eq!(inside.path().vertices()[1], Point::new(1.0, 0.0));
        let hidden = TicksState {
            visible: false,
            ..inside
        };
        assert_eq!(hidden.inward(), 0.0);
    }

    #[test]
    fn marks_point_away_from_their_labels() {
        let tick = TickInfo {
            loc: Point::new(0.5, 0.0),
            angle_normal: 90.0,
            angle_tangent: 0.0,
            label: "0.5".into(),
        };
        let (marks, labels) = tick_info(std::slice::from_ref(&tick), 0.0);
        assert_eq!(marks[0].1, 90.0, "the normal points away from labels below");
        assert_eq!(labels[0].angle, -90.0);
        let (marks, _) = tick_info(&[tick], 180.0);
        assert_eq!(marks[0].1, 270.0);
    }
}
