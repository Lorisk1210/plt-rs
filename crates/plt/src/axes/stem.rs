//! Stem plots (matplotlib's `stem`): a segment from a baseline to each
//! value, a marker on its head and the baseline itself.

use super::{Axes, PlotError};
use crate::artist::ArtistHandle;
use crate::collections::Collection;
use crate::colors::Color;
use crate::lines::{FormatString, Line2D, LineProps, LineStyle};
use crate::markers::{Marker, MarkerSymbol};
use crate::orientation::Orientation;
use crate::units::AxisValues;

/// Options of [`Axes::stem`]; matplotlib's format strings parse with
/// [`str::parse`], as in `linefmt: "C2-.".parse()?`.
#[derive(Clone, Debug, PartialEq)]
pub struct StemOptions {
    /// Format of the stems (`linefmt`, `"C0-"`); only its colour and line
    /// style are used.
    pub linefmt: FormatString,
    /// Format of the heads (`markerfmt`, `"o"`); its colour defaults to the
    /// stems' colour, and `" "` hides the heads (matplotlib's
    /// `markerfmt=""`).
    pub markerfmt: FormatString,
    /// Format of the baseline (`basefmt`, `"C3-"`).
    pub basefmt: FormatString,
    /// Where the stems start.
    pub bottom: f64,
    /// Vertical stems rise from a horizontal baseline; horizontal ones
    /// extend from a vertical baseline.
    pub orientation: Orientation,
    /// Legend label of the stem container.
    pub label: Option<String>,
}

impl Default for StemOptions {
    fn default() -> Self {
        Self {
            linefmt: solid_format("C0"),
            markerfmt: FormatString {
                linestyle: Some(LineStyle::None),
                marker: Some(Marker::Symbol(MarkerSymbol::Circle)),
                color: None,
            },
            basefmt: solid_format("C3"),
            bottom: 0.0,
            orientation: Orientation::Vertical,
            label: None,
        }
    }
}

/// The format of a solid line in `color` without markers (`"C0-"`).
pub(crate) fn solid_format(color: &str) -> FormatString {
    FormatString {
        linestyle: Some(LineStyle::Solid),
        marker: Some(Marker::default()),
        color: Some(Color::from(color)),
    }
}

/// The artists of a stem plot (matplotlib's `StemContainer`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StemContainer {
    /// The heads.
    pub markerline: Line2D,
    /// The stems.
    pub stemlines: Collection,
    /// The baseline.
    pub baseline: Line2D,
    /// Legend label.
    pub label: Option<String>,
}

impl Axes {
    /// Draws a stem from `options.bottom` to each of `heads` at `locs`
    /// (matplotlib's `stem`). `locs` go along x (along y for horizontal
    /// stems) and `heads` along the other axis, numbers, dates or
    /// categories; `options.bottom` is a number of the heads' axis (see
    /// [`Axis::convert_value`](crate::axes::Axis::convert_value)).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::StemOptions;
    /// let ax = Figure::new().subplot();
    /// let stems = ax.stem([1.0, 2.0, 3.0], [0.5, -1.0, 2.0], StemOptions::default())?;
    /// assert_eq!(stems.baseline.xdata(), [1.0, 3.0]);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `locs` and `heads` differ in length.
    pub fn stem(
        &self,
        locs: impl Into<AxisValues>,
        heads: impl Into<AxisValues>,
        options: StemOptions,
    ) -> Result<StemContainer, PlotError> {
        let (loc_axis, head_axis) = match options.orientation {
            Orientation::Vertical => (self.xaxis(), self.yaxis()),
            Orientation::Horizontal => (self.yaxis(), self.xaxis()),
        };
        let (locs, heads) = (loc_axis.convert_units(locs), head_axis.convert_units(heads));
        let (locs, heads) = (locs.as_slice(), heads.as_slice());
        if locs.len() != heads.len() {
            return Err(PlotError::LengthMismatch {
                x: locs.len(),
                y: heads.len(),
            });
        }
        let (line, marker, base) = (options.linefmt, options.markerfmt, options.basefmt);
        let bottom = [options.bottom];
        let stemlines = match options.orientation {
            Orientation::Vertical => self.vlines(locs, bottom, heads)?,
            Orientation::Horizontal => self.hlines(locs, bottom, heads)?,
        };
        if let Some(color) = &line.color {
            stemlines.set_edgecolor(color.clone());
        }
        if let Some(style) = &line.linestyle {
            stemlines.set_linestyles(vec![style.clone()]);
        }
        // The parts are shown in legends through the container only.
        stemlines.set_label("_nolegend_");

        let span = [
            locs.iter().copied().fold(f64::INFINITY, f64::min),
            locs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ];
        let (head_x, head_y, base_x, base_y) = match options.orientation {
            Orientation::Vertical => (locs, heads, span, [options.bottom; 2]),
            Orientation::Horizontal => (heads, locs, [options.bottom; 2], span),
        };
        let markerline = self.plot_with(
            head_x,
            head_y,
            LineProps {
                color: marker.color.or(line.color),
                ..LineProps::from(FormatString {
                    color: None,
                    ..marker
                })
            }
            .label("_nolegend_"),
        )?;
        let baseline = self.plot_with(base_x, base_y, LineProps::from(base).label("_nolegend_"))?;
        let container = StemContainer {
            markerline,
            stemlines,
            baseline,
            label: options.label,
        };
        self.add_container(container.clone());
        Ok(container)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn default_formats_are_matplotlibs() {
        let defaults = StemOptions::default();
        let parse = |format: &str| format.parse::<FormatString>().unwrap();
        assert_eq!(defaults.linefmt, parse("C0-"));
        assert_eq!(defaults.markerfmt, parse("o"));
        assert_eq!(defaults.basefmt, parse("C3-"));
    }

    #[test]
    fn default_stems_follow_matplotlib() {
        let ax = Figure::new().subplot();
        let stems = ax
            .stem([0.0, 1.0, 2.0], [1.0, -2.0, 3.0], StemOptions::default())
            .unwrap();
        assert_eq!(
            stems.markerline.marker(),
            Marker::Symbol(MarkerSymbol::Circle)
        );
        assert_eq!(stems.markerline.linestyle(), LineStyle::None);
        assert_eq!(stems.markerline.color(), Color::from("C0"));
        assert_eq!(stems.baseline.color(), Color::from("C3"));
        assert_eq!(stems.baseline.ydata(), [0.0, 0.0]);
        assert_eq!(ax.ylim(), (-2.25, 3.25));
    }

    #[test]
    fn horizontal_stems_and_formats() {
        let ax = Figure::new().subplot();
        let options = StemOptions {
            markerfmt: "rD".parse().unwrap(),
            bottom: 1.0,
            orientation: Orientation::Horizontal,
            ..StemOptions::default()
        };
        let stems = ax.stem([0.0, 5.0], [2.0, 3.0], options).unwrap();
        assert_eq!(stems.markerline.xdata(), [2.0, 3.0]);
        assert_eq!(stems.markerline.color(), Color::from("r"));
        assert_eq!(stems.baseline.xdata(), [1.0, 1.0]);
        assert_eq!(stems.baseline.ydata(), [0.0, 5.0]);
    }

    #[test]
    fn blank_marker_format_hides_heads() {
        let ax = Figure::new().subplot();
        let options = StemOptions {
            markerfmt: " ".parse().unwrap(),
            ..StemOptions::default()
        };
        let stems = ax.stem([0.0], [1.0], options).unwrap();
        assert_eq!(stems.markerline.marker(), Marker::default());
        assert_eq!(stems.markerline.linestyle(), LineStyle::None);
    }

    #[test]
    fn mismatched_lengths_are_rejected() {
        let ax = Figure::new().subplot();
        assert!(ax.stem([0.0], [1.0, 2.0], StemOptions::default()).is_err());
    }
}
