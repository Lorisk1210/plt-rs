//! `custom_projection`: the gallery's Hammer projection written as user
//! code and registered as `custom_hammer`, with the example's abstract
//! geographic base (`GeoAxes`), its tick formatter and the Hammer
//! transform with its inverse.

use std::error::Error;
use std::f64::consts::{FRAC_PI_2, PI, SQRT_2, TAU};
use std::marker::PhantomData;
use std::rc::Rc;

use plt::axes::{Adjustable, Axis, AxisName, TickParams, TickPosition};
use plt::numeric::arange;
use plt::path::Path;
use plt::prelude::*;
use plt::projections::{self, AxesOutline, Projection, ProjectionView, TickLabelPlacement};
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::text::{HAlign, VAlign};
use plt::ticker::{
    FixedLocator, FormattedTicks, Formatter, NullLocator, TickContext, TickerError, pyfmt,
};
use plt::transforms::{Affine2D, Anchor, Point, PointMap, ResolvedTransform, Step};

use crate::figures::{CaseResult, FigureCase};

/// The example's `GeoAxes.ThetaFormatter`: radians as degrees rounded to
/// a multiple of `round_to`, with a degree sign.
#[derive(Debug)]
struct ThetaFormatter {
    round_to: f64,
}

impl ThetaFormatter {
    /// `round(np.rad2deg(x) / round_to) * round_to` as `'{:0.0f}°'`;
    /// Python's `round` gives an integer, which has no negative zero.
    fn label(&self, radians: f64) -> String {
        let units = (radians.to_degrees() / self.round_to).round_ties_even();
        let units = if units == 0.0 { 0.0 } else { units };
        format!("{}\u{b0}", pyfmt::fixed(units * self.round_to, 0))
    }
}

impl Formatter for ThetaFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        _: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        Ok(FormattedTicks {
            labels: locs.iter().map(|&x| self.label(x)).collect(),
            offset: String::new(),
        })
    }

    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(self.label(value))
    }
}

/// What a subclass of the example's `GeoAxes` provides: its name and its
/// core transform.
trait GeoCore: PointMap + 'static {
    /// The projection's name.
    const NAME: &'static str;

    /// The core transform, cutting every path segment into `resolution`
    /// pieces (`_get_core_transform`).
    fn with_resolution(resolution: usize) -> Self;
}

/// The example's abstract `GeoAxes`: longitude and latitude drawn through
/// the core transform `C` inside an ellipse, with fixed limits and grids
/// every so many degrees.
#[derive(Debug)]
struct GeoAxes<C> {
    /// The latitude, in radians, where longitude gridlines stop
    /// (`_longitude_cap`).
    longitude_cap: f64,
    core: PhantomData<C>,
}

impl<C: GeoCore> GeoAxes<C> {
    /// Pieces every path segment is cut into before it is projected.
    const RESOLUTION: usize = 75;

    /// `HammerAxes.__init__`: longitude gridlines reach the poles until
    /// the set-up stops them.
    fn new() -> Self {
        Self {
            longitude_cap: FRAC_PI_2,
            core: PhantomData,
        }
    }

    /// `transProjection`.
    fn trans_projection() -> Rc<dyn PointMap> {
        Rc::new(C::with_resolution(Self::RESOLUTION))
    }

    /// `_get_affine_transform`: the projected map onto the unit square.
    fn trans_affine() -> Affine2D {
        let transform = C::with_resolution(1);
        let xscale = transform.forward(Point::new(PI, 0.0)).x;
        let yscale = transform.forward(Point::new(0.0, FRAC_PI_2)).y;
        Affine2D::scaling(0.5 / xscale, 0.5 / yscale).translate(0.5, 0.5)
    }

    /// `_xaxis_pretransform`: longitude gridlines span the latitudes up to
    /// the cap.
    fn xaxis_pretransform(&self) -> Affine2D {
        Affine2D::scaling(1.0, self.longitude_cap * 2.0).translate(0.0, -self.longitude_cap)
    }

    /// `yaxis_stretch`: latitude gridlines span every longitude.
    fn yaxis_stretch() -> Affine2D {
        Affine2D::scaling(TAU, 1.0).translate(-PI, 0.0)
    }

    /// `set_longitude_grid_ends`.
    fn set_longitude_grid_ends(&mut self, degrees: f64) {
        self.longitude_cap = degrees.to_radians();
    }
}

/// `set_longitude_grid`: a gridline every `degrees`, skipping ±180°.
fn set_longitude_grid(axes: &Axes, degrees: f64) {
    install_grid(&axes.xaxis(), degrees, 180.0);
}

/// `set_latitude_grid`: a gridline every `degrees`, skipping the poles.
fn set_latitude_grid(axes: &Axes, degrees: f64) {
    install_grid(&axes.yaxis(), degrees, 90.0);
}

/// Gridlines every `degrees` strictly between `-limit` and `limit` on
/// `axis`, labelled to multiples of `degrees`.
fn install_grid(axis: &Axis, degrees: f64, limit: f64) {
    let grid = arange(-limit + degrees, limit, degrees);
    axis.set_major_locator(FixedLocator::new(
        grid.into_iter().map(f64::to_radians).collect(),
    ))
    .set_major_formatter(ThetaFormatter { round_to: degrees });
}

impl<C: GeoCore> Projection for GeoAxes<C> {
    fn name(&self) -> &'static str {
        C::NAME
    }

    /// `transProjection + transAffine`.
    fn data_to_axes(&self, _: &ProjectionView<'_>) -> ResolvedTransform {
        ResolvedTransform::with_step(Step::Plane(Self::trans_projection()), Self::trans_affine())
    }

    /// `HammerAxes.__init__` after the base class, then `GeoAxes.clear`.
    fn set_up(&mut self, axes: &Axes) -> Result<(), Box<dyn Error>> {
        axes.set_aspect(Aspect::Ratio(0.5))?
            .set_adjustable(Adjustable::Box)?
            .set_anchor(Anchor::C);
        set_longitude_grid(axes, 30.0);
        set_latitude_grid(axes, 15.0);
        self.set_longitude_grid_ends(75.0);
        for axis in [axes.xaxis(), axes.yaxis()] {
            axis.set_minor_locator(NullLocator);
            axis.set_ticks_position(TickPosition::None)?;
        }
        axes.yaxis().set_tick_params(&TickParams {
            labelleft: Some(true),
            ..TickParams::default()
        });
        axes.grid(plt::rc::params().axes_grid);
        axes.set_xlim(-PI, PI)?.set_ylim(-FRAC_PI_2, FRAC_PI_2)?;
        Ok(())
    }

    /// `_xaxis_transform` and `_yaxis_transform`.
    fn axis_transform(
        &self,
        view: &ProjectionView<'_>,
        axis: AxisName,
    ) -> Option<ResolvedTransform> {
        let pretransform = match axis {
            AxisName::X => self.xaxis_pretransform(),
            AxisName::Y => Self::yaxis_stretch(),
        };
        Some(ResolvedTransform::affine(pretransform).then(self.data_to_axes(view)))
    }

    /// `_xaxis_text1_transform` and friends: longitude labels 4 pixels off
    /// the equator, latitude labels 8 pixels off the edge, spread out in
    /// projected coordinates (`yaxis_space`).
    fn tick_label(
        &self,
        view: &ProjectionView<'_>,
        side: SpineSide,
        loc: f64,
    ) -> Option<TickLabelPlacement> {
        let (anchor, offset, halign, valign) = match side {
            SpineSide::Bottom | SpineSide::Top => {
                let equator = self
                    .data_to_axes(view)
                    .transform_point(Point::new(loc, 0.0));
                if side == SpineSide::Bottom {
                    (
                        equator,
                        Point::new(0.0, 4.0),
                        HAlign::Center,
                        VAlign::Bottom,
                    )
                } else {
                    (equator, Point::new(0.0, -4.0), HAlign::Center, VAlign::Top)
                }
            }
            SpineSide::Left | SpineSide::Right => {
                let edge = if side == SpineSide::Left { 0.0 } else { 1.0 };
                let lonlat = Self::yaxis_stretch().transform_point(Point::new(edge, loc));
                let projected = Self::trans_projection().forward(lonlat);
                let yaxis_space = Affine2D::scaling(1.0, 1.1);
                let anchor = yaxis_space
                    .then(Self::trans_affine())
                    .transform_point(projected);
                if side == SpineSide::Left {
                    (anchor, Point::new(-8.0, 0.0), HAlign::Right, VAlign::Center)
                } else {
                    (anchor, Point::new(8.0, 0.0), HAlign::Left, VAlign::Center)
                }
            }
        };
        Some(TickLabelPlacement {
            anchor,
            offset,
            halign,
            valign,
        })
    }

    /// `_gen_axes_patch` and the circular `geo` spine.
    fn outline(&self) -> AxesOutline {
        AxesOutline::Path(Path::circle(Point::new(0.5, 0.5), 0.5))
    }

    /// `set_xlim` and `set_ylim` raise.
    fn fixed_limits(&self) -> bool {
        true
    }

    /// `set_xscale` and `set_yscale` raise for other scales.
    fn linear_only(&self) -> bool {
        true
    }

    fn data_ratio(&self) -> Option<f64> {
        Some(1.0)
    }

    fn can_zoom(&self) -> bool {
        false
    }

    fn can_pan(&self) -> bool {
        false
    }

    /// Degrees north or south and east or west.
    fn format_coord(&self, lon: f64, lat: f64) -> Option<String> {
        let (lon, lat) = (lon.to_degrees(), lat.to_degrees());
        let ns = if lat >= 0.0 { 'N' } else { 'S' };
        let ew = if lon >= 0.0 { 'E' } else { 'W' };
        Some(format!(
            "{}\u{b0}{ns}, {}\u{b0}{ew}",
            pyfmt::fixed(lat.abs(), 6),
            pyfmt::fixed(lon.abs(), 6)
        ))
    }
}

/// The example's `HammerAxes.HammerTransform`, whose inverse is
/// `InvertedHammerTransform`.
#[derive(Debug)]
struct HammerTransform {
    resolution: usize,
}

impl GeoCore for HammerTransform {
    const NAME: &'static str = "custom_hammer";

    fn with_resolution(resolution: usize) -> Self {
        Self { resolution }
    }
}

impl PointMap for HammerTransform {
    fn forward(&self, lonlat: Point) -> Point {
        let (longitude, latitude) = (lonlat.x, lonlat.y);
        let half_long = longitude / 2.0;
        let cos_latitude = latitude.cos();
        let alpha = (1.0 + cos_latitude * half_long.cos()).sqrt();
        Point::new(
            (2.0 * SQRT_2) * (cos_latitude * half_long.sin()) / alpha,
            (SQRT_2 * latitude.sin()) / alpha,
        )
    }

    fn inverse(&self, xy: Point) -> Point {
        let (x, y) = (xy.x, xy.y);
        let z = (1.0 - (x / 4.0).powi(2) - (y / 2.0).powi(2)).sqrt();
        let longitude = 2.0 * ((z * x) / (2.0 * (2.0 * z.powi(2) - 1.0))).atan();
        Point::new(longitude, (y * z).asin())
    }

    /// `transform_path_non_affine`: the path interpolated, then projected.
    fn transform_path(&self, path: &Path) -> Path {
        let mut projected = path.interpolated(self.resolution);
        for p in projected.vertices_mut() {
            *p = self.forward(*p);
        }
        projected
    }
}

/// The example's `HammerAxes`.
type HammerAxes = GeoAxes<HammerTransform>;

fn custom_projection(fig: &Figure) -> CaseResult {
    let kind = projections::register(HammerAxes::new)?;
    let ax = fig.subplot_with_projection(kind)?;
    ax.plot_fmt([-1.0, 1.0, 1.0], [-1.0, -1.0, 1.0], "o-")?;
    ax.grid(true);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[FigureCase::new(
    "gallery_misc/custom_projection",
    custom_projection,
)];

#[cfg(test)]
mod tests {
    use plt::axes::ProjectionKind;

    use super::*;

    #[test]
    fn the_hammer_transform_inverts() {
        let hammer = HammerTransform::with_resolution(1);
        let point = Point::new(-2.0, 0.7);
        let back = hammer.inverse(hammer.forward(point));
        assert!((back.x - point.x).abs() < 1e-12 && (back.y - point.y).abs() < 1e-12);
        let ax = Figure::new()
            .subplot_with_projection(projections::register(HammerAxes::new).unwrap())
            .unwrap();
        assert_eq!(ax.projection(), ProjectionKind::Registered("custom_hammer"));
        assert_eq!(ax.format_coord(-PI / 2.0, 0.5), "28.647890°N, 90.000000°W");
        assert!(!ax.can_zoom() && !ax.can_pan());
        let cap = ax.with_projection(|hammer: &HammerAxes| hammer.longitude_cap);
        assert_eq!(cap, Some(75f64.to_radians()));
    }
}
