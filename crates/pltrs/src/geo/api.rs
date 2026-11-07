//! The [`GeoAxes`] handle: creating geographic axes and their grid
//! settings (matplotlib's `GeoAxes` methods).

use std::fmt;
use std::ops::Deref;

use super::ticker::{GeoThetaFormatter, grid_locator};
use super::{GeoProjection, GeoState, LATITUDE_EDGE, LONGITUDE_EDGE};
use crate::axes::{Axes, Axis};
use crate::figure::{AxesRectError, Figure};
use crate::gridspec::{GridSpecError, SubplotSpec};
use crate::projections::{OUTLINE_SPINE, install};
use crate::spines::Spine;
use crate::ticker::pyfmt;

/// Why a geographic setting was refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeoError {
    /// A grid spacing of zero degrees, or one that is not finite.
    GridSpacing(f64),
    /// A centre given to axes drawn through another projection than
    /// Lambert's, the one that has a centre.
    NotLambert,
}

impl fmt::Display for GeoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GridSpacing(degrees) => write!(
                f,
                "grid spacing must be a finite, non-zero number of degrees, not {degrees}"
            ),
            Self::NotLambert => f.write_str("only a Lambert projection has a centre"),
        }
    }
}

impl std::error::Error for GeoError {}

/// A handle to geographic axes: longitude (radians, `-π..π`) along x and
/// latitude (radians, `-π/2..π/2`) along y, drawn through a map
/// projection.
///
/// It dereferences to [`Axes`], so everything that plots on axes plots
/// here, in `(longitude, latitude)` coordinates. The view limits are
/// fixed: changing them is refused like matplotlib refuses it.
///
/// ```
/// use pltrs::figure::Figure;
/// use pltrs::geo::GeoProjection;
/// let fig = Figure::new();
/// let ax = fig.geo_subplot(GeoProjection::Mollweide);
/// ax.plot([-1.0, 0.0, 1.0], [0.2, 0.5, -0.3])?;
/// ax.set_longitude_grid(45.0)?.set_latitude_grid(30.0)?;
/// ax.grid(true);
/// assert_eq!(ax.format_coord(0.5, -0.25), "14.323945°S, 28.647890°E");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeoAxes {
    axes: Axes,
}

impl Deref for GeoAxes {
    type Target = Axes;

    fn deref(&self) -> &Axes {
        &self.axes
    }
}

impl GeoAxes {
    /// `axes` as geographic axes, if it is one.
    pub(crate) fn new(axes: Axes) -> Option<Self> {
        axes.with_projection(|_: &GeoState| ())
            .map(|()| Self { axes })
    }

    /// Turns freshly created rectilinear `axes` into geographic axes
    /// drawn through `projection` (matplotlib's `GeoAxes.__init__` and
    /// `clear`).
    pub(crate) fn convert(axes: Axes, projection: GeoProjection) -> Self {
        install(&axes, Box::new(GeoState::new(projection)))
            .expect("geographic axes set up whole-globe limits and valid aspects");
        Self { axes }
    }

    /// The underlying axes.
    pub fn axes(&self) -> &Axes {
        &self.axes
    }

    fn settings<R>(&self, read: impl FnOnce(&GeoState) -> R) -> R {
        self.axes
            .with_projection(read)
            .expect("a GeoAxes handle refers to geographic axes")
    }

    /// The map projection.
    pub fn geo_projection(&self) -> GeoProjection {
        self.settings(|settings| settings.projection)
    }

    /// Centres the Lambert projection the axes are drawn through at
    /// `longitude` and `latitude`, in radians (matplotlib's
    /// `LambertAxes(center_longitude=..., center_latitude=...)`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::geo::GeoProjection;
    /// let ax = Figure::new().geo_subplot(GeoProjection::LAMBERT);
    /// ax.set_lambert_center(0.5, -0.25)?;
    /// assert_eq!(
    ///     ax.geo_projection(),
    ///     GeoProjection::Lambert { center_longitude: 0.5, center_latitude: -0.25 }
    /// );
    /// # Ok::<(), pltrs::geo::GeoError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the axes are drawn through another projection.
    pub fn set_lambert_center(&self, longitude: f64, latitude: f64) -> Result<&Self, GeoError> {
        self.axes
            .with_projection_mut(|settings: &mut GeoState| match &mut settings.projection {
                GeoProjection::Lambert {
                    center_longitude,
                    center_latitude,
                } => {
                    *center_longitude = longitude;
                    *center_latitude = latitude;
                    Ok(())
                }
                _ => Err(GeoError::NotLambert),
            })
            .expect("a GeoAxes handle refers to geographic axes")?;
        Ok(self)
    }

    /// Puts a longitude gridline every `degrees`, skipping the edges at
    /// ±180°, labelled to multiples of `degrees`.
    ///
    /// # Errors
    ///
    /// When `degrees` is zero or not finite.
    pub fn set_longitude_grid(&self, degrees: f64) -> Result<&Self, GeoError> {
        check_spacing(degrees)?;
        install_grid(&self.xaxis(), degrees, LONGITUDE_EDGE);
        Ok(self)
    }

    /// Puts a latitude gridline every `degrees`, skipping the poles,
    /// labelled to multiples of `degrees`.
    ///
    /// # Errors
    ///
    /// When `degrees` is zero or not finite.
    pub fn set_latitude_grid(&self, degrees: f64) -> Result<&Self, GeoError> {
        check_spacing(degrees)?;
        install_grid(&self.yaxis(), degrees, LATITUDE_EDGE);
        Ok(self)
    }

    /// Stops the longitude gridlines at `degrees` north and south.
    pub fn set_longitude_grid_ends(&self, degrees: f64) -> &Self {
        self.axes.with_projection_mut(|settings: &mut GeoState| {
            settings.longitude_cap = degrees.to_radians();
        });
        self
    }

    /// The latitude, in degrees, where longitude gridlines stop.
    pub fn longitude_grid_ends(&self) -> f64 {
        self.settings(|settings| settings.longitude_cap.to_degrees())
    }

    /// The spine drawing the outline of the map (matplotlib's
    /// `spines['geo']`).
    pub fn spine(&self) -> Spine {
        self.axes.spines().get(OUTLINE_SPINE).clone()
    }

    /// The position `(longitude, latitude)` in radians as degrees north
    /// or south and east or west, as the status bar shows it (matplotlib's
    /// `GeoAxes.format_coord`).
    pub fn format_coord(&self, longitude: f64, latitude: f64) -> String {
        format_lonlat(longitude, latitude)
    }
}

/// Puts gridlines every `degrees` strictly between `-limit` and `limit`
/// on `axis`, labelled to multiples of `degrees`.
pub(super) fn install_grid(axis: &Axis, degrees: f64, limit: f64) {
    axis.set_major_locator(grid_locator(degrees, limit))
        .set_major_formatter(GeoThetaFormatter::new(degrees));
}

/// `'%f°N, %f°E'` with the hemispheres of `latitude` and `longitude`.
pub(super) fn format_lonlat(longitude: f64, latitude: f64) -> String {
    let (lon, lat) = (longitude.to_degrees(), latitude.to_degrees());
    let ns = if lat >= 0.0 { 'N' } else { 'S' };
    let ew = if lon >= 0.0 { 'E' } else { 'W' };
    format!(
        "{}\u{b0}{ns}, {}\u{b0}{ew}",
        pyfmt::fixed(lat.abs(), 6),
        pyfmt::fixed(lon.abs(), 6)
    )
}

/// Refuses grid spacings of no gridline at all: zero and those that are
/// not finite (a negative spacing puts none, as matplotlib's `arange`
/// counting down from below the top puts none).
fn check_spacing(degrees: f64) -> Result<(), GeoError> {
    if degrees.is_finite() && degrees != 0.0 {
        Ok(())
    } else {
        Err(GeoError::GridSpacing(degrees))
    }
}

impl Figure {
    /// Adds geographic axes filling the default subplot area
    /// (`fig.add_subplot(projection="mollweide")` and friends).
    pub fn geo_subplot(&self, projection: GeoProjection) -> GeoAxes {
        GeoAxes::convert(self.subplot(), projection)
    }

    /// Adds geographic axes in cell `index` (1-based, row-major) of an
    /// `nrows × ncols` grid.
    ///
    /// # Errors
    ///
    /// When the grid is empty or `index` is outside it.
    pub fn add_geo_subplot(
        &self,
        projection: GeoProjection,
        nrows: usize,
        ncols: usize,
        index: usize,
    ) -> Result<GeoAxes, GridSpecError> {
        self.add_subplot(nrows, ncols, index)
            .map(|axes| GeoAxes::convert(axes, projection))
    }

    /// Adds geographic axes covering a grid cell or block.
    ///
    /// # Errors
    ///
    /// When the subplot parameters leave no room for the cell.
    pub fn add_geo_subplot_spec(
        &self,
        projection: GeoProjection,
        spec: &SubplotSpec,
    ) -> Result<GeoAxes, GridSpecError> {
        self.add_subplot_spec(spec)
            .map(|axes| GeoAxes::convert(axes, projection))
    }

    /// Adds geographic axes occupying `[left, bottom, width, height]` in
    /// figure fractions.
    ///
    /// # Errors
    ///
    /// As [`Figure::add_axes`].
    pub fn add_geo_axes(
        &self,
        projection: GeoProjection,
        rect: [f64; 4],
    ) -> Result<GeoAxes, AxesRectError> {
        self.add_axes(rect)
            .map(|axes| GeoAxes::convert(axes, projection))
    }
}

impl Axes {
    /// Adds geographic inset axes at `[left, bottom, width, height]` in
    /// this axes' fractions (matplotlib's `inset_axes(...,
    /// projection="hammer")` and friends; see [`Axes::inset_axes`]).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::geo::GeoProjection;
    /// let host = Figure::new().subplot();
    /// let inset = host.geo_inset_axes([0.2, 0.2, 0.3, 0.3], GeoProjection::Hammer);
    /// assert_eq!(inset.projection().name(), "hammer");
    /// assert_eq!(host.child_axes().len(), 1);
    /// ```
    #[must_use]
    pub fn geo_inset_axes(&self, bounds: [f64; 4], projection: GeoProjection) -> GeoAxes {
        GeoAxes::convert(self.inset_axes(bounds), projection)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use pltrs_core::Point;

    use super::*;
    use crate::axes::{AxisName, ProjectionKind};
    use crate::scale::ScaleName;
    use crate::testing::RecordingRenderer;

    #[test]
    fn geo_insets_are_child_axes() {
        // matplotlib's test_inset_projection.
        let fig = Figure::new();
        let host = fig.subplot();
        let inset = host.geo_inset_axes([0.2, 0.2, 0.3, 0.3], GeoProjection::Hammer);
        assert_eq!(inset.projection(), ProjectionKind::Hammer);
        assert_eq!(host.child_axes(), std::slice::from_ref(&inset.axes));
        assert_eq!(fig.axes(), [host]);
    }

    #[test]
    fn new_geo_axes_follow_matplotlib_defaults() {
        let fig = Figure::new();
        let ax = fig.geo_subplot(GeoProjection::Hammer);
        assert_eq!(ax.projection(), ProjectionKind::Hammer);
        assert_eq!(ax.projection().name(), "hammer");
        assert_eq!(ax.xlim(), (-PI, PI));
        assert_eq!(ax.ylim(), (-FRAC_PI_2, FRAC_PI_2));
        assert!((ax.longitude_grid_ends() - 75.0).abs() < 1e-12);
        assert!(fig.subplot().as_geo().is_none());
        assert_eq!(fig.axes()[0].as_geo(), Some(ax.clone()));
        let labels = |name| -> Vec<String> {
            ax.ticks(name)
                .unwrap()
                .major
                .into_iter()
                .map(|t| t.label)
                .collect()
        };
        assert_eq!(labels(AxisName::X)[..3], ["-150°", "-120°", "-90°"]);
        assert_eq!(labels(AxisName::Y).len(), 11);
        assert!(!ax.has_grid(AxisName::X, true));
    }

    #[test]
    fn limits_scales_and_navigation_stay_fixed() {
        let fig = Figure::new();
        let ax = fig.geo_subplot(GeoProjection::Mollweide);
        ax.plot(vec![-10.0, 10.0], vec![-5.0, 5.0]).unwrap();
        assert_eq!(ax.xlim(), (-PI, PI));
        ax.set_xlim(-PI, PI).expect("finite limits");
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        assert!(fig.draw(&mut renderer).is_ok());
        ax.set_xlim(0.0, 1.0).expect("finite limits");
        ax.set_yscale("log").unwrap();
        assert_eq!((ax.xlim(), ax.yscale()), ((-PI, PI), ScaleName::Linear));
        assert_eq!(fig.draw(&mut renderer).unwrap_err().0.len(), 2);
        assert!(!ax.can_zoom() && !ax.can_pan());
    }

    #[test]
    fn grid_settings() {
        let ax = Figure::new().geo_subplot(GeoProjection::Aitoff);
        ax.set_longitude_grid(45.0).unwrap();
        ax.set_latitude_grid(30.0).unwrap();
        ax.set_longitude_grid_ends(60.0);
        assert!((ax.longitude_grid_ends() - 60.0).abs() < 1e-12);
        let x = ax.ticks(AxisName::X).unwrap();
        assert_eq!(x.major.len(), 7);
        assert_eq!(x.major[0].label, "-135°");
        assert_eq!(ax.ticks(AxisName::Y).unwrap().major.len(), 5);
        assert_eq!(
            ax.set_longitude_grid(0.0).err(),
            Some(GeoError::GridSpacing(0.0))
        );
        assert!(ax.set_latitude_grid(f64::NAN).is_err());
        ax.set_latitude_grid(-15.0).unwrap();
        assert!(
            ax.ticks(AxisName::Y).unwrap().major.is_empty(),
            "counting down puts none"
        );
    }

    #[test]
    fn only_lambert_axes_take_a_centre() {
        let ax = Figure::new().geo_subplot(GeoProjection::LAMBERT);
        ax.set_lambert_center(1.0, 0.5).unwrap();
        let projected = ax.geo_projection().forward(Point::new(1.0, 0.5));
        assert!(projected.x.abs() < 1e-12 && projected.y.abs() < 1e-12);
        let aitoff = Figure::new().geo_subplot(GeoProjection::Aitoff);
        assert_eq!(
            aitoff.set_lambert_center(1.0, 0.5).err(),
            Some(GeoError::NotLambert)
        );
        assert_eq!(aitoff.geo_projection(), GeoProjection::Aitoff);
    }

    #[test]
    fn lambert_hides_latitude_labels_and_keeps_its_centre() {
        let projection = GeoProjection::Lambert {
            center_longitude: 0.5,
            center_latitude: -0.25,
        };
        let ax = Figure::new().geo_subplot(projection);
        assert_eq!(ax.geo_projection(), projection);
        assert_eq!(ax.projection(), ProjectionKind::Lambert);
        let y = ax.ticks(AxisName::Y).unwrap();
        assert!(y.major.iter().all(|t| t.label.is_empty()));
    }

    #[test]
    fn coordinates_name_their_hemispheres() {
        assert_eq!(format_lonlat(-PI / 2.0, 0.0), "0.000000°N, 90.000000°W");
        assert_eq!(format_lonlat(f64::NAN, f64::NAN), "nan°S, nan°W");
    }
}
