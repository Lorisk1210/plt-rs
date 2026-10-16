//! Drawing the axes of polar axes: θ gridlines along the radii with
//! labels around the circle, radial gridlines as arcs with labels along
//! one radius or the sector edge (matplotlib's `ThetaTick` and
//! `RadialTick`).

use std::f64::consts::{FRAC_PI_2, PI};

use plt_core::{Affine2D, Bbox, Path, Point};
use plt_render::ClipPath;
use plt_text::{BlockStyle, HAlign, RotationMode, VAlign};

use super::transform::{PolarView, full_circle_deg};
use crate::artist::{ArtistId, DrawContext};
use crate::axes::axis_draw::{AxisPainter, AxisText, TickMark, grid_gc};
use crate::axes::{AxesState, AxisName, GridStyle, Tick, TickDirection, TickStyle};
use crate::fonts::FontProperties;
use crate::markers::{FillStyle, MarkerStyle, MarkerSymbol};
use crate::rc::RcParams;
use crate::transforms::{ResolvedTransform, Transform, unit_to_bbox};

/// Segments per gridline, so that projections can bend them
/// (matplotlib's `GRIDLINE_INTERPOLATION_STEPS`).
const GRIDLINE_INTERPOLATION_STEPS: usize = 180;

/// Extra θ label padding in points, keeping matplotlib's look of
/// centred labels (`ThetaTick.update_position`).
const THETA_LABEL_EXTRA_PAD: f64 = 7.0;

/// Everything needed to draw one axis of polar axes.
pub(crate) struct PolarAxisPainter<'a> {
    axes_id: ArtistId,
    axes: &'a AxesState,
    polar: PolarView<'a>,
    name: AxisName,
}

/// The data transform and the clip outline shared by all ticks.
struct Frame {
    data: ResolvedTransform,
    clip: ClipPath,
}

/// One label side of a tick.
struct LabelSide {
    /// Whether this is the first side (label1).
    first: bool,
    /// The label's display anchor.
    anchor: Point,
    /// Alignment about the anchor when the style does not override it.
    align: (HAlign, VAlign),
}

impl<'a> PolarAxisPainter<'a> {
    /// The painter of axis `name` of polar `axes` with geometry `polar`.
    pub(crate) fn new(
        axes_id: ArtistId,
        axes: &'a AxesState,
        polar: PolarView<'a>,
        name: AxisName,
    ) -> Self {
        Self {
            axes_id,
            axes,
            polar,
            name,
        }
    }

    /// Draws gridlines, tick marks and labels, then the axis label and
    /// offset text; problems are reported on the axes.
    pub(crate) fn draw(&self, ctx: &mut DrawContext<'_>, rc: &RcParams) {
        let axis = self.axes.axis(self.name);
        if !axis.visible {
            return;
        }
        let ticks = match &axis.ticks {
            Ok(ticks) => ticks,
            Err(message) => return ctx.report(self.axes_id, message),
        };
        let (Some(data), Some(display)) = (
            ctx.resolve(&Transform::Data(self.axes_id)),
            ctx.layout().axes_extent(self.axes_id),
        ) else {
            return;
        };
        let frame = Frame {
            data,
            clip: ClipPath {
                path: self.axes.patch_path(),
                transform: unit_to_bbox(&display),
            },
        };
        let mut label_boxes = Vec::new();
        for (set, style) in [
            (&ticks.major, &axis.major_style),
            (&ticks.minor, &axis.minor_style),
        ] {
            for tick in set {
                let sides = match self.name {
                    AxisName::X => self.theta_tick(ctx, &frame, tick, style, rc),
                    AxisName::Y => self.radial_tick(ctx, &frame, tick, style, rc),
                };
                for side in sides {
                    if let Some(bbox) = self.draw_label(ctx, tick, style, &side, rc) {
                        label_boxes.push((side.first, bbox));
                    }
                }
            }
        }
        AxisPainter::new(self.axes_id, self.axes, self.name).draw_annotations(
            ctx,
            rc,
            &ticks.offset,
            &label_boxes,
        );
    }

    /// Draws the gridline and marks of a θ tick and returns its label
    /// sides: label 1 outside the outer circle, label 2 inside the inner
    /// one, both pushed outwards along the tick's angle.
    fn theta_tick(
        &self,
        ctx: &mut DrawContext<'_>,
        frame: &Frame,
        tick: &Tick,
        style: &TickStyle,
        rc: &RcParams,
    ) -> Vec<LabelSide> {
        let angle = tick.loc * self.polar.direction() + self.polar.settings().theta_offset;
        let along =
            ResolvedTransform::affine(self.polar.radius_fraction()).then(frame.data.clone());
        if let Some(grid) = &style.grid {
            let line = Path::polyline(vec![Point::new(tick.loc, 0.0), Point::new(tick.loc, 1.0)]);
            self.draw_gridline(ctx, frame, &along, &line, grid, rc);
        }
        let (mark1, mark2) = match style.direction {
            TickDirection::Out => (MarkerSymbol::TickDown, MarkerSymbol::TickUp),
            TickDirection::In => (MarkerSymbol::TickUp, MarkerSymbol::TickDown),
            TickDirection::InOut => (MarkerSymbol::VLine, MarkerSymbol::VLine),
        };
        for (on, symbol, across) in [
            (style.ticks.first, mark1, 0.0),
            (style.ticks.second, mark2, 1.0),
        ] {
            if on {
                let point = Point::new(tick.loc, across);
                self.draw_mark(
                    ctx,
                    &along,
                    point,
                    symbol,
                    theta_mark(symbol, angle - FRAC_PI_2),
                    style,
                );
            }
        }
        let pad = ctx.points_to_pixels(style.label_offset() + THETA_LABEL_EXTRA_PAD);
        let shift = Point::new(pad * angle.cos(), pad * angle.sin());
        let align = (HAlign::Center, VAlign::Center);
        [(style.labels.first, true), (style.labels.second, false)]
            .into_iter()
            .filter(|(on, _)| *on)
            .map(|(_, first)| {
                // The labels are flipped along the radius: label 1 sits
                // on the outer edge.
                let edge =
                    along.transform_point(Point::new(tick.loc, if first { 1.0 } else { 0.0 }));
                LabelSide {
                    first,
                    anchor: if first { edge + shift } else { edge - shift },
                    align,
                }
            })
            .collect()
    }

    /// Draws the gridline and marks of a radial tick and returns its
    /// label sides: along the label radius on full circles, else at the
    /// start (label 1) and end (label 2) edges of the sector.
    fn radial_tick(
        &self,
        ctx: &mut DrawContext<'_>,
        frame: &Frame,
        tick: &Tick,
        style: &TickStyle,
        rc: &RcParams,
    ) -> Vec<LabelSide> {
        let settings = self.polar.settings();
        let direction = self.polar.direction();
        let view = self.polar.view();
        let (theta_min, theta_max) = (
            view.x0().min(view.x1()).to_degrees(),
            view.x0().max(view.x1()).to_degrees(),
        );
        let offset = settings.theta_offset.to_degrees();
        let full = full_circle_deg(theta_min, theta_max);
        let along = ResolvedTransform::affine(self.polar.theta_fraction()).then(frame.data.clone());
        if let Some(grid) = &style.grid {
            let line = Path::polyline(vec![Point::new(0.0, tick.loc), Point::new(1.0, tick.loc)]);
            self.draw_gridline(ctx, frame, &along, &line, grid, rc);
        }
        let side_transform = if full {
            ResolvedTransform::affine(Affine2D::translation(settings.rlabel_position, 0.0))
                .then(frame.data.clone())
        } else {
            along
        };
        let edge_angle = |theta: f64| (theta * direction + offset).rem_euclid(360.0) - 90.0;
        let mark_angle = |angle: f64| {
            if direction > 0.0 {
                angle.to_radians()
            } else {
                (angle + 180.0).to_radians()
            }
        };
        let (mark1, mark2) = match style.direction {
            TickDirection::Out => (MarkerSymbol::TickLeft, MarkerSymbol::TickRight),
            TickDirection::In => (MarkerSymbol::TickRight, MarkerSymbol::TickLeft),
            TickDirection::InOut => (MarkerSymbol::HLine, MarkerSymbol::HLine),
        };
        let real = self.polar.real_view();
        let (real_min, real_max) = (real.x0().min(real.x1()), real.x0().max(real.x1()));
        let pad = ctx.points_to_pixels(style.label_offset());
        // matplotlib's `_ThetaShift`: away from the start or the end edge.
        let edge_shift = |towards_end: bool| {
            let angle = if towards_end {
                real_max + FRAC_PI_2
            } else {
                real_min - FRAC_PI_2
            };
            Point::new(pad * angle.cos(), pad * angle.sin())
        };
        let mut sides = Vec::new();
        for (first, theta) in [(true, theta_min), (false, theta_max)] {
            if !first && full {
                continue;
            }
            let angle = edge_angle(theta);
            let point = Point::new(if first { 0.0 } else { 1.0 }, tick.loc);
            let (on, symbol) = if first {
                (style.ticks.first, mark1)
            } else {
                (style.ticks.second, mark2)
            };
            if on {
                let rotation = if full { 0.0 } else { mark_angle(angle) };
                self.draw_mark(
                    ctx,
                    &side_transform,
                    point,
                    symbol,
                    radial_mark(symbol, rotation),
                    style,
                );
            }
            let labelled = if first {
                style.labels.first
            } else {
                style.labels.second
            };
            if !labelled {
                continue;
            }
            let base = side_transform.transform_point(point);
            sides.push(if full {
                LabelSide {
                    first,
                    anchor: base,
                    align: (HAlign::Left, VAlign::Bottom),
                }
            } else {
                let at_start = first == (direction > 0.0);
                LabelSide {
                    first,
                    anchor: base + edge_shift(!at_start),
                    align: sector_anchor(angle, at_start),
                }
            });
        }
        sides
    }

    /// Draws `line` (in the coordinates of `transform`) as a gridline
    /// clipped to the axes wedge.
    fn draw_gridline(
        &self,
        ctx: &mut DrawContext<'_>,
        frame: &Frame,
        transform: &ResolvedTransform,
        line: &Path,
        grid: &GridStyle,
        rc: &RcParams,
    ) {
        if !grid.linestyle.is_visible() {
            return;
        }
        let mut gc = match grid_gc(grid, rc) {
            Ok(gc) => gc,
            Err(err) => return ctx.report(self.axes_id, err),
        };
        gc.clip_path = Some(frame.clip.clone());
        let line = line
            .clone()
            .with_interpolation_steps(GRIDLINE_INTERPOLATION_STEPS);
        let (path, affine) = transform.split_path(&line);
        ctx.renderer().draw_path(&gc, &path, &affine, None);
    }

    /// Draws tick mark `symbol` with marker transform `transform` at
    /// `point` (in the coordinates of `along`).
    fn draw_mark(
        &self,
        ctx: &mut DrawContext<'_>,
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
        .draw(ctx, self.axes_id, &points, &affine);
    }

    /// Draws one label side of `tick` and returns its display extent.
    fn draw_label(
        &self,
        ctx: &mut DrawContext<'_>,
        tick: &Tick,
        style: &TickStyle,
        side: &LabelSide,
        rc: &RcParams,
    ) -> Option<Bbox> {
        if tick.label.is_empty() || !side.anchor.is_finite() {
            return None;
        }
        let font = FontProperties {
            size: style.label_size,
            ..FontProperties::from_rc(rc)
        };
        let halign = style.label_halign.unwrap_or(side.align.0);
        AxisText {
            text: &tick.label,
            font: &font,
            color: &style.label_color,
            block: BlockStyle {
                halign,
                valign: style.label_valign.unwrap_or(side.align.1),
                multialign: halign,
                rotation: style.label_rotation,
                rotation_mode: RotationMode::Anchor,
                ..BlockStyle::default()
            },
        }
        .draw(ctx, self.axes_id, side.anchor, rc)
    }
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
/// (the edge's direction minus 90°), on the start or the end edge
/// (matplotlib's `RadialTick._determine_anchor` without auto rotation).
fn sector_anchor(angle: f64, start: bool) -> (HAlign, VAlign) {
    use HAlign::{Center as Middle, Left, Right};
    use VAlign::{Bottom, Center, Top};
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
        assert_eq!(sector_anchor(-45.0, true), (HAlign::Left, VAlign::Top));
        assert_eq!(sector_anchor(-45.0, false), (HAlign::Right, VAlign::Bottom));
        assert_eq!(sector_anchor(0.0, true), (HAlign::Left, VAlign::Center));
        assert_eq!(sector_anchor(260.0, true), (HAlign::Center, VAlign::Top));
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
