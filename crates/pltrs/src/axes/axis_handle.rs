//! Handles to the x and y axis of an axes (matplotlib's `XAxis` and
//! `YAxis`): the tick helpers, visibility, direction and colour. Ticks and their
//! labels ([`super::axis_ticks`]), the axis label ([`super::axis_label`])
//! and unit conversion ([`super::axis_units`]) have modules of their own.

use std::rc::Rc;

use pltrs_text::{HAlign, VAlign};

use super::axis::{AxisState, TickHelperKind, TickerDefaults};
use super::{Axes, AxesState, TickParams};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::ticker::{AxisName, Formatter, Locator, TickerError};

/// One axis of an axes, from [`Axes::xaxis`], [`Axes::yaxis`] or
/// [`Axes::axis_of`].
///
/// ```
/// use pltrs::prelude::*;
/// let ax = Figure::new().subplot();
/// ax.xaxis().set_ticks([1.0, 2.0, 3.0]).set_ticklabels(["one", "two", "three"])?;
/// # Ok::<(), pltrs::axes::PlotError>(())
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
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert_eq!(ax.yaxis().axes(), &ax);
    /// ```
    pub fn axes(&self) -> &Axes {
        &self.axes
    }

    /// Whether LaTeX typesets the axis's tick labels: `text.usetex` as it
    /// was when the axes was made (matplotlib copies it to every tick
    /// label of the axis).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let plain = Figure::new().subplot();
    /// assert!(!plain.xaxis().usetex());
    /// ```
    pub fn usetex(&self) -> bool {
        self.axes
            .raw()
            .with::<AxesState, _>(|axes| axes.rc.text_usetex)
    }

    /// The alignment of the tick labels on the first or second side
    /// before any of their own: `xtick.alignment` along an x axis and
    /// `ytick.alignment` across a y axis as they were when the axes was
    /// made (matplotlib's `get_xaxis_text1_transform` and friends).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs_text::{HAlign, VAlign};
    /// let ax = Figure::new().subplot();
    /// assert_eq!(ax.yaxis().tick_label_alignment(true), (HAlign::Right, VAlign::CenterBaseline));
    /// ```
    pub fn tick_label_alignment(&self, first_side: bool) -> (HAlign, VAlign) {
        self.axes.raw().with::<AxesState, _>(|axes| {
            super::axis_layout::tick_label_alignment(self.name, first_side, &axes.rc)
        })
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

    /// Places the major ticks with a locator that may be shared, the one
    /// [`major_locator`](Self::major_locator) then returns.
    pub fn set_shared_major_locator(&self, locator: &Rc<dyn Locator>) -> &Self {
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
    /// use pltrs::prelude::*;
    /// use pltrs::ticker::{AutoLocator, MultipleLocator};
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

    /// Places the minor ticks with a locator that may be shared, the one
    /// [`minor_locator`](Self::minor_locator) then returns.
    pub fn set_shared_minor_locator(&self, locator: &Rc<dyn Locator>) -> &Self {
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

    /// Labels the major ticks with a formatter that may be shared, the one
    /// [`major_formatter`](Self::major_formatter) then returns.
    pub fn set_shared_major_formatter(&self, formatter: &Rc<dyn Formatter>) -> &Self {
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
        self.set_shared_minor_formatter(&(Rc::new(formatter) as Rc<dyn Formatter>))
    }

    /// Labels the minor ticks with a formatter that may be shared, the one
    /// [`minor_formatter`](Self::minor_formatter) then returns.
    pub fn set_shared_minor_formatter(&self, formatter: &Rc<dyn Formatter>) -> &Self {
        self.update_shared(|axis| axis.set_minor_formatter(Rc::clone(formatter)));
        self
    }

    /// Puts `new` in place of the locator `old` wherever this axis and the
    /// axes sharing it use `old`, major or minor, leaving whether it counts
    /// as the scale's default alone: matplotlib's tick helpers change in
    /// place (`axis.get_major_locator().set_params(...)`). Returns the
    /// locator now in use where `old` was (on a radial axis, `new` wrapped
    /// as a major locator is), `None` when `old` was nowhere in use.
    ///
    /// ```
    /// use std::rc::Rc;
    /// use pltrs::prelude::*;
    /// use pltrs::ticker::{Locator, MaxNLocator, Nbins};
    /// let ax = Figure::new().subplot();
    /// let old = ax.xaxis().major_locator();
    /// let new: Rc<dyn Locator> = Rc::new(MaxNLocator::new(Nbins::Fixed(3)));
    /// let installed = ax.xaxis().replace_locator(&old, &new).expect("in use");
    /// assert!(Rc::ptr_eq(&ax.xaxis().major_locator(), &installed));
    /// assert!(ax.xaxis().is_default_major_locator());
    /// assert!(ax.xaxis().replace_locator(&old, &new).is_none());
    /// ```
    pub fn replace_locator(
        &self,
        old: &Rc<dyn Locator>,
        new: &Rc<dyn Locator>,
    ) -> Option<Rc<dyn Locator>> {
        let mut installed = None;
        self.update_shared(|axis| {
            if Rc::ptr_eq(&axis.major_locator, old) {
                axis.major_locator = axis.for_role(Rc::clone(new));
                installed = Some(Rc::clone(&axis.major_locator));
            }
            if Rc::ptr_eq(&axis.minor_locator, old) {
                axis.minor_locator = Rc::clone(new);
                installed = Some(Rc::clone(new));
            }
        });
        installed
    }

    /// Puts `new` in place of the formatter `old` wherever this axis and
    /// the axes sharing it use `old`, as
    /// [`replace_locator`](Self::replace_locator) does for locators;
    /// returns whether `old` was in use.
    pub fn replace_formatter(&self, old: &Rc<dyn Formatter>, new: &Rc<dyn Formatter>) -> bool {
        let mut found = false;
        self.update_shared(|axis| {
            for slot in [&mut axis.major_formatter, &mut axis.minor_formatter] {
                if Rc::ptr_eq(slot, old) {
                    *slot = Rc::clone(new);
                    found = true;
                }
            }
        });
        found
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
    /// use pltrs::prelude::*;
    /// use pltrs::ticker::MultipleLocator;
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

    /// Counts the tick helpers in use as the scale's own (matplotlib's
    /// `_set_axes_scale` once a scale installed its helpers).
    pub fn mark_default_tick_helpers(&self) -> &Self {
        self.update_shared(|axis| axis.defaults = TickerDefaults::default());
        self
    }

    /// Counts the major (or `minor`) `helper` as the scale's own, or not
    /// (matplotlib's `isDefault_majloc = False`, ...): units such as dates
    /// or categories replace only default helpers.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::TickHelperKind;
    /// let ax = Figure::new().subplot();
    /// ax.xaxis().set_default_tick_helper(false, TickHelperKind::Locator, false);
    /// assert!(!ax.xaxis().is_default_major_locator());
    /// assert!(ax.xaxis().is_default_major_formatter());
    /// ```
    pub fn set_default_tick_helper(
        &self,
        minor: bool,
        helper: TickHelperKind,
        default: bool,
    ) -> &Self {
        self.update_shared(|axis| axis.defaults.set(minor, helper, default));
        self
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

    /// The offset (or scale) text of the major ticks in the current view,
    /// empty when there is none (matplotlib's `offsetText.get_text()`).
    ///
    /// # Errors
    ///
    /// When the locator or the formatter fails.
    pub fn offset_text(&self) -> Result<String, TickerError> {
        Ok(self.axes.ticks(self.name)?.offset)
    }

    /// The offset (or scale) text the ticks last placed gave, empty
    /// before any were (matplotlib's `offsetText.get_text()`, which its
    /// axis sets as it places the ticks to draw or measure them); reading
    /// it neither autoscales nor places ticks.
    pub fn kept_offset_text(&self) -> String {
        self.read(|axis| {
            axis.ticks
                .as_ref()
                .map(|ticks| ticks.offset.clone())
                .unwrap_or_default()
        })
    }

    /// Shows or hides the offset (or scale) text (matplotlib's
    /// `offsetText.set_visible`).
    pub fn set_offset_text_visible(&self, visible: bool) -> &Self {
        self.update(|axis| axis.offset_visible = visible);
        self
    }

    /// Whether the offset (or scale) text is shown (matplotlib's
    /// `offsetText.get_visible`); [`Axes::label_outer`] hides it on inner
    /// edges.
    pub fn offset_text_visible(&self) -> bool {
        self.read(|axis| axis.offset_visible)
    }

    /// The font size of the offset (or scale) text in points: the tick
    /// label size when the axis was made or cleared, which
    /// [`Axes::tick_params`] leaves (matplotlib's `offsetText.get_fontsize`).
    ///
    /// ```
    /// use pltrs::axes::TickParams;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let size = ax.yaxis().offset_text_size();
    /// ax.tick_params(TickParams { labelsize: Some(20.0.into()), ..TickParams::default() });
    /// assert_eq!(ax.yaxis().offset_text_size(), size);
    /// ```
    pub fn offset_text_size(&self) -> f64 {
        self.read(|axis| axis.offset_size)
    }

    /// Sets the font size of the offset (or scale) text in points
    /// (matplotlib's `offsetText.set_fontsize`).
    pub fn set_offset_text_size(&self, size: f64) -> &Self {
        self.update(|axis| axis.offset_size = size);
        self
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
    /// range (matplotlib's `Axis.set_inverted`, which the fixed limits of
    /// geographic axes let through).
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
        self.axes.raw().figure_state().borrow_mut().set_axis_view(
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

    /// Includes or leaves out the axis' tick labels, label and offset
    /// text when layouts (constrained and tight) make room for the axes
    /// (matplotlib's `Axis.set_in_layout`); a tight `savefig` still counts
    /// them.
    ///
    /// ```
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.xaxis().set_in_layout(false);
    /// assert!(!ax.xaxis().in_layout() && ax.yaxis().in_layout());
    /// ```
    pub fn set_in_layout(&self, in_layout: bool) -> &Self {
        self.update(|axis| axis.in_layout = in_layout);
        self
    }

    /// Whether layouts make room for the axis' texts.
    pub fn in_layout(&self) -> bool {
        self.axes
            .raw()
            .with::<AxesState, _>(|axes| axes.axis(self.name).in_layout)
    }

    /// Colours the axis with its decorations: the spines along it (on both
    /// sides), the major ticks and their labels, and the axis label
    /// (matplotlib's `SecondaryAxis.set_color`, also its `color` keyword
    /// of `secondary_xaxis`; here any axis takes it).
    ///
    /// ```
    /// use pltrs::axes::SecondaryLocation;
    /// use pltrs::colors::Color;
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// let sec = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), None)?;
    /// sec.xaxis().set_color("red");
    /// let red = Color::from("red");
    /// assert_eq!(sec.spines().bottom.edgecolor(), red);
    /// assert_eq!(sec.xaxis().tick_style(false).label_color, red);
    /// assert_eq!(sec.xaxis().label_color(), red);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.set_tick_params(TickParams {
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

    use crate::axes::TickHelperKind;
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
    fn default_flags_are_set_one_helper_at_a_time() {
        let ax = Figure::new().subplot();
        let yaxis = ax.yaxis();
        yaxis.set_default_tick_helper(true, TickHelperKind::Locator, false);
        assert!(!yaxis.is_default_minor_locator());
        assert!(yaxis.is_default_major_locator() && yaxis.is_default_minor_formatter());
        yaxis.set_default_tick_helper(true, TickHelperKind::Locator, true);
        assert!(yaxis.is_default_minor_locator());
        yaxis.set_default_tick_helper(false, TickHelperKind::Formatter, false);
        assert!(!yaxis.is_default_major_formatter());
        yaxis.mark_default_tick_helpers();
        assert!(yaxis.is_default_major_formatter());
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
