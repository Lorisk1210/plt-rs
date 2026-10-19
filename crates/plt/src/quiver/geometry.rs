//! Sizing and placing quiver arrows from the axes geometry at draw time
//! (matplotlib's `Quiver._init`, `_set_transform`, `_dots_per_unit`,
//! `_angles_lengths` and `_make_verts`).

use std::any::Any;

use plt_core::{Affine2D, Bbox, Path, Point};

use super::arrows::{ArrowShape, Pivot};
use crate::artist::{ArtistId, LayoutContext};
use crate::axes::AxesState;
use crate::collections::DrawTimeShapes;
use crate::transforms::{ResolvedTransform, Transform};

/// Units of quiver arrow dimensions (matplotlib's `units` and
/// `scale_units`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum QuiverUnits {
    /// The width of the axes.
    #[default]
    Width,
    /// The height of the axes.
    Height,
    /// Pixels.
    Dots,
    /// Inches.
    Inches,
    /// x data units.
    X,
    /// y data units.
    Y,
    /// The diagonal of the view in data units; as scale units, arrows
    /// span `(u, v)` in data coordinates.
    Xy,
}

/// How arrow directions are computed (matplotlib's `angles`).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Angles {
    /// `atan2(v, u)` on the screen, whatever the aspect ratio.
    #[default]
    Uv,
    /// From `(x, y)` towards `(x + u, y + v)` in data coordinates.
    Xy,
    /// Explicit angles in degrees counter-clockwise from the x axis, one
    /// per arrow or one for all.
    Degrees(Vec<f64>),
}

/// The arrows of one quiver plot, rebuilt at every draw.
#[derive(Debug)]
pub(crate) struct QuiverGeometry {
    axes: ArtistId,
    xy: Vec<Point>,
    /// Components, 1 where masked (matplotlib's `filled(1)`).
    u: Vec<f64>,
    v: Vec<f64>,
    mask: Vec<bool>,
    sizing: ArrowSizing,
    /// Whether `u` and `v` were scalars broadcast to every position.
    scalar_components: bool,
}

/// Everything that sizes and orients the arrows of a quiver plot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ArrowSizing {
    pub(crate) scale: Option<f64>,
    pub(crate) scale_units: Option<QuiverUnits>,
    pub(crate) units: QuiverUnits,
    pub(crate) angles: Angles,
    pub(crate) width: Option<f64>,
    pub(crate) shape: ArrowShape,
    pub(crate) pivot: Pivot,
}

/// The arrow sizes for one draw: pixels per width unit, the shaft width
/// and the scale (matplotlib's `_trans_scale`, `width` and `scale` after
/// `_init`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Resolved {
    pub(crate) trans_scale: f64,
    width: f64,
    scale: f64,
    widthu_per_lenu: f64,
}

/// Vectors to turn into arrows.
struct Vectors<'a> {
    xy: &'a [Point],
    u: &'a [f64],
    v: &'a [f64],
    angles: &'a Angles,
}

impl QuiverGeometry {
    /// The arrows at `xy` of the components `u`, `v` (`mask` marks the
    /// vectors not drawn) in the axes `axes`.
    pub(crate) fn new(
        axes: ArtistId,
        xy: Vec<Point>,
        (u, v): (Vec<f64>, Vec<f64>),
        mask: Vec<bool>,
        sizing: ArrowSizing,
    ) -> Self {
        let fill = |values: Vec<f64>| -> Vec<f64> {
            values
                .into_iter()
                .zip(&mask)
                .map(|(value, &masked)| if masked { 1.0 } else { value })
                .collect()
        };
        Self {
            axes,
            xy,
            u: fill(u),
            v: fill(v),
            mask,
            sizing,
            scalar_components: false,
        }
    }

    /// Records whether the components were scalars broadcast to every
    /// position. matplotlib keeps scalar `U` and `V` as they are, so it
    /// draws one arrow at every position, unless the positions or
    /// per-arrow angles orient the arrows one by one.
    #[must_use]
    pub(crate) fn with_scalar_components(mut self, scalar: bool) -> Self {
        self.scalar_components = scalar;
        self
    }

    /// The axes the arrows belong to.
    pub(crate) fn axes(&self) -> ArtistId {
        self.axes
    }

    fn vectors(&self) -> Vectors<'_> {
        Vectors {
            xy: &self.xy,
            u: &self.u,
            v: &self.v,
            angles: &self.sizing.angles,
        }
    }

    /// The vectors to make arrows of: only the first one when it serves
    /// every position (see [`Self::with_scalar_components`]).
    fn arrow_vectors(&self) -> Vectors<'_> {
        let per_position = self.sizing.scale_units == Some(QuiverUnits::Xy)
            || match &self.sizing.angles {
                Angles::Uv => false,
                Angles::Xy => true,
                Angles::Degrees(degrees) => degrees.len() > 1,
            };
        let uniform_mask = self.mask.windows(2).all(|pair| pair[0] == pair[1]);
        if self.scalar_components && !per_position && uniform_mask && !self.xy.is_empty() {
            Vectors {
                xy: &self.xy[..1],
                u: &self.u[..1],
                v: &self.v[..1],
                angles: &self.sizing.angles,
            }
        } else {
            self.vectors()
        }
    }

    /// The pixel scale, width and arrow scale for the current geometry
    /// (matplotlib's `_init`, autoscaling `scale` from the mean magnitude
    /// of the unmasked vectors when it is unset).
    pub(crate) fn resolve(&self, frame: &AxesFrame) -> Resolved {
        let sizing = &self.sizing;
        let trans_scale = frame.dots_per_unit(sizing.units);
        let inverse = 1.0 / trans_scale;
        let span = frame.bbox.p1.x * inverse - frame.bbox.p0.x * inverse;
        let n = self.xy.len() as f64;
        let width = sizing
            .width
            .unwrap_or_else(|| 0.06 * span / n.sqrt().clamp(8.0, 25.0));
        let widthu_per_lenu = match sizing.scale_units {
            None => 1.0,
            Some(QuiverUnits::Xy) => 1.0 / trans_scale,
            Some(units) => frame.dots_per_unit(units) / trans_scale,
        };
        let scale = sizing.scale.unwrap_or_else(|| {
            let (_, magnitudes) = self.directions_and_magnitudes(frame, &self.vectors());
            let unmasked: Vec<f64> = magnitudes
                .iter()
                .zip(&self.mask)
                .filter_map(|(&a, &masked)| (!masked).then_some(a))
                .collect();
            let mean = crate::stats::mean(&unmasked);
            let auto = 1.8 * mean * n.sqrt().max(10.0) / span;
            auto * widthu_per_lenu
        });
        Resolved {
            trans_scale,
            width,
            scale,
            widthu_per_lenu,
        }
    }

    /// The display angles (for `xy` angles or scale units) and the
    /// magnitudes the arrow lengths are proportional to.
    fn directions_and_magnitudes(
        &self,
        frame: &AxesFrame,
        vectors: &Vectors<'_>,
    ) -> (Option<Vec<f64>>, Vec<f64>) {
        let xy_angles = matches!(vectors.angles, Angles::Xy);
        let xy_scale = self.sizing.scale_units == Some(QuiverUnits::Xy);
        let display = if xy_angles && xy_scale {
            // eps = 1 so that vectors diffed from the positions connect
            // them, whatever the axis scales.
            Some(frame.angles_lengths(vectors, 1.0))
        } else if xy_angles || xy_scale {
            Some(frame.angles_lengths(vectors, frame.eps()))
        } else {
            None
        };
        let named_angles = !matches!(vectors.angles, Angles::Degrees(_));
        match display {
            Some((angles, lengths)) if named_angles && xy_scale => (Some(angles), lengths),
            display => {
                let magnitudes = vectors
                    .u
                    .iter()
                    .zip(vectors.v)
                    .map(|(u, v)| u.hypot(*v))
                    .collect();
                (display.map(|(angles, _)| angles), magnitudes)
            }
        }
    }

    /// The outlines of `vectors` in arrow-width units, rotated and scaled
    /// by the shaft width (`None` for masked vectors).
    fn outlines(
        &self,
        frame: &AxesFrame,
        resolved: &Resolved,
        vectors: &Vectors<'_>,
        pivot: Pivot,
        mask: Option<&[bool]>,
    ) -> Vec<Option<[Point; 8]>> {
        let (display_angles, magnitudes) = self.directions_and_magnitudes(frame, vectors);
        let factor = resolved.widthu_per_lenu / (resolved.scale * resolved.width);
        (0..vectors.xy.len())
            .map(|i| {
                if mask.is_some_and(|m| m[i]) {
                    return None;
                }
                let theta = match (vectors.angles, &display_angles) {
                    (Angles::Xy, Some(angles)) => angles[i],
                    (Angles::Degrees(degrees), _) => {
                        let value = degrees[i % degrees.len()].to_radians();
                        if value.is_finite() { value } else { 0.0 }
                    }
                    _ => complex_angle(vectors.u[i], vectors.v[i]),
                };
                let outline = self.sizing.shape.outline(magnitudes[i] * factor, pivot);
                let (sin, cos) = theta.sin_cos();
                Some(outline.map(|p| {
                    Point::new(
                        (p.x * cos - p.y * sin) * resolved.width,
                        (p.x * sin + p.y * cos) * resolved.width,
                    )
                }))
            })
            .collect()
    }

    /// The reference arrow of a quiver key, `(u, v)` at the origin with
    /// the key's pivot and this plot's scale, and the pixels per arrow
    /// width unit (matplotlib's `QuiverKey._init`).
    pub(crate) fn key_arrow(
        &self,
        frame: &AxesFrame,
        (u, v): (f64, f64),
        pivot: Pivot,
    ) -> ([Point; 8], f64) {
        let resolved = self.resolve(frame);
        let vectors = Vectors {
            xy: &[Point::ORIGIN],
            u: &[u],
            v: &[v],
            angles: &Angles::Uv,
        };
        let outline = self.outlines(frame, &resolved, &vectors, pivot, None)[0]
            .expect("an unmasked vector has an outline");
        (outline, resolved.trans_scale)
    }
}

/// The angle of `u + v·i` as numpy builds and measures the complex
/// number (`np.angle(U + V * 1j)`): its arithmetic turns a `-0` component
/// into `+0` (unless `u` and `v` are both negative zeros), so e.g. the zero
/// vector `(-0, 0)` points along +x, not -x.
fn complex_angle(u: f64, v: f64) -> f64 {
    let re = u + (v * 0.0 - 0.0);
    let im = 0.0 + (v + 0.0);
    im.atan2(re)
}

impl DrawTimeShapes for QuiverGeometry {
    fn shapes(&self, ctx: LayoutContext<'_>) -> Result<(Vec<Path>, Transform), String> {
        let frame = AxesFrame::new(self.axes, ctx)?;
        let resolved = self.resolve(&frame);
        let paths = self
            .outlines(
                &frame,
                &resolved,
                &self.arrow_vectors(),
                self.sizing.pivot,
                Some(&self.mask),
            )
            .into_iter()
            .map(|outline| outline.map_or_else(Path::default, |p| Path::polyline(p.to_vec())))
            .collect();
        let scale = resolved.trans_scale;
        Ok((paths, Transform::Affine(Affine2D::scaling(scale, scale))))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The geometry of an axes that quiver arrows depend on.
pub(crate) struct AxesFrame {
    /// The axes in display coordinates.
    bbox: Bbox,
    /// The view limits.
    view: Bbox,
    /// The data limits.
    data_lim: Bbox,
    dpi: f64,
    data: ResolvedTransform,
}

impl AxesFrame {
    /// The current geometry of `axes`.
    pub(crate) fn new(axes: ArtistId, ctx: LayoutContext<'_>) -> Result<Self, String> {
        let gone = || "its axes no longer exist".to_owned();
        let state = ctx.store().get_as::<AxesState>(axes).ok_or_else(gone)?;
        Ok(Self {
            bbox: ctx.axes_extent(axes).ok_or_else(gone)?,
            view: state.view_lim,
            data_lim: state.data_lim.bbox,
            dpi: ctx.dpi(),
            data: ctx.resolve(&Transform::Data(axes)).ok_or_else(gone)?,
        })
    }

    /// Pixels per unit (matplotlib's `_dots_per_unit`).
    fn dots_per_unit(&self, units: QuiverUnits) -> f64 {
        let (bb, vl) = (&self.bbox, &self.view);
        match units {
            QuiverUnits::X => bb.width() / vl.width(),
            QuiverUnits::Y => bb.height() / vl.height(),
            QuiverUnits::Xy => bb.width().hypot(bb.height()) / vl.width().hypot(vl.height()),
            QuiverUnits::Width => bb.width(),
            QuiverUnits::Height => bb.height(),
            QuiverUnits::Dots => 1.0,
            QuiverUnits::Inches => self.dpi,
        }
    }

    /// A step small against the data extent, for measuring directions
    /// without round-off.
    fn eps(&self) -> f64 {
        let Bbox { p0, p1 } = self.data_lim;
        [p0.x, p0.y, p1.x, p1.y]
            .into_iter()
            .map(f64::abs)
            .fold(f64::NEG_INFINITY, f64::max)
            * 0.001
    }

    /// The display angle and length (per `eps` data step) of every vector.
    fn angles_lengths(&self, vectors: &Vectors<'_>, eps: f64) -> (Vec<f64>, Vec<f64>) {
        (0..vectors.xy.len())
            .map(|i| {
                let p = vectors.xy[i];
                let start = self.data.transform_point(p);
                let end = self.data.transform_point(Point::new(
                    p.x + eps * vectors.u[i],
                    p.y + eps * vectors.v[i],
                ));
                let (dx, dy) = (end.x - start.x, end.y - start.y);
                (dy.atan2(dx), dx.hypot(dy) / eps)
            })
            .unzip()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    fn axes() -> ArtistId {
        Figure::new().subplot().id()
    }

    fn frame() -> AxesFrame {
        AxesFrame {
            bbox: Bbox::from_extents(80.0, 52.8, 576.0, 422.4),
            view: Bbox::from_extents(-2.2, -2.2, 2.2, 2.2),
            data_lim: Bbox::from_extents(-2.0, -2.0, 2.0, 2.0),
            dpi: 100.0,
            data: ResolvedTransform::identity(),
        }
    }

    fn sizing() -> ArrowSizing {
        ArrowSizing {
            scale: None,
            scale_units: None,
            units: QuiverUnits::Width,
            angles: Angles::Uv,
            width: None,
            shape: ArrowShape::default(),
            pivot: Pivot::Tail,
        }
    }

    #[test]
    fn units_follow_the_axes_geometry() {
        let frame = frame();
        assert_eq!(frame.dots_per_unit(QuiverUnits::Width), 496.0);
        assert_eq!(frame.dots_per_unit(QuiverUnits::Dots), 1.0);
        assert_eq!(frame.dots_per_unit(QuiverUnits::Inches), 100.0);
        assert!((frame.dots_per_unit(QuiverUnits::X) - 496.0 / 4.4).abs() < 1e-12);
        assert!((frame.eps() - 0.002).abs() < 1e-18);
    }

    #[test]
    fn autoscaling_matches_matplotlib() {
        // matplotlib: quiver([0, 1], [0, 0], [3, 4], [4, 3]) in a 6.4 x 4.8
        // figure: span 0.9999999999999999, width 0.06 * span / 8.
        let geometry = QuiverGeometry::new(
            axes(),
            vec![Point::new(0.0, 0.0), Point::new(1.0, 0.0)],
            (vec![3.0, 4.0], vec![4.0, 3.0]),
            vec![false, false],
            sizing(),
        );
        let resolved = geometry.resolve(&frame());
        assert_eq!(resolved.trans_scale, 496.0);
        let span = 576.0 * (1.0 / 496.0) - 80.0 * (1.0 / 496.0);
        assert_eq!(resolved.width, 0.06 * span / 8.0);
        assert_eq!(resolved.scale, 1.8 * 5.0 * 10.0 / span);
    }

    #[test]
    fn masked_vectors_have_no_outline_and_do_not_scale() {
        let geometry = QuiverGeometry::new(
            axes(),
            vec![Point::ORIGIN; 2],
            (vec![1.0, f64::NAN], vec![0.0, 0.0]),
            vec![false, true],
            sizing(),
        );
        let frame = frame();
        let resolved = geometry.resolve(&frame);
        assert!(resolved.scale.is_finite());
        let outlines = geometry.outlines(
            &frame,
            &resolved,
            &geometry.vectors(),
            Pivot::Tail,
            Some(&geometry.mask),
        );
        assert!(outlines[0].is_some());
        assert!(outlines[1].is_none());
    }

    #[test]
    fn scalar_components_make_one_arrow_unless_positions_orient_them() {
        let positions = vec![Point::ORIGIN, Point::new(1.0, 0.0), Point::new(2.0, 0.0)];
        let geometry = |angles: Angles| {
            let sizing = ArrowSizing { angles, ..sizing() };
            QuiverGeometry::new(
                axes(),
                positions.clone(),
                (vec![2.0; 3], vec![2.0; 3]),
                vec![false; 3],
                sizing,
            )
        };
        assert_eq!(geometry(Angles::Uv).arrow_vectors().xy.len(), 3);
        let shared = geometry(Angles::Uv).with_scalar_components(true);
        assert_eq!(shared.arrow_vectors().xy.len(), 1);
        let oriented = geometry(Angles::Xy).with_scalar_components(true);
        assert_eq!(oriented.arrow_vectors().xy.len(), 3);
        let per_arrow =
            geometry(Angles::Degrees(vec![0.0, 90.0, 180.0])).with_scalar_components(true);
        assert_eq!(per_arrow.arrow_vectors().xy.len(), 3);
    }

    #[test]
    fn zero_vectors_point_like_numpy() {
        assert_eq!(complex_angle(-0.0, 0.0), 0.0);
        assert_eq!(complex_angle(-0.0, -0.0), std::f64::consts::PI);
        assert_eq!(complex_angle(-1.0, -0.0), std::f64::consts::PI);
        assert_eq!(complex_angle(0.0, -1.0), -std::f64::consts::FRAC_PI_2);
        assert_eq!(complex_angle(-1.0, 0.0), std::f64::consts::PI);
    }

    #[test]
    fn degrees_and_xy_angles() {
        let mut sizing = sizing();
        sizing.angles = Angles::Degrees(vec![90.0]);
        sizing.scale = Some(1.0);
        sizing.width = Some(0.01);
        let geometry = QuiverGeometry::new(
            axes(),
            vec![Point::ORIGIN],
            (vec![1.0], vec![0.0]),
            vec![false],
            sizing,
        );
        let frame = frame();
        let resolved = geometry.resolve(&frame);
        let outline = geometry.outlines(&frame, &resolved, &geometry.vectors(), Pivot::Tail, None)
            [0]
        .expect("unmasked");
        assert!(
            outline[3].x.abs() < 1e-12 && outline[3].y > 0.0,
            "points up"
        );
    }
}
