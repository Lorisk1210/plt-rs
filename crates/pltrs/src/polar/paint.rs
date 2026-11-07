//! The ticks of polar axes: θ gridlines along the radii with labels
//! around the circle, radial gridlines as arcs with labels along one
//! radius or the sector edge (matplotlib's `ThetaTick` and `RadialTick`).
//! The axis painter calls [`draw_tick_lines`] for gridlines and marks and
//! [`tick_label_anchor`] to place labels.

use std::f64::consts::{FRAC_PI_2, PI};

use pltrs_core::{Affine2D, Path, Point};
use pltrs_render::ClipPath;
use pltrs_text::{HAlign, VAlign};

use super::transform::{PolarView, full_circle_deg};
use crate::artist::{ArtistId, DrawContext, FigureReader, LayoutContext};
use crate::axes::axis_artists::TickGroups;
use crate::axes::axis_draw::{TickMark, grid_clip, grid_gc};
use crate::axes::{AxisName, GridStyle, Tick, TickDirection, TickPart, TickStyle};
use crate::markers::{FillStyle, MarkerStyle, MarkerSymbol};
use crate::patheffects::with_path_effects;
use crate::rc::RcParams;
use crate::transforms::{ResolvedTransform, Transform, unit_to_bbox};

/// Segments per gridline, so that projections can bend them
/// (matplotlib's `GRIDLINE_INTERPOLATION_STEPS`).
pub(crate) const GRIDLINE_INTERPOLATION_STEPS: usize = 180;

/// Extra θ label padding in points, keeping matplotlib's look of
/// centred labels (`ThetaTick.update_position`).
const THETA_LABEL_EXTRA_PAD: f64 = 7.0;

/// One side of a radial tick: where its mark and label sit.
struct RadialSide {
    /// Coordinates of `point`: data, or fractions of the θ view.
    transform: ResolvedTransform,
    /// The mark's position.
    point: Point,
    /// The edge direction minus 90°, in degrees.
    angle: f64,
    /// Whether the θ view is a full circle.
    full: bool,
}

/// A tick of polar axes with the geometry its marks and label need.
struct PolarTick<'a> {
    polar: &'a PolarView<'a>,
    data: ResolvedTransform,
    name: AxisName,
    loc: f64,
}

impl PolarTick<'_> {
    /// The direction of a θ tick on screen, radians anticlockwise from
    /// east.
    fn theta_angle(&self) -> f64 {
        self.loc * self.polar.direction() + self.polar.settings().theta_offset
    }

    /// The coordinates gridlines and θ marks are drawn in: fractions of
    /// the radial view along θ ticks, fractions of the θ view along
    /// radial ticks.
    fn along(&self) -> ResolvedTransform {
        let fraction = match self.name {
            AxisName::X => self.polar.radius_fraction(),
            AxisName::Y => self.polar.theta_fraction(),
        };
        ResolvedTransform::affine(fraction).then(self.data.clone())
    }

    /// The gridline in the coordinates of [`PolarTick::along`].
    fn gridline(&self) -> Path {
        let ends = match self.name {
            AxisName::X => [Point::new(self.loc, 0.0), Point::new(self.loc, 1.0)],
            AxisName::Y => [Point::new(0.0, self.loc), Point::new(1.0, self.loc)],
        };
        Path::polyline(ends.to_vec()).with_interpolation_steps(GRIDLINE_INTERPOLATION_STEPS)
    }

    /// The first (start edge, or label radius on full circles) or second
    /// (end edge) side of a radial tick; full circles have no second side.
    fn radial_side(&self, first: bool) -> Option<RadialSide> {
        let settings = self.polar.settings();
        let view = self.polar.view();
        let (theta_min, theta_max) = (
            view.x0().min(view.x1()).to_degrees(),
            view.x0().max(view.x1()).to_degrees(),
        );
        let full = full_circle_deg(theta_min, theta_max);
        if full && !first {
            return None;
        }
        let theta = if first { theta_min } else { theta_max };
        let angle = (theta * self.polar.direction() + settings.theta_offset.to_degrees())
            .rem_euclid(360.0)
            - 90.0;
        let transform = if full {
            ResolvedTransform::affine(Affine2D::translation(settings.rlabel_position, 0.0))
                .then(self.data.clone())
        } else {
            self.along()
        };
        Some(RadialSide {
            transform,
            point: Point::new(if first { 0.0 } else { 1.0 }, self.loc),
            angle,
            full,
        })
    }

    /// The rotation of a radial mark on `side` (matplotlib's
    /// `tick_angle`).
    fn radial_mark_angle(&self, side: &RadialSide) -> f64 {
        if side.full {
            0.0
        } else if self.polar.direction() > 0.0 {
            side.angle.to_radians()
        } else {
            (side.angle + 180.0).to_radians()
        }
    }
}

/// The tick at `tick` of axis `name` with its data transform, `None` if
/// the axes is no longer laid out.
fn polar_tick<'a>(
    layout: LayoutContext<'_>,
    axes: ArtistId,
    polar: &'a PolarView<'a>,
    name: AxisName,
    tick: &Tick,
) -> Option<PolarTick<'a>> {
    Some(PolarTick {
        polar,
        data: layout.resolve(&Transform::Data(axes))?,
        name,
        loc: tick.loc,
    })
}

/// Draws the gridline (clipped to the axes wedge) and the marks of one
/// tick of axis `name` of polar `axes`, each in the group `groups` names.
#[expect(
    clippy::too_many_arguments,
    reason = "the tick, its axis and axes, its style and its groups"
)]
pub(crate) fn draw_tick_lines(
    ctx: &mut DrawContext<'_>,
    axes: ArtistId,
    polar: &PolarView<'_>,
    name: AxisName,
    tick: &Tick,
    style: &TickStyle,
    rc: &RcParams,
    groups: TickGroups<'_>,
) {
    let layout = ctx.layout();
    let (Some(ptick), Some(display)) = (
        polar_tick(layout, axes, polar, name, tick),
        layout.axes_extent(axes),
    ) else {
        return;
    };
    if style.grid_on {
        let clip = ClipPath {
            path: polar.patch_path(),
            transform: unit_to_bbox(&display),
        };
        let drawn = groups.part(ctx, TickPart::Gridline, |ctx| {
            with_path_effects(ctx, &style.grid.path_effects, |ctx| {
                draw_gridline(ctx, axes, &ptick, &style.grid, clip, rc);
            })
        });
        if let Err(err) = drawn {
            ctx.report(axes, err);
        }
    }
    let drawn = with_path_effects(ctx, &rc.path_effects, |ctx| match name {
        AxisName::X => draw_theta_marks(ctx, axes, &ptick, style, groups),
        AxisName::Y => draw_radial_marks(ctx, axes, &ptick, style, groups),
    });
    if let Err(err) = drawn {
        ctx.report(axes, err);
    }
}

fn draw_gridline(
    ctx: &mut DrawContext<'_>,
    axes: ArtistId,
    ptick: &PolarTick<'_>,
    grid: &GridStyle,
    clip: ClipPath,
    rc: &RcParams,
) {
    if !grid.linestyle.is_visible() {
        return;
    }
    let mut gc = match grid_gc(grid, rc) {
        Ok(gc) => gc,
        Err(err) => return ctx.report(axes, err),
    };
    (gc.clip_rect, gc.clip_path) = grid_clip(ctx, axes, &grid.line, Some(clip));
    let line = ptick.gridline();
    let along = ptick.along();
    ctx.free_for(&along);
    let (path, affine) = along.split_path(&line);
    ctx.renderer().draw_path(&gc, &path, &affine, None);
}

/// θ marks at the inner (first) and outer (second) radial limit,
/// perpendicular to the circle.
fn draw_theta_marks(
    ctx: &mut DrawContext<'_>,
    axes: ArtistId,
    ptick: &PolarTick<'_>,
    style: &TickStyle,
    groups: TickGroups<'_>,
) {
    let (mark1, mark2) = match style.direction {
        TickDirection::Out => (MarkerSymbol::TickDown, MarkerSymbol::TickUp),
        TickDirection::In => (MarkerSymbol::TickUp, MarkerSymbol::TickDown),
        TickDirection::InOut => (MarkerSymbol::VLine, MarkerSymbol::VLine),
    };
    let along = ptick.along();
    let rotation = ptick.theta_angle() - FRAC_PI_2;
    for (on, symbol, second) in [
        (style.ticks.first, mark1, false),
        (style.ticks.second, mark2, true),
    ] {
        if on {
            let point = Point::new(ptick.loc, if second { 1.0 } else { 0.0 });
            groups.part(ctx, TickPart::mark(second), |ctx| {
                draw_mark(
                    ctx,
                    axes,
                    &along,
                    point,
                    symbol,
                    theta_mark(symbol, rotation),
                    &style.mark_style(second),
                );
            });
        }
    }
}

/// Radial marks along the label radius (full circles) or the sector
/// edges.
fn draw_radial_marks(
    ctx: &mut DrawContext<'_>,
    axes: ArtistId,
    ptick: &PolarTick<'_>,
    style: &TickStyle,
    groups: TickGroups<'_>,
) {
    let (mark1, mark2) = match style.direction {
        TickDirection::Out => (MarkerSymbol::TickLeft, MarkerSymbol::TickRight),
        TickDirection::In => (MarkerSymbol::TickRight, MarkerSymbol::TickLeft),
        TickDirection::InOut => (MarkerSymbol::HLine, MarkerSymbol::HLine),
    };
    for (first, on, symbol) in [
        (true, style.ticks.first, mark1),
        (false, style.ticks.second, mark2),
    ] {
        if let Some(side) = ptick.radial_side(first).filter(|_| on) {
            let rotation = ptick.radial_mark_angle(&side);
            let transform = radial_mark(symbol, rotation);
            groups.part(ctx, TickPart::mark(!first), |ctx| {
                draw_mark(
                    ctx,
                    axes,
                    &side.transform,
                    side.point,
                    symbol,
                    transform,
                    &style.mark_style(!first),
                );
            });
        }
    }
}

/// Draws tick mark `symbol` with marker transform `transform` at `point`
/// (in the coordinates of `along`).
fn draw_mark(
    ctx: &mut DrawContext<'_>,
    axes: ArtistId,
    along: &ResolvedTransform,
    point: Point,
    symbol: MarkerSymbol,
    transform: Affine2D,
    style: &TickStyle,
) {
    let marker = MarkerStyle::new(symbol, FillStyle::Full).expect("tick markers are valid");
    let points = Path::polyline(vec![point]);
    let (points, affine) = along.split_path(&points);
    TickMark {
        marker: &marker,
        transform,
        style,
    }
    .draw(ctx, axes, &points, &affine);
}

/// Where a tick label of polar axes goes, how it is aligned about that
/// anchor and turned (labels rotate about the anchor).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PolarLabel {
    /// The anchor in display pixels.
    pub(crate) anchor: Point,
    /// The default alignment about the anchor.
    pub(crate) align: (HAlign, VAlign),
    /// Counter-clockwise rotation in degrees.
    pub(crate) rotation: f64,
}

/// Where the first or second label of a tick of polar axes goes. `None`
/// when the side has no label (the second radial side of a full circle)
/// or the axes is no longer laid out. A draw lets the figure go while a
/// transform of the program's places the label (`cx`).
pub(crate) fn tick_label_anchor(
    cx: &mut impl FigureReader,
    axes: ArtistId,
    polar: &PolarView<'_>,
    name: AxisName,
    tick: &Tick,
    style: &TickStyle,
    first: bool,
) -> Option<PolarLabel> {
    let ptick = polar_tick(cx.read(), axes, polar, name, tick)?;
    // matplotlib's `_labelrotation`: turned with the tick plus the user's
    // angle in `auto` mode, else the user's angle alone.
    let rotation = |auto_angle: f64| {
        if style.label_rotation_auto {
            auto_angle + style.label_rotation
        } else {
            style.label_rotation
        }
    };
    match name {
        AxisName::X => {
            let angle = ptick.theta_angle();
            let pad = label_pad(
                style.label_offset() + THETA_LABEL_EXTRA_PAD,
                angle,
                cx.read().dpi(),
            );
            // The labels are flipped along the radius: label 1 sits on the
            // outer edge, pushed outwards; label 2 on the inner edge,
            // pushed inwards.
            let (across, shift) = if first {
                (1.0, pad)
            } else {
                (0.0, Point::new(-pad.x, -pad.y))
            };
            let along = ptick.along();
            cx.release_for(&along);
            Some(PolarLabel {
                anchor: padded(along, shift, Point::new(tick.loc, across)),
                align: (HAlign::Center, VAlign::Center),
                rotation: rotation(theta_label_angle(angle.to_degrees())),
            })
        }
        AxisName::Y => {
            let side = ptick.radial_side(first)?;
            cx.release_for(&side.transform);
            if side.full {
                let settings = polar.settings();
                let angle = (settings.rlabel_position.to_degrees() * polar.direction()
                    + settings.theta_offset.to_degrees())
                .rem_euclid(360.0)
                    - 90.0;
                return Some(PolarLabel {
                    anchor: side.transform.transform_point(side.point),
                    align: (HAlign::Left, VAlign::Bottom),
                    rotation: rotation(radial_label_angle(angle)),
                });
            }
            let at_start = first == (polar.direction() > 0.0);
            let real = polar.real_view();
            let edge = if at_start {
                real.x0().min(real.x1()) - FRAC_PI_2
            } else {
                real.x0().max(real.x1()) + FRAC_PI_2
            };
            // matplotlib's `_ThetaShift`: away from the edge.
            let shift = label_pad(style.label_offset(), edge, cx.read().dpi());
            Some(PolarLabel {
                anchor: padded(side.transform, shift, side.point),
                align: sector_anchor(side.angle, at_start, style.label_rotation_auto),
                rotation: rotation(radial_label_angle(side.angle)),
            })
        }
    }
}

/// The padding of a polar tick label, `pad` points towards `angle`
/// (radians anticlockwise from east), in display pixels at `dpi`: as
/// matplotlib's `ScaledTranslation` of the inches `pad · cos(angle) / 72`
/// through the figure's `dpi_scale_trans` (`ThetaTick._update_padding`,
/// `_ThetaShift`) gives it.
fn label_pad(pad: f64, angle: f64, dpi: f64) -> Point {
    Point::new(
        dpi * (pad * angle.cos() / 72.0),
        dpi * (pad * angle.sin() / 72.0),
    )
}

/// Where `transform` followed by the translation `shift` takes `point`,
/// computed as matplotlib computes a label's transform followed by its
/// padding: the translation joins the transform's affine part, which then
/// maps the point.
fn padded(transform: ResolvedTransform, shift: Point, point: Point) -> Point {
    transform
        .then(ResolvedTransform::affine(Affine2D::translation(
            shift.x, shift.y,
        )))
        .transform_point(point)
}

/// The automatic rotation of a θ label whose tick points `angle` degrees
/// anticlockwise from east: along the circle, kept upright
/// (matplotlib's `ThetaTick.update_position`).
fn theta_label_angle(angle: f64) -> f64 {
    let along = angle.rem_euclid(360.0) - 90.0;
    if along > 90.0 {
        along - 180.0
    } else if along < -90.0 {
        along + 180.0
    } else {
        along
    }
}

/// The automatic rotation of a radial label on an edge at `angle`
/// degrees (the edge's direction minus 90°): along the edge, within
/// ±90° (matplotlib's `RadialTick.update_position`).
fn radial_label_angle(angle: f64) -> f64 {
    (angle + 90.0).rem_euclid(180.0) - 90.0
}

/// The marker transform of a θ tick mark pointing along `angle`
/// (matplotlib's `ThetaTick.update_position`, which replaces the marker's
/// own transform).
fn theta_mark(symbol: MarkerSymbol, angle: f64) -> Affine2D {
    match symbol {
        MarkerSymbol::TickDown => Affine2D::scaling(1.0, -1.0).rotate(angle),
        _ => Affine2D::rotation(angle),
    }
}

/// The marker transform of a radial tick mark at `angle` (matplotlib's
/// `RadialTick.update_position`).
fn radial_mark(symbol: MarkerSymbol, angle: f64) -> Affine2D {
    match symbol {
        MarkerSymbol::HLine => Affine2D::rotation(angle + PI / 2.0),
        MarkerSymbol::TickRight => Affine2D::scaling(-1.0, 1.0).rotate(angle),
        _ => Affine2D::rotation(angle),
    }
}

/// The alignment of a radial label on a sector edge at `angle` degrees
/// (the edge's direction minus 90°), on the start or the end edge:
/// facing away from the edge, or along it for labels turned with their
/// ticks (`auto`; matplotlib's `RadialTick._determine_anchor`).
fn sector_anchor(angle: f64, start: bool, auto: bool) -> (HAlign, VAlign) {
    use HAlign::{Center as Middle, Left, Right};
    use VAlign::{Bottom, Center, Top};
    if auto {
        let facing_east = (-90.0..=90.0).contains(&angle);
        return (if facing_east == start { Left } else { Right }, Center);
    }
    let (at_start, at_end) = if angle < -68.5 {
        ((Middle, Top), (Middle, Bottom))
    } else if angle < -23.5 {
        ((Left, Top), (Right, Bottom))
    } else if angle < 22.5 {
        ((Left, Center), (Right, Center))
    } else if angle < 67.5 {
        ((Left, Bottom), (Right, Top))
    } else if angle < 112.5 {
        ((Middle, Bottom), (Middle, Top))
    } else if angle < 157.5 {
        ((Right, Bottom), (Left, Top))
    } else if angle < 202.5 {
        ((Right, Center), (Left, Center))
    } else if angle < 247.5 {
        ((Right, Top), (Left, Bottom))
    } else {
        ((Middle, Top), (Middle, Bottom))
    };
    if start { at_start } else { at_end }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sector_labels_face_away_from_the_edge() {
        assert_eq!(
            sector_anchor(-45.0, true, false),
            (HAlign::Left, VAlign::Top)
        );
        assert_eq!(
            sector_anchor(-45.0, false, false),
            (HAlign::Right, VAlign::Bottom)
        );
        assert_eq!(
            sector_anchor(0.0, true, false),
            (HAlign::Left, VAlign::Center)
        );
        assert_eq!(
            sector_anchor(260.0, true, false),
            (HAlign::Center, VAlign::Top)
        );
    }

    #[test]
    fn auto_rotated_labels_run_along_their_ticks() {
        assert_eq!(
            sector_anchor(-45.0, true, true),
            (HAlign::Left, VAlign::Center)
        );
        assert_eq!(
            sector_anchor(135.0, true, true),
            (HAlign::Right, VAlign::Center)
        );
        assert_eq!(
            sector_anchor(135.0, false, true),
            (HAlign::Left, VAlign::Center)
        );
        // θ labels stay upright: at 0° the text runs down the circle.
        assert_eq!(theta_label_angle(0.0), -90.0);
        assert_eq!(theta_label_angle(90.0), 0.0);
        assert_eq!(theta_label_angle(270.0), 0.0);
        assert_eq!(theta_label_angle(225.0), -45.0);
        // Radial labels follow their edge within ±90°.
        assert_eq!(radial_label_angle(225.0), 45.0);
        assert_eq!(radial_label_angle(-75.0), -75.0);
    }

    #[test]
    fn tick_marks_replace_the_marker_transform() {
        let up = theta_mark(MarkerSymbol::TickUp, 0.0);
        assert!(up.is_identity());
        let down = theta_mark(MarkerSymbol::TickDown, 0.0);
        assert_eq!(
            down.transform_point(Point::new(0.0, 1.0)),
            Point::new(0.0, -1.0)
        );
        let right = radial_mark(MarkerSymbol::TickRight, 0.0);
        assert_eq!(
            right.transform_point(Point::new(1.0, 0.0)),
            Point::new(-1.0, 0.0)
        );
    }
}
