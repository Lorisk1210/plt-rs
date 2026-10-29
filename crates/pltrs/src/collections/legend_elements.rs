//! Legend entries summarising the colours or the sizes of a scatter plot
//! (matplotlib's `PathCollection.legend_elements`).

use std::fmt;
use std::rc::Rc;

use super::{Collection, CollectionArtist};
use crate::artist::{Artist, ArtistHandle};
use crate::colors::Color;
use crate::legend::LegendHandle;
use crate::lines::{LineArtist, LineStyle};
use crate::markers::Marker;
use crate::numeric::{argsort, interp, linspace};
use crate::ticker::{
    AxisInfo, FixedLocator, Formatter, Locator, MaxNLocator, Nbins, OffsetMode, ScalarFormatter,
    TickContext, TickerError, TickerSettings,
};

/// The entries `"auto"` makes at most before it picks values with a
/// locator.
const AUTO_ENTRIES: usize = 9;

/// The steps of the locator that picks `num` values.
const STEPS: [f64; 8] = [1.0, 2.0, 2.5, 3.0, 5.0, 6.0, 8.0, 10.0];

/// What the entries of [`Collection::legend_elements`] tell apart
/// (matplotlib's `prop`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LegendProperty {
    /// The colours of the colour-mapped values.
    #[default]
    Colors,
    /// The marker sizes.
    Sizes,
}

/// Which values [`Collection::legend_elements`] makes entries for
/// (matplotlib's `num`).
#[derive(Clone, Debug, Default)]
pub enum ElementCount {
    /// Every distinct value when there are at most nine, else about nine
    /// values at round steps.
    #[default]
    Auto,
    /// Every distinct value.
    All,
    /// About this many values at round steps across the data range.
    Target(usize),
    /// These values, those outside the data range left out.
    Values(Vec<f64>),
    /// The values a locator picks across the data range.
    Locator(Rc<dyn Locator>),
}

/// Maps the values of a collection to the values the labels of
/// [`Collection::legend_elements`] show (matplotlib's `func`): the
/// inverse of what made the values from the data, such as sizes scaled
/// to be seen. The identity by default. It runs while the collection is
/// read, so it is `Send` and cannot use the figure's handles. Clones
/// share the function.
///
/// ```
/// use pltrs::collections::LegendValueFn;
/// let sizes_to_population = LegendValueFn::new(|size| size / 20.0);
/// assert_eq!(sizes_to_population.apply(60.0), 3.0);
/// assert_eq!(LegendValueFn::default().apply(60.0), 60.0);
/// ```
#[derive(Clone)]
pub struct LegendValueFn(Rc<dyn Fn(f64) -> f64>);

impl LegendValueFn {
    /// The function `func`.
    pub fn new(func: impl Fn(f64) -> f64 + Send + 'static) -> Self {
        Self(Rc::new(func))
    }

    /// The value the label of `value` shows.
    pub fn apply(&self, value: f64) -> f64 {
        (self.0)(value)
    }
}

impl Default for LegendValueFn {
    /// The identity.
    fn default() -> Self {
        Self::new(|value| value)
    }
}

impl fmt::Debug for LegendValueFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LegendValueFn").finish_non_exhaustive()
    }
}

/// Options of [`Collection::legend_elements`] (matplotlib's keyword
/// arguments).
#[derive(Debug, Default)]
pub struct LegendElementsOptions {
    /// What the entries show.
    pub prop: LegendProperty,
    /// Which values get entries.
    pub num: ElementCount,
    /// The label formatter; by default a scalar formatter without offset
    /// that writes multipliers as mathtext.
    pub fmt: Option<Box<dyn Formatter>>,
    /// Maps the collection's values to the values the labels show
    /// (matplotlib's `func`): `LegendValueFn::new(|size| size / 20.0)`.
    pub func: LegendValueFn,
    /// The marker colour of size entries; `pltrs.ink` (black) by default.
    pub color: Option<Color>,
    /// The marker size of colour entries in points; `lines.markersize` by
    /// default.
    pub size: Option<f64>,
    /// The opacity of the entries; the collection's by default.
    pub alpha: Option<f64>,
}

/// The legend handles and their labels.
pub type LegendElements = (Vec<LegendHandle>, Vec<String>);

impl Collection {
    /// Legend handles and labels for the distinct colours or sizes of a
    /// scatter plot: one marker line per value, drawn with the
    /// collection's first shape (matplotlib's `legend_elements`). Colour
    /// entries need a colour mapping; without one there are none.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::{MappedColors, ScatterOptions};
    /// use pltrs::collections::LegendElementsOptions;
    /// use pltrs::legend::LegendOptions;
    /// let ax = Figure::new().subplot();
    /// let options = ScatterOptions {
    ///     colors: MappedColors::new(vec![1.0, 2.0, 2.0, 3.0]).into(),
    ///     ..ScatterOptions::default()
    /// };
    /// let points = ax.scatter_with([0.0, 1.0, 2.0, 3.0], [0.0, 1.0, 0.0, 1.0], options)?;
    /// let (handles, labels) = points.legend_elements(LegendElementsOptions::default())?;
    /// assert_eq!(labels, ["$\\mathdefault{1}$", "$\\mathdefault{2}$", "$\\mathdefault{3}$"]);
    /// ax.legend(LegendOptions::default().handles(handles).labels(labels))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the locator or the formatter fails.
    pub fn legend_elements(
        &self,
        options: LegendElementsOptions,
    ) -> Result<LegendElements, TickerError> {
        self.raw()
            .with::<CollectionArtist, _>(|collection| elements(collection, &options))
    }
}

fn elements(
    collection: &CollectionArtist,
    options: &LegendElementsOptions,
) -> Result<LegendElements, TickerError> {
    let array: Vec<f64> = match options.prop {
        LegendProperty::Colors => match &collection.mapping {
            Some(mapped) => mapped.source.values.iter().flatten().copied().collect(),
            None => return Ok((Vec::new(), Vec::new())),
        },
        LegendProperty::Sizes => collection.sizes.clone().unwrap_or_default(),
    };
    let unique = sorted_unique(&array);
    let (Some(&lowest), Some(&highest)) = (unique.first(), unique.last()) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let num = match &options.num {
        ElementCount::Auto if unique.len() <= AUTO_ENTRIES => &ElementCount::All,
        ElementCount::Auto => &ElementCount::Target(AUTO_ENTRIES),
        num => num,
    };
    let shown: Vec<f64> = unique.iter().map(|&v| options.func.apply(v)).collect();
    let shown_range = shown
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    let (values, label_values) = match num {
        ElementCount::All => (unique.clone(), shown),
        num => located(num, (lowest, highest), shown_range, options)?,
    };
    let labels = format_labels(options.fmt.as_deref(), &label_values, shown_range)?;
    let marker = Marker::Path(collection.paths.first().cloned().unwrap_or_default());
    let edge_width = collection.linewidths_or_default()[0];
    let alpha = options.alpha.or(collection.base().alpha);
    let rc = &collection.rc;
    let mut handles = Vec::new();
    let mut kept_labels = Vec::new();
    for (value, label) in values.into_iter().zip(labels) {
        let (color, size) = match options.prop {
            LegendProperty::Colors => {
                let color = collection
                    .mapping
                    .as_ref()
                    .map_or_else(Color::none, |mapped| value_color(&mapped.source, value));
                let size = options.size.unwrap_or(rc.lines_markersize);
                (color, size)
            }
            LegendProperty::Sizes => {
                let size = value.sqrt();
                if is_close_to_zero(size) {
                    continue;
                }
                let color = options
                    .color
                    .clone()
                    .unwrap_or_else(|| rc.pltrs_ink.clone());
                (color, size)
            }
        };
        let mut line = LineArtist::new(vec![0.0], vec![0.0], Rc::clone(rc));
        line.linestyle = LineStyle::None;
        line.color = color;
        line.markersize = size;
        line.marker = marker.clone();
        line.markeredgewidth = edge_width;
        line.base_mut().alpha = alpha;
        line.recache();
        handles.push(LegendHandle::proxy(line));
        kept_labels.push(label);
    }
    Ok((handles, kept_labels))
}

/// The values a locator picks across `(low, high)`, the range the labels
/// show, and the collection values in `(lowest, highest)` they stand for
/// (inverted through `func` by linear interpolation, as matplotlib does).
fn located(
    num: &ElementCount,
    (lowest, highest): (f64, f64),
    (low, high): (f64, f64),
    options: &LegendElementsOptions,
) -> Result<(Vec<f64>, Vec<f64>), TickerError> {
    let shown = linspace(lowest, highest, 256);
    let mapped: Vec<f64> = shown.iter().map(|&v| options.func.apply(v)).collect();
    let settings = TickerSettings::from(&*crate::rc::params());
    let ctx = TickContext::detached(&settings);
    let ticks = match num {
        ElementCount::Locator(locator) => locator.tick_values(low, high, &ctx)?,
        ElementCount::Values(values) => {
            FixedLocator::new(values.clone()).tick_values(low, high, &ctx)?
        }
        ElementCount::Target(n) => MaxNLocator::new(Nbins::Fixed(*n))
            .with_min_n_ticks(n.saturating_sub(1))
            .with_steps(&STEPS)?
            .tick_values(low, high, &ctx)?,
        ElementCount::Auto | ElementCount::All => Vec::new(),
    };
    let label_values: Vec<f64> = ticks
        .into_iter()
        .filter(|&v| v >= low && v <= high)
        .collect();
    let order = argsort(&mapped);
    let xp: Vec<f64> = order.iter().map(|&i| mapped[i]).collect();
    let fp: Vec<f64> = order.iter().map(|&i| shown[i]).collect();
    let values = label_values
        .iter()
        .map(|&v| interp(v, &xp, &fp, None))
        .collect();
    Ok((values, label_values))
}

/// The labels of `values`, formatted together on an axis spanning the
/// values the labels show.
fn format_labels(
    fmt: Option<&dyn Formatter>,
    values: &[f64],
    (vmin, vmax): (f64, f64),
) -> Result<Vec<String>, TickerError> {
    let settings = TickerSettings::from(&*crate::rc::params());
    let axis = AxisInfo::with_view(vmin, vmax);
    let ctx = TickContext::new(&axis, &settings);
    let labels = match fmt {
        Some(fmt) => fmt.format_ticks(values, &ctx)?,
        None => ScalarFormatter::from_settings(&settings)
            .with_use_offset(OffsetMode::Off)
            .with_use_mathtext(true)
            .format_ticks(values, &ctx)?,
    };
    Ok(labels.labels)
}

/// The colour `value` maps to.
fn value_color(mapping: &super::ColorMapping, value: f64) -> Color {
    let mut mappable = mapping.mappable.clone();
    mappable
        .to_rgba(&[Some(value)], None)
        .ok()
        .and_then(|colors| colors.first().copied())
        .map_or_else(Color::none, Color::from)
}

/// The distinct finite values, ascending (numpy's `unique`).
fn sorted_unique(values: &[f64]) -> Vec<f64> {
    let mut unique: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    unique.sort_by(f64::total_cmp);
    unique.dedup();
    unique
}

/// numpy's `isclose(size, 0.0)` with its default tolerances.
fn is_close_to_zero(size: f64) -> bool {
    size.abs() <= 1e-8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::{MappedColors, ScatterOptions};
    use crate::figure::Figure;

    fn scatter(options: ScatterOptions) -> Collection {
        let ax = Figure::new().subplot();
        let x: Vec<f64> = (0..12).map(f64::from).collect();
        ax.scatter_with(x.clone(), x, options)
            .expect("a valid scatter")
    }

    #[test]
    fn colours_list_every_distinct_value_up_to_nine() {
        let values = vec![3.0, 1.0, 2.0, 2.0, 1.0, 3.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0];
        let points = scatter(ScatterOptions {
            colors: MappedColors::new(values).into(),
            ..ScatterOptions::default()
        });
        let (handles, labels) = points
            .legend_elements(LegendElementsOptions::default())
            .expect("labels");
        assert_eq!(handles.len(), 3);
        assert_eq!(labels[2], "$\\mathdefault{3}$");
    }

    #[test]
    fn sizes_use_a_locator_and_func() {
        let sizes: Vec<f64> = (0..12).map(|i| 10.0 * f64::from(i)).collect();
        let points = scatter(ScatterOptions {
            sizes: Some(sizes),
            ..ScatterOptions::default()
        });
        let options = LegendElementsOptions {
            prop: LegendProperty::Sizes,
            num: ElementCount::Target(3),
            func: LegendValueFn::new(|s| s / 10.0),
            fmt: Some(Box::new(
                crate::ticker::StrMethodFormatter::new("{x:.0f}").expect("a template"),
            )),
            ..LegendElementsOptions::default()
        };
        let (handles, labels) = points.legend_elements(options).expect("labels");
        // matplotlib 3.11 gives these; size 0 has no marker and is left out.
        assert_eq!(labels, ["5", "10"]);
        assert_eq!(handles.len(), 2);
    }

    #[test]
    fn colours_need_a_mapping_and_empty_collections_give_nothing() {
        let plain = scatter(ScatterOptions::default());
        let (handles, labels) = plain
            .legend_elements(LegendElementsOptions::default())
            .expect("no entries");
        assert!(handles.is_empty() && labels.is_empty());
        let ax = Figure::new().subplot();
        let empty = ax
            .scatter(Vec::<f64>::new(), Vec::<f64>::new())
            .expect("an empty scatter");
        let sizes = LegendElementsOptions {
            prop: LegendProperty::Sizes,
            ..LegendElementsOptions::default()
        };
        // The default marker size stands for every point.
        assert_eq!(empty.legend_elements(sizes).expect("one entry").1.len(), 1);
    }
}
