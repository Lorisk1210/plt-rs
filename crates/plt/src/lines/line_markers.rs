//! The markers of a line: which points get one (`markevery`), their faces
//! and their drawing (matplotlib's `Line2D.draw` marker half).

use std::fmt;

use plt_core::{Affine2D, Path, Point};
use plt_render::SnapMode;

use super::line::LineArtist;
use super::{MarkEveryError, StepError};
use crate::artist::{DrawContext, LayoutContext};
use crate::colors::Color;
use crate::markers::{FillStyle, Marker, MarkerStyle, marker_sketch};
use crate::rc::ColorOrAuto;
use crate::transforms::ResolvedTransform;

impl LineArtist {
    pub(super) fn draw_markers(
        &self,
        ctx: &mut DrawContext<'_>,
        resolved: &ResolvedTransform,
        points: &Path,
    ) -> Result<(), String> {
        if self.marker.is_nothing() || self.markersize <= 0.0 || points.is_empty() {
            return Ok(());
        }
        let mut style =
            MarkerStyle::new(self.marker.clone(), self.fillstyle).map_err(|e| e.to_string())?;
        if let Some(rotation) = &self.marker_rotation {
            style = style.rotated(rotation.angle(*ctx.layout(), self.base.parent));
        }
        let (points, affine) = resolved.split_points(points);
        let points = match self
            .select_markers(*ctx.layout(), &points, &affine)
            .map_err(|e| e.to_string())?
        {
            Some(indices) => {
                Path::polyline(indices.iter().map(|&i| points.vertices()[i]).collect())
            }
            None => points.into_owned(),
        };
        if points.is_empty() {
            return Ok(());
        }
        let alpha = self.base.alpha;
        let face_spec = self.marker_face(&style, false);
        let face = face_spec.resolve_alpha(alpha).map_err(|e| e.to_string())?;
        let face_alt = self
            .marker_face(&style, true)
            .resolve_alpha(alpha)
            .map_err(|e| e.to_string())?;
        let mut edge = self
            .marker_edge()
            .resolve_alpha(alpha)
            .map_err(|e| e.to_string())?;
        // An automatic edge takes the line colour but the face's alpha.
        if self.markeredgecolor == ColorOrAuto::Auto && !face_spec.is_none() {
            edge = edge.with_alpha(face.a);
        }

        let mut gc = self.line_gc(ctx);
        gc.sketch = marker_sketch(gc.sketch);
        gc.linewidth = self.markeredgewidth;
        gc.foreground = edge;
        gc.join = style.joinstyle();
        gc.cap = style.capstyle();
        let size = ctx.points_to_pixels(self.markersize);
        gc.snap = match style.snap_threshold() {
            Some(threshold) if size >= threshold => SnapMode::Always,
            Some(_) => SnapMode::Never,
            None => gc.snap,
        };
        let is_pixel = matches!(
            self.marker,
            Marker::Symbol(crate::markers::MarkerSymbol::Pixel)
        );
        let scale = |t: Affine2D| if is_pixel { t } else { t.scale(size, size) };
        if is_pixel {
            gc.linewidth = 0.0;
        }
        ctx.renderer().draw_markers(
            &gc,
            style.path(),
            &scale(style.transform()),
            &points,
            &affine,
            Some(face),
        );
        if let (Some(alt_path), Some(alt_transform)) = (style.alt_path(), style.alt_transform()) {
            ctx.renderer().draw_markers(
                &gc,
                alt_path,
                &scale(alt_transform),
                &points,
                &affine,
                Some(face_alt),
            );
        }
        Ok(())
    }

    /// The face colour of the marker (or its alternate half).
    pub(super) fn marker_face(&self, style: &MarkerStyle, alt: bool) -> Color {
        if style.fillstyle() == FillStyle::None {
            return Color::none();
        }
        if alt {
            return self.markerfacecoloralt.clone();
        }
        self.markerfacecolor.or(&self.color).clone()
    }

    /// The indices of `points` (non-affine coordinates, mapped to pixels
    /// by `affine`) that get markers, or `None` for all of them.
    pub(super) fn select_markers(
        &self,
        layout: LayoutContext<'_>,
        points: &Path,
        affine: &Affine2D,
    ) -> Result<Option<Vec<usize>>, MarkEveryError> {
        let Some(markevery) = &self.markevery else {
            return Ok(None);
        };
        let display: Vec<Point> = points
            .vertices()
            .iter()
            .map(|p| affine.transform_point(*p))
            .collect();
        let diagonal = self
            .base
            .parent
            .and_then(|axes| layout.axes_extent(axes))
            .map(|bbox| (bbox.p1.x - bbox.p0.x).hypot(bbox.p1.y - bbox.p0.y));
        markevery.select(&display, diagonal).map(Some)
    }

    /// The indices of the data points that get markers, in drawing order.
    pub(super) fn marker_indices(
        &self,
        layout: LayoutContext<'_>,
    ) -> Result<Vec<usize>, MarkerError> {
        let points = self.points().map_err(MarkerError::Data)?;
        if points.is_empty() {
            return Ok(Vec::new());
        }
        let resolved = layout
            .resolve(&self.transform())
            .ok_or(MarkerError::Detached)?;
        let (points, affine) = resolved.split_points(&points);
        Ok(self
            .select_markers(layout, &points, &affine)
            .map_err(MarkerError::MarkEvery)?
            .unwrap_or_else(|| (0..points.len()).collect()))
    }
}

/// Why the marker positions of a line cannot be determined.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkerError {
    /// The line's data cannot be drawn.
    Data(StepError),
    /// The line's coordinate system no longer exists.
    Detached,
    /// The `markevery` selection does not fit the line.
    MarkEvery(MarkEveryError),
}

impl fmt::Display for MarkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data(err) => write!(f, "invalid line data: {err}"),
            Self::Detached => f.write_str("the line's coordinate system no longer exists"),
            Self::MarkEvery(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for MarkerError {}
