//! The axis artist: the line, ticks, tick labels, offset text and label
//! of one axis line, drawn instead of the axes' spines and axes
//! (matplotlib's `axisartist.axis_artist.AxisArtist`).

use std::rc::Rc;

use plt_core::{Affine2D, Bbox, Path, Point};
use plt_render::GraphicsContext;

use super::helper::{AxisHelper, TickInfo};
use super::labels::{TextStyle, TickLabels, axis_label, offset_text, tick_labels};
use super::line::AxislineStyle;
use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext, LayoutContext};
use crate::axes::axis_draw::draw_text;
use crate::axes::{AxesState, PlacedText, TickDirection};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::patches::{DisplayPath, PatchStyle, paint_paths};
use crate::rc::RcParams;
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, Transform};

/// The z-order of axis artists (matplotlib's `AxisArtist.zorder`).
const ZORDER: f64 = 2.5;

/// The default gap between tick labels and the axis label, in points.
const LABEL_PAD: f64 = 5.0;

/// The problem reported when the axes of an axis artist was removed.
const GONE: &str = "the axes of the axis artist is gone";

/// The name of the renderer group of an axis artist (matplotlib's module
/// name).
const GROUP: &str = "mpl_toolkits.axisartist.axis_artist";

/// Tick marks of one level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ticks {
    /// Length in points.
    pub(crate) size: f64,
    pub(crate) direction: TickDirection,
    pub(crate) visible: bool,
}

impl Ticks {
    /// The mark from the tick location, before scaling to the tick length
    /// and turning to the tick angle (`Ticks._tick_paths`).
    fn path(self) -> Path {
        let (start, end) = match self.direction {
            TickDirection::In => (0.0, 1.0),
            TickDirection::InOut => (-0.5, 0.5),
            TickDirection::Out => (-1.0, 0.0),
        };
        Path::polyline(vec![Point::new(start, 0.0), Point::new(end, 0.0)])
    }

    /// Draws the marks at `marks` (locations in `transform`'s coordinates
    /// with angles) inside the view limits `view` (matplotlib's
    /// `Ticks.draw`).
    fn draw(
        self,
        ctx: &mut DrawContext<'_>,
        marks: &[(Point, f64)],
        transform: &ResolvedTransform,
        view: Bbox,
        gc: &GraphicsContext,
    ) {
        let size = ctx.points_to_pixels(self.size);
        let path = self.path();
        let affine = transform.affine_part();
        for &(loc, angle) in marks {
            let loc = transform.transform_point_non_affine(loc);
            if !view.contains(loc) {
                continue;
            }
            let marker = Affine2D::scaling(size, size).rotate_deg(angle);
            let points = Path::polyline(vec![loc]);
            ctx.renderer()
                .draw_markers(gc, &path, &marker, &points, &affine, None);
        }
    }
}

/// Tick labels of one level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TickLabelStyle {
    /// Gap to the ticks, in points.
    pub(crate) pad: f64,
    pub(crate) visible: bool,
}

/// An axis artist (matplotlib's `AxisArtist`).
#[derive(Debug)]
pub(crate) struct AxisArtistState {
    base: ArtistBase,
    /// Its key in the axes' axis dictionary.
    pub(crate) name: String,
    axes: ArtistId,
    helper: AxisHelper,
    /// Which way tick labels and the label face (`axis_direction`).
    pub(crate) direction: SpineSide,
    /// The line style; `None` draws a plain line.
    pub(crate) line_style: Option<AxislineStyle>,
    /// Whether the line is clipped to the axes box.
    pub(crate) line_clipped: bool,
    pub(crate) major_ticks: Ticks,
    pub(crate) minor_ticks: Ticks,
    pub(crate) major_labels: TickLabelStyle,
    pub(crate) minor_labels: TickLabelStyle,
    pub(crate) label_visible: bool,
    rc: Rc<RcParams>,
}

/// The marks (with their locations and angles) and labels of one tick
/// level, laid out.
struct Level {
    marks: Option<(Ticks, Vec<(Point, f64)>)>,
    labels: Vec<PlacedText>,
}

/// Everything an axis artist draws, laid out: the major then the minor
/// tick level, the offset text, the line and the label.
struct Drawing {
    levels: Vec<Level>,
    offset: Option<PlacedText>,
    line: Vec<DisplayPath>,
    label: Option<PlacedText>,
}

impl AxisArtistState {
    /// An axis artist of `axes` along `helper`, facing `direction`, styled
    /// from `rc`.
    pub(crate) fn new(
        name: &str,
        axes: ArtistId,
        helper: AxisHelper,
        direction: SpineSide,
        rc: Rc<RcParams>,
    ) -> Self {
        let x = direction.axis() == AxisName::X;
        let (major_size, minor_size, major_pad, minor_pad, tick_dir) = if x {
            (
                rc.xtick_major_size,
                rc.xtick_minor_size,
                rc.xtick_major_pad,
                rc.xtick_minor_pad,
                &rc.xtick_direction,
            )
        } else {
            (
                rc.ytick_major_size,
                rc.ytick_minor_size,
                rc.ytick_major_pad,
                rc.ytick_minor_pad,
                &rc.ytick_direction,
            )
        };
        let direction_of_ticks = tick_dir.parse().unwrap_or_default();
        let ticks = |size| Ticks {
            size,
            direction: direction_of_ticks,
            visible: true,
        };
        let labels = |pad| TickLabelStyle { pad, visible: true };
        Self {
            base: ArtistBase::with_zorder(ZORDER),
            name: name.to_owned(),
            axes,
            helper,
            direction,
            line_style: None,
            line_clipped: false,
            major_ticks: ticks(major_size),
            minor_ticks: ticks(minor_size),
            major_labels: labels(major_pad),
            minor_labels: labels(minor_pad),
            label_visible: true,
            rc,
        }
    }

    /// The axis whose tick colours and label text the artist uses: y for
    /// left and right artists, x otherwise.
    fn style_axis(&self) -> AxisName {
        self.direction.axis()
    }

    /// How much tick labels and the label turn from the line: 180° for
    /// left and top artists (matplotlib's `set_ticklabel_direction` and
    /// `set_axislabel_direction`).
    fn add_angle(&self) -> f64 {
        match self.direction {
            SpineSide::Left | SpineSide::Top => 180.0,
            SpineSide::Right | SpineSide::Bottom => 0.0,
        }
    }

    /// The font and colour of the tick labels.
    fn ticklabel_style(&self) -> TextStyle {
        let rc = &self.rc;
        let size = match self.style_axis() {
            AxisName::X => rc.xtick_labelsize,
            AxisName::Y => rc.ytick_labelsize,
        };
        TextStyle {
            font: FontProperties::from_rc(rc).with_size(size, rc.font_size),
            color: rc.text_color.clone(),
        }
    }

    /// Lays out everything the artist draws (matplotlib's `_update_ticks`,
    /// `_update_offsetText`, `_draw_line` and `_update_label`).
    fn drawing(&self, layout: LayoutContext<'_>) -> Result<Drawing, String> {
        let axes = layout.store().get_as::<AxesState>(self.axes).ok_or(GONE)?;
        let ticked = axes.axis(self.helper.nth()).ticks.as_ref()?;
        let style_axis = axes.axis(self.style_axis());
        let transform = layout
            .resolve(&self.helper.tick_transform(self.axes))
            .ok_or(GONE)?;
        let add_angle = self.add_angle();
        let external = layout.points_to_pixels(self.major_ticks.size)
            * self.major_ticks.direction.outward_fraction()
            * f64::from(u8::from(self.major_ticks.visible));
        let text_style = self.ticklabel_style();
        let mut levels = Vec::with_capacity(2);
        let mut axislabel_pad = f64::NEG_INFINITY;
        for (ticks_of_level, ticks, labels) in [
            (&ticked.major, self.major_ticks, self.major_labels),
            (&ticked.minor, self.minor_ticks, self.minor_labels),
        ] {
            let infos = self.helper.ticks(axes, ticks_of_level);
            let marks = ticks
                .visible
                .then(|| (ticks, Self::mark_angles(&infos, add_angle)));
            let (texts, pad) = if labels.visible {
                let pad = external + layout.points_to_pixels(labels.pad);
                let laid_out = tick_labels(
                    layout,
                    &infos,
                    &transform,
                    (self.direction, add_angle),
                    pad,
                    &text_style,
                    &self.rc,
                )?;
                let TickLabels {
                    texts,
                    axislabel_pad,
                } = laid_out;
                (texts, axislabel_pad)
            } else {
                (Vec::new(), external)
            };
            axislabel_pad = axislabel_pad.max(pad);
            levels.push(Level {
                marks,
                labels: texts,
            });
        }
        let offset = if self.major_labels.visible || self.minor_labels.visible {
            let offset = style_axis.ticks.as_ref().ok().map(|t| t.offset.as_str());
            self.offset(layout, offset.unwrap_or_default(), &text_style)?
        } else {
            None
        };
        // Tick labels and the label always face the same way here, so the
        // label clears the tick labels (matplotlib's `_update_label`).
        let label = if self.label_visible && !style_axis.label.text.is_empty() {
            let radius = axislabel_pad + layout.points_to_pixels(LABEL_PAD);
            let text = (style_axis.label.text.as_str(), &style_axis.label.color);
            Some(self.label(layout, axes, text, radius)?)
        } else {
            None
        };
        Ok(Drawing {
            levels,
            offset,
            line: self.line(layout, axes)?,
            label,
        })
    }

    /// The tick locations with the angles of their marks, which point
    /// away from the tick labels (matplotlib's `_get_tick_info`).
    fn mark_angles(infos: &[TickInfo], add_angle: f64) -> Vec<(Point, f64)> {
        infos
            .iter()
            .map(|info| {
                let label_angle = info.angle_tangent - 90.0 + add_angle;
                let turn = (label_angle - info.angle_normal).rem_euclid(360.0);
                let angle = if (90.0..=270.0).contains(&turn) {
                    info.angle_normal
                } else {
                    info.angle_normal + 180.0
                };
                (info.loc, angle)
            })
            .collect()
    }

    /// The offset text `text` of the style axis, unless it is empty.
    fn offset(
        &self,
        layout: LayoutContext<'_>,
        text: &str,
        style: &TextStyle,
    ) -> Result<Option<PlacedText>, String> {
        if text.is_empty() {
            return Ok(None);
        }
        let extent = layout.axes_extent(self.axes).ok_or(GONE)?;
        let style = TextStyle {
            color: self.rc.xtick_color.clone(),
            ..style.clone()
        };
        let rise = self.major_labels.pad + style.font.size + 2.0;
        offset_text(layout, text, extent, self.direction, rise, &style, &self.rc).map(Some)
    }

    /// The line in display pixels, plain or arrowed.
    fn line(
        &self,
        layout: LayoutContext<'_>,
        axes: &AxesState,
    ) -> Result<Vec<DisplayPath>, String> {
        let transform = layout.resolve(&Transform::Axes(self.axes)).ok_or(GONE)?;
        let path = self.helper.line(axes);
        match &self.line_style {
            None => {
                let (path, affine) = transform.split_path(&path);
                Ok(vec![DisplayPath::filled(path.into_owned(), affine)])
            }
            Some(style) => style
                .display_paths(
                    &transform.transform_path(&path),
                    self.ticklabel_style().font.size,
                    self.rc.patch_linewidth,
                    layout.points_to_pixels(1.0),
                )
                .map_err(|e| e.to_string()),
        }
    }

    /// The axis label, `radius` pixels from the line.
    fn label(
        &self,
        layout: LayoutContext<'_>,
        axes: &AxesState,
        (text, color): (&str, &Color),
        radius: f64,
    ) -> Result<PlacedText, String> {
        let to_display = |p: Point| {
            layout
                .resolve(&Transform::Axes(self.axes))
                .map(|t| t.transform_point(p))
                .ok_or(GONE)
        };
        // Outside the axes a floating label keeps matplotlib's initial
        // placement, the axes origin facing right.
        let (xy, ref_angle) = self
            .helper
            .label_position(axes)
            .map_or((Point::ORIGIN, 0.0), |(xy, tangent)| {
                (xy, tangent - 90.0 + self.add_angle())
            });
        let rc = &self.rc;
        let style = TextStyle {
            font: FontProperties::from_rc(rc)
                .with_size(rc.axes_labelsize, rc.font_size)
                .with_weight(rc.axes_labelweight),
            color: color.clone(),
        };
        axis_label(
            layout,
            text,
            (to_display(xy)?, ref_angle),
            self.direction,
            radius,
            &style,
            rc,
        )
    }

    /// The look of the plain line (matplotlib's `PathPatch` in
    /// `_init_line`) or of the arrowed one.
    fn line_style_of_patch(&self) -> (PatchStyle, bool) {
        let rc = &self.rc;
        match &self.line_style {
            None => (
                PatchStyle {
                    edgecolor: Some(rc.axes_edgecolor.clone()),
                    facecolor: Some(rc.axes_edgecolor.clone()),
                    fill: false,
                    linewidth: Some(rc.axes_linewidth),
                    capstyle: rc.lines_solid_capstyle,
                    joinstyle: rc.lines_solid_joinstyle,
                    ..PatchStyle::from_rc(rc)
                },
                false,
            ),
            Some(style) => (style.patch_style(rc), true),
        }
    }

    /// The pen of the tick marks: the colour and width of the style axis'
    /// major ticks (matplotlib's `Ticks` copy them from `tick1line`).
    fn mark_gc(&self, axes: &AxesState) -> Result<GraphicsContext, String> {
        let style = &axes.axis(self.style_axis()).major_style;
        Ok(GraphicsContext {
            foreground: style.color.resolve().map_err(|e| e.to_string())?,
            linewidth: style.width,
            alpha: self.base.alpha,
            ..GraphicsContext::default()
        })
    }

    /// The artist properties of the line: the artist's opacity, clipped
    /// to the axes box if asked.
    fn line_base(&self) -> ArtistBase {
        let mut base = ArtistBase::default();
        base.alpha = self.base.alpha;
        base.clip_box = self.line_clipped.then_some(self.axes);
        base
    }
}

impl Artist for AxisArtistState {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let id = self.base.id();
        let drawing = match self.drawing(*ctx.layout()) {
            Ok(drawing) => drawing,
            Err(problem) => return ctx.report(id, problem),
        };
        let layout = *ctx.layout();
        let (Some(axes), Some(transform)) = (
            layout.store().get_as::<AxesState>(self.axes),
            layout.resolve(&self.helper.tick_transform(self.axes)),
        ) else {
            return;
        };
        let gc = match self.mark_gc(axes) {
            Ok(gc) => gc,
            Err(problem) => return ctx.report(id, problem),
        };
        ctx.renderer().open_group(GROUP, self.base.gid.as_deref());
        for level in &drawing.levels {
            if let Some((ticks, marks)) = &level.marks {
                ticks.draw(ctx, marks, &transform, axes.view_lim, &gc);
            }
            for text in &level.labels {
                draw_text(ctx, text, &self.rc);
            }
        }
        if let Some(offset) = &drawing.offset {
            draw_text(ctx, offset, &self.rc);
        }
        let (style, edge_default) = self.line_style_of_patch();
        let line_base = self.line_base();
        if let Err(err) = paint_paths(
            ctx,
            &line_base,
            &style,
            &self.rc,
            edge_default,
            &drawing.line,
        ) {
            ctx.report(id, err);
        }
        if let Some(label) = &drawing.label {
            draw_text(ctx, label, &self.rc);
        }
        ctx.renderer().close_group(GROUP);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let drawing = self.drawing(*ctx).ok()?;
        drawing
            .levels
            .iter()
            .flat_map(|level| &level.labels)
            .chain(&drawing.label)
            .chain(&drawing.offset)
            .map(PlacedText::bbox)
            .chain(drawing.line.iter().map(|p| p.to_display().extents()))
            .filter(|b| b.width() != 0.0 || b.height() != 0.0)
            .reduce(|a, b| a.union(&b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_paths_point_away_from_the_line() {
        let out = Ticks {
            size: 3.5,
            direction: TickDirection::Out,
            visible: true,
        };
        assert_eq!(out.path().vertices()[0], Point::new(-1.0, 0.0));
        let inside = Ticks {
            direction: TickDirection::In,
            ..out
        };
        assert_eq!(inside.path().vertices()[1], Point::new(1.0, 0.0));
    }
}
