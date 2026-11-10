//! The [`PolarAxes`] handle and its settings (matplotlib's `PolarAxes`
//! methods).

use std::f64::consts::TAU;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

use pltrs_core::Bbox;

use super::{Compass, PolarFrame, PolarState, ThetaDirection};
use crate::artist::ArtistHandle;
use crate::axes::axis::AxisRole;
use crate::axes::projection::ProjectionState;
use crate::axes::{Adjustable, Axes, AxesState, AxisName, PlotError, ProjectionKind, Sides};
use crate::numeric::Number;
use crate::rc::Aspect;
use crate::scale::{LinearScale, ScaleError, ScaleName};
use crate::transforms::Anchor;
use crate::units::AxisValues;

/// Why a polar setting was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolarError {
    /// θ limits more than a full circle apart.
    ThetaRange,
    /// A θ limit that is NaN or infinite.
    NonFiniteLimit,
    /// A polygon frame with fewer than three vertices.
    PolygonVertices(usize),
    /// A frame path without vertices.
    EmptyFrame,
}

impl fmt::Display for PolarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ThetaRange => f.write_str("The angle range must be less than a full circle"),
            Self::NonFiniteLimit => f.write_str("θ limits cannot be NaN or Inf"),
            Self::PolygonVertices(n) => {
                write!(f, "a polygon frame needs at least 3 vertices, not {n}")
            }
            Self::EmptyFrame => f.write_str("a frame path needs vertices"),
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
/// use pltrs::figure::Figure;
/// use pltrs::polar::{Compass, ThetaDirection};
/// let fig = Figure::new();
/// let ax = fig.polar_subplot();
/// ax.plot([0.0, 1.0, 2.0], [0.5, 1.0, 1.5])?;
/// ax.set_theta_zero_location(Compass::N, 0.0)
///     .set_theta_direction(ThetaDirection::Clockwise);
/// assert_eq!(ax.thetamax(), 360.0);
/// # Ok::<(), pltrs::axes::PlotError>(())
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
            state.projection = ProjectionState::Polar(PolarState::default());
            for (axis, role) in [
                (&mut state.xaxis, AxisRole::Theta),
                (&mut state.yaxis, AxisRole::Radius),
            ] {
                axis.role = role;
                axis.set_scale(Rc::new(LinearScale));
                axis.restyle_ticks(|style| style.ticks = Sides::default());
                // matplotlib's `PolarAxes.__init__` clears the axes again
                // once `Axes.__init__` connected its handler of units
                // changes, which disconnects it.
                axis.follows_units = false;
            }
        });
        let spines: Vec<_> = axes.spines().iter().cloned().collect();
        for spine in spines {
            // matplotlib's arc spines ignore `axes.spines.*`, and the
            // drawing decides whether the other two show.
            spine.raw().with_base_mut(|base| base.visible = true);
        }
        axes.set_xlim(0.0, TAU)
            .expect("a full circle has finite limits");
        axes.grid(crate::rc::params().polaraxes_grid);
        axes.set_aspect(Aspect::Equal)
            .expect("an equal aspect is always valid");
        axes.store_adjustable(Adjustable::Box).set_anchor(Anchor::C);
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
        self.axes.raw().with_mut::<AxesState, _>(|state| {
            change(
                state
                    .projection
                    .polar_mut()
                    .expect("a PolarAxes handle refers to polar axes"),
            );
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

    /// Sets the outline of the axes: a circle, the polygon of matplotlib's
    /// radar chart example (its `RadarAxes` with `frame='polygon'`), which
    /// the background, the clipping, the outer spine and the gridlines
    /// follow, or a closed path in axes fractions the background and the
    /// clipping follow (the patch of a subclass of matplotlib's
    /// `PolarAxes`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::polar::{Compass, PolarFrame};
    /// let fig = Figure::new();
    /// let ax = fig.polar_subplot();
    /// ax.set_frame(PolarFrame::Polygon(5))?
    ///     .set_theta_zero_location(Compass::N, 0.0);
    /// assert_eq!(ax.frame(), PolarFrame::Polygon(5));
    /// # Ok::<(), pltrs::polar::PolarError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`PolarError::PolygonVertices`] for a polygon of fewer than three
    /// vertices, [`PolarError::EmptyFrame`] for a path without vertices.
    pub fn set_frame(&self, frame: PolarFrame) -> Result<&Self, PolarError> {
        match &frame {
            PolarFrame::Polygon(n) if *n < 3 => return Err(PolarError::PolygonVertices(*n)),
            PolarFrame::Path(path) if path.is_empty() => return Err(PolarError::EmptyFrame),
            _ => {}
        }
        Ok(self.update(|settings| settings.frame = frame))
    }

    /// The outline of the axes.
    pub fn frame(&self) -> PolarFrame {
        self.settings(|settings| settings.frame.clone())
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
    /// When a limit is NaN or infinite, or the limits are more than a full
    /// circle apart; the limits are left unchanged.
    pub fn set_thetalim(&self, min: f64, max: f64) -> Result<&Self, PolarError> {
        let original = self.axes.xlim();
        self.axes
            .set_xlim(min, max)
            .map_err(|_| PolarError::NonFiniteLimit)?;
        let (low, high) = self.axes.xlim();
        if (high - low).abs() > TAU {
            // Only limits that were not finite (`set_thetamin` takes any
            // number) cannot be restored; they stay as matplotlib leaves
            // them.
            let _ = self.axes.set_xlim(original.0, original.1);
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
    /// [`set_ylim`](Axes::set_ylim) (matplotlib's `set_rlim`).
    ///
    /// # Errors
    ///
    /// When a limit is NaN or infinite; then the limits are unchanged.
    pub fn set_rlim(&self, rmin: f64, rmax: f64) -> Result<&Self, PlotError> {
        self.axes.set_ylim(rmin, rmax)?;
        Ok(self)
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

    /// The radius set to be drawn at the centre, `None` when it follows
    /// the inner radial limit (matplotlib's locked `y0` of the view limits
    /// it keeps for the origin).
    pub fn rorigin_set(&self) -> Option<f64> {
        self.settings(|settings| settings.rorigin)
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
    pub fn set_rscale(&self, name: impl Into<ScaleName>) -> Result<&Self, ScaleError> {
        self.axes.set_yscale(name)?;
        Ok(self)
    }

    /// Puts the radial ticks exactly at `radii`, like
    /// [`set_yticks`](Axes::set_yticks), which takes the same values.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().polar_subplot();
    /// ax.set_rticks([0.5, 1.0, 1.5]).set_thetagrids([0, 90, 180, 270]);
    /// assert_eq!(ax.yaxis().ticklocs(false)?, [0.5, 1.0, 1.5]);
    /// assert_eq!(ax.xaxis().ticklabels(false)?, ["0°", "90°", "180°", "270°"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_rticks(&self, radii: impl Into<AxisValues>) -> &Self {
        self.axes.set_yticks(radii);
        self
    }

    /// Puts θ gridlines at `angles` degrees, keeping the degree labels.
    pub fn set_thetagrids(&self, angles: impl IntoIterator<Item: Number>) -> &Self {
        let radians: Vec<f64> = angles
            .into_iter()
            .map(|a| a.to_f64().to_radians())
            .collect();
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
        angles: impl IntoIterator<Item: Number>,
        labels: impl IntoIterator<Item = S>,
    ) -> Result<&Self, PlotError> {
        self.set_thetagrids(angles);
        self.axes.set_xticklabels(labels)?;
        Ok(self)
    }

    /// Puts radial gridlines at `radii`, with their labels at `angle`
    /// degrees (the current label angle when `None`).
    pub fn set_rgrids(&self, radii: impl Into<AxisValues>, angle: Option<f64>) -> &Self {
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
        radii: impl Into<AxisValues>,
        labels: impl IntoIterator<Item = S>,
        angle: Option<f64>,
    ) -> Result<&Self, PlotError> {
        self.set_rgrids(radii, angle);
        self.axes.set_yticklabels(labels)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

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
    fn reference_lines_and_spans_follow_the_polar_grid() {
        // matplotlib's test_axhvlinespan_interpolation: a fraction across
        // the data axis is a fraction of the view range there.
        let fig = Figure::with_size(2.0, 2.0, 72.0);
        let ax = fig
            .add_polar_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_ylim(0.0, 2.0).expect("finite limits");
        let close = |a: pltrs_core::Point, b: (f64, f64)| {
            (a.x - b.0).abs() < 1e-9 && (a.y - b.1).abs() < 1e-9
        };
        // A quarter of the angular range at radius 1: straight up, half
        // way to the edge.
        let yaxis = fig.resolve_transform(&ax.trans_yaxis()).unwrap();
        assert!(close(
            yaxis.transform_point(pltrs_core::Point::new(0.25, 1.0)),
            (72.0, 108.0)
        ));
        // Half the radial range at θ = 0: half way to the right edge.
        let xaxis = fig.resolve_transform(&ax.trans_xaxis()).unwrap();
        assert!(close(
            xaxis.transform_point(pltrs_core::Point::new(0.0, 0.5)),
            (108.0, 72.0)
        ));
        let circle = ax.axhline(1.0, crate::lines::LineProps::default());
        assert_eq!(
            circle.path().unwrap().interpolation_steps(),
            super::super::GRIDLINE_INTERPOLATION_STEPS
        );
    }

    #[test]
    fn radial_tick_labels_follow_fixed_ticks() {
        // matplotlib's test_set_rticks_ticklabels_no_warning: the labels
        // belong to the fixed radii the radial locator wraps.
        let ax = Figure::new().polar_subplot();
        ax.set_rticks([0.0, 1.0, 2.0, 3.0]);
        ax.yaxis()
            .set_ticklabels(["zero", "one", "two", "three"])
            .unwrap();
        assert_eq!(
            ax.yaxis().ticklabels(false).unwrap(),
            ["one", "two", "three"]
        );
        assert!(ax.yaxis().set_ticklabels(["zero", "one"]).is_err());
    }

    #[test]
    fn polar_axes_refuse_to_adjust_their_data_limits() {
        let ax = Figure::new().polar_subplot();
        assert_eq!(
            ax.set_adjustable(Adjustable::Datalim).err(),
            Some(crate::axes::AspectError::FixedDataRatio(
                ProjectionKind::Polar
            ))
        );
        assert_eq!(ax.adjustable(), Adjustable::Box);
        let before = ax.ylim();
        assert!(ax.axis_mode(crate::axes::AxisMode::Equal).is_err());
        assert_eq!(
            (ax.adjustable(), ax.ylim()),
            (Adjustable::Box, before),
            "refused without a change"
        );
        assert!(ax.axis_mode(crate::axes::AxisMode::Scaled).is_ok());
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
        assert_eq!(
            ax.set_thetalim(f64::NAN, 1.0),
            Err(PolarError::NonFiniteLimit)
        );
        assert!((ax.thetamax() - 135.0).abs() < 1e-12);
        assert_eq!(
            ax.set_rlim(0.0, f64::INFINITY).err(),
            Some(PlotError::NonFiniteLimit(AxisName::Y))
        );
        assert_eq!(ax.rmin(), 0.5);
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
    fn locator_params_reach_the_sector_locator() {
        use crate::axes::LocatorParams;

        let ax = Figure::new().polar_subplot();
        ax.set_thetamin(15.0).set_thetamax(50.0);
        let degrees = |ax: &PolarAxes| -> Vec<f64> {
            let ticks = ax.ticks(AxisName::X).unwrap();
            ticks
                .major
                .iter()
                .map(|t| (t.loc.to_degrees() * 1e6).round() / 1e6)
                .collect()
        };
        assert!(degrees(&ax).contains(&22.5), "steps of 4.5°");
        // matplotlib's test_polar_theta_limits sets the base's steps.
        ax.locator_params(LocatorParams {
            axis: Some(AxisName::X),
            steps: Some(vec![1.0, 2.0, 2.5, 5.0, 10.0]),
            ..LocatorParams::default()
        })
        .unwrap();
        assert_eq!(
            degrees(&ax),
            [15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0]
        );
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
