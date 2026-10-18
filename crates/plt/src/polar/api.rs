//! The [`PolarAxes`] handle: creating polar axes and their settings
//! (matplotlib's `PolarAxes` methods).

use std::f64::consts::TAU;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

use plt_core::Bbox;

use super::{Compass, PolarState, ThetaDirection};
use crate::artist::ArtistHandle;
use crate::axes::axis::AxisRole;
use crate::axes::projection::Projection;
use crate::axes::{Adjustable, Axes, AxesState, AxisName, PlotError, ProjectionKind, Sides};
use crate::figure::Figure;
use crate::gridspec::{GridSpecError, SubplotSpec};
use crate::rc::Aspect;
use crate::scale::{LinearScale, ScaleError};
use crate::transforms::Anchor;

/// Why a polar setting was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolarError {
    /// θ limits more than a full circle apart.
    ThetaRange,
}

impl fmt::Display for PolarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ThetaRange => f.write_str("The angle range must be less than a full circle"),
        }
    }
}

impl std::error::Error for PolarError {}

/// A handle to polar axes: θ (radians) along x, r along y.
///
/// It dereferences to [`Axes`], so everything that plots on axes plots
/// here, in `(θ, r)` coordinates.
///
/// ```
/// use plt::figure::Figure;
/// use plt::polar::{Compass, ThetaDirection};
/// let fig = Figure::new();
/// let ax = fig.polar_subplot();
/// ax.plot([0.0, 1.0, 2.0], [0.5, 1.0, 1.5])?;
/// ax.set_theta_zero_location(Compass::N, 0.0)
///     .set_theta_direction(ThetaDirection::Clockwise);
/// assert_eq!(ax.thetamax(), 360.0);
/// # Ok::<(), plt::axes::PlotError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolarAxes {
    axes: Axes,
}

impl Deref for PolarAxes {
    type Target = Axes;

    fn deref(&self) -> &Axes {
        &self.axes
    }
}

impl PolarAxes {
    /// `axes` as polar axes, if it is one.
    pub(crate) fn new(axes: Axes) -> Option<Self> {
        (axes.projection() == ProjectionKind::Polar).then_some(Self { axes })
    }

    /// Turns freshly created rectilinear `axes` into polar axes
    /// (matplotlib's `PolarAxes.__init__` and `clear`).
    pub(crate) fn convert(axes: Axes) -> Self {
        axes.raw().with_mut::<AxesState, _>(|state| {
            state.projection = Projection::Polar(PolarState::default());
            for (axis, role) in [
                (&mut state.xaxis, AxisRole::Theta),
                (&mut state.yaxis, AxisRole::Radius),
            ] {
                axis.role = role;
                axis.set_scale(Rc::new(LinearScale));
                axis.restyle_ticks(|style| style.ticks = Sides::default());
            }
        });
        let spines: Vec<_> = axes.spines().iter().cloned().collect();
        for spine in spines {
            // matplotlib's arc spines ignore `axes.spines.*`, and the
            // drawing decides whether the other two show.
            spine.raw().with_base_mut(|base| base.visible = true);
        }
        axes.set_xlim(0.0, TAU);
        axes.grid(crate::rc::params().polaraxes_grid);
        axes.set_aspect(Aspect::Equal)
            .expect("an equal aspect is always valid");
        axes.set_adjustable(Adjustable::Box).set_anchor(Anchor::C);
        Self { axes }
    }

    /// The underlying axes.
    pub fn axes(&self) -> &Axes {
        &self.axes
    }

    fn settings<R>(&self, read: impl FnOnce(&PolarState) -> R) -> R {
        self.axes.raw().with::<AxesState, _>(|state| {
            read(
                state
                    .polar()
                    .expect("a PolarAxes handle refers to polar axes"),
            )
        })
    }

    fn update(&self, change: impl FnOnce(&mut PolarState)) -> &Self {
        self.axes
            .raw()
            .with_mut::<AxesState, _>(|state| match &mut state.projection {
                Projection::Polar(settings) => change(settings),
                Projection::Rectilinear | Projection::Geo(_) => {
                    unreachable!("a PolarAxes handle refers to polar axes")
                }
            });
        self
    }

    /// The view limits after pending autoscaling (matplotlib's `viewLim`).
    fn view(&self) -> Bbox {
        self.axes.unstale_view();
        self.axes.raw().with::<AxesState, _>(|state| state.view_lim)
    }

    /// Replaces one view interval directly, without turning autoscaling
    /// off (matplotlib's assignments to `viewLim.x0` and friends).
    fn set_view(&self, name: AxisName, interval: (f64, f64)) -> &Self {
        self.axes.unstale_view();
        self.axes
            .raw()
            .with_mut::<AxesState, _>(|state| state.set_view_interval(name, interval));
        self
    }

    /// Sets where θ = 0 points, in radians anticlockwise from east.
    pub fn set_theta_offset(&self, offset: f64) -> &Self {
        self.update(|settings| settings.theta_offset = offset)
    }

    /// Where θ = 0 points, in radians anticlockwise from east.
    pub fn theta_offset(&self) -> f64 {
        self.settings(|settings| settings.theta_offset)
    }

    /// Puts θ = 0 at a compass point, turned anticlockwise by
    /// `offset_degrees` whatever the θ direction.
    pub fn set_theta_zero_location(&self, location: Compass, offset_degrees: f64) -> &Self {
        self.set_theta_offset(location.angle() + offset_degrees.to_radians())
    }

    /// Sets the way θ increases.
    pub fn set_theta_direction(&self, direction: ThetaDirection) -> &Self {
        self.update(|settings| settings.direction = direction)
    }

    /// The way θ increases.
    pub fn theta_direction(&self) -> ThetaDirection {
        self.settings(|settings| settings.direction)
    }

    /// Sets the lower θ limit in degrees.
    pub fn set_thetamin(&self, degrees: f64) -> &Self {
        let (_, high) = self.view().interval_x();
        self.set_view(AxisName::X, (degrees.to_radians(), high))
    }

    /// Sets the upper θ limit in degrees.
    pub fn set_thetamax(&self, degrees: f64) -> &Self {
        let (low, _) = self.view().interval_x();
        self.set_view(AxisName::X, (low, degrees.to_radians()))
    }

    /// The lower θ limit in degrees.
    pub fn thetamin(&self) -> f64 {
        self.view().xmin().to_degrees()
    }

    /// The upper θ limit in degrees.
    pub fn thetamax(&self) -> f64 {
        self.view().xmax().to_degrees()
    }

    /// Sets both θ limits in radians, like
    /// [`set_xlim`](Axes::set_xlim) (matplotlib's `set_thetalim`).
    ///
    /// # Errors
    ///
    /// When the limits are more than a full circle apart; the limits are
    /// left unchanged.
    pub fn set_thetalim(&self, min: f64, max: f64) -> Result<&Self, PolarError> {
        let original = self.axes.xlim();
        self.axes.set_xlim(min, max);
        let (low, high) = self.axes.xlim();
        if (high - low).abs() > TAU {
            self.axes.set_xlim(original.0, original.1);
            return Err(PolarError::ThetaRange);
        }
        Ok(self)
    }

    /// Sets the inner radial limit.
    pub fn set_rmin(&self, rmin: f64) -> &Self {
        let (_, high) = self.view().interval_y();
        self.set_view(AxisName::Y, (rmin, high))
    }

    /// Sets the outer radial limit.
    pub fn set_rmax(&self, rmax: f64) -> &Self {
        let (low, _) = self.view().interval_y();
        self.set_view(AxisName::Y, (low, rmax))
    }

    /// The inner radial limit.
    pub fn rmin(&self) -> f64 {
        self.view().ymin()
    }

    /// The outer radial limit.
    pub fn rmax(&self) -> f64 {
        self.view().ymax()
    }

    /// Sets both radial limits and turns radial autoscaling off, like
    /// [`set_ylim`](Axes::set_ylim).
    pub fn set_rlim(&self, rmin: f64, rmax: f64) -> &Self {
        self.axes.set_ylim(rmin, rmax);
        self
    }

    /// Sets the radius drawn at the centre (`None` follows the inner
    /// radial limit); a smaller origin leaves a hole in the middle.
    pub fn set_rorigin(&self, rorigin: Option<f64>) -> &Self {
        self.update(|settings| settings.rorigin = rorigin)
    }

    /// The radius drawn at the centre.
    pub fn rorigin(&self) -> f64 {
        let view = self.view();
        self.settings(|settings| settings.rorigin.unwrap_or(view.y0()))
    }

    /// Sets the angle of the radial tick labels in degrees.
    pub fn set_rlabel_position(&self, degrees: f64) -> &Self {
        self.update(|settings| settings.rlabel_position = degrees.to_radians())
    }

    /// The angle of the radial tick labels in degrees.
    pub fn rlabel_position(&self) -> f64 {
        self.settings(|settings| settings.rlabel_position.to_degrees())
    }

    /// Sets the radial scale by name, like [`set_yscale`](Axes::set_yscale).
    ///
    /// # Errors
    ///
    /// When no scale is registered under `name`.
    pub fn set_rscale(&self, name: &str) -> Result<&Self, ScaleError> {
        self.axes.set_yscale(name)?;
        Ok(self)
    }

    /// Puts the radial ticks exactly at `radii`, like
    /// [`set_yticks`](Axes::set_yticks).
    pub fn set_rticks(&self, radii: &[f64]) -> &Self {
        self.axes.set_yticks(radii.to_vec());
        self
    }

    /// Puts θ gridlines at `angles` degrees, keeping the degree labels.
    pub fn set_thetagrids(&self, angles: &[f64]) -> &Self {
        let radians: Vec<f64> = angles.iter().map(|a| a.to_radians()).collect();
        self.axes.set_xticks(radians);
        self
    }

    /// Puts θ gridlines at `angles` degrees labelled with `labels`.
    ///
    /// # Errors
    ///
    /// When the number of labels differs from the number of angles.
    pub fn set_thetagrids_labeled<S: Into<String>>(
        &self,
        angles: &[f64],
        labels: impl IntoIterator<Item = S>,
    ) -> Result<&Self, PlotError> {
        self.set_thetagrids(angles);
        self.axes.set_xticklabels(labels)?;
        Ok(self)
    }

    /// Puts radial gridlines at `radii`, with their labels at `angle`
    /// degrees (the current label angle when `None`).
    pub fn set_rgrids(&self, radii: &[f64], angle: Option<f64>) -> &Self {
        self.set_rticks(radii);
        self.set_rlabel_position(angle.unwrap_or_else(|| self.rlabel_position()))
    }

    /// Puts radial gridlines at `radii` labelled with `labels`, at `angle`
    /// degrees (the current label angle when `None`).
    ///
    /// # Errors
    ///
    /// When the number of labels differs from the number of radii.
    pub fn set_rgrids_labeled<S: Into<String>>(
        &self,
        radii: &[f64],
        labels: impl IntoIterator<Item = S>,
        angle: Option<f64>,
    ) -> Result<&Self, PlotError> {
        self.set_rgrids(radii, angle);
        self.axes.set_yticklabels(labels)?;
        Ok(self)
    }
}

impl Figure {
    /// Adds polar axes filling the default subplot area
    /// (`fig.add_subplot(projection="polar")`).
    pub fn polar_subplot(&self) -> PolarAxes {
        PolarAxes::convert(self.subplot())
    }

    /// Adds polar axes in cell `index` (1-based, row-major) of an
    /// `nrows × ncols` grid.
    ///
    /// # Errors
    ///
    /// When the grid is empty or `index` is outside it.
    pub fn add_polar_subplot(
        &self,
        nrows: usize,
        ncols: usize,
        index: usize,
    ) -> Result<PolarAxes, GridSpecError> {
        self.add_subplot(nrows, ncols, index)
            .map(PolarAxes::convert)
    }

    /// Adds polar axes covering a grid cell or block.
    ///
    /// # Errors
    ///
    /// When the subplot parameters leave no room for the cell.
    pub fn add_polar_subplot_spec(&self, spec: &SubplotSpec) -> Result<PolarAxes, GridSpecError> {
        self.add_subplot_spec(spec).map(PolarAxes::convert)
    }

    /// Adds polar axes occupying `[left, bottom, width, height]` in figure
    /// fractions.
    pub fn add_polar_axes(&self, rect: [f64; 4]) -> PolarAxes {
        PolarAxes::convert(self.add_axes(rect))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_polar_axes_follow_matplotlib_defaults() {
        let fig = Figure::new();
        let ax = fig.polar_subplot();
        assert_eq!(ax.projection(), ProjectionKind::Polar);
        assert_eq!(ax.xlim(), (0.0, TAU));
        assert_eq!(ax.ylim(), (0.0, 1.0));
        assert_eq!(ax.rlabel_position(), 22.5);
        assert!(fig.subplot().as_polar().is_none());
        assert_eq!(fig.axes()[0].as_polar(), Some(ax));
    }

    #[test]
    fn limits_and_origin() {
        let ax = Figure::new().polar_subplot();
        ax.plot(vec![0.0, 1.0], vec![0.0, 2.0]).unwrap();
        assert_eq!(ax.ylim(), (0.0, 2.1));
        ax.set_thetamin(45.0).set_thetamax(135.0);
        assert!((ax.thetamin() - 45.0).abs() < 1e-12);
        assert!((ax.thetamax() - 135.0).abs() < 1e-12);
        ax.set_rmin(0.5).set_rorigin(Some(-1.0));
        assert_eq!((ax.rmin(), ax.rorigin()), (0.5, -1.0));
        assert_eq!(ax.set_thetalim(0.0, 7.0), Err(PolarError::ThetaRange));
        assert!((ax.thetamax() - 135.0).abs() < 1e-12);
    }

    #[test]
    fn theta_ticks_are_degrees_and_radial_ticks_skip_the_origin() {
        let fig = Figure::new();
        let ax = fig.polar_subplot();
        ax.plot(vec![0.0, 1.0], vec![0.0, 2.0]).unwrap();
        let theta = ax.ticks(AxisName::X).unwrap();
        let labels: Vec<&str> = theta.major.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(
            labels,
            ["0°", "45°", "90°", "135°", "180°", "225°", "270°", "315°"]
        );
        let radial = ax.ticks(AxisName::Y).unwrap();
        assert!(radial.major.iter().all(|t| t.loc > 0.0));
    }

    #[test]
    fn drawing_decides_the_visibility_of_the_sector_spines() {
        let fig = Figure::new();
        let ax = fig.polar_subplot();
        let spines = ax.spines();
        spines.left.set_visible(true);
        fig.layout();
        assert!(!spines.left.visible(), "a full circle has no start spine");
        ax.set_thetamax(270.0);
        spines.right.set_visible(false);
        fig.layout();
        assert!(spines.left.visible() && spines.right.visible());
        assert!(
            !spines.bottom.visible(),
            "no hole without an origin below rmin"
        );
        ax.set_rorigin(Some(-1.0));
        fig.layout();
        assert!(spines.bottom.visible());
    }

    #[test]
    fn error_bar_caps_turn_with_theta() {
        use crate::axes::ErrorbarOptions;
        use crate::lines::LineArtist;
        use crate::markers::{Marker, MarkerSymbol};

        let ax = Figure::new().polar_subplot();
        let options = ErrorbarOptions {
            xerr: Some(0.1.into()),
            yerr: Some(0.2.into()),
            capsize: Some(3.0),
            ..ErrorbarOptions::default()
        };
        let bars = ax
            .errorbar([0.0, 1.0, 2.0], [1.0, 1.5, 2.0], options)
            .unwrap();
        // Both ends of both directions, one line per point.
        assert_eq!(bars.caplines.len(), 12);
        let turns: Vec<(Marker, f64)> = bars
            .caplines
            .iter()
            .map(|cap| {
                cap.raw().with::<LineArtist, _>(|line| {
                    let rotation = line.marker_rotation.expect("a polar cap");
                    (line.marker.clone(), rotation.extra)
                })
            })
            .collect();
        assert!(
            turns
                .iter()
                .all(|(m, _)| *m == Marker::Symbol(MarkerSymbol::HLine))
        );
        assert_eq!(turns[0].1, 0.0, "angular bars have radial caps");
        assert_eq!(turns[11].1, std::f64::consts::FRAC_PI_2);
    }
}
