//! The tests of `test_ticker.py` that read the ticks of an axes, replayed
//! in a fresh figure in the probe's style (see the module docs of
//! `test_mpl_ticker.py` for the order in which the axes is set up).

use std::any::Any;

use plt::axes::{Axes, Axis, AxisName, BoxplotOptions, BxpOptions, HistPlotOptions};
use plt::dates::{DateFormatter, Datetime64, RRuleLocator};
use plt::figure::{Figure, Share};
use plt::rc;
use plt::scale::{LogScale, NonPositive};
use plt::stats::{Bins, HistOptions};
use plt::style::{self, Style};
use plt::ticker::{
    AutoMinorLocator, Formatter, Locator, LogLabelStyle, MaxNLocator, MultipleLocator, Nbins,
    ScalarFormatter, TickContext, TickerSettings,
};

use super::ticker::{self, integer_label, minor_divisions};
use crate::json::Json;
use crate::probes::areas::text_metrics::rc_text;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// Runs `f` under the reference configuration with the probe's `style`
/// (a list of style names) and `rc` applied.
pub(crate) fn styled<T>(
    args: Args<'_>,
    f: impl FnOnce() -> Result<T, ProbeError>,
) -> Result<T, ProbeError> {
    rc::context(|| {
        rc::reset();
        rc::set("font.family", "DejaVu Sans").map_err(ProbeError::rejected)?;
        rc::set("text.hinting", "none").map_err(ProbeError::rejected)?;
        let styles = args
            .json("style")?
            .as_array()
            .and_then(|names| {
                names
                    .iter()
                    .map(|name| name.as_str().map(Style::from))
                    .collect::<Option<Vec<_>>>()
            })
            .ok_or_else(|| ProbeError::BadArgs("style is not a list of names".into()))?;
        style::use_styles(&styles).map_err(ProbeError::rejected)?;
        for (key, value) in args.get("rc").and_then(Json::as_object).unwrap_or(&[]) {
            rc::set(key, &rc_text(value)?).map_err(ProbeError::rejected)?;
        }
        f()
    })
}

/// The `params` object of a locator or formatter spec, as the argument
/// layout the `ticker` area's builders read.
fn params_args(params: Option<&Json>) -> Json {
    Json::Object(vec![(
        "params".to_owned(),
        params.cloned().unwrap_or(Json::Object(Vec::new())),
    )])
}

/// Sets the major (or minor) locator of `axis`.
fn install_locator(axis: &Axis, minor: bool, locator: impl Locator) {
    if minor {
        axis.set_minor_locator(locator);
    } else {
        axis.set_major_locator(locator);
    }
}

/// Sets the major (or minor) formatter of `axis`.
fn install_formatter(axis: &Axis, minor: bool, formatter: impl Formatter) {
    if minor {
        axis.set_minor_formatter(formatter);
    } else {
        axis.set_major_formatter(formatter);
    }
}

/// A `[name, arg?]` spec.
fn spec_parts(spec: &Json) -> Result<(&str, Option<&Json>), ProbeError> {
    match spec.as_array() {
        Some([name, rest @ ..]) if rest.len() <= 1 => name
            .as_str()
            .map(|name| (name, rest.first()))
            .ok_or_else(|| ProbeError::BadArgs(format!("bad spec {spec}"))),
        _ => Err(ProbeError::BadArgs(format!("bad spec {spec}"))),
    }
}

/// Installs the locator a spec describes: `["AutoMinorLocator", n?]`,
/// `["MultipleLocator", base]`, `["MaxNLocator", nbins]` or
/// `["LogLocator", {base, subs}]`.
fn set_locator(axis: &Axis, minor: bool, spec: &Json) -> Result<(), ProbeError> {
    let bad = || ProbeError::BadArgs(format!("bad locator {spec}"));
    match spec_parts(spec)? {
        ("AutoMinorLocator", n) => {
            install_locator(
                axis,
                minor,
                AutoMinorLocator::new(n.map(minor_divisions).transpose()?),
            );
        }
        ("MultipleLocator", Some(base)) => {
            let base = base.as_f64().ok_or_else(bad)?;
            install_locator(
                axis,
                minor,
                MultipleLocator::new(base).map_err(ProbeError::rejected)?,
            );
        }
        ("MaxNLocator", Some(nbins)) => {
            let nbins = nbins
                .as_i64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(bad)?;
            install_locator(axis, minor, MaxNLocator::new(Nbins::Fixed(nbins)));
        }
        ("LogLocator", params) => {
            let layout = params_args(params);
            install_locator(axis, minor, ticker::log_locator(Args::new(&layout))?);
        }
        _ => return Err(bad()),
    }
    Ok(())
}

/// Installs the formatter a spec describes: `["ScalarFormatter",
/// {scientific, powerlimits}]`, `["LogFormatter", {labelOnlyBase}]` or
/// `["EngFormatter", {...}]`.
fn set_formatter(axis: &Axis, minor: bool, spec: &Json) -> Result<(), ProbeError> {
    let settings = TickerSettings::from(&*rc::params());
    let (name, params) = spec_parts(spec)?;
    let layout = params_args(params);
    let args = Args::new(&layout);
    match name {
        "ScalarFormatter" => install_formatter(axis, minor, ticker::scalar(args, &settings)?),
        "LogFormatter" => install_formatter(
            axis,
            minor,
            ticker::log_formatter(LogLabelStyle::Plain, args, &settings)?,
        ),
        "EngFormatter" => install_formatter(axis, minor, ticker::eng(args, &settings)?),
        _ => return Err(ProbeError::BadArgs(format!("bad formatter {spec}"))),
    }
    Ok(())
}

/// Applies the config of one axis: scale, limits, ticks, minor ticks on
/// or off, locators and formatters, in that order.
fn configure(ax: &Axes, name: AxisName, config: Args<'_>) -> Result<(), ProbeError> {
    let axis = ax.axis_of(name);
    if let Some(scale) = config.opt_str("scale")? {
        match name {
            AxisName::X => ax.set_xscale(scale),
            AxisName::Y => ax.set_yscale(scale),
        }
        .map_err(ProbeError::rejected)?;
    }
    if config.get("lim").is_some() {
        match config.f64_vec("lim")?[..] {
            [low, high] => match name {
                AxisName::X => ax.set_xlim(low, high),
                AxisName::Y => ax.set_ylim(low, high),
            },
            _ => return Err(ProbeError::BadArgs("lim needs two values".into())),
        };
    }
    if config.get("ticks").is_some() {
        axis.set_ticks(config.f64_vec("ticks")?);
    }
    match config.opt_bool("minorticks")? {
        Some(true) => {
            axis.minorticks_on();
        }
        Some(false) => {
            axis.minorticks_off();
        }
        None => {}
    }
    for (key, minor) in [("major", false), ("minor", true)] {
        if let Some(spec) = config.get(key) {
            set_locator(&axis, minor, spec)?;
        }
    }
    for (key, minor) in [("major_formatter", false), ("minor_formatter", true)] {
        if let Some(spec) = config.get(key) {
            set_formatter(&axis, minor, spec)?;
        }
    }
    Ok(())
}

/// The axes of `Figure().add_subplot(*subplot, projection=projection)`.
fn new_axes(figure: &Figure, args: Args<'_>) -> Result<Axes, ProbeError> {
    let subplot = match args.get("subplot") {
        None => vec![1.0, 1.0, 1.0],
        Some(_) => args.f64_vec("subplot")?,
    };
    let [rows, cols, index] = subplot[..] else {
        return Err(ProbeError::BadArgs("subplot needs three numbers".into()));
    };
    match args.opt_str("projection")? {
        None => figure
            .add_subplot(rows as usize, cols as usize, index as usize)
            .map_err(ProbeError::rejected),
        Some("polar") if subplot == [1.0, 1.0, 1.0] => Ok(figure.polar_subplot().axes().clone()),
        Some(other) => Err(ProbeError::BadArgs(format!("projection {other}"))),
    }
}

/// `plot(y)` / `plot(x, y)` and one `boxplot(x, positions=...)` per box.
fn add_data(ax: &Axes, args: Args<'_>) -> Result<(), ProbeError> {
    if let Some(plot) = args.get("plot") {
        let plot = Args::new(plot);
        let y = plot.f64_vec("y")?;
        if plot.get("x").is_some() {
            ax.plot(plot.f64_vec("x")?, y)
                .map_err(ProbeError::rejected)?;
        } else {
            ax.plot_y(y);
        }
    }
    for b in args.get("boxplots").and_then(Json::as_array).unwrap_or(&[]) {
        let b = Args::new(b);
        let positions = b.json("positions")?;
        let options = BoxplotOptions {
            draw: BxpOptions {
                positions: Some(b.f64_vec("positions")?),
                integer_positions: positions
                    .as_array()
                    .is_some_and(|items| items.iter().all(Json::is_integer)),
                ..BxpOptions::default()
            },
            ..BoxplotOptions::default()
        };
        ax.boxplot(&[b.f64_vec("x")?], options)
            .map_err(ProbeError::rejected)?;
    }
    Ok(())
}

/// Builds the probe's axes and answers `query` about the axis named by
/// `axis`.
fn on_axes(args: Args<'_>, query: impl FnOnce(&Axes, AxisName) -> Outcome) -> Outcome {
    styled(args, || {
        let figure = Figure::new();
        let ax = new_axes(&figure, args)?;
        add_data(&ax, args)?;
        if args.opt_bool("minorticks_on")? == Some(true) {
            ax.minorticks_on();
        }
        for (key, name) in [("x", AxisName::X), ("y", AxisName::Y)] {
            if let Some(config) = args.get(key) {
                configure(&ax, name, Args::new(config))?;
            }
        }
        let name = match args.str("axis")? {
            "x" => AxisName::X,
            "y" => AxisName::Y,
            other => return Err(ProbeError::BadArgs(format!("axis {other}"))),
        };
        query(&ax, name)
    })
}

/// `axis.get_ticklocs(minor=minor)`.
pub(super) fn ticklocs(args: Args<'_>) -> Outcome {
    on_axes(args, |ax, name| {
        let locs = ax
            .axis_of(name)
            .ticklocs(args.bool("minor")?)
            .map_err(ProbeError::rejected)?;
        Ok(Json::from(locs))
    })
}

/// `{labels, offset}`: the tick label texts and the offset text after
/// drawing.
pub(super) fn ticklabels(args: Args<'_>) -> Outcome {
    on_axes(args, |ax, name| {
        let labels = ax
            .axis_of(name)
            .ticklabels(args.bool("minor")?)
            .map_err(ProbeError::rejected)?;
        let offset = ax.ticks(name).map_err(ProbeError::rejected)?.offset;
        Ok(encode::object([
            ("labels", Json::from(labels)),
            ("offset", Json::from(offset)),
        ]))
    })
}

/// `axis.get_major_formatter().format_data(value)`.
pub(super) fn format_data(args: Args<'_>) -> Outcome {
    on_axes(args, |ax, name| {
        let settings = TickerSettings::from(&*rc::params());
        let formatter = ax.axis_of(name).major_formatter();
        let text = formatter
            .format_data(args.f64("value")?, &TickContext::detached(&settings))
            .map_err(ProbeError::rejected)?;
        Ok(Json::from(text.trim()))
    })
}

/// `axis.get_major_formatter().format_data_short(value)`: what the status
/// bar shows, in as many digits as a screen pixel resolves.
pub(super) fn format_data_short(args: Args<'_>) -> Outcome {
    on_axes(args, |ax, name| {
        let formatter = ax.axis_of(name).major_formatter();
        let scalar = (formatter.as_ref() as &dyn Any).is::<ScalarFormatter>();
        let settings = TickerSettings::from(&*rc::params());
        if let Some(label) = integer_label(args.json("value")?, &settings).filter(|_| scalar) {
            return Ok(Json::from(label));
        }
        let text = ax
            .format_data(name, args.f64("value")?)
            .ok_or_else(|| ProbeError::rejected("the value cannot be formatted"))?;
        Ok(Json::from(text.trim()))
    })
}

/// `{yticks, ylim}` of `len(data)` axes sharing x and y, each with a
/// histogram of its data on a clipping log y scale, in the classic style.
pub(super) fn shared_hist(args: Args<'_>) -> Outcome {
    let datasets = args
        .json("data")?
        .as_array()
        .and_then(|sets| {
            sets.iter()
                .map(Json::as_f64_vec)
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| ProbeError::BadArgs("data is not a list of lists".into()))?;
    let bins = usize::try_from(args.i64("bins")?).map_err(ProbeError::rejected)?;
    let classic = Json::Array(vec!["classic".into(), "_classic_test_patch".into()]);
    let layout = Json::Object(vec![("style".to_owned(), classic)]);
    styled(Args::new(&layout), || {
        let figure = Figure::new();
        let axes = figure
            .subplots_shared(datasets.len(), 1, Share::All, Share::All)
            .map_err(ProbeError::rejected)?;
        let options = HistPlotOptions {
            layout: HistOptions {
                bins: Some(Bins::Count(bins)),
                ..HistOptions::default()
            },
            ..HistPlotOptions::default()
        };
        let mut yticks = Vec::new();
        let mut ylims = Vec::new();
        for (row, data) in datasets.iter().enumerate() {
            let ax = &axes[(row, 0)];
            ax.hist(&[data], options.clone())
                .map_err(ProbeError::rejected)?;
            let log = LogScale::new(10.0)
                .map_err(ProbeError::rejected)?
                .with_nonpositive(NonPositive::Clip);
            ax.set_scale(AxisName::Y, Box::new(log));
        }
        for row in 0..datasets.len() {
            let ax = &axes[(row, 0)];
            let ticks = ax.yaxis().ticklocs(false).map_err(ProbeError::rejected)?;
            yticks.push(Json::from(ticks));
            let (bottom, top) = ax.ylim();
            ylims.push(Json::from(vec![bottom, top]));
        }
        Ok(encode::object([
            ("yticks", Json::Array(yticks)),
            ("ylim", Json::Array(ylims)),
        ]))
    })
}

/// `{locs, labels}` of the minor ticks of `test_remove_overlap`'s date
/// axis: three days plotted, a tick and a weekday label every day, minor
/// ticks every six hours, overlapping locations kept or removed.
pub(super) fn remove_overlap(args: Args<'_>) -> Outcome {
    let classic = Json::Array(vec!["classic".into(), "_classic_test_patch".into()]);
    let layout = Json::Object(vec![("style".to_owned(), classic)]);
    styled(Args::new(&layout), || {
        let days = ["2018-11-03", "2018-11-04", "2018-11-05"]
            .into_iter()
            .map(|day| {
                Datetime64::parse(day)
                    .map_err(ProbeError::rejected)?
                    .ok_or_else(|| ProbeError::BadArgs("NaT".into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let ax = Figure::new().subplot();
        ax.plot(days, vec![1.0; 3]).map_err(ProbeError::rejected)?;
        let axis = ax.xaxis();
        axis.set_major_locator(RRuleLocator::days(&[], 1).map_err(ProbeError::rejected)?)
            .set_major_formatter(DateFormatter::new("\n%a"))
            .set_minor_locator(RRuleLocator::hours(&[0, 6, 12, 18], 1))
            .set_minor_formatter(DateFormatter::new("%H:%M"));
        if let Some(remove) = args.opt_bool("remove_overlapping_locs")? {
            axis.set_remove_overlapping_locs(remove);
        }
        Ok(encode::object([
            (
                "locs",
                Json::from(axis.ticklocs(true).map_err(ProbeError::rejected)?),
            ),
            (
                "labels",
                Json::from(axis.ticklabels(true).map_err(ProbeError::rejected)?),
            ),
        ]))
    })
}
