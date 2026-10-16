//! Line plotting: `plot`.

use super::children::DataUpdate;
use super::{Axes, AxesState, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::colors::Color;
use crate::lines::{Line2D, LineArtist, LineProps};
use crate::rc::{CycleValue, Cycler};
use crate::scale::LogScale;
use crate::ticker::AxisName;
use crate::units::AxisValues;

impl Axes {
    /// Plots `y` against `x` as a line using the next property-cycle style
    /// (matplotlib's `ax.plot(x, y)`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().add_axes([0.1, 0.1, 0.8, 0.8]);
    /// let line = ax.plot([1.0, 2.0, 3.0], [1.0, 4.0, 9.0])?;
    /// line.set_label("squares");
    /// assert_eq!(ax.xlim(), (0.9, 3.1));
    /// # Ok::<(), plt::axes::PlotError>(())
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
    /// let ax = Figure::new().add_axes([0.1, 0.1, 0.8, 0.8]);
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
        let x = self.xaxis().convert_units(x);
        let y = self.yaxis().convert_units(y);
        Ok(self.add_line(x, y, props))
    }

    /// Plots `y` against its indices `0, 1, 2, ...` (matplotlib's `ax.plot(y)`).
    pub fn plot_y(&self, y: impl AsRef<[f64]>) -> Line2D {
        let y = y.as_ref().to_vec();
        let x = (0..y.len()).map(|i| i as f64).collect();
        self.add_line(x, y, LineProps::default())
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

    fn add_line(&self, x: Vec<f64>, y: Vec<f64>, props: LineProps) -> Line2D {
        let line = self.cycled_line(x, y, props);
        self.add_line_artist(line)
    }

    /// A new data line styled by `props`, with the property cycle filling
    /// in the cycled properties `props` leaves unset (matplotlib's
    /// `_makeline`).
    pub(crate) fn cycled_line(&self, x: Vec<f64>, y: Vec<f64>, props: LineProps) -> LineArtist {
        let mut line = LineArtist::new(x, y, crate::rc::params());
        let cycled = self.raw().with_mut::<AxesState, _>(|axes| {
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
        line.base_mut().transform = Some(self.trans_data());
        line
    }

    /// Adds a line in data coordinates and extends the data limits to its
    /// drawn vertices.
    pub(crate) fn add_line_artist(&self, line: LineArtist) -> Line2D {
        let update = DataUpdate::data(line.drawn_points());
        Line2D::from_raw(self.adopt(line, &update))
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
        self.raw().with_mut::<AxesState, _>(|axes| {
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
            line.linestyle = crate::lines::LineStyle::dashes(0.0, pattern);
        }
        _ => {}
    }
}
