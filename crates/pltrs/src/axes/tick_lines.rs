//! The marks and gridlines of the ticks of an axis (matplotlib's
//! `tick1line`, `tick2line` and `gridline` of the ticks, `get_ticklines`
//! and `get_gridlines`), and the look of those of a single tick.

use std::borrow::Cow;

use super::axis::AxisState;
use super::axis_handle::Axis;
use super::tick_set::Tick;
use super::ticks::{GridStyle, TickDirection, TickStyle};
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::spines::SpineSide;
use crate::ticker::TickerError;

/// A tick mark of an axis (matplotlib's `tick1line` or `tick2line`).
#[derive(Clone, Debug, PartialEq)]
pub struct TickLine {
    /// The tick's location along the axis, in data coordinates.
    pub loc: f64,
    /// The side the mark is on.
    pub side: SpineSide,
    /// Whether the side has marks (matplotlib's `get_visible`).
    pub visible: bool,
    /// Whether the tick is in view; visible marks are drawn only then.
    pub in_view: bool,
    /// Length in points.
    pub size: f64,
    /// Line width in points.
    pub width: f64,
    /// Which way the mark points.
    pub direction: TickDirection,
    /// The colour of the mark (matplotlib's `get_color`).
    pub color: Color,
    /// The colour its edge is drawn in (matplotlib's
    /// `get_markeredgecolor`).
    pub edgecolor: Color,
}

/// The gridline of a major tick (matplotlib's `gridline`).
#[derive(Clone, Debug, PartialEq)]
pub struct GridLine {
    /// The tick's location along the axis, in data coordinates.
    pub loc: f64,
    /// Whether gridlines are on (matplotlib's `get_visible`).
    pub visible: bool,
    /// Whether the tick is in view; a visible gridline is drawn only then.
    pub in_view: bool,
    /// How the line is drawn.
    pub style: GridStyle,
}

/// The look of one tick's marks and gridline on top of the style of its
/// level; unset fields keep it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickLineProps {
    /// Edge colour of the tick marks, the colour of matplotlib's line-like
    /// tick markers (`tick1line.set_markeredgecolor(...)`).
    pub markeredgecolor: Option<Color>,
    /// Colour of the tick marks (`tick1line.set_color(...)`). Their edges,
    /// all a mark draws, keep the colour they had unless `markeredgecolor`
    /// is set, as matplotlib's tick lines are made with an edge colour of
    /// their own.
    pub color: Option<Color>,
    /// Length of the tick marks in points (`set_markersize`).
    pub size: Option<f64>,
    /// Line width of the tick marks in points (`set_markeredgewidth`).
    pub width: Option<f64>,
    /// Whether the mark on the first side (bottom or left) is drawn
    /// (`tick1line.set_visible(...)`, `tick1On`).
    pub tick1_visible: Option<bool>,
    /// Whether the mark on the second side (top or right) is drawn
    /// (`tick2line.set_visible(...)`, `tick2On`).
    pub tick2_visible: Option<bool>,
    /// Whether the gridline is drawn (`gridline.set_visible(...)`).
    pub grid_visible: Option<bool>,
    /// Gridline colour.
    pub grid_color: Option<Color>,
    /// Gridline width in points.
    pub grid_linewidth: Option<f64>,
    /// Gridline style.
    pub grid_linestyle: Option<LineStyle>,
    /// Gridline opacity.
    pub grid_alpha: Option<f64>,
}

pltrs_core::archive_struct!(TickLineProps {
    markeredgecolor,
    color,
    size,
    width,
    tick1_visible,
    tick2_visible,
    grid_visible,
    grid_color,
    grid_linewidth,
    grid_linestyle,
    grid_alpha
});

impl TickLineProps {
    /// `self` with every property set in `newer` replaced.
    fn overlay(&self, newer: &Self) -> Self {
        Self {
            markeredgecolor: newer
                .markeredgecolor
                .clone()
                .or_else(|| self.markeredgecolor.clone()),
            color: newer.color.clone().or_else(|| self.color.clone()),
            size: newer.size.or(self.size),
            width: newer.width.or(self.width),
            tick1_visible: newer.tick1_visible.or(self.tick1_visible),
            tick2_visible: newer.tick2_visible.or(self.tick2_visible),
            grid_visible: newer.grid_visible.or(self.grid_visible),
            grid_color: newer.grid_color.clone().or_else(|| self.grid_color.clone()),
            grid_linewidth: newer.grid_linewidth.or(self.grid_linewidth),
            grid_linestyle: newer
                .grid_linestyle
                .clone()
                .or_else(|| self.grid_linestyle.clone()),
            grid_alpha: newer.grid_alpha.or(self.grid_alpha),
        }
    }

    /// Sets the mark and gridline properties of `style` that are set here.
    pub(crate) fn apply(&self, style: &mut TickStyle) {
        if let Some(color) = &self.color {
            if style.edgecolor.is_none() {
                style.edgecolor = Some(style.color.clone());
            }
            style.color = color.clone();
        }
        if self.markeredgecolor.is_some() {
            style.edgecolor.clone_from(&self.markeredgecolor);
        }
        if let Some(size) = self.size {
            style.size = size;
        }
        if let Some(width) = self.width {
            style.width = width;
        }
        if let Some(visible) = self.tick1_visible {
            style.ticks.first = visible;
        }
        if let Some(visible) = self.tick2_visible {
            style.ticks.second = visible;
        }
        if let Some(visible) = self.grid_visible {
            style.grid_on = visible;
        }
        if let Some(color) = &self.grid_color {
            style.grid.color = color.clone();
        }
        if let Some(width) = self.grid_linewidth {
            style.grid.linewidth = width;
        }
        if let Some(linestyle) = &self.grid_linestyle {
            style.grid.linestyle = linestyle.clone();
        }
        if let Some(alpha) = self.grid_alpha {
            style.grid.alpha = alpha;
        }
    }
}

impl Axis {
    /// The tick marks of every major (or minor) tick the locator places,
    /// in view or not: those of the first tick on the first then the
    /// second side, and so on (matplotlib's `get_ticklines`,
    /// `get_majorticklines` and `get_minorticklines`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// ax.set_xticks([0.0, 0.5, 1.0]);
    /// let marks = ax.xaxis().ticklines(false)?;
    /// assert_eq!(marks.len(), 6);
    /// assert_eq!((marks[0].side, marks[0].visible), (SpineSide::Bottom, true));
    /// assert_eq!((marks[1].side, marks[1].visible), (SpineSide::Top, false));
    /// # Ok::<(), pltrs::ticker::TickerError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the locator fails.
    pub fn ticklines(&self, minor: bool) -> Result<Vec<TickLine>, TickerError> {
        let ticks = self.axes.ticks(self.name)?;
        let (unclipped, in_view) = if minor {
            (&ticks.minor_unclipped, &ticks.minor_index)
        } else {
            (&ticks.major_unclipped, &ticks.major_index)
        };
        let name = self.name;
        Ok(self.read(|axis| {
            styled(axis, unclipped, in_view, minor)
                .flat_map(|(tick, in_view, style)| {
                    [false, true].map(|high| TickLine {
                        loc: tick.loc,
                        side: SpineSide::of(name, high),
                        visible: if high {
                            style.ticks.second
                        } else {
                            style.ticks.first
                        },
                        in_view,
                        size: style.size,
                        width: style.width,
                        direction: style.direction,
                        color: style.color.clone(),
                        edgecolor: style
                            .edgecolor
                            .clone()
                            .unwrap_or_else(|| style.color.clone()),
                    })
                })
                .collect()
        }))
    }

    /// The gridlines of every major tick the locator places, in view or
    /// not (matplotlib's `get_gridlines`).
    ///
    /// # Errors
    ///
    /// When the locator fails.
    pub fn gridlines(&self) -> Result<Vec<GridLine>, TickerError> {
        let ticks = self.axes.ticks(self.name)?;
        Ok(self.read(|axis| {
            styled(axis, &ticks.major_unclipped, &ticks.major_index, false)
                .map(|(tick, in_view, style)| GridLine {
                    loc: tick.loc,
                    visible: style.grid_on,
                    in_view,
                    style: style.grid.clone(),
                })
                .collect()
        }))
    }

    /// Colours the tick lines of both levels without their marks' edges
    /// (matplotlib's `set_color` on the `Line2D`s of the ticks, which
    /// `Artist.findobj` reaches): the marks are open paths whose edge
    /// keeps its colour, so only the face, seen in vector output, changes.
    pub fn set_tick_line_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|axis| {
            axis.restyle_ticks(|style| {
                style.edgecolor.get_or_insert_with(|| style.color.clone());
                style.color = color.clone();
            });
        });
        self
    }

    /// Changes the marks and gridline of the major tick at `index` among
    /// those the major locator places, in view or not, on top of the style
    /// of the major ticks (matplotlib's `tick.tick1line.set(...)` and
    /// `tick.gridline.set(...)` on `axis.get_major_ticks()[index]`).
    ///
    /// ```
    /// use pltrs::axes::TickLineProps;
    /// use pltrs::figure::Figure;
    /// use pltrs::lines::LineStyle;
    /// let ax = Figure::new().subplot();
    /// ax.set_xticks([0.0, 0.5, 1.0]);
    /// let dotted = TickLineProps {
    ///     grid_visible: Some(true),
    ///     grid_linestyle: Some(LineStyle::Dotted),
    ///     ..TickLineProps::default()
    /// };
    /// ax.xaxis().set_major_tick_line_props(1, dotted);
    /// ```
    pub fn set_major_tick_line_props(&self, index: usize, props: TickLineProps) -> &Self {
        self.change_tick_line_props(false, index, |old| old.overlay(&props))
    }

    /// Replaces the look of the marks and gridline of the major (or
    /// `minor`) tick at `index` with `props`, unset fields following the
    /// style of its level (what matplotlib's `tick1line`, `tick2line` and
    /// `gridline` keep of their own once changed one by one).
    pub fn set_tick_line_props(&self, minor: bool, index: usize, props: TickLineProps) -> &Self {
        self.change_tick_line_props(minor, index, |_| props)
    }

    /// Sets the mark and gridline properties of the major (or `minor`)
    /// tick at `index` to `change(old)`.
    fn change_tick_line_props(
        &self,
        minor: bool,
        index: usize,
        change: impl FnOnce(&TickLineProps) -> TickLineProps,
    ) -> &Self {
        self.update(|axis| {
            let lines = &mut axis.overrides_mut(minor).lines;
            if lines.len() <= index {
                lines.resize_with(index + 1, TickLineProps::default);
            }
            lines[index] = change(&lines[index]);
        });
        self
    }

    /// The look given to the marks and gridline of the major tick at
    /// `index` with [`set_major_tick_line_props`](Self::set_major_tick_line_props),
    /// unset fields following the style of the major ticks.
    pub fn major_tick_line_props(&self, index: usize) -> TickLineProps {
        self.tick_line_props(false, index)
    }

    /// The look of the marks and gridline of the major (or `minor`) tick
    /// at `index` of their own (see
    /// [`set_tick_line_props`](Self::set_tick_line_props)).
    pub fn tick_line_props(&self, minor: bool, index: usize) -> TickLineProps {
        self.read(|axis| {
            (axis.overrides(minor).lines.get(index))
                .cloned()
                .unwrap_or_default()
        })
    }
}

/// Each of `ticks` with whether it is in view (its index is among
/// `in_view`) and its style: that of its level, with the overrides of its
/// index.
fn styled<'a>(
    axis: &'a AxisState,
    ticks: &'a [Tick],
    in_view: &'a [usize],
    minor: bool,
) -> impl Iterator<Item = (&'a Tick, bool, Cow<'a, TickStyle>)> {
    ticks.iter().enumerate().map(move |(index, tick)| {
        let style = axis.tick_style_at(minor, index);
        (tick, in_view.contains(&index), style)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::AxesState;
    use crate::figure::Figure;

    #[test]
    fn tick_lines_and_gridlines_cover_every_located_tick() {
        let ax = Figure::new().subplot();
        ax.set_xticks([0.0, 0.5, 1.0])
            .set_xlim(0.25, 1.0)
            .expect("finite limits");
        ax.xaxis().set_major_tick_line_props(
            2,
            TickLineProps {
                grid_visible: Some(true),
                markeredgecolor: Some(Color::from("red")),
                ..TickLineProps::default()
            },
        );
        let marks = ax.xaxis().ticklines(false).expect("ticks");
        let summary: Vec<(f64, SpineSide, bool, bool)> = marks
            .iter()
            .map(|mark| (mark.loc, mark.side, mark.visible, mark.in_view))
            .collect();
        assert_eq!(
            summary,
            [
                (0.0, SpineSide::Bottom, true, false),
                (0.0, SpineSide::Top, false, false),
                (0.5, SpineSide::Bottom, true, true),
                (0.5, SpineSide::Top, false, true),
                (1.0, SpineSide::Bottom, true, true),
                (1.0, SpineSide::Top, false, true),
            ]
        );
        assert_eq!(marks[0].edgecolor, marks[0].color);
        assert_eq!(marks[4].edgecolor, Color::from("red"));
        let grid: Vec<(f64, bool)> = (ax.xaxis().gridlines().expect("ticks").iter())
            .map(|line| (line.loc, line.visible))
            .collect();
        assert_eq!(grid, [(0.0, false), (0.5, false), (1.0, true)]);
        assert_eq!(ax.xaxis().ticklines(true), Ok(Vec::new()), "no minor ticks");
    }

    #[test]
    fn one_tick_gets_its_own_mark_colour_and_gridline() {
        let ax = Figure::new().subplot();
        ax.set_xticks([0.0, 0.5, 1.0]);
        let red = TickLineProps {
            markeredgecolor: Some(Color::from("red")),
            grid_visible: Some(true),
            grid_linewidth: Some(1.5),
            ..TickLineProps::default()
        };
        let xaxis = ax.xaxis();
        xaxis.set_major_tick_line_props(1, red);
        xaxis.set_major_tick_line_props(
            1,
            TickLineProps {
                grid_linestyle: Some(LineStyle::Dotted),
                ..TickLineProps::default()
            },
        );
        let (plain, styled) = ax.raw().with::<AxesState, _>(|axes| {
            (
                axes.xaxis.tick_style_at(false, 0).into_owned(),
                axes.xaxis.tick_style_at(false, 1).into_owned(),
            )
        });
        assert_eq!(
            plain,
            ax.raw()
                .with::<AxesState, _>(|a| a.xaxis.major_style.clone())
        );
        assert_eq!(styled.edgecolor, Some(Color::from("red")));
        let props = xaxis.major_tick_line_props(1);
        assert_eq!(props.grid_linewidth, Some(1.5));
        assert_eq!(props.grid_linestyle, Some(LineStyle::Dotted));
        assert_eq!(xaxis.major_tick_line_props(7), TickLineProps::default());
        assert!(styled.grid_on && !plain.grid_on);
        assert_eq!(styled.grid.linewidth, 1.5);
        assert_eq!(
            styled.grid.linestyle,
            LineStyle::Dotted,
            "later calls add up"
        );
    }
}
