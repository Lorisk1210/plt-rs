//! The markers of a line: which points get one (`markevery`), their faces
//! and their drawing (matplotlib's `Line2D.draw` marker half).

use std::fmt;

use plt_core::{Affine2D, Path, Point};
use plt_render::SnapMode;

use super::line::{LineArtist, LineProps};
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
        let mut style = self.marker_style().map_err(|e| e.to_string())?;
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
        // Drawn even when `markevery` picks no point, as matplotlib does:
        // PostScript and PGF still define the marker.
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

    /// The marker as drawn: its shape and fill style with the user
    /// transform, cap and join styles (matplotlib's `Line2D._marker`).
    fn marker_style(&self) -> Result<MarkerStyle, crate::markers::MarkerError> {
        let style = MarkerStyle::with_styles(
            self.marker.clone(),
            self.fillstyle,
            self.marker_capstyle,
            self.marker_joinstyle,
        )?;
        Ok(match self.marker_transform {
            Some(transform) => style.transformed(transform),
            None => style,
        })
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
    ) -> Result<Vec<usize>, MarkerIndicesError> {
        let points = self.points().map_err(MarkerIndicesError::Data)?;
        if points.is_empty() {
            return Ok(Vec::new());
        }
        let resolved = layout
            .resolve(&self.transform())
            .ok_or(MarkerIndicesError::Detached)?;
        let (points, affine) = resolved.split_points(&points);
        Ok(self
            .select_markers(layout, &points, &affine)
            .map_err(MarkerIndicesError::MarkEvery)?
            .unwrap_or_else(|| (0..points.len()).collect()))
    }
}

impl LineProps {
    /// These properties with every part of `style`: the marker, its fill
    /// style and its user transform, cap and join styles (matplotlib's
    /// `plot(..., marker=MarkerStyle(...))`).
    ///
    /// ```
    /// use plt::lines::LineProps;
    /// use plt::markers::{FillStyle, MarkerStyle};
    /// use plt::transforms::Affine2D;
    /// let tilted = MarkerStyle::new("o".parse::<plt::markers::Marker>()?, FillStyle::Left)?
    ///     .rotated_deg(30.0);
    /// let props = LineProps::default().marker_style(&tilted);
    /// assert_eq!(props.fillstyle, Some(FillStyle::Left));
    /// assert_eq!(props.marker_transform, Some(Some(Affine2D::identity().rotate_deg(30.0))));
    /// # Ok::<(), plt::markers::MarkerError>(())
    /// ```
    #[must_use]
    pub fn marker_style(self, style: &MarkerStyle) -> Self {
        self.marker(style.marker().clone())
            .fillstyle(style.fillstyle())
            .marker_transform(style.user_transform())
            .marker_capstyle(style.user_capstyle())
            .marker_joinstyle(style.user_joinstyle())
    }
}

/// Why the marker positions of a line cannot be determined.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkerIndicesError {
    /// The line's data cannot be drawn.
    Data(StepError),
    /// The line's coordinate system no longer exists.
    Detached,
    /// The `markevery` selection does not fit the line.
    MarkEvery(MarkEveryError),
}

impl fmt::Display for MarkerIndicesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data(err) => write!(f, "invalid line data: {err}"),
            Self::Detached => f.write_str("the line's coordinate system no longer exists"),
            Self::MarkEvery(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for MarkerIndicesError {}

#[cfg(test)]
mod tests {
    use plt_core::Point;
    use plt_render::{CapStyle, JoinStyle};

    use super::LineArtist;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::lines::LineProps;
    use crate::markers::{FillStyle, MarkerStyle, MarkerSymbol};
    use crate::testing::{Call, RecordingRenderer};

    #[test]
    fn marker_styles_carry_their_transform_and_edge_styles() {
        let style = MarkerStyle::with_styles(
            MarkerSymbol::Star,
            FillStyle::Left,
            Some(CapStyle::Round),
            Some(JoinStyle::Bevel),
        )
        .unwrap()
        .rotated_deg(30.0);
        let ax = Figure::new().subplot();
        let line = ax
            .plot_with([0.0], [0.0], LineProps::default().marker_style(&style))
            .unwrap();
        let drawn = line
            .raw()
            .with::<LineArtist, _>(LineArtist::marker_style)
            .unwrap();
        assert_eq!(drawn.fillstyle(), FillStyle::Left);
        assert_eq!(drawn.user_transform(), style.user_transform());
        assert_eq!(
            (drawn.capstyle(), drawn.joinstyle()),
            (CapStyle::Round, JoinStyle::Bevel)
        );
    }

    #[test]
    fn markers_are_drawn_through_their_transform() {
        let fig = Figure::with_size(2.0, 2.0, 72.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 1.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .expect("finite limits");
        let upright = MarkerStyle::new(MarkerSymbol::HLine, FillStyle::Full)
            .unwrap()
            .rotated_deg(90.0);
        let props = LineProps::default().marker_style(&upright).markersize(10.0);
        ax.plot_with([0.5], [0.5], props).unwrap();
        let mut renderer = RecordingRenderer::new(144.0, 144.0, 72.0);
        fig.draw(&mut renderer).unwrap();
        let near_centre = |points: &[Point]| {
            points
                .iter()
                .all(|p| (p.x - 72.0).abs() <= 6.0 && (p.y - 72.0).abs() <= 6.0)
        };
        let mut marker = renderer
            .calls
            .iter()
            .find_map(|call| match call {
                Call::Path { points, .. } if points.len() == 2 && near_centre(points) => {
                    Some(points.clone())
                }
                _ => None,
            })
            .unwrap();
        marker.sort_by(|a, b| a.y.total_cmp(&b.y));
        let expected = [Point::new(72.0, 67.0), Point::new(72.0, 77.0)];
        for (point, expected) in marker.iter().zip(expected) {
            assert!((point.x - expected.x).abs() < 1e-9, "{point:?}");
            assert!((point.y - expected.y).abs() < 1e-9, "{point:?}");
        }
    }
}
