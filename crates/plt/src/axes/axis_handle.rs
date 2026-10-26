//! Handles to the x and y axis of an axes (matplotlib's `XAxis` and
//! `YAxis`): the tick helpers, visibility, direction and colour. Ticks and their
//! labels ([`super::axis_ticks`]), the axis label ([`super::axis_label`])
//! and unit conversion ([`super::axis_units`]) have modules of their own.

use std::rc::Rc;

use super::axis::AxisState;
use super::{Axes, AxesState, TickParams};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::ticker::{AxisName, Formatter, Locator};

/// One axis of an axes, from [`Axes::xaxis`], [`Axes::yaxis`] or
/// [`Axes::axis_of`].
///
/// ```
/// use plt::prelude::*;
/// let ax = Figure::new().subplot();
/// ax.xaxis().set_ticks([1.0, 2.0, 3.0]).set_ticklabels(["one", "two", "three"])?;
/// # Ok::<(), plt::axes::PlotError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Axis {
    pub(super) axes: Axes,
    pub(super) name: AxisName,
}

impl Axis {
    /// Which axis this is.
    pub fn name(&self) -> AxisName {
        self.name
    }

    /// The axes the axis belongs to (matplotlib's `axis.axes`), which unit
    /// converters may consult.
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert_eq!(ax.yaxis().axes(), &ax);
    /// ```
    pub fn axes(&self) -> &Axes {
        &self.axes
    }

    /// Reads this axis's state.
    pub(super) fn read<R>(&self, f: impl FnOnce(&AxisState) -> R) -> R {
        self.axes
            .raw()
            .with::<AxesState, _>(|axes| f(axes.axis(self.name)))
    }

    /// Changes this axis's state.
    pub(super) fn update<R>(&self, f: impl FnOnce(&mut AxisState) -> R) -> R {
        self.axes
            .raw()
            .with_mut::<AxesState, _>(|axes| f(axes.axis_mut(self.name)))
    }

    /// Changes the styles of this axis's tick labels; the layout engine
    /// runs again before positions are next used, as their extents may
    /// have changed (matplotlib lays out at every draw).
    pub(super) fn update_labels(&self, f: impl FnOnce(&mut AxisState)) {
        self.update(f);
        self.axes.raw().figure_state().borrow_mut().layout.pending = true;
    }

    /// Applies `f` to this axis and to the same axis of every axes
    /// sharing it (matplotlib shares the tickers between them).
    pub(super) fn update_shared(&self, f: impl FnMut(&mut AxisState)) {
        self.axes
            .raw()
            .figure_state()
            .borrow_mut()
            .update_shared_axis(self.axes.id(), self.name, f);
    }

    /// Places the major ticks with `locator`.
    pub fn set_major_locator(&self, locator: impl Locator) -> &Self {
        self.set_shared_major_locator(&(Rc::new(locator) as Rc<dyn Locator>))
    }

    /// Places the major ticks with a locator that may be shared.
    pub(crate) fn set_shared_major_locator(&self, locator: &Rc<dyn Locator>) -> &Self {
        let owner = self.axes.id();
        self.update_shared(|axis| {
            axis.set_major_locator(Rc::clone(locator));
            axis.ticker_owner = Some(owner);
        });
        self
    }

    /// The locator placing the major ticks (matplotlib's
    /// `get_major_locator`). It is the one the axis uses, shared with the
    /// axes sharing this axis rather than copied, as in matplotlib; find
    /// its type with [`Any`](std::any::Any):
    ///
    /// ```
    /// use std::any::Any;
    /// use plt::prelude::*;
    /// use plt::ticker::{AutoLocator, MultipleLocator};
    /// let ax = Figure::new().subplot();
    /// let locator = ax.xaxis().major_locator();
    /// assert!((locator.as_ref() as &dyn Any).is::<AutoLocator>());
    /// ax.xaxis().set_major_locator(MultipleLocator::new(0.25)?);
    /// let locator = ax.xaxis().major_locator();
    /// assert!((locator.as_ref() as &dyn Any).is::<MultipleLocator>());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn major_locator(&self) -> Rc<dyn Locator> {
        self.read(|axis| Rc::clone(&axis.major_locator))
    }

    /// Places the minor ticks with `locator`.
    pub fn set_minor_locator(&self, locator: impl Locator) -> &Self {
        self.set_shared_minor_locator(&(Rc::new(locator) as Rc<dyn Locator>))
    }

    /// Places the minor ticks with a locator that may be shared.
    pub(crate) fn set_shared_minor_locator(&self, locator: &Rc<dyn Locator>) -> &Self {
        let owner = self.axes.id();
        self.update_shared(|axis| {
            axis.set_minor_locator(Rc::clone(locator));
            axis.ticker_owner = Some(owner);
        });
        self
    }

    /// The locator placing the minor ticks (matplotlib's
    /// `get_minor_locator`), shared like [`major_locator`](Self::major_locator).
    pub fn minor_locator(&self) -> Rc<dyn Locator> {
        self.read(|axis| Rc::clone(&axis.minor_locator))
    }

    /// Labels the major ticks with `formatter`.
    pub fn set_major_formatter(&self, formatter: impl Formatter) -> &Self {
        self.set_shared_major_formatter(&(Rc::new(formatter) as Rc<dyn Formatter>))
    }

    /// Labels the major ticks with a formatter that may be shared.
    pub(crate) fn set_shared_major_formatter(&self, formatter: &Rc<dyn Formatter>) -> &Self {
        self.update_shared(|axis| axis.set_major_formatter(Rc::clone(formatter)));
        self
    }

    /// The formatter labelling the major ticks (matplotlib's
    /// `get_major_formatter`), shared like
    /// [`major_locator`](Self::major_locator).
    pub fn major_formatter(&self) -> Rc<dyn Formatter> {
        self.read(|axis| Rc::clone(&axis.major_formatter))
    }

    /// Labels the minor ticks with `formatter`.
    pub fn set_minor_formatter(&self, formatter: impl Formatter) -> &Self {
        let formatter: Rc<dyn Formatter> = Rc::new(formatter);
        self.update_shared(|axis| axis.set_minor_formatter(Rc::clone(&formatter)));
        self
    }

    /// The formatter labelling the minor ticks (matplotlib's
    /// `get_minor_formatter`), shared like
    /// [`major_locator`](Self::major_locator).
    pub fn minor_formatter(&self) -> Rc<dyn Formatter> {
        self.read(|axis| Rc::clone(&axis.minor_formatter))
    }

    /// Whether the major locator is still the one the scale installed
    /// (matplotlib's `isDefault_majloc`): setting a locator clears it,
    /// setting a scale restores it. Units such as dates or categories
    /// replace only default helpers.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::ticker::MultipleLocator;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.xaxis().is_default_major_locator());
    /// ax.xaxis().set_major_locator(MultipleLocator::new(0.5)?);
    /// assert!(!ax.xaxis().is_default_major_locator());
    /// assert!(ax.yaxis().is_default_major_locator());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn is_default_major_locator(&self) -> bool {
        self.read(|axis| axis.defaults.major.locator)
    }

    /// Whether the major formatter is still the one the scale installed
    /// (matplotlib's `isDefault_majfmt`); see
    /// [`is_default_major_locator`](Self::is_default_major_locator).
    pub fn is_default_major_formatter(&self) -> bool {
        self.read(|axis| axis.defaults.major.formatter)
    }

    /// Whether the minor locator is still the one the scale installed
    /// (matplotlib's `isDefault_minloc`); see
    /// [`is_default_major_locator`](Self::is_default_major_locator).
    pub fn is_default_minor_locator(&self) -> bool {
        self.read(|axis| axis.defaults.minor.locator)
    }

    /// Whether the minor formatter is still the one the scale installed
    /// (matplotlib's `isDefault_minfmt`); see
    /// [`is_default_major_locator`](Self::is_default_major_locator).
    pub fn is_default_minor_formatter(&self) -> bool {
        self.read(|axis| axis.defaults.minor.formatter)
    }

    /// Whether the offset (or scale) text is shown (matplotlib's
    /// `offsetText.get_visible`); [`Axes::label_outer`] hides it on inner
    /// edges.
    pub fn offset_text_visible(&self) -> bool {
        self.read(|axis| axis.offset_visible)
    }

    /// Whether the axis runs from high to low values (matplotlib's
    /// `Axis.get_inverted`).
    pub fn inverted(&self) -> bool {
        let (low, high) = match self.name {
            AxisName::X => self.axes.xlim(),
            AxisName::Y => self.axes.ylim(),
        };
        high < low
    }

    /// Makes the axis run from high to low values or back, keeping its
    /// range (matplotlib's `Axis.set_inverted`).
    pub fn set_inverted(&self, inverted: bool) -> &Self {
        let (low, high) = match self.name {
            AxisName::X => self.axes.xlim(),
            AxisName::Y => self.axes.ylim(),
        };
        let (low, high) = if low <= high {
            (low, high)
        } else {
            (high, low)
        };
        let limits = if inverted { (high, low) } else { (low, high) };
        self.axes.raw().figure_state().borrow_mut().set_axis_limits(
            self.axes.id(),
            self.name,
            limits,
            None,
            true,
        );
        crate::colorbar::follow_view(&self.axes);
        super::run_limit_callbacks(self.axes.raw().figure_state());
        self
    }

    /// Shows or hides the whole axis: ticks, tick labels, label and offset
    /// text (matplotlib's `Axis.set_visible`).
    pub fn set_visible(&self, visible: bool) -> &Self {
        self.update(|axis| axis.visible = visible);
        self
    }

    /// Whether the axis is drawn.
    pub fn visible(&self) -> bool {
        self.axes
            .raw()
            .with::<AxesState, _>(|axes| axes.axis(self.name).visible)
    }

    /// Colours the axis with its decorations: the spines along it (on both
    /// sides), the major ticks and their labels, and the axis label
    /// (matplotlib's `SecondaryAxis.set_color`, also its `color` keyword
    /// of `secondary_xaxis`; here any axis takes it).
    ///
    /// ```
    /// use plt::axes::SecondaryLocation;
    /// use plt::colors::Color;
    /// use plt::prelude::*;
    /// use plt::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// let sec = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), None)?;
    /// sec.xaxis().set_color("red");
    /// let red = Color::from("red");
    /// assert_eq!(sec.spines().bottom.edgecolor(), red);
    /// assert_eq!(sec.xaxis().tick_style(false).label_color, red);
    /// assert_eq!(sec.xaxis().label_color(), red);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn set_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.set_tick_params(&TickParams {
            colors: Some(color.clone()),
            ..TickParams::default()
        });
        for spine in self.axes.spines().iter() {
            if spine.side().axis() == self.name {
                spine.set_color(color.clone());
            }
        }
        self.set_label_color(color)
    }
}

impl Axes {
    /// The x axis.
    pub fn xaxis(&self) -> Axis {
        self.axis_of(AxisName::X)
    }

    /// The y axis.
    pub fn yaxis(&self) -> Axis {
        self.axis_of(AxisName::Y)
    }

    /// The axis called `name`.
    pub fn axis_of(&self, name: AxisName) -> Axis {
        Axis {
            axes: self.clone(),
            name,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;

    use crate::figure::{Figure, Share};
    use crate::ticker::{AutoLocator, FixedFormatter, MultipleLocator, NullLocator};

    #[test]
    fn tick_helpers_read_back_what_was_set() {
        let ax = Figure::new().subplot();
        let xaxis = ax.xaxis();
        assert!((xaxis.major_locator().as_ref() as &dyn Any).is::<AutoLocator>());
        assert!((xaxis.minor_locator().as_ref() as &dyn Any).is::<NullLocator>());
        xaxis
            .set_major_locator(MultipleLocator::new(0.5).expect("a positive base"))
            .set_minor_formatter(FixedFormatter::new(vec!["a".into()]));
        let major = xaxis.major_locator();
        let multiple = (major.as_ref() as &dyn Any).downcast_ref::<MultipleLocator>();
        assert!(multiple.is_some());
        assert!((xaxis.minor_formatter().as_ref() as &dyn Any).is::<FixedFormatter>());
        assert!(
            (ax.yaxis().major_locator().as_ref() as &dyn Any).is::<AutoLocator>(),
            "the other axis keeps its own"
        );
    }

    #[test]
    fn axes_sharing_an_axis_share_its_tick_helpers() {
        let fig = Figure::new();
        let axs = fig
            .subplots_shared(1, 2, Share::All, Share::None)
            .expect("a grid");
        axs[(0, 0)]
            .xaxis()
            .set_major_locator(MultipleLocator::new(0.25).expect("a positive base"));
        let (left, right) = (axs[(0, 0)].xaxis(), axs[(0, 1)].xaxis());
        assert!(std::rc::Rc::ptr_eq(
            &left.major_locator(),
            &right.major_locator()
        ));
        assert!(std::rc::Rc::ptr_eq(
            &left.major_formatter(),
            &right.major_formatter()
        ));
    }

    #[test]
    fn inverting_keeps_the_range() {
        let ax = Figure::new().subplot();
        ax.set_xlim(1.0, 3.0).expect("finite limits");
        ax.xaxis().set_inverted(true);
        assert_eq!(ax.xlim(), (3.0, 1.0));
        assert!(ax.xaxis().inverted());
        ax.xaxis().set_inverted(false);
        assert_eq!(ax.xlim(), (1.0, 3.0));
    }
}
