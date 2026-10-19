//! Pie charts (matplotlib's `pie`).

use std::fmt;
use std::rc::Rc;

use super::{Axes, PlotError};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::fonts::FontSize;
use crate::patches::{Patch, PatchProps, Shadow, ShadowError, Wedge};
use crate::rc::Aspect;
use crate::text::Text;
use crate::ticker::pyfmt::{FormatError, PrintfFormat};

/// How the percentage labels of a pie are written (matplotlib's
/// `autopct`).
#[derive(Clone)]
pub enum PieLabelFormat {
    /// A printf-style template applied to the percentage, e.g. `"%1.1f%%"`.
    Printf(String),
    /// A function of the percentage.
    Func(Rc<dyn Fn(f64) -> String>),
}

impl fmt::Debug for PieLabelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Printf(template) => f.debug_tuple("Printf").field(template).finish(),
            Self::Func(_) => f.write_str("Func(..)"),
        }
    }
}

/// Which way the wedges of a pie follow each other (matplotlib's
/// `counterclock`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PieDirection {
    /// Counter-clockwise (the default).
    #[default]
    CounterClockwise,
    /// Clockwise.
    Clockwise,
}

/// The drop shadow drawn under every wedge (matplotlib's `shadow=True`,
/// or a dict of `Shadow` arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct PieShadow {
    /// Horizontal offset in points.
    pub ox: f64,
    /// Vertical offset in points.
    pub oy: f64,
    /// How much darker than its wedge the shadow is, in `[0, 1]`.
    pub shade: f64,
    /// Properties overriding the shadow's, which otherwise copies its
    /// wedge's in the darker colour (the other keys of matplotlib's
    /// shadow dict, such as `edgecolor` or `linewidth`).
    pub props: PatchProps,
}

impl Default for PieShadow {
    /// matplotlib's pie shadow: offset by `(-0.02, -0.02)`, shade 0.7.
    fn default() -> Self {
        Self {
            ox: -0.02,
            oy: -0.02,
            shade: 0.7,
            props: PatchProps::default(),
        }
    }
}

/// Options of [`Axes::pie`].
#[derive(Clone, Debug)]
pub struct PieOptions {
    /// Offset of each wedge from the centre, as a fraction of the radius
    /// (empty: none).
    pub explode: Vec<f64>,
    /// One label per wedge, placed outside it (empty: none).
    pub labels: Vec<String>,
    /// Wedge colours, cycled (empty: the patch property cycle).
    pub colors: Vec<Color>,
    /// Percentage labels inside the wedges.
    pub autopct: Option<PieLabelFormat>,
    /// Distance of the percentage labels from the centre, in radii.
    pub pctdistance: f64,
    /// Distance of the labels from the centre, in radii (`None`: no
    /// labels).
    pub labeldistance: Option<f64>,
    /// Angle where the first wedge starts, in degrees from the x axis.
    pub startangle: f64,
    /// Pie radius.
    pub radius: f64,
    /// Which way the wedges follow each other.
    pub direction: PieDirection,
    /// Ring width of every wedge, for donuts (matplotlib's
    /// `wedgeprops={"width": ...}`).
    pub wedge_width: Option<f64>,
    /// Style of every wedge.
    pub wedgeprops: PatchProps,
    /// Font size of all labels (labels default to `xtick.labelsize`).
    pub text_size: Option<FontSize>,
    /// Colour of all labels.
    pub text_color: Option<Color>,
    /// Centre of the pie.
    pub center: (f64, f64),
    /// Keep the axes frame and ticks and autoscale to the pie.
    pub frame: bool,
    /// Rotate each label to its wedge's angle.
    pub rotatelabels: bool,
    /// Scale the values to a full pie (otherwise their sum must not
    /// exceed 1 and the pie may stay partial).
    pub normalize: bool,
    /// Hatch patterns, cycled over the wedges (empty: none).
    pub hatches: Vec<String>,
    /// A shadow under every wedge (`None`: no shadows).
    pub shadow: Option<PieShadow>,
}

impl Default for PieOptions {
    fn default() -> Self {
        Self {
            explode: Vec::new(),
            labels: Vec::new(),
            colors: Vec::new(),
            autopct: None,
            pctdistance: 0.6,
            labeldistance: Some(1.1),
            startangle: 0.0,
            radius: 1.0,
            direction: PieDirection::CounterClockwise,
            wedge_width: None,
            wedgeprops: PatchProps::default(),
            text_size: None,
            text_color: None,
            center: (0.0, 0.0),
            frame: false,
            rotatelabels: false,
            normalize: true,
            hatches: Vec::new(),
            shadow: None,
        }
    }
}

/// The artists of a pie chart (matplotlib's `PieContainer`).
#[derive(Clone, Debug, PartialEq)]
pub struct PieContainer {
    /// The wedges.
    pub wedges: Vec<Patch<Wedge>>,
    /// The values the pie shows.
    pub values: Vec<f64>,
    /// The fraction of the pie each wedge covers.
    pub fracs: Vec<f64>,
    /// The labels outside the wedges.
    pub labels: Vec<Text>,
    /// The percentage labels.
    pub pct_labels: Vec<Text>,
    /// The labels added with [`Axes::pie_label`], one list per call.
    pub added_labels: Vec<Vec<Text>>,
}

/// Why a pie cannot be drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum PieError {
    /// A value is negative.
    Negative,
    /// A value is NaN or infinite.
    NonFinite,
    /// Every value is zero.
    AllZero,
    /// Unnormalised values summing to more than 1.
    SumAboveOne,
    /// The radius is not positive.
    Radius(f64),
    /// `labels` or `explode` hold neither nothing nor one entry per wedge.
    Count {
        /// The option.
        argument: &'static str,
        /// Entries given.
        len: usize,
        /// Wedges.
        wedges: usize,
    },
    /// `autopct` is not a valid printf template.
    Format(String),
    /// A list of labels does not have one label per wedge.
    LabelCount {
        /// Labels given.
        labels: usize,
        /// Wedges.
        wedges: usize,
    },
    /// A label template is invalid for the values.
    LabelFormat(FormatError),
    /// The wedge shadows cannot be made.
    Shadow(ShadowError),
}

impl fmt::Display for PieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Negative => f.write_str("Wedge sizes 'x' must be non negative values"),
            Self::NonFinite => f.write_str("Wedge sizes must be finite numbers"),
            Self::AllZero => f.write_str("All wedge sizes are zero"),
            Self::SumAboveOne => f.write_str("Cannot plot an unnormalized pie with sum(x) > 1"),
            Self::Radius(r) => write!(f, "'radius' must be a positive number, not {r}"),
            Self::Count {
                argument,
                len,
                wedges,
            } => write!(
                f,
                "'{argument}' must be of length 'x' ({wedges}), not {len}"
            ),
            Self::Format(why) => write!(f, "invalid autopct: {why}"),
            Self::LabelCount { labels, wedges } => write!(
                f,
                "The number of labels ({labels}) must match the number of wedges ({wedges})"
            ),
            Self::LabelFormat(err) => write!(f, "invalid pie label template: {err}"),
            Self::Shadow(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for PieError {}

impl Axes {
    /// Draws a pie chart of `values` (matplotlib's `pie`): one wedge per
    /// value, an equal aspect, and unless `frame` a hidden frame with fixed
    /// limits around the pie.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::PieOptions;
    /// let ax = Figure::new().subplot();
    /// let pie = ax.pie(&[1.0, 3.0], &PieOptions::default())?;
    /// assert_eq!(pie.fracs, [0.25, 0.75]);
    /// assert_eq!(ax.xlim(), (-1.25, 1.25));
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the values are negative, non-finite or all zero, unnormalised
    /// values sum above 1, the radius is not positive, `labels` or
    /// `explode` do not match the values, `autopct` is invalid, or the
    /// shadow's shade is outside `[0, 1]`.
    pub fn pie(&self, values: &[f64], options: &PieOptions) -> Result<PieContainer, PlotError> {
        self.set_aspect(Aspect::Equal)
            .expect("an equal aspect is always valid");
        let fracs = fractions(values, options.normalize).map_err(PlotError::Pie)?;
        let n = values.len();
        for (argument, len) in [
            ("labels", options.labels.len()),
            ("explode", options.explode.len()),
        ] {
            if len != 0 && len != n {
                return Err(PlotError::Pie(PieError::Count {
                    argument,
                    len,
                    wedges: n,
                }));
            }
        }
        if options.radius <= 0.0 || options.radius.is_nan() {
            return Err(PlotError::Pie(PieError::Radius(options.radius)));
        }
        let wedges = self
            .pie_wedges(&fracs, options)
            .map_err(|err| PlotError::Pie(PieError::Shadow(err)))?;
        let (cx, cy) = options.center;
        let label_size = options
            .text_size
            .unwrap_or(crate::rc::params().xtick_labelsize);
        let labels = match options.labeldistance {
            Some(distance) if !options.labels.is_empty() => self.place_pie_labels(
                &wedges,
                &options.labels,
                &super::pie_label::LabelPlacement {
                    distance,
                    outer: true,
                    rotate: options.rotatelabels,
                },
                Some(label_size),
                options.text_color.as_ref(),
            ),
            _ => Vec::new(),
        };
        let pct_labels = match &options.autopct {
            Some(format) => {
                let texts = fracs
                    .iter()
                    .map(|frac| percent_label(format, 100.0 * frac))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(PlotError::Pie)?;
                self.place_pie_labels(
                    &wedges,
                    &texts,
                    &super::pie_label::LabelPlacement {
                        distance: options.pctdistance,
                        outer: false,
                        rotate: false,
                    },
                    options.text_size,
                    options.text_color.as_ref(),
                )
            }
            None => Vec::new(),
        };
        if options.frame {
            self.autoscale_view(None);
        } else {
            self.set_frame_on(false);
            self.set_xticks(Vec::<f64>::new())
                .set_yticks(Vec::<f64>::new());
            self.set_xlim(-1.25 + cx, 1.25 + cx)
                .set_ylim(-1.25 + cy, 1.25 + cy);
        }
        Ok(PieContainer {
            wedges,
            values: values.to_vec(),
            fracs,
            labels,
            pct_labels,
            added_labels: Vec::new(),
        })
    }

    /// One wedge per fraction, laid out from `startangle`, each followed
    /// by its shadow if any.
    fn pie_wedges(
        &self,
        fracs: &[f64],
        options: &PieOptions,
    ) -> Result<Vec<Patch<Wedge>>, ShadowError> {
        let mut theta1 = options.startangle / 360.0;
        fracs
            .iter()
            .enumerate()
            .map(|(i, frac)| {
                let theta2 = match options.direction {
                    PieDirection::CounterClockwise => theta1 + frac,
                    PieDirection::Clockwise => theta1 - frac,
                };
                let thetam = 2.0 * std::f64::consts::PI * 0.5 * (theta1 + theta2);
                let offset = options.explode.get(i).copied().unwrap_or(0.0);
                let center = (
                    options.center.0 + offset * thetam.cos(),
                    options.center.1 + offset * thetam.sin(),
                );
                let mut shape = Wedge::new(
                    center,
                    options.radius,
                    360.0 * theta1.min(theta2),
                    360.0 * theta1.max(theta2),
                );
                shape.width = options.wedge_width;
                let face = if options.colors.is_empty() {
                    self.next_patch_color()
                } else {
                    options.colors[i % options.colors.len()].clone()
                };
                let mut props = PatchProps::default().facecolor(face);
                if !options.hatches.is_empty() {
                    props = props.hatch(options.hatches[i % options.hatches.len()].clone());
                }
                let wedge = self.add_patch(shape);
                wedge.update_props(props.overlay(options.wedgeprops.clone()));
                wedge.set_clip_on(false);
                if let Some(label) = options.labels.get(i) {
                    wedge.set_label(label.clone());
                }
                if let Some(PieShadow {
                    ox,
                    oy,
                    shade,
                    props,
                }) = &options.shadow
                {
                    let shadow = self.add_patch(Shadow::of_shaded(&wedge, *ox, *oy, *shade)?);
                    shadow.update_props(props.clone());
                    shadow.set_clip_on(false).set_label("_nolegend_");
                }
                theta1 = theta2;
                Ok(wedge)
            })
            .collect()
    }
}

/// The fraction of the pie each value covers.
fn fractions(values: &[f64], normalize: bool) -> Result<Vec<f64>, PieError> {
    if values.iter().any(|&v| v < 0.0) {
        return Err(PieError::Negative);
    }
    if values.iter().any(|v| !v.is_finite()) {
        return Err(PieError::NonFinite);
    }
    let sum: f64 = values.iter().sum();
    if sum == 0.0 {
        return Err(PieError::AllZero);
    }
    if normalize {
        Ok(values.iter().map(|v| v / sum).collect())
    } else if sum > 1.0 {
        Err(PieError::SumAboveOne)
    } else {
        Ok(values.to_vec())
    }
}

/// The percentage label `format` writes for `percent`.
fn percent_label(format: &PieLabelFormat, percent: f64) -> Result<String, PieError> {
    match format {
        PieLabelFormat::Printf(template) => PrintfFormat::parse(template)
            .and_then(|f| f.format(percent))
            .map_err(|e| PieError::Format(e.to_string())),
        PieLabelFormat::Func(f) => Ok(f(percent)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn fractions_validate_like_matplotlib() {
        assert_eq!(fractions(&[1.0, 3.0], true), Ok(vec![0.25, 0.75]));
        assert_eq!(fractions(&[0.2, 0.3], false), Ok(vec![0.2, 0.3]));
        assert_eq!(fractions(&[0.7, 0.6], false), Err(PieError::SumAboveOne));
        assert_eq!(fractions(&[-1.0, 2.0], true), Err(PieError::Negative));
        assert_eq!(fractions(&[0.0, 0.0], true), Err(PieError::AllZero));
        assert_eq!(fractions(&[f64::NAN], true), Err(PieError::NonFinite));
    }

    #[test]
    fn percentage_labels_use_printf() {
        let format = PieLabelFormat::Printf("%1.1f%%".into());
        assert_eq!(percent_label(&format, 35.0), Ok("35.0%".into()));
        let integer = PieLabelFormat::Printf("%d%%".into());
        assert_eq!(percent_label(&integer, 12.6), Ok("12%".into()));
    }

    #[test]
    fn wedges_follow_start_angle_and_direction() {
        let ax = Figure::new().subplot();
        let options = PieOptions {
            startangle: 90.0,
            direction: PieDirection::Clockwise,
            labels: vec!["a".into(), "b".into()],
            ..PieOptions::default()
        };
        let pie = ax.pie(&[1.0, 1.0], &options).unwrap();
        let first = pie.wedges[0].shape();
        assert_eq!((first.theta1, first.theta2), (-90.0, 90.0));
        assert_eq!(pie.labels.len(), 2);
        let framed = ax
            .raw()
            .with::<crate::axes::AxesState, _>(|a| a.decorations.frame_on);
        assert!(!framed, "a frameless pie hides the frame");
        let mismatched = PieOptions {
            explode: vec![0.1, 0.2],
            ..PieOptions::default()
        };
        assert!(ax.pie(&[1.0], &mismatched).is_err());
    }

    #[test]
    fn wedges_carry_the_labels_for_legends() {
        let ax = Figure::new().subplot();
        let options = PieOptions {
            labels: vec!["a".into(), "b".into()],
            labeldistance: None,
            ..PieOptions::default()
        };
        let pie = ax.pie(&[1.0, 2.0], &options).unwrap();
        assert!(pie.labels.is_empty(), "no labels drawn without a distance");
        assert_eq!(ax.legend_handles_labels().1, ["a", "b"]);
    }
}
