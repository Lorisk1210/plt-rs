//! Line plotting: `plot`.

use super::children::DataUpdate;
use super::{Axes, AxesState, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::colors::Color;
use crate::lines::{Line2D, LineArtist, LineProps};
use crate::numeric::{Number, to_f64s};
use crate::rc::{CycleValue, Cycler};
use crate::scale::LogScale;
use crate::ticker::AxisName;
use crate::units::{AxisValues, UnitHandle, UnitSources};

impl Axes {
    /// Plots `y` against `x` as a line using the next property-cycle style
    /// (matplotlib's `ax.plot(x, y)`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().add_axes([0.1, 0.1, 0.8, 0.8])?;
    /// let line = ax.plot([1.0, 2.0, 3.0], [1.0, 4.0, 9.0])?;
    /// line.set_label("squares");
    /// assert_eq!(ax.xlim(), (0.9, 3.1));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn plot(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
    ) -> Result<Line2D, PlotError> {
        self.plot_with(x, y, LineProps::default())
    }

    /// Plots `y` against `x` styled by a format string such as `"ro--"`
    /// (matplotlib's `ax.plot(x, y, fmt)`).
    pub fn plot_fmt(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        format: &str,
    ) -> Result<Line2D, PlotError> {
        self.plot_with(x, y, LineProps::fmt(format)?)
    }

    /// Plots `y` against `x` with explicit properties; the property cycle
    /// supplies only the properties left unset, and does not advance when
    /// every cycled property is set (matplotlib's `ax.plot(x, y, **kwargs)`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().add_axes([0.1, 0.1, 0.8, 0.8])?;
    /// let props = LineProps::fmt("o--")?.markersize(4.0).label("data");
    /// let line = ax.plot_with([1.0, 2.0], [3.0, 4.0], props)?;
    /// assert_eq!(line.markersize(), 4.0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn plot_with(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        props: LineProps,
    ) -> Result<Line2D, PlotError> {
        let (x, y) = (x.into(), y.into());
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let sources = UnitSources::of(&x, &y);
        let x = self.xaxis().convert_plot_units(x);
        let y = self.yaxis().convert_plot_units(y);
        let line = self.add_cycled_line(x, y, props);
        line.set_unit_sources(sources);
        Ok(line)
    }

    /// Plots each of `columns` against `x` as a line with the next
    /// property-cycle style (matplotlib's `ax.plot(x, y)` with a 2-D `y`,
    /// whose columns are the datasets). `props` style every line, its
    /// label included; `labels` gives each line its own label instead
    /// (matplotlib's `label` given as a list).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let columns = [vec![1.0, 2.0, 4.0], vec![2.0, 5.0, 9.0]];
    /// let labels = ["low".to_owned(), "high".to_owned()];
    /// let lines = ax.plot_columns([1.0, 2.0, 3.0], &columns, &LineProps::default(), Some(&labels))?;
    /// assert_eq!(lines[1].label(), "high");
    /// assert_ne!(lines[0].color(), lines[1].color());
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a column differs in length from `x`, or `labels` does not hold
    /// one label per column (nothing is plotted then).
    pub fn plot_columns(
        &self,
        x: impl Into<AxisValues>,
        columns: &[Vec<f64>],
        props: &LineProps,
        labels: Option<&[String]>,
    ) -> Result<Vec<Line2D>, PlotError> {
        let x = x.into();
        if let Some(labels) = labels
            && labels.len() != columns.len()
        {
            return Err(PlotError::DatasetCount {
                argument: "label",
                len: labels.len(),
                datasets: columns.len(),
            });
        }
        if let Some(column) = columns.iter().find(|column| column.len() != x.len()) {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: column.len(),
            });
        }
        columns
            .iter()
            .enumerate()
            .map(|(i, column)| {
                let mut props = props.clone();
                if let Some(labels) = labels {
                    props = props.label(labels[i].clone());
                }
                self.plot_with(x.clone(), column.clone(), props)
            })
            .collect()
    }

    /// Plots `y` against its indices `0, 1, 2, ...` (matplotlib's `ax.plot(y)`).
    pub fn plot_y(&self, y: impl IntoIterator<Item: Number>) -> Line2D {
        let y = to_f64s(y);
        let x = (0..y.len()).map(|i| i as f64).collect();
        self.add_cycled_line(x, y, LineProps::default())
    }

    /// Plots on logarithmic x and y axes (matplotlib's `ax.loglog`).
    pub fn loglog(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
    ) -> Result<Line2D, PlotError> {
        self.set_log_scales(true, true);
        self.plot(x, y)
    }

    /// Plots on a logarithmic x axis (matplotlib's `ax.semilogx`).
    pub fn semilogx(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
    ) -> Result<Line2D, PlotError> {
        self.set_log_scales(true, false);
        self.plot(x, y)
    }

    /// Plots on a logarithmic y axis (matplotlib's `ax.semilogy`).
    pub fn semilogy(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
    ) -> Result<Line2D, PlotError> {
        self.set_log_scales(false, true);
        self.plot(x, y)
    }

    fn set_log_scales(&self, x: bool, y: bool) {
        let log = || Box::new(LogScale::new(10.0).expect("10 is a valid logarithm base"));
        if x {
            self.set_scale(AxisName::X, log());
        }
        if y {
            self.set_scale(AxisName::Y, log());
        }
    }

    /// Adds a data line styled by `props` and the property cycle and
    /// autoscales the view (matplotlib's `plot`, which requests
    /// autoscaling after `add_line`).
    fn add_cycled_line(&self, x: Vec<f64>, y: Vec<f64>, props: LineProps) -> Line2D {
        let line = self.add_line_artist(self.cycled_line(x, y, props));
        self.raw()
            .with_mut::<AxesState, _>(|axes| axes.request_autoscale(true, true));
        line
    }

    /// Adds a line through `(x, y)` in data coordinates, styled from the
    /// rc parameters and `props` without the property cycle, extending the
    /// data limits without autoscaling the view (matplotlib's
    /// `ax.add_line(Line2D(...))`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// let line = ax.add_line(&[0.0, 1.0], &[0.0, 2.0], LineProps::default().linewidth(3.0))?;
    /// assert_eq!(line.linewidth(), 3.0);
    /// assert_eq!(line.color(), Color::from("C0"), "`lines.color`, not the next cycle colour");
    /// assert_eq!(ax.lines().len(), 2);
    /// assert_eq!(ax.data_lim().bbox.p1.y, 2.0);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `x` and `y` differ in length.
    pub fn add_line(&self, x: &[f64], y: &[f64], props: LineProps) -> Result<Line2D, PlotError> {
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let mut line = LineArtist::new(x.to_vec(), y.to_vec(), crate::rc::params());
        props.apply(&mut line);
        Ok(self.add_line_artist(line))
    }

    /// A new data line styled by `props`, with the property cycle (of the
    /// host, for parasite axes) filling in the cycled properties `props`
    /// leaves unset (matplotlib's `_makeline`).
    pub(crate) fn cycled_line(&self, x: Vec<f64>, y: Vec<f64>, props: LineProps) -> LineArtist {
        let mut line = LineArtist::new(x, y, crate::rc::params());
        let owner = self.line_cycle_owner();
        let cycled = owner.raw().with_mut::<AxesState, _>(|axes| {
            let entry: Vec<(String, CycleValue)> = axes
                .prop_cycle
                .entry(axes.line_cycle)
                .filter(|(key, _)| !props.is_set(key))
                .map(|(key, value)| (key.to_owned(), value.clone()))
                .collect();
            // matplotlib's `_getdefaults`: only advance when the cycle
            // supplies something.
            if !entry.is_empty() {
                axes.line_cycle += 1;
            }
            entry
        });
        for (key, value) in cycled {
            apply_cycle(&mut line, &key, value);
        }
        props.apply(&mut line);
        line.base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data());
        line
    }

    /// The lines of the axes in the order they were added (matplotlib's
    /// `ax.lines`).
    pub fn lines(&self) -> Vec<Line2D> {
        self.children_of::<LineArtist>()
            .into_iter()
            .map(Line2D::from_raw)
            .collect()
    }

    /// Adds a line, in data coordinates unless it has a transform, and
    /// extends the data limits to its drawn vertices without autoscaling
    /// the view (matplotlib's `add_line`).
    pub(crate) fn add_line_artist(&self, mut line: LineArtist) -> Line2D {
        let base = line.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        let update = DataUpdate::extending(line.limit_points(self.id()));
        Line2D::from_raw(self.adopt_in_autoscale(line, &update))
    }

    /// Replaces the property cycle of lines and patches and restarts it;
    /// `None` restores `axes.prop_cycle` (matplotlib's `set_prop_cycle`).
    pub fn set_prop_cycle(&self, cycle: Option<Cycler>) -> &Self {
        let cycle = cycle.unwrap_or_else(|| crate::rc::params().axes_prop_cycle.clone());
        self.raw().with_mut::<AxesState, _>(|axes| {
            axes.prop_cycle = cycle;
            axes.line_cycle = 0;
            axes.patch_cycle = 0;
        });
        self
    }

    /// The next colour of the line property cycle (matplotlib's
    /// `_get_lines.get_next_color`).
    pub(crate) fn next_line_color(&self) -> Color {
        self.line_cycle_owner()
            .raw()
            .with_mut::<AxesState, _>(|axes| {
                let color = cycle_color(&axes.prop_cycle, axes.line_cycle);
                if color.is_some() {
                    axes.line_cycle += 1;
                }
                color.unwrap_or_else(|| Color::from("k"))
            })
    }
}

/// The colour of entry `index` of a property cycle, if it cycles colours.
pub(crate) fn cycle_color(cycle: &Cycler, index: usize) -> Option<Color> {
    cycle
        .entry(index)
        .find_map(|(key, value)| match (key, value) {
            ("color", CycleValue::Color(color)) => Some(color.clone()),
            _ => None,
        })
}

/// Applies one property-cycle entry to a new line.
fn apply_cycle(line: &mut LineArtist, key: &str, value: CycleValue) {
    match (key, value) {
        ("color", CycleValue::Color(color)) => line.color = color,
        ("linestyle", CycleValue::LineStyle(style)) => line.linestyle = style,
        ("linewidth", CycleValue::Number(width)) => line.linewidth = width,
        ("marker", CycleValue::Marker(marker)) => line.marker = marker,
        ("markersize", CycleValue::Number(size)) => line.markersize = size,
        ("markeredgewidth", CycleValue::Number(width)) => line.markeredgewidth = width,
        ("markeredgecolor", CycleValue::Color(color)) => line.markeredgecolor = color.into(),
        ("markerfacecolor", CycleValue::Color(color)) => line.markerfacecolor = color.into(),
        ("alpha", CycleValue::Number(alpha)) => line.base_mut().alpha = Some(alpha),
        ("dashes", CycleValue::Dashes(pattern)) => {
            // matplotlib's `set_dashes`: an empty sequence draws a solid line.
            line.linestyle = if pattern.is_empty() {
                crate::lines::LineStyle::Solid
            } else {
                crate::lines::LineStyle::dashes(0.0, pattern)
            };
        }
        ("markevery", CycleValue::MarkEvery(markevery)) => line.markevery = markevery,
        ("fillstyle", CycleValue::Text(fillstyle)) => {
            if let Ok(fillstyle) = fillstyle.parse() {
                line.fillstyle = fillstyle;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::axes::PlotError;
    use crate::figure::Figure;
    use crate::lines::{LineProps, LineStyle};

    #[test]
    fn columns_share_one_label_or_take_one_each() {
        let ax = Figure::new().subplot();
        let columns = [vec![1.0, 2.0], vec![3.0, 4.0]];
        let shared = LineProps::default().label("one");
        let lines = ax
            .plot_columns([0.0, 1.0], &columns, &shared, None)
            .unwrap();
        let labels: Vec<String> = lines.iter().map(ArtistHandle::label).collect();
        assert_eq!(labels, ["one", "one"]);
        let own = ["a".to_owned(), "b".to_owned()];
        let lines = ax
            .plot_columns([0.0, 1.0], &columns, &LineProps::default(), Some(&own))
            .unwrap();
        assert_eq!(lines[1].label(), "b");
        assert_eq!(ax.lines().len(), 4);
    }

    #[test]
    fn columns_need_one_label_each_and_the_length_of_x() {
        let ax = Figure::new().subplot();
        let columns = [vec![1.0, 2.0], vec![3.0, 4.0]];
        let three = ["a".to_owned(), "b".to_owned(), "c".to_owned()];
        assert_eq!(
            ax.plot_columns([0.0, 1.0], &columns, &LineProps::default(), Some(&three)),
            Err(PlotError::DatasetCount {
                argument: "label",
                len: 3,
                datasets: 2
            })
        );
        assert_eq!(
            ax.plot_columns([0.0], &columns, &LineProps::default(), None),
            Err(PlotError::LengthMismatch { x: 1, y: 2 })
        );
        assert_eq!(ax.lines(), Vec::new(), "nothing is plotted");
    }

    #[test]
    fn empty_dash_sequences_in_a_cycle_draw_solid_lines() {
        let ax = Figure::new().subplot();
        let cycle = "cycler('dashes', [[], [13, 2]])".parse().expect("valid");
        ax.set_prop_cycle(Some(cycle));
        let solid = ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).expect("plotted");
        let dashed = ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).expect("plotted");
        assert_eq!(solid.linestyle(), LineStyle::Solid);
        assert!(!solid.linestyle().is_dashed(), "solid caps and joins");
        assert_eq!(dashed.linestyle(), LineStyle::dashes(0.0, vec![13.0, 2.0]));
    }
}
