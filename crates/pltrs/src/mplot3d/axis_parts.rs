//! What a 3-D axis tells about its parts, which matplotlib keeps as
//! objects of its own (the `Tick`s, `offsetText` and `label` of an
//! `axis3d.Axis`, and its locators and formatters): the tick helpers it
//! uses, the style of each tick level and the pad of each major tick, its
//! label pad, its offset text, and where the axes draw its texts.

use std::cell::Cell;
use std::rc::Rc;

use pltrs_core::Bbox;

use super::axis_draw::{AxisText3D, axis_extents, axis_texts};
use super::axis_handle::Axis3D;
use super::handle::Axes3D;
use super::state::Axes3DState;
use crate::artist::{ArtistHandle, ArtistLookup, LayoutContext};
use crate::axes::{TickHelperKind, TickStyle, TickerDefaults};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::ticker::{Formatter, Locator, TickerError};
use crate::transforms::TransformContext;

impl Axis3D {
    /// The 3-D axes the axis belongs to.
    pub fn axes(&self) -> &Axes3D {
        &self.axes
    }

    /// The locator placing the major ticks (matplotlib's
    /// `get_major_locator`), shared with the axes sharing the axis.
    pub fn major_locator(&self) -> Rc<dyn Locator> {
        self.read(|axes| Rc::clone(&axes.axis(self.dir).tickers.major_locator))
    }

    /// The locator placing the minor ticks (`get_minor_locator`).
    pub fn minor_locator(&self) -> Rc<dyn Locator> {
        self.read(|axes| Rc::clone(&axes.axis(self.dir).tickers.minor_locator))
    }

    /// The formatter labelling the major ticks (`get_major_formatter`).
    pub fn major_formatter(&self) -> Rc<dyn Formatter> {
        self.read(|axes| Rc::clone(&axes.axis(self.dir).tickers.major_formatter))
    }

    /// The formatter labelling the minor ticks (`get_minor_formatter`).
    pub fn minor_formatter(&self) -> Rc<dyn Formatter> {
        self.read(|axes| Rc::clone(&axes.axis(self.dir).tickers.minor_formatter))
    }

    /// Places the major (or `minor`) ticks with a locator that may be
    /// shared, the one [`major_locator`](Self::major_locator) then
    /// returns (matplotlib's `set_major_locator`).
    pub fn set_shared_locator(&self, minor: bool, locator: &Rc<dyn Locator>) -> &Self {
        self.set_tickers(|t| {
            if minor {
                t.minor_locator = Rc::clone(locator);
                t.defaults.minor.locator = false;
            } else {
                t.major_locator = Rc::clone(locator);
                t.defaults.major.locator = false;
            }
        });
        if minor {
            self
        } else {
            self.own_major_locator()
        }
    }

    /// Labels the major (or `minor`) ticks with a formatter that may be
    /// shared (matplotlib's `set_major_formatter`).
    pub fn set_shared_formatter(&self, minor: bool, formatter: &Rc<dyn Formatter>) -> &Self {
        self.set_tickers(|t| {
            if minor {
                t.minor_formatter = Rc::clone(formatter);
                t.defaults.minor.formatter = false;
            } else {
                t.major_formatter = Rc::clone(formatter);
                t.defaults.major.formatter = false;
            }
        })
    }

    /// Puts `new` in place of the locator `old` wherever this axis and the
    /// axes sharing it use `old`, keeping whether it counts as the
    /// scale's (a helper changed in place, as matplotlib's are); returns
    /// the locator installed, `None` when `old` was in use nowhere.
    ///
    /// ```
    /// use std::rc::Rc;
    /// use pltrs::figure::Figure;
    /// use pltrs::ticker::{Locator, MaxNLocator, Nbins};
    /// let ax = Figure::new().subplot_3d();
    /// let old = ax.zaxis().major_locator();
    /// let new: Rc<dyn Locator> = Rc::new(MaxNLocator::new(Nbins::Fixed(3)));
    /// let installed = ax.zaxis().replace_locator(&old, &new).expect("in use");
    /// assert!(Rc::ptr_eq(&ax.zaxis().major_locator(), &installed));
    /// assert!(ax.zaxis().is_default_locator(false));
    /// assert!(ax.zaxis().replace_locator(&old, &new).is_none());
    /// ```
    pub fn replace_locator(
        &self,
        old: &Rc<dyn Locator>,
        new: &Rc<dyn Locator>,
    ) -> Option<Rc<dyn Locator>> {
        let found = Cell::new(false);
        self.set_tickers(|t| {
            for slot in [&mut t.major_locator, &mut t.minor_locator] {
                if Rc::ptr_eq(slot, old) {
                    *slot = Rc::clone(new);
                    found.set(true);
                }
            }
        });
        found.get().then(|| Rc::clone(new))
    }

    /// Puts `new` in place of the formatter `old` wherever this axis and
    /// the axes sharing it use `old`, as
    /// [`replace_locator`](Self::replace_locator) does for locators;
    /// returns whether `old` was in use.
    pub fn replace_formatter(&self, old: &Rc<dyn Formatter>, new: &Rc<dyn Formatter>) -> bool {
        let found = Cell::new(false);
        self.set_tickers(|t| {
            for slot in [&mut t.major_formatter, &mut t.minor_formatter] {
                if Rc::ptr_eq(slot, old) {
                    *slot = Rc::clone(new);
                    found.set(true);
                }
            }
        });
        found.get()
    }

    /// Counts the tick helpers in use as the scale's own (matplotlib's
    /// `_set_axes_scale` once a scale of the program's installed them).
    pub fn mark_default_tick_helpers(&self) -> &Self {
        self.set_tickers(|t| t.defaults = TickerDefaults::default())
    }

    /// Counts the major (or `minor`) `helper` as the scale's own, or not
    /// (matplotlib's `isDefault_majloc = False`, ...); see
    /// [`Axis::set_default_tick_helper`](crate::axes::Axis::set_default_tick_helper).
    pub fn set_default_tick_helper(
        &self,
        minor: bool,
        helper: TickHelperKind,
        default: bool,
    ) -> &Self {
        self.set_tickers(|t| t.defaults.set(minor, helper, default))
    }

    /// Whether the label text was never set, so that a unit converter's
    /// label replaces it (matplotlib's `isDefault_label`).
    pub fn is_default_label(&self) -> bool {
        self.read(|axes| axes.axis(self.dir).label.is_default)
    }

    /// Says whether the label text counts as never set (matplotlib's
    /// `isDefault_label` assigned).
    pub fn set_default_label(&self, default: bool) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).label.is_default = default);
        self
    }

    /// Whether the major (or `minor`) locator is still the one the scale
    /// installed (matplotlib's `isDefault_majloc`, `isDefault_minloc`).
    pub fn is_default_locator(&self, minor: bool) -> bool {
        self.read(|axes| {
            let defaults = &axes.axis(self.dir).tickers.defaults;
            if minor {
                defaults.minor.locator
            } else {
                defaults.major.locator
            }
        })
    }

    /// Whether the major (or `minor`) formatter is still the scale's
    /// (`isDefault_majfmt`, `isDefault_minfmt`).
    pub fn is_default_formatter(&self, minor: bool) -> bool {
        self.read(|axes| {
            let defaults = &axes.axis(self.dir).tickers.defaults;
            if minor {
                defaults.minor.formatter
            } else {
                defaults.major.formatter
            }
        })
    }

    /// The tick parameters of the major (or `minor`) ticks: every tick of
    /// a 3-D axis is drawn with those of its level, but for its label pad
    /// (see [`tick_pad`](Self::tick_pad)).
    pub fn tick_style(&self, minor: bool) -> TickStyle {
        self.read(|axes| {
            let axis = axes.axis(self.dir);
            if minor {
                axis.minor_style.clone()
            } else {
                axis.major_style.clone()
            }
        })
    }

    /// The label pad of the major (or `minor`) tick at `index` among those
    /// the locator places, in points (matplotlib's `tick.get_pad()`).
    pub fn tick_pad(&self, minor: bool, index: usize) -> f64 {
        self.read(|axes| axes.axis(self.dir).tick_pad(minor, index))
    }

    /// Gives the major tick at `index` among those the locator places its
    /// own label pad, in points (matplotlib's `tick.set_pad`); the other
    /// ticks keep theirs.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let pad = ax.yaxis().tick_pad(false, 1);
    /// ax.yaxis().set_major_tick_pad(1, pad + 5.0);
    /// assert_eq!(ax.yaxis().major_tick_pads()?[..3], [pad, pad + 5.0, pad]);
    /// # Ok::<(), pltrs::ticker::TickerError>(())
    /// ```
    pub fn set_major_tick_pad(&self, index: usize, pad: f64) -> &Self {
        self.set_level_tick_pad(false, index, pad)
    }

    /// The distance of the axis label from the axis in points, on top of
    /// the fixed offset 3-D axes keep (matplotlib's `labelpad`).
    pub fn label_pad(&self) -> f64 {
        self.read(|axes| axes.axis(self.dir).label.pad)
    }

    /// The colour of the axis label.
    pub fn label_color(&self) -> Color {
        self.read(|axes| axes.axis(self.dir).label.color.clone())
    }

    /// Colours the axis label (matplotlib's `label.set_color`).
    pub fn set_label_color(&self, color: Color) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).label.color = color);
        self
    }

    /// The font of the axis label.
    pub fn label_font(&self) -> FontProperties {
        self.read(|axes| axes.axis(self.dir).label.font.clone())
    }

    /// Sets the font of the axis label (matplotlib's
    /// `label.set_fontproperties`).
    pub fn set_label_font(&self, font: FontProperties) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).label.font = font);
        self
    }

    /// The offset (or scale) text of the major ticks, after a layout pass
    /// (matplotlib's `offsetText.get_text()`).
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails.
    pub fn offset_text(&self) -> Result<String, TickerError> {
        Ok(self.ticks()?.offset)
    }

    /// Whether the offset text is drawn: with the major tick labels.
    pub fn offset_text_visible(&self) -> bool {
        self.read(|axes| axes.axis(self.dir).offset_visible())
    }

    /// The colour of the offset text.
    pub fn offset_text_color(&self) -> Color {
        self.read(|axes| axes.axis(self.dir).offset_color.clone())
    }

    /// The tick labels, offset text and label of the axis where the axes
    /// draw them with the ticks of the last layout pass (where matplotlib
    /// moves its `Text` objects when it draws the axis), without laying
    /// the figure out; none when the axes has no place in its figure.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::AxisTextPart;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_zlabel("height");
    /// ax.zaxis().ticks()?;
    /// let texts = ax.zaxis().texts();
    /// let label = texts.iter().find(|text| text.part == AxisTextPart::Label).unwrap();
    /// assert!(label.extent.width() > 0.0);
    /// let major = |part| matches!(part, AxisTextPart::TickLabel { minor: false, .. });
    /// assert!(texts.iter().any(|text| major(text.part)));
    /// # Ok::<(), pltrs::ticker::TickerError>(())
    /// ```
    pub fn texts(&self) -> Vec<AxisText3D> {
        self.laid_out(|axes, layout, extent| axis_texts(axes, layout, extent, self.dir))
    }

    /// The box enclosing the line and texts of the axis, without its label
    /// when `for_layout_only`, in display pixels, after a layout pass
    /// (matplotlib's `Axis3D.get_tightbbox`); `None` for an axes with no
    /// place in its figure, or an axis hidden. The axis of an axes turned
    /// off is measured as it would be drawn, as matplotlib measures it.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let shown = ax.zaxis().tight_bbox(false)?;
    /// ax.set_axis_off();
    /// assert_eq!(ax.zaxis().tight_bbox(false)?, shown);
    /// # Ok::<(), pltrs::ticker::TickerError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails.
    pub fn tight_bbox(&self, for_layout_only: bool) -> Result<Option<Bbox>, TickerError> {
        self.ticks()?;
        self.read(|axes| {
            let axis = axes.axis(self.dir);
            if axis.visible {
                // matplotlib updates the ticks once per edge they are on.
                let span = axes.limits.span(self.dir);
                for _ in 0..axis.tick_edges() {
                    axis.update_at_site(&span);
                }
            }
        });
        let boxes = self.laid_out(|axes, layout, extent| {
            if axes.axis(self.dir).visible {
                axis_extents(axes, layout, extent, for_layout_only, &[self.dir])
            } else {
                Vec::new()
            }
        });
        Ok(boxes.into_iter().reduce(|union, bbox| union.union(&bbox)))
    }

    /// Reads the state of the axes.
    fn read<R>(&self, f: impl FnOnce(&Axes3DState) -> R) -> R {
        self.axes.with(f)
    }

    /// Runs `f` on the axes, the figure's layout and the display extent of
    /// the axes, in the figure or removed from it; a default value when
    /// the axes has no place in its figure.
    fn laid_out<R: Default>(
        &self,
        f: impl FnOnce(&Axes3DState, LayoutContext<'_>, &Bbox) -> R,
    ) -> R {
        let figure = self.axes.raw().figure_state().borrow();
        let axes = figure.artist_as::<Axes3DState>(self.axes.id());
        match (axes, figure.axes_bbox(self.axes.id())) {
            (Some(axes), Some(extent)) => f(axes, LayoutContext::new(&figure), &extent),
            _ => R::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::axes::TickHelperKind;
    use crate::figure::Figure;
    use crate::mplot3d::AxisTextPart;

    #[test]
    fn the_offset_text_is_among_the_texts_and_in_the_tight_box() {
        let ax = Figure::new().subplot_3d();
        ax.set_zlim(0.1, 0.100_000_001).unwrap();
        let axis = ax.zaxis();
        assert_ne!(axis.offset_text().unwrap(), "");
        let texts = axis.texts();
        let offset = texts
            .iter()
            .find(|text| text.part == AxisTextPart::Offset)
            .expect("an offset text");
        let tight = axis.tight_bbox(false).unwrap().expect("a box");
        assert!(tight.contains(offset.extent.p0) && tight.contains(offset.extent.p1));
        assert!(axis.offset_text_visible());
    }

    #[test]
    fn helpers_replaced_in_place_stay_defaults_and_set_ones_do_not() {
        let ax = Figure::new().subplot_3d();
        let axis = ax.xaxis();
        let locator = axis.minor_locator();
        axis.set_shared_locator(false, &locator);
        assert!(!axis.is_default_locator(false));
        assert!(axis.is_default_locator(true));
        let formatter = axis.major_formatter();
        assert!(axis.replace_formatter(&formatter, &axis.minor_formatter()));
        assert!(axis.is_default_formatter(false));
        assert!(!axis.replace_formatter(&formatter, &axis.minor_formatter()));
    }

    #[test]
    fn default_flags_are_set_one_helper_at_a_time() {
        let ax = Figure::new().subplot_3d();
        let axis = ax.zaxis();
        axis.set_default_tick_helper(true, TickHelperKind::Formatter, false);
        assert!(!axis.is_default_formatter(true));
        assert!(axis.is_default_formatter(false) && axis.is_default_locator(true));
        axis.set_default_tick_helper(true, TickHelperKind::Formatter, true);
        assert!(axis.is_default_formatter(true));
    }
}
