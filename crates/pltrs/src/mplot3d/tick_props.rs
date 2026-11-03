//! What single ticks of a 3-D axis have of their own (matplotlib's `Tick`
//! artists of an `axis3d.Axis` changed one by one, which `Axis3D.draw`
//! draws as they are but for the width of their marks): the properties
//! of their labels and marks, and the pads of the minor ticks; and
//! whether the offset text is shown apart from the tick labels.

use super::axis_handle::Axis3D;
use crate::axes::{TickLabelProps, TickLineProps, TickStyle};

impl Axis3D {
    /// Replaces the text properties of the label on the first or the
    /// second side of the major (or `minor`) tick at `index` among those
    /// the locator places with `props`, unset fields following the labels
    /// of its level; the ticks the locator adds later copy those of the
    /// first one, as matplotlib's do.
    ///
    /// ```
    /// use pltrs::axes::TickLabelProps;
    /// use pltrs::colors::Color;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let red = TickLabelProps { color: Some(Color::from("red")), ..TickLabelProps::default() };
    /// ax.xaxis().set_tick_label_props_on(false, 1, true, red.clone());
    /// assert_eq!(ax.xaxis().tick_label_props_on(false, 1, true), red);
    /// assert_eq!(ax.xaxis().tick_style_at(false, 1, true).label_color, Color::from("red"));
    /// assert_ne!(ax.xaxis().tick_style_at(false, 0, true).label_color, Color::from("red"));
    /// ```
    pub fn set_tick_label_props_on(
        &self,
        minor: bool,
        index: usize,
        first_side: bool,
        props: TickLabelProps,
    ) -> &Self {
        let props = props.with_absolute_size(crate::rc::params().font_size);
        let dir = self.dir;
        self.axes.with_mut(|axes| {
            (axes.axis_mut(dir).overrides_mut(minor)).change_label(index, first_side, |_| props);
        });
        self
    }

    /// The text properties the label on the first or the second side of
    /// the major (or `minor`) tick at `index` has of its own.
    pub fn tick_label_props_on(
        &self,
        minor: bool,
        index: usize,
        first_side: bool,
    ) -> TickLabelProps {
        self.axes.with(|axes| {
            axes.axis(self.dir)
                .overrides(minor)
                .label(index, first_side)
        })
    }

    /// Replaces the properties of the mark of the major (or `minor`) tick
    /// at `index` with `props` (its colour; a 3-D axis draws every mark
    /// as wide as its level says, and no gridline of a tick).
    pub fn set_tick_line_props(&self, minor: bool, index: usize, props: TickLineProps) -> &Self {
        let dir = self.dir;
        self.axes.with_mut(|axes| {
            let lines = &mut axes.axis_mut(dir).overrides_mut(minor).lines;
            if lines.len() <= index {
                lines.resize_with(index + 1, TickLineProps::default);
            }
            lines[index] = props;
        });
        self
    }

    /// The properties the mark of the major (or `minor`) tick at `index`
    /// has of its own.
    pub fn tick_line_props(&self, minor: bool, index: usize) -> TickLineProps {
        self.axes.with(|axes| {
            (axes.axis(self.dir).overrides(minor).lines.get(index))
                .cloned()
                .unwrap_or_default()
        })
    }

    /// The style the label on the first or the second side of the major
    /// (or `minor`) tick at `index` is drawn with, its mark and its pad:
    /// the style of its level with what the tick has of its own.
    pub fn tick_style_at(&self, minor: bool, index: usize, first_side: bool) -> TickStyle {
        self.axes.with(|axes| {
            (axes.axis(self.dir))
                .tick_style_at(minor, index, first_side)
                .into_owned()
        })
    }

    /// Gives the minor tick at `index` among those the locator places its
    /// own label pad, in points (matplotlib's `tick.set_pad`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.xaxis().set_minor_tick_pad(0, 9.0);
    /// assert_eq!(ax.xaxis().tick_pad(true, 0), 9.0);
    /// ```
    pub fn set_minor_tick_pad(&self, index: usize, pad: f64) -> &Self {
        self.set_level_tick_pad(true, index, pad)
    }

    /// Gives the major (or `minor`) tick at `index` its own label pad.
    pub(super) fn set_level_tick_pad(&self, minor: bool, index: usize, pad: f64) -> &Self {
        let dir = self.dir;
        self.axes.with_mut(|axes| {
            let pads = &mut axes.axis_mut(dir).overrides_mut(minor).pads;
            if pads.len() <= index {
                pads.resize(index + 1, None);
            }
            pads[index] = Some(pad);
        });
        self
    }

    /// Whether minor ticks falling on major ones are dropped (matplotlib's
    /// `remove_overlapping_locs`, on by default).
    pub fn remove_overlapping_locs(&self) -> bool {
        self.axes
            .with(|axes| axes.axis(self.dir).remove_overlapping)
    }

    /// Drops minor ticks falling on major ones, or keeps them.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.xaxis().set_remove_overlapping_locs(false);
    /// assert!(!ax.xaxis().remove_overlapping_locs());
    /// ```
    pub fn set_remove_overlapping_locs(&self, remove: bool) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).remove_overlapping = remove);
        self
    }

    /// Shows or hides the offset text whatever the major tick labels do
    /// (matplotlib's `offsetText.set_visible`); `None` shows it with them.
    pub fn set_offset_text_shown(&self, shown: Option<bool>) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).offset_shown = shown);
        self
    }
}
