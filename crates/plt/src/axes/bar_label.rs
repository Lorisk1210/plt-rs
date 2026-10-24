//! Labels on bars (matplotlib's `bar_label`).

use std::fmt;
use std::rc::Rc;

use plt_core::Point;

use super::{Axes, BarContainer};
use crate::artist::ArtistHandle;
use crate::coords::{CoordError, CoordOrigin, CoordUnit, Coords};
use crate::orientation::Orientation;
use crate::text::{AnnotateOptions, Annotation, HAlign, VAlign};
use crate::ticker::pyfmt::{FormatError, auto_format_value};

/// Where bar labels sit (matplotlib's `label_type`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BarLabelType {
    /// Beyond the end of each bar (or of its error bar), showing the end
    /// position.
    #[default]
    Edge,
    /// In the middle of each bar, showing its length.
    Center,
}

/// How bar values become label texts (matplotlib's `fmt`).
#[derive(Clone)]
pub enum BarLabelFormat {
    /// A `%`-style or `{}`-style template; `%` takes precedence (default
    /// `"%g"`).
    Template(String),
    /// A function of the value.
    Func(Rc<dyn Fn(f64) -> String>),
}

impl Default for BarLabelFormat {
    fn default() -> Self {
        Self::Template("%g".to_owned())
    }
}

impl fmt::Debug for BarLabelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Template(template) => f.debug_tuple("Template").field(template).finish(),
            Self::Func(_) => f.write_str("Func(..)"),
        }
    }
}

/// Options of [`Axes::bar_label`]; style the returned annotations for
/// font size, colour, rotation and other text properties.
#[derive(Clone, Debug, Default)]
pub struct BarLabelOptions {
    /// Label texts, one per bar; bars beyond the list show their
    /// formatted value.
    pub labels: Vec<String>,
    /// How values are formatted.
    pub fmt: BarLabelFormat,
    /// Where the labels sit.
    pub label_type: BarLabelType,
    /// Distance from the bar end in points: one for all bars or one per
    /// bar (empty: 0).
    pub padding: Vec<f64>,
}

/// Why bars cannot be labelled.
#[derive(Clone, Debug, PartialEq)]
pub enum BarLabelError {
    /// Paddings neither one nor one per bar.
    PaddingCount {
        /// Paddings given.
        len: usize,
        /// Number of bars.
        bars: usize,
    },
    /// The format template accepts no value.
    Format(FormatError),
    /// A label cannot be placed.
    Coords(CoordError),
}

impl fmt::Display for BarLabelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PaddingCount { bars, .. } => {
                write!(
                    f,
                    "padding must be of length {bars} when passed as a sequence"
                )
            }
            Self::Format(err) => err.fmt(f),
            Self::Coords(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for BarLabelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PaddingCount { .. } => None,
            Self::Format(err) => Some(err),
            Self::Coords(err) => Some(err),
        }
    }
}

/// `1` for non-negative values, `-1` otherwise (NaN included), as
/// matplotlib's `sign` in `bar_label`.
fn sign(value: f64) -> f64 {
    if value >= 0.0 { 1.0 } else { -1.0 }
}

impl Axes {
    /// Labels the bars of `container` with their values (or
    /// `options.labels`), placed at the bar ends or centres (matplotlib's
    /// `bar_label`). Bars with a NaN value get an empty label.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::{BarLabelOptions, BarOptions};
    /// let ax = Figure::new().subplot();
    /// let bars = ax.bar([0.0, 1.0], [3.0, 4.5], BarOptions::default())?;
    /// let labels = ax.bar_label(&bars, BarLabelOptions::default())?;
    /// assert_eq!(labels[1].text(), "4.5");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the paddings do not match the bars or the format template
    /// accepts no value.
    pub fn bar_label(
        &self,
        container: &BarContainer,
        options: BarLabelOptions,
    ) -> Result<Vec<Annotation>, BarLabelError> {
        let (y0, y1) = self.ylim();
        let (x0, x1) = self.xlim();
        let inverted = match container.orientation {
            Orientation::Vertical => y0 > y1,
            Orientation::Horizontal => x0 > x1,
        };
        let bars = container.patches.len();
        let padding = match options.padding.as_slice() {
            [] => vec![0.0; bars],
            &[pad] => vec![pad; bars],
            pads if pads.len() == bars => pads.to_vec(),
            pads => {
                return Err(BarLabelError::PaddingCount {
                    len: pads.len(),
                    bars,
                });
            }
        };
        let error_ends = error_bar_ends(container);
        // Whole values format as Python integers, as matplotlib's integer
        // arrays of bar heights do.
        let whole = container
            .datavalues
            .iter()
            .all(|v| v.is_finite() && v.fract() == 0.0 && v.abs() < 2f64.powi(63));
        container
            .patches
            .iter()
            .zip(&container.datavalues)
            .zip(padding)
            .enumerate()
            .map(|(i, ((bar, &value), pad))| {
                let rect = bar.shape();
                let (start, end) = match container.orientation {
                    Orientation::Vertical => (rect.xy.y, rect.xy.y + rect.height),
                    Orientation::Horizontal => (rect.xy.x, rect.xy.x + rect.width),
                };
                let extremum = if value >= 0.0 {
                    start.max(end)
                } else {
                    start.min(end)
                };
                let end = match error_ends.get(i).and_then(Option::as_ref) {
                    Some(&(_, high)) if value >= 0.0 => high,
                    Some(&(low, _)) => low,
                    None => extremum,
                };
                let placement = LabelPlacement {
                    orientation: container.orientation,
                    label_type: options.label_type,
                    inverted,
                    value,
                };
                let text = match options.labels.get(i) {
                    _ if value.is_nan() => String::new(),
                    Some(label) => label.clone(),
                    None => format_value(&options.fmt, value, whole)?,
                };
                let center = (
                    (rect.xy.x + (rect.xy.x + rect.width)) / 2.0,
                    (rect.xy.y + (rect.xy.y + rect.height)) / 2.0,
                );
                let (xy, xycoords) = match options.label_type {
                    BarLabelType::Center => (Point::new(0.5, 0.5), Coords::ClippedArtist(bar.id())),
                    BarLabelType::Edge => (placement.edge_point(center, end), Coords::Data),
                };
                let note = self
                    .annotate(
                        text,
                        xy,
                        AnnotateOptions {
                            xytext: Some(placement.offset(pad)),
                            xycoords,
                            textcoords: Some(Coords::Relative {
                                origin: CoordOrigin::Offset,
                                unit: CoordUnit::Points,
                            }),
                            ..AnnotateOptions::default()
                        },
                    )
                    .map_err(BarLabelError::Coords)?;
                let (halign, valign) = placement.alignment();
                note.set_halign(halign).set_valign(valign);
                Ok(note)
            })
            .collect()
    }
}

/// The value-direction extent `(low, high)` of each bar's error bar,
/// if it has one (matplotlib reads the first bar-line collection).
fn error_bar_ends(container: &BarContainer) -> Vec<Option<(f64, f64)>> {
    let Some(lines) = container
        .errorbar
        .as_ref()
        .and_then(|errorbar| errorbar.barlinecols.first())
    else {
        return Vec::new();
    };
    lines
        .paths()
        .iter()
        .map(|segment| {
            let values = segment
                .vertices()
                .iter()
                .map(|p| match container.orientation {
                    Orientation::Vertical => p.y,
                    Orientation::Horizontal => p.x,
                });
            let low = values.clone().fold(f64::INFINITY, f64::min);
            let high = values.fold(f64::NEG_INFINITY, f64::max);
            (low <= high).then_some((low, high))
        })
        .collect()
}

/// What decides where one label goes and how it is aligned.
struct LabelPlacement {
    orientation: Orientation,
    label_type: BarLabelType,
    inverted: bool,
    value: f64,
}

impl LabelPlacement {
    /// The labelled point of an edge label: the bar's centre across, its
    /// end along.
    fn edge_point(&self, (xc, yc): (f64, f64), end: f64) -> Point {
        match self.orientation {
            Orientation::Vertical => Point::new(xc, end),
            Orientation::Horizontal => Point::new(end, yc),
        }
    }

    /// The text offset in points: `pad` away from the bar, in the
    /// direction the bar grows on screen.
    fn offset(&self, pad: f64) -> Point {
        let direction = if self.inverted { -1.0 } else { 1.0 };
        let along = direction * sign(self.value) * pad;
        match self.orientation {
            Orientation::Vertical => Point::new(0.0, along),
            Orientation::Horizontal => Point::new(along, 0.0),
        }
    }

    /// Edge labels align away from the bar; centre labels are centred.
    fn alignment(&self) -> (HAlign, VAlign) {
        if self.label_type == BarLabelType::Center {
            return (HAlign::Center, VAlign::Center);
        }
        // NaN values count as positive, as in matplotlib.
        let toward_start = if self.inverted {
            self.value > 0.0
        } else {
            self.value < 0.0
        };
        match self.orientation {
            Orientation::Vertical => (
                HAlign::Center,
                if toward_start {
                    VAlign::Top
                } else {
                    VAlign::Bottom
                },
            ),
            Orientation::Horizontal => (
                if toward_start {
                    HAlign::Right
                } else {
                    HAlign::Left
                },
                VAlign::Center,
            ),
        }
    }
}

/// The label text of `value` (matplotlib's `_auto_format_str` or the
/// callable), an integer for Python when `whole`.
fn format_value(format: &BarLabelFormat, value: f64, whole: bool) -> Result<String, BarLabelError> {
    match format {
        BarLabelFormat::Template(template) => {
            auto_format_value(template, value, whole).map_err(BarLabelError::Format)
        }
        BarLabelFormat::Func(f) => Ok(f(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::{BarOptions, ErrorSize, ErrorbarOptions};
    use crate::figure::Figure;

    #[test]
    fn edge_labels_follow_the_bar_direction() {
        let ax = Figure::new().subplot();
        let bars = ax
            .bar([0.0, 1.0], [2.0, -1.5], BarOptions::default())
            .unwrap();
        let options = BarLabelOptions {
            fmt: BarLabelFormat::Template("%+.1f".into()),
            padding: vec![3.0],
            ..BarLabelOptions::default()
        };
        let labels = ax.bar_label(&bars, options).unwrap();
        assert_eq!(labels[0].text(), "+2.0");
        assert_eq!(labels[1].text(), "-1.5");
        assert_eq!(labels[0].valign(), VAlign::Bottom);
        assert_eq!(labels[1].valign(), VAlign::Top);
        assert_eq!(labels[1].position(), Point::new(0.0, -3.0));
    }

    #[test]
    fn error_bars_push_edge_labels_out() {
        let ax = Figure::new().subplot();
        let options = BarOptions {
            errors: ErrorbarOptions {
                yerr: Some(ErrorSize::Symmetric(vec![0.5])),
                ..ErrorbarOptions::default()
            },
            ..BarOptions::default()
        };
        let bars = ax.bar([0.0], [2.0], options).unwrap();
        assert_eq!(error_bar_ends(&bars), vec![Some((1.5, 2.5))]);
        let labels = ax.bar_label(&bars, BarLabelOptions::default()).unwrap();
        assert_eq!(labels[0].text(), "2");
        let plain = ax.bar([1.0], [2.0], BarOptions::default()).unwrap();
        assert_eq!(error_bar_ends(&plain), Vec::new());
    }

    #[test]
    fn labels_and_paddings_are_checked() {
        let ax = Figure::new().subplot();
        let bars = ax
            .barh([0.0, 1.0, 2.0], [1.0, f64::NAN, 3.0], BarOptions::default())
            .unwrap();
        let options = BarLabelOptions {
            labels: vec!["a".into()],
            fmt: BarLabelFormat::Template("{:.0%}".into()),
            label_type: BarLabelType::Center,
            ..BarLabelOptions::default()
        };
        let labels = ax.bar_label(&bars, options).unwrap();
        let texts: Vec<String> = labels.iter().map(|l| l.text()).collect();
        assert_eq!(texts, ["a", "", "300%"]);
        let whole = ax.bar([5.0], [7.0], BarOptions::default()).unwrap();
        let integer = BarLabelOptions {
            fmt: BarLabelFormat::Template("{:d}".into()),
            ..BarLabelOptions::default()
        };
        assert_eq!(ax.bar_label(&whole, integer).unwrap()[0].text(), "7");
        assert_eq!(labels[2].halign(), HAlign::Center);
        let padded = BarLabelOptions {
            padding: vec![1.0, 2.0],
            ..BarLabelOptions::default()
        };
        assert_eq!(
            ax.bar_label(&bars, padded).err(),
            Some(BarLabelError::PaddingCount { len: 2, bars: 3 })
        );
    }
}
