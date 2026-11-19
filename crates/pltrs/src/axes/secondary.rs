//! Secondary axes (matplotlib's `secondary_xaxis` / `secondary_yaxis`): an
//! extra axis on a host axes, showing the host's range converted by a pair
//! of functions.

use std::rc::Rc;

use super::{Axes, AxesState, AxisName, PlotError, TickPosition};
use crate::artist::{ArtistHandle, ArtistId};
use crate::figure::{FigureState, insert_axes};
use crate::scale::{FuncScale, FuncScaleLog, FuncTransform, Scale, ScaleName};
use crate::spines::SpineSide;
use crate::ticker::{LogBase, NullLocator};
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
    host_scale: Option<ScaleName>,
}

pltrs_core::archive_struct!(Secondary {
    host,
    orientation,
    data_location,
    functions,
    host_scale
});

impl Axes {
    /// Adds a secondary x axis showing this axes' x range converted by
    /// `functions` (host units to secondary units, and back; `None` is the
    /// identity), returned as an axes for labelling and ticks (matplotlib's
    /// `secondary_xaxis`). Its limits are the host's converted once the
    /// figure is drawn or laid out (or its aspect applied), as matplotlib's
    /// draw sets them: until then, it has the limits of a new axes.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::SecondaryLocation;
    /// use pltrs::scale::FuncTransform;
    /// use pltrs::spines::SpineSide;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.set_xlim(0.0, 180.0)?;
    /// let radians = FuncTransform::new(f64::to_radians, f64::to_degrees);
    /// let sec = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(radians))?;
    /// assert_eq!(sec.xlim(), (0.0, 1.0));
    /// fig.layout();
    /// let (low, high) = sec.xlim();
    /// assert!(low.abs() < 1e-12 && (high - std::f64::consts::PI).abs() < 1e-12);
    /// # Ok::<(), pltrs::axes::PlotError>(())
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

    /// Converts the host's range with `functions` from now on (`None` is
    /// the identity), keeping the axis' side (matplotlib's
    /// `SecondaryAxis.set_functions`); does nothing to an axes that is no
    /// secondary axis. The axis' scale keeps the functions it was made
    /// with until the host's scale changes, as matplotlib's `_set_scale`
    /// remakes it only then: the limits follow the new functions, the
    /// ticks are placed through the old ones.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::SecondaryLocation;
    /// use pltrs::scale::FuncTransform;
    /// use pltrs::spines::SpineSide;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.set_xlim(1.0, 2.0)?;
    /// let sec = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), None)?;
    /// sec.set_secondary_functions(Some(FuncTransform::new(|x| 10.0 * x, |x| x / 10.0)));
    /// // The limits follow when the figure is drawn or laid out.
    /// fig.layout();
    /// assert_eq!(sec.xlim(), (10.0, 20.0));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_secondary_functions(&self, functions: Option<FuncTransform>) -> &Self {
        self.update_secondary(|link| {
            link.functions = functions.unwrap_or_else(FuncTransform::identity);
        })
    }

    /// Moves this secondary axis to `location` across its host, keeping
    /// the side its spine, ticks and label face (matplotlib's
    /// `SecondaryAxis.set_location`); does nothing to an axes that is no
    /// secondary axis.
    ///
    /// # Errors
    ///
    /// When `location` is a side of the other axis.
    pub fn set_secondary_location(&self, location: SecondaryLocation) -> Result<&Self, PlotError> {
        let Some(orientation) = self
            .raw
            .with::<AxesState, _>(|axes| axes.secondary.as_ref().map(|link| link.orientation))
        else {
            return Ok(self);
        };
        let pos = location_fraction(orientation, location)?;
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.position = secondary_bounds(orientation, pos));
        Ok(self.update_secondary(|link| link.data_location = data_location(location)))
    }

    /// Changes the link of this secondary axis to its host with `change`,
    /// then brings the axis' scale up to date with its host (its limits
    /// follow at the next draw).
    fn update_secondary(&self, change: impl FnOnce(&mut Secondary)) -> &Self {
        let linked = self
            .raw
            .with_mut::<AxesState, _>(|axes| axes.secondary.as_mut().map(change).is_some());
        if linked {
            self.raw
                .figure_state()
                .borrow_mut()
                .sync_secondary_scale(self.id());
        }
        self
    }

    fn secondary(
        &self,
        orientation: AxisName,
        location: SecondaryLocation,
        functions: Option<FuncTransform>,
    ) -> Result<Self, PlotError> {
        let pos = location_fraction(orientation, location)?;
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
                data_location: data_location(location),
                functions: functions.unwrap_or_else(FuncTransform::identity),
                host_scale: None,
            });
        });
        figure.borrow_mut().sync_secondary_scale(secondary.id());
        Ok(secondary)
    }
}

impl FigureState {
    /// Brings a secondary axis up to date with its host: its scale follows
    /// the host's (a function scale, logarithmic over a log host), its
    /// limits are the host's converted and an axis at a data value of the
    /// host moves there (matplotlib's `_set_scale` and `_set_lims`, which
    /// its draw and `apply_aspect` run). Does nothing for other axes.
    ///
    /// matplotlib sets the limits wherever it applies the axis' aspect or
    /// draws it, and the program's functions are called each time: when
    /// the figure draws the axes it holds (`_get_draw_artists`), where its
    /// host places its titles, when it measures the axis for a tight box,
    /// and twice when it draws the axis (see `tick_sites`).
    pub(crate) fn sync_secondary(&mut self, axes: ArtistId) {
        let Some(link) = self.sync_secondary_scale(axes) else {
            return;
        };
        self.unstale_axes(link.host);
        let Some(host) = self.store.get_as::<AxesState>(link.host) else {
            return;
        };
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
        // matplotlib's `_set_lims` hands the functions the host's limits as
        // one array.
        let mut limits = [low, high];
        link.functions.forward_all(&mut limits);
        let [mut v0, mut v1] = limits;
        if (v0 < v1) != (low < high) {
            (v0, v1) = (v1, v0);
        }
        if let Some(state) = self.store.get_as::<AxesState>(axes) {
            state.warn_identical_limits(link.orientation, (v0, v1));
        }
        self.set_axis_limits(axes, link.orientation, (v0, v1), Some(false), true);
    }

    /// Gives a secondary axis the scale of its host's (matplotlib's
    /// `_set_scale`, which making the axis and `set_functions` run),
    /// returning its link to the host; `None` for other axes. Only the
    /// name of the host's scale is read: its limits wait to be autoscaled
    /// as they did (a scale set on the host afterwards still rescales it).
    pub(crate) fn sync_secondary_scale(&mut self, axes: ArtistId) -> Option<Secondary> {
        let link = self
            .store
            .get_as::<AxesState>(axes)
            .and_then(|state| state.secondary.clone())?;
        let host_scale = self
            .store
            .get_as::<AxesState>(link.host)?
            .axis(link.orientation)
            .scale()
            .name();
        if link.host_scale.as_ref() != Some(&host_scale) {
            let reversed = link.functions.reversed();
            let scale: Rc<dyn Scale> = if host_scale == ScaleName::Log {
                Rc::new(FuncScaleLog::from_base(reversed, LogBase::TEN))
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
        Some(link)
    }
}

/// The fraction of the host across `orientation` at which a secondary
/// axis at `location` starts out (a data location is placed when the axis
/// is synchronised with its host).
fn location_fraction(orientation: AxisName, location: SecondaryLocation) -> Result<f64, PlotError> {
    match location {
        SecondaryLocation::Side(side) if side.axis() != orientation => Err(PlotError::WrongSide {
            axis: orientation,
            side,
        }),
        SecondaryLocation::Side(side) => Ok(f64::from(u8::from(side.is_high()))),
        SecondaryLocation::Fraction(pos) | SecondaryLocation::Data(pos) => Ok(pos),
    }
}

/// The host data value a secondary axis at `location` follows, if any.
const fn data_location(location: SecondaryLocation) -> Option<f64> {
    match location {
        SecondaryLocation::Data(value) => Some(value),
        SecondaryLocation::Side(_) | SecondaryLocation::Fraction(_) => None,
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
        assert_eq!(sec.yscale(), ScaleName::FunctionLog);
        assert_eq!(fig.axes().len(), 1, "secondary axes are not figure axes");
        let host = ax.position();
        let placed = sec.raw.with::<AxesState, _>(|a| a.position);
        assert_eq!(placed.x0(), 0.3);
        assert!(host.width() > 0.0);
    }

    #[test]
    fn a_scale_set_after_a_secondary_axis_rescales_its_host() {
        // matplotlib's limits: the margins are added in the symlog space,
        // the secondary axis reading only the name of its host's scale.
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.plot([0.75, 2.25], [0.0, 1.0]).expect("finite data");
        ax.secondary_yaxis(SecondaryLocation::Side(SpineSide::Right), None)
            .unwrap();
        ax.set_xscale("symlog").unwrap();
        fig.layout();
        let (low, high) = ax.xlim();
        assert!((low - 0.682_896).abs() < 1e-6, "{low}");
        assert!((high - 2.451_672).abs() < 1e-6, "{high}");
    }

    #[test]
    fn locations_and_functions_change_after_creation() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(1.0, 2.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let sec = ax
            .secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), None)
            .unwrap();
        sec.set_secondary_functions(Some(FuncTransform::new(|x| -x, |x| -x)));
        assert_eq!(sec.xlim(), (0.0, 1.0), "the limits follow at the next draw");
        fig.layout();
        assert_eq!(sec.xlim(), (-2.0, -1.0), "flipped to keep the order");
        sec.set_secondary_location(SecondaryLocation::Data(5.0))
            .unwrap();
        fig.layout();
        let placed = sec.raw.with::<AxesState, _>(|a| a.position.y0());
        assert_eq!(placed, 0.5);
        let wrong = sec.set_secondary_location(SecondaryLocation::Side(SpineSide::Left));
        assert!(matches!(wrong, Err(PlotError::WrongSide { .. })));
        assert!(
            ax.set_secondary_location(SecondaryLocation::Fraction(0.5))
                .is_ok(),
            "a host is no secondary axis: nothing changes"
        );
    }

    #[test]
    fn identical_limits_warn_as_matplotlib_sets_them() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&seen);
        crate::warnings::set_handler(move |message| sink.borrow_mut().push(message.to_owned()));
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(-1.0, 1.0).expect("finite limits");
        let square = FuncTransform::new(|x| x * x, f64::sqrt);
        let sec = ax
            .secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(square))
            .unwrap();
        fig.layout();
        let (v0, v1) = sec.xlim();
        assert!(v0 < 1.0 && v1 > 1.0, "expanded around the identical limits");
        assert!(!seen.borrow().is_empty());
        assert!(seen.borrow().iter().all(|message| message
            == "Attempting to set identical low and high xlims makes transformation \
                singular; automatically expanding."));
        // matplotlib's `get_tightbbox` applies the aspect once: one warning.
        seen.borrow_mut().clear();
        sec.apply_aspect_to_measure().expect("an aspect");
        assert!(sec.tight_bbox(false).is_some());
        crate::warnings::clear_handler();
        assert_eq!(seen.borrow().len(), 1);
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
