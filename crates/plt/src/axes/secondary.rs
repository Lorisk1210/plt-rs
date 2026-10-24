//! Secondary axes (matplotlib's `secondary_xaxis` / `secondary_yaxis`): an
//! extra axis on a host axes, showing the host's range converted by a pair
//! of functions.

use std::rc::Rc;

use super::{Axes, AxesState, AxisName, PlotError, TickPosition};
use crate::artist::{ArtistHandle, ArtistId};
use crate::figure::{FigureState, insert_axes};
use crate::scale::{FuncScale, FuncScaleLog, FuncTransform, Scale};
use crate::spines::SpineSide;
use crate::ticker::NullLocator;
use crate::transforms::{AxisMap, Bbox};

/// Where a secondary axis sits on its host.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SecondaryLocation {
    /// On one side of the host (`top`/`bottom` for x, `left`/`right` for
    /// y).
    Side(SpineSide),
    /// At this fraction of the host across the axis.
    Fraction(f64),
    /// At this data value of the host across the axis, following the
    /// host's limits (matplotlib's location with `transform=ax.transData`).
    Data(f64),
}

/// What makes an axes a secondary axis of a host (matplotlib's
/// `SecondaryAxis`).
#[derive(Clone, Debug)]
pub(crate) struct Secondary {
    /// The host axes.
    host: ArtistId,
    /// The axis this one mirrors.
    orientation: AxisName,
    /// The host data value across the axis it sits at, if it follows the
    /// host's data rather than a fixed fraction.
    data_location: Option<f64>,
    /// Host units to secondary units and back.
    functions: FuncTransform,
    /// The host scale the current scale was built for.
    host_scale: Option<&'static str>,
}

impl Axes {
    /// Adds a secondary x axis showing this axes' x range converted by
    /// `functions` (host units to secondary units, and back; `None` is the
    /// identity), returned as an axes for labelling and ticks (matplotlib's
    /// `secondary_xaxis`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::SecondaryLocation;
    /// use plt::scale::FuncTransform;
    /// use plt::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// ax.set_xlim(0.0, 180.0)?;
    /// let radians = FuncTransform::new(f64::to_radians, f64::to_degrees);
    /// let sec = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(radians))?;
    /// let (low, high) = sec.xlim();
    /// assert!(low.abs() < 1e-12 && (high - std::f64::consts::PI).abs() < 1e-12);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `location` is a side of the y axis.
    pub fn secondary_xaxis(
        &self,
        location: SecondaryLocation,
        functions: Option<FuncTransform>,
    ) -> Result<Self, PlotError> {
        self.secondary(AxisName::X, location, functions)
    }

    /// Adds a secondary y axis (matplotlib's `secondary_yaxis`); see
    /// [`Axes::secondary_xaxis`].
    ///
    /// # Errors
    ///
    /// When `location` is a side of the x axis.
    pub fn secondary_yaxis(
        &self,
        location: SecondaryLocation,
        functions: Option<FuncTransform>,
    ) -> Result<Self, PlotError> {
        self.secondary(AxisName::Y, location, functions)
    }

    fn secondary(
        &self,
        orientation: AxisName,
        location: SecondaryLocation,
        functions: Option<FuncTransform>,
    ) -> Result<Self, PlotError> {
        let pos = match location {
            SecondaryLocation::Side(side) if side.axis() != orientation => {
                return Err(PlotError::WrongSide {
                    axis: orientation,
                    side,
                });
            }
            SecondaryLocation::Side(side) => f64::from(u8::from(side.is_high())),
            // Placed at the host's data value when synchronised below.
            SecondaryLocation::Fraction(pos) | SecondaryLocation::Data(pos) => pos,
        };
        let figure = self.raw().figure_state();
        let secondary = insert_axes(figure, self.id(), secondary_bounds(orientation, pos), None);
        let other = across(orientation);
        secondary
            .axis_of(other)
            .set_major_locator(NullLocator)
            .set_ticks_position(TickPosition::None)?;
        // The spine, ticks and label face away from the host's centre.
        let side = SpineSide::of(orientation, pos >= 0.5);
        let spines = secondary.spines();
        for spine in spines.iter() {
            spine.set_visible(spine.side() == side);
        }
        secondary
            .axis_of(orientation)
            .set_ticks_position(TickPosition::Side(side))?
            .set_label_position(side)?;
        secondary.raw.with_mut::<AxesState, _>(|axes| {
            axes.secondary = Some(Secondary {
                host: self.id(),
                orientation,
                data_location: match location {
                    SecondaryLocation::Data(value) => Some(value),
                    SecondaryLocation::Side(_) | SecondaryLocation::Fraction(_) => None,
                },
                functions: functions.unwrap_or_else(FuncTransform::identity),
                host_scale: None,
            });
        });
        figure.borrow_mut().sync_secondary(secondary.id());
        Ok(secondary)
    }
}

impl FigureState {
    /// Brings a secondary axis up to date with its host: its scale follows
    /// the host's (a function scale, logarithmic over a log host) and its
    /// limits are the host's converted (matplotlib's `_set_scale` and
    /// `_set_lims`). Does nothing for other axes.
    pub(crate) fn sync_secondary(&mut self, axes: ArtistId) {
        let Some(link) = self
            .store
            .get_as::<AxesState>(axes)
            .and_then(|state| state.secondary.clone())
        else {
            return;
        };
        self.unstale_axes(link.host);
        let Some(host) = self.store.get_as::<AxesState>(link.host) else {
            return;
        };
        let host_scale = host.axis(link.orientation).scale().name();
        let (low, high) = host.view_interval(link.orientation);
        let placed = link.data_location.map(|value| {
            // Where the value sits across the host, in its scaled view.
            let other = across(link.orientation);
            let (start, end) = host.view_interval(other);
            let scaled = host.axis(other).scale().forward(&[value, start, end]);
            secondary_bounds(
                link.orientation,
                (scaled[0] - scaled[1]) / (scaled[2] - scaled[1]),
            )
        });
        if let Some(bounds) = placed
            && let Some(state) = self.store.get_as_mut::<AxesState>(axes)
        {
            state.position = bounds;
        }
        if link.host_scale != Some(host_scale) {
            let reversed = link.functions.reversed();
            let scale: Rc<dyn Scale> = if host_scale == "log" {
                Rc::new(FuncScaleLog::new(reversed, 10.0).expect("10 is a valid log base"))
            } else {
                Rc::new(FuncScale::from_transform(reversed))
            };
            self.set_shared_scale(axes, link.orientation, &scale);
            if let Some(state) = self.store.get_as_mut::<AxesState>(axes)
                && let Some(secondary) = &mut state.secondary
            {
                secondary.host_scale = Some(host_scale);
            }
        }
        let (mut v0, mut v1) = (link.functions.forward(low), link.functions.forward(high));
        if (v0 < v1) != (low < high) {
            (v0, v1) = (v1, v0);
        }
        self.set_axis_limits(axes, link.orientation, (v0, v1), Some(false), true);
    }
}

/// The axis across `orientation`.
const fn across(orientation: AxisName) -> AxisName {
    match orientation {
        AxisName::X => AxisName::Y,
        AxisName::Y => AxisName::X,
    }
}

/// The box of a secondary axis at fraction `pos` of its host across the
/// axis: infinitely thin, spanning the host along it.
fn secondary_bounds(orientation: AxisName, pos: f64) -> Bbox {
    match orientation {
        AxisName::X => Bbox::from_bounds(0.0, pos, 1.0, 1e-10),
        AxisName::Y => Bbox::from_bounds(pos, 0.0, 1e-10, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn sides_must_belong_to_the_axis() {
        let ax = Figure::new().subplot();
        let wrong = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Left), None);
        assert!(matches!(wrong, Err(PlotError::WrongSide { .. })));
    }

    #[test]
    fn follows_the_host_limits_and_scale() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let double = FuncTransform::new(|x| 2.0 * x, |x| x / 2.0);
        let sec = ax
            .secondary_yaxis(SecondaryLocation::Fraction(0.3), Some(double))
            .unwrap();
        ax.set_ylim(1.0, 100.0)
            .expect("finite limits")
            .set_yscale("log")
            .unwrap();
        fig.layout();
        assert_eq!(sec.ylim(), (2.0, 200.0));
        assert_eq!(sec.yscale(), "functionlog");
        assert_eq!(fig.axes().len(), 1, "secondary axes are not figure axes");
        let host = ax.position();
        let placed = sec.raw.with::<AxesState, _>(|a| a.position);
        assert_eq!(placed.x0(), 0.3);
        assert!(host.width() > 0.0);
    }

    #[test]
    fn data_locations_follow_the_host_view() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_ylim(0.0, 10.0).expect("finite limits");
        let sec = ax
            .secondary_xaxis(SecondaryLocation::Data(2.5), None)
            .unwrap();
        let placed = || sec.raw.with::<AxesState, _>(|a| a.position.y0());
        fig.layout();
        assert_eq!(placed(), 0.25);
        ax.set_ylim(0.0, 5.0).expect("finite limits");
        fig.layout();
        assert_eq!(placed(), 0.5);
        assert_eq!(sec.xaxis().label_position(), SpineSide::Top);
    }
}
