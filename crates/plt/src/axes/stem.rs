//! Stem plots (matplotlib's `stem`): a segment from a baseline to each
//! value, a marker on its head and the baseline itself.

use super::{Axes, PlotError};
use crate::artist::ArtistHandle;
use crate::collections::Collection;
use crate::lines::{FormatString, Line2D, LineProps};
use crate::orientation::Orientation;
use crate::rc::RcParams;

/// Options of [`Axes::stem`].
#[derive(Clone, Debug, PartialEq)]
pub struct StemOptions {
    /// Format string of the stems (`linefmt`); only its colour and line
    /// style are used.
    pub linefmt: String,
    /// Format string of the heads (`markerfmt`); its colour defaults to
    /// the stems' colour and an empty string hides the heads.
    pub markerfmt: String,
    /// Format string of the baseline (`basefmt`).
    pub basefmt: String,
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
            linefmt: "C0-".into(),
            markerfmt: "o".into(),
            basefmt: "C3-".into(),
            bottom: 0.0,
            orientation: Orientation::Vertical,
            label: None,
        }
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
    /// (matplotlib's `stem`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::StemOptions;
    /// let ax = Figure::new().subplot();
    /// let stems = ax.stem([1.0, 2.0, 3.0], [0.5, -1.0, 2.0], &StemOptions::default())?;
    /// assert_eq!(stems.baseline.xdata(), [1.0, 3.0]);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `locs` and `heads` differ in length or a format string is
    /// invalid.
    pub fn stem(
        &self,
        locs: impl AsRef<[f64]>,
        heads: impl AsRef<[f64]>,
        options: &StemOptions,
    ) -> Result<StemContainer, PlotError> {
        let (locs, heads) = (locs.as_ref(), heads.as_ref());
        if locs.len() != heads.len() {
            return Err(PlotError::LengthMismatch {
                x: locs.len(),
                y: heads.len(),
            });
        }
        let rc = crate::rc::params();
        let line = FormatString::parse_with(&options.linefmt, &rc)?;
        let marker = marker_format(&options.markerfmt, &rc)?;
        let base = FormatString::parse_with(&options.basefmt, &rc)?;

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
            label: options.label.clone(),
        };
        self.add_container(container.clone());
        Ok(container)
    }
}

/// The heads' format: an empty string means no heads, which matplotlib
/// spells `' '` (an invisible line style without marker).
fn marker_format(format: &str, rc: &RcParams) -> Result<FormatString, PlotError> {
    let format = if format.is_empty() { " " } else { format };
    Ok(FormatString::parse_with(format, rc)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Color;
    use crate::figure::Figure;
    use crate::lines::LineStyle;
    use crate::markers::{Marker, MarkerSymbol};

    #[test]
    fn default_stems_follow_matplotlib() {
        let ax = Figure::new().subplot();
        let stems = ax
            .stem([0.0, 1.0, 2.0], [1.0, -2.0, 3.0], &StemOptions::default())
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
            markerfmt: "rD".into(),
            bottom: 1.0,
            orientation: Orientation::Horizontal,
            ..StemOptions::default()
        };
        let stems = ax.stem([0.0, 5.0], [2.0, 3.0], &options).unwrap();
        assert_eq!(stems.markerline.xdata(), [2.0, 3.0]);
        assert_eq!(stems.markerline.color(), Color::from("r"));
        assert_eq!(stems.baseline.xdata(), [1.0, 1.0]);
        assert_eq!(stems.baseline.ydata(), [0.0, 5.0]);
    }

    #[test]
    fn empty_marker_format_hides_heads() {
        let ax = Figure::new().subplot();
        let options = StemOptions {
            markerfmt: String::new(),
            ..StemOptions::default()
        };
        let stems = ax.stem([0.0], [1.0], &options).unwrap();
        assert_eq!(stems.markerline.marker(), Marker::default());
        assert_eq!(stems.markerline.linestyle(), LineStyle::None);
    }

    #[test]
    fn mismatched_lengths_are_rejected() {
        let ax = Figure::new().subplot();
        assert!(ax.stem([0.0], [1.0, 2.0], &StemOptions::default()).is_err());
    }
}
