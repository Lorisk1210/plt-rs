//! The marks and gridline of a single major tick (matplotlib's
//! `tick1line`, `tick2line` and `gridline` of one of
//! `axis.get_major_ticks()`).

use super::axis_handle::Axis;
use super::ticks::TickStyle;
use crate::colors::Color;
use crate::lines::LineStyle;

/// The look of one major tick's marks and gridline on top of the style of
/// the major ticks; unset fields keep it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickLineProps {
    /// Edge colour of the tick marks, the colour of matplotlib's line-like
    /// tick markers (`tick1line.set_markeredgecolor(...)`).
    pub markeredgecolor: Option<Color>,
    /// Whether the gridline is drawn (`gridline.set_visible(...)`).
    pub grid_visible: Option<bool>,
    /// Gridline colour.
    pub grid_color: Option<Color>,
    /// Gridline width in points.
    pub grid_linewidth: Option<f64>,
    /// Gridline style.
    pub grid_linestyle: Option<LineStyle>,
}

impl TickLineProps {
    /// `self` with every property set in `newer` replaced.
    fn overlay(&self, newer: &Self) -> Self {
        Self {
            markeredgecolor: newer
                .markeredgecolor
                .clone()
                .or_else(|| self.markeredgecolor.clone()),
            grid_visible: newer.grid_visible.or(self.grid_visible),
            grid_color: newer.grid_color.clone().or_else(|| self.grid_color.clone()),
            grid_linewidth: newer.grid_linewidth.or(self.grid_linewidth),
            grid_linestyle: newer
                .grid_linestyle
                .clone()
                .or_else(|| self.grid_linestyle.clone()),
        }
    }

    /// Sets the mark and gridline properties of `style` that are set here.
    pub(crate) fn apply(&self, style: &mut TickStyle) {
        if self.markeredgecolor.is_some() {
            style.edgecolor.clone_from(&self.markeredgecolor);
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
    }
}

impl Axis {
    /// Changes the marks and gridline of the major tick at `index` among
    /// those the major locator places, in view or not, on top of the style
    /// of the major ticks (matplotlib's `tick.tick1line.set(...)` and
    /// `tick.gridline.set(...)` on `axis.get_major_ticks()[index]`).
    ///
    /// ```
    /// use plt::axes::TickLineProps;
    /// use plt::figure::Figure;
    /// use plt::lines::LineStyle;
    /// let ax = Figure::new().subplot();
    /// ax.set_xticks([0.0, 0.5, 1.0]);
    /// let dotted = TickLineProps {
    ///     grid_visible: Some(true),
    ///     grid_linestyle: Some(LineStyle::Dotted),
    ///     ..TickLineProps::default()
    /// };
    /// ax.xaxis().set_major_tick_line_props(1, &dotted);
    /// ```
    pub fn set_major_tick_line_props(&self, index: usize, props: &TickLineProps) -> &Self {
        self.update(|axis| {
            let overrides = &mut axis.major_line_overrides;
            if overrides.len() <= index {
                overrides.resize_with(index + 1, TickLineProps::default);
            }
            overrides[index] = overrides[index].overlay(props);
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::AxesState;
    use crate::figure::Figure;

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
        xaxis.set_major_tick_line_props(1, &red);
        xaxis.set_major_tick_line_props(
            1,
            &TickLineProps {
                grid_linestyle: Some(LineStyle::Dotted),
                ..TickLineProps::default()
            },
        );
        let (plain, styled) = ax.raw().with::<AxesState, _>(|axes| {
            (
                axes.xaxis.major_tick_style(0).into_owned(),
                axes.xaxis.major_tick_style(1).into_owned(),
            )
        });
        assert_eq!(
            plain,
            ax.raw()
                .with::<AxesState, _>(|a| a.xaxis.major_style.clone())
        );
        assert_eq!(styled.edgecolor, Some(Color::from("red")));
        assert!(styled.grid_on && !plain.grid_on);
        assert_eq!(styled.grid.linewidth, 1.5);
        assert_eq!(
            styled.grid.linestyle,
            LineStyle::Dotted,
            "later calls add up"
        );
    }
}
