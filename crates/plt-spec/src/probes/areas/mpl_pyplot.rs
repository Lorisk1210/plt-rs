//! Probe operations for the `mpl_pyplot` area: matplotlib's own tests of
//! pyplot's state, figures and axes (see
//! `spec/reference/probes/test_mpl_pyplot.py`).

use plt::artist::ArtistHandle;
use plt::axes::{Axes, Axis, ImshowOptions, ProjectionKind, StackplotOptions};
use plt::figure::{Figure, Mosaic, MosaicOptions, SubfigureOptions};
use plt::numeric::Grid;
use plt::pyplot;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("pyplot.box", box_toggle),
    Op::new("pyplot.stackplot", stackplot),
    Op::new("pyplot.isinteractive", isinteractive),
    Op::new("pyplot.subplot", subplot),
    Op::new("pyplot.axes", axes),
    Op::new("pyplot.gca", gca),
    Op::new("pyplot.polar", polar),
    Op::new("pyplot.figure", figure),
    Op::new("pyplot.sca", sca),
    Op::new("pyplot.xticks", xticks),
    Op::new("pyplot.yticks", yticks),
    Op::new("pyplot.switch_backend", switch_backend),
    Op::new("pyplot.matshow", matshow),
    Op::new("pyplot.subplots", subplots),
    Op::new("pyplot.subplot_mosaic", subplot_mosaic),
];

/// Runs `scenario` between two `close_all()`, as every op does.
fn fresh(scenario: impl FnOnce() -> Outcome) -> Outcome {
    pyplot::close_all();
    let outcome = scenario();
    pyplot::close_all();
    outcome
}

/// A boolean as JSON.
fn yes(value: bool) -> Json {
    Json::from(value)
}

/// A count as JSON.
fn count(n: usize) -> Json {
    Json::from(n as f64)
}

/// A pair of numbers as JSON.
fn pair((a, b): (f64, f64)) -> Json {
    Json::Array(vec![Json::from(a), Json::from(b)])
}

/// The numbers of the open figures as JSON.
fn fignums() -> Json {
    Json::Array(
        pyplot::get_fignums()
            .into_iter()
            .map(|n| Json::from(f64::from(n)))
            .collect(),
    )
}

/// The number of the current figure.
fn current_number() -> u32 {
    pyplot::get_current_fig_manager().num()
}

fn box_toggle(_: Args<'_>) -> Outcome {
    fresh(|| {
        let (_, axes) = pyplot::subplots(1, 1).map_err(ProbeError::rejected)?;
        let ax = &axes[(0, 0)];
        Ok(Json::Array(
            [Some(false), Some(true), None, None]
                .into_iter()
                .map(|on| {
                    pyplot::r#box(on);
                    yes(ax.frame_on())
                })
                .collect(),
        ))
    })
}

fn stackplot(args: Args<'_>) -> Outcome {
    fresh(|| {
        pyplot::stackplot(
            args.f64_vec("x")?,
            &[args.f64_vec("y")?],
            StackplotOptions::default(),
        )
        .map_err(ProbeError::rejected)?;
        let ax = pyplot::gca();
        Ok(Json::Array(vec![
            count(ax.collections().len()),
            pair(ax.xlim()),
            pair(ax.ylim()),
        ]))
    })
}

/// Runs the interactive-mode `steps`, recording the mode at every
/// `"check"` in `seen`.
fn run_steps(steps: &Json, seen: &mut Vec<Json>) -> Result<(), ProbeError> {
    let steps = steps
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("{steps} is not a list of steps")))?;
    for step in steps {
        match (step.as_str(), step.as_array()) {
            (Some("check"), _) => seen.push(yes(pyplot::isinteractive())),
            (Some("ion"), _) => pyplot::ion(),
            (Some("ioff"), _) => pyplot::ioff(),
            (_, Some([kind, inner])) => {
                let on = match kind.as_str() {
                    Some("with_ion") => true,
                    Some("with_ioff") => false,
                    _ => return Err(ProbeError::BadArgs(format!("unknown block {kind}"))),
                };
                pyplot::interactive_context(on, || run_steps(inner, seen))?;
            }
            _ => return Err(ProbeError::BadArgs(format!("unknown step {step}"))),
        }
    }
    Ok(())
}

fn isinteractive(args: Args<'_>) -> Outcome {
    let mut seen = Vec::new();
    let outcome = run_steps(args.json("steps")?, &mut seen);
    pyplot::ioff();
    outcome.map(|()| Json::Array(seen))
}

/// `plt.subplot(nrows, ncols, index)`.
fn cell(nrows: usize, ncols: usize, index: usize) -> Result<Axes, ProbeError> {
    pyplot::subplot(nrows, ncols, index).map_err(ProbeError::rejected)
}

/// `plt.subplot(nrows, ncols, index, projection=projection)`.
fn projected_cell(
    nrows: usize,
    ncols: usize,
    index: usize,
    projection: ProjectionKind,
) -> Result<Axes, ProbeError> {
    pyplot::subplot_with_projection(nrows, ncols, index, projection).map_err(ProbeError::rejected)
}

/// Whether the current figure has `ax`.
fn in_figure(ax: &Axes) -> bool {
    pyplot::gcf().axes().contains(ax)
}

/// A projection's name as JSON.
fn name(ax: &Axes) -> Json {
    Json::from(ax.projection().name())
}

fn subplot_reuse() -> Result<Vec<Json>, ProbeError> {
    let ax1 = cell(1, 2, 1)?;
    let first = ax1 == pyplot::gca();
    let ax2 = cell(1, 2, 2)?;
    let second = ax2 == pyplot::gca();
    let ax3 = cell(1, 2, 1)?;
    Ok(vec![
        yes(first),
        yes(second),
        yes(ax1 == pyplot::gca()),
        yes(ax1 == ax3),
    ])
}

fn subplot_replace_projection() -> Result<Vec<Json>, ProbeError> {
    pyplot::figure();
    let ax = cell(1, 2, 1)?;
    let ax1 = cell(1, 2, 1)?;
    let ax2 = cell(1, 2, 2)?;
    let ax3 = projected_cell(1, 2, 1, ProjectionKind::Polar)?;
    let ax4 = projected_cell(1, 2, 1, ProjectionKind::Polar)?;
    Ok(vec![
        yes(ax1 == ax),
        yes(ax2 != ax),
        yes(ax3 != ax),
        yes(ax3 == ax4),
        yes(in_figure(&ax)),
        yes(in_figure(&ax2)),
        yes(in_figure(&ax3)),
        name(&ax),
        name(&ax2),
        name(&ax3),
    ])
}

fn subplot_projection_reuse() -> Result<Vec<Json>, ProbeError> {
    let ax1 = cell(1, 1, 1)?;
    let mut seen = vec![yes(ax1 == pyplot::gca()), yes(ax1 == cell(1, 1, 1)?)];
    ax1.remove();
    let ax2 = projected_cell(1, 1, 1, ProjectionKind::Polar)?;
    seen.extend([
        yes(ax2 == pyplot::gca()),
        yes(!in_figure(&ax1)),
        yes(ax2 == cell(1, 1, 1)?),
    ]);
    ax2.remove();
    let ax3 = projected_cell(1, 1, 1, ProjectionKind::Rectilinear)?;
    seen.extend([
        yes(ax3 == pyplot::gca()),
        yes(ax3 != ax2),
        yes(!in_figure(&ax2)),
    ]);
    Ok(seen)
}

/// `polar=True` asks for the polar projection, as `projection="polar"`
/// does: in plt-rs both are [`ProjectionKind::Polar`].
fn subplot_polar_normalization() -> Result<Vec<Json>, ProbeError> {
    let ax1 = projected_cell(1, 1, 1, ProjectionKind::Polar)?;
    let ax2 = projected_cell(1, 1, 1, ProjectionKind::Polar)?;
    let ax3 = projected_cell(1, 1, 1, ProjectionKind::Polar)?;
    Ok(vec![yes(ax1 == ax2), yes(ax1 == ax3)])
}

fn subplot_change_projection() -> Result<Vec<Json>, ProbeError> {
    let mut ax = cell(1, 1, 1)?;
    let mut created = vec![ax.clone()];
    let mut seen = Vec::new();
    for projection in [
        ProjectionKind::Aitoff,
        ProjectionKind::Hammer,
        ProjectionKind::Lambert,
        ProjectionKind::Mollweide,
        ProjectionKind::Polar,
        ProjectionKind::Rectilinear,
    ] {
        ax.remove();
        ax = projected_cell(1, 1, 1, projection)?;
        seen.push(Json::Array(vec![yes(ax == cell(1, 1, 1)?), name(&ax)]));
        if !created.contains(&ax) {
            created.push(ax.clone());
        }
    }
    Ok(vec![Json::Array(seen), count(created.len())])
}

fn subplot(args: Args<'_>) -> Outcome {
    let scenario = match args.str("case")? {
        "reuse" => subplot_reuse,
        "replace_projection" => subplot_replace_projection,
        "projection_reuse" => subplot_projection_reuse,
        "polar_normalization" => subplot_polar_normalization,
        "change_projection" => subplot_change_projection,
        other => return Err(ProbeError::BadArgs(format!("unknown case {other}"))),
    };
    fresh(|| scenario().map(Json::Array))
}

fn axes(_: Args<'_>) -> Outcome {
    fresh(|| {
        let mut seen = Vec::new();
        for (first, second) in [
            (ProjectionKind::Rectilinear, ProjectionKind::Rectilinear),
            (ProjectionKind::Polar, ProjectionKind::Polar),
            (ProjectionKind::Polar, ProjectionKind::Rectilinear),
        ] {
            pyplot::figure();
            let ax = pyplot::axes_with_projection(None, first).map_err(ProbeError::rejected)?;
            let ax1 = pyplot::axes_with_projection(None, second).map_err(ProbeError::rejected)?;
            seen.extend([yes(ax1 != ax), name(&ax1)]);
            pyplot::close();
        }
        Ok(Json::Array(seen))
    })
}

fn gca(_: Args<'_>) -> Outcome {
    fresh(|| {
        pyplot::figure();
        let ax = pyplot::gca();
        Ok(Json::Array(vec![
            yes(pyplot::gca() == ax),
            count(pyplot::gcf().axes().len()),
        ]))
    })
}

fn polar(_: Args<'_>) -> Outcome {
    fresh(|| {
        let first = pyplot::polar(vec![0.0], vec![1.0]).map_err(ProbeError::rejected)?;
        let second = pyplot::polar(vec![1.57], vec![0.5]).map_err(ProbeError::rejected)?;
        let ax = pyplot::gca();
        let lines = ax.lines();
        Ok(Json::Array(vec![
            yes(lines.contains(&first) && lines.contains(&second)),
            name(&ax),
        ]))
    })
}

fn figure_subfigure() -> Result<Vec<Json>, ProbeError> {
    let fig1 = pyplot::figure();
    let subfigs = fig1
        .subfigures(2, 1, SubfigureOptions::default())
        .map_err(ProbeError::rejected)?;
    pyplot::figure();
    let other = pyplot::gcf() != fig1;
    pyplot::figure_activate(&subfigs[(1, 0)]);
    let current = pyplot::gcf();
    Ok(vec![
        yes(other),
        yes(current == fig1),
        yes(current.id() == fig1.id()),
    ])
}

fn figure_register_existing() -> Result<Vec<Json>, ProbeError> {
    let fig = Figure::new();
    let number = pyplot::figure_activate(&fig);
    pyplot::suptitle("my title");
    pyplot::figure();
    let fig2_number = current_number();
    Ok(vec![
        Json::from(f64::from(number)),
        Json::String(fig.suptitle_text()),
        Json::from(f64::from(fig2_number)),
        yes(pyplot::figure_num(1) == fig),
        yes(pyplot::gcf() == fig),
    ])
}

fn figure_numbering() -> Result<Vec<Json>, ProbeError> {
    pyplot::figure();
    pyplot::figure();
    pyplot::close_num(1);
    pyplot::figure();
    let after_close = current_number();
    let open = fignums();
    pyplot::close_all();
    pyplot::figure_num(5);
    pyplot::figure();
    let after_five = current_number();
    pyplot::close_all();
    pyplot::figure_label("a");
    pyplot::figure_num(7);
    pyplot::figure_label("b");
    let labelled = current_number();
    let labels = pyplot::get_figlabels()
        .into_iter()
        .map(Json::String)
        .collect();
    Ok(vec![
        Json::from(f64::from(after_close)),
        open,
        Json::from(f64::from(after_five)),
        Json::from(f64::from(labelled)),
        Json::Array(labels),
    ])
}

fn figure(args: Args<'_>) -> Outcome {
    let scenario = match args.str("case")? {
        "subfigure" => figure_subfigure,
        "register_existing" => figure_register_existing,
        "numbering" => figure_numbering,
        other => return Err(ProbeError::BadArgs(format!("unknown case {other}"))),
    };
    fresh(|| scenario().map(Json::Array))
}

fn sca(_: Args<'_>) -> Outcome {
    fresh(|| {
        let fig = pyplot::figure();
        let subfigs = fig
            .subfigures(2, 1, SubfigureOptions::default())
            .map_err(ProbeError::rejected)?;
        let single = |sub: &Figure| {
            sub.subplots(1, 1)
                .map(|axes| axes[(0, 0)].clone())
                .map_err(ProbeError::rejected)
        };
        let ax = single(&subfigs[(0, 0)])?;
        single(&subfigs[(1, 0)])?;
        let before = pyplot::gca() != ax;
        pyplot::sca(&ax);
        Ok(Json::Array(vec![yes(before), yes(pyplot::gca() == ax)]))
    })
}

/// `plt.figure(); plt.plot(np.arange(1, 10))`.
fn plotted_range() {
    pyplot::figure();
    let y: Vec<f64> = (1..10).map(f64::from).collect();
    pyplot::plot_y(y);
}

/// The minor tick locations and labels of `axis` as JSON.
fn minor_ticks(axis: &Axis) -> Outcome {
    let locs = axis.ticklocs(true).map_err(ProbeError::rejected)?;
    let labels = axis.ticklabels(true).map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        Json::Array(locs.into_iter().map(Json::from).collect()),
        Json::Array(labels.into_iter().map(Json::String).collect()),
    ]))
}

fn xticks(_: Args<'_>) -> Outcome {
    fresh(|| {
        plotted_range();
        minor_ticks(&pyplot::gca().xaxis())
    })
}

fn yticks(args: Args<'_>) -> Outcome {
    fresh(|| {
        plotted_range();
        let labels = args
            .json("labels")?
            .as_array()
            .and_then(|labels| labels.iter().map(Json::as_str).collect::<Option<Vec<_>>>())
            .ok_or_else(|| ProbeError::BadArgs("labels are not strings".into()))?;
        pyplot::yticks_minor(args.f64_vec("ticks")?, &labels).map_err(ProbeError::rejected)?;
        minor_ticks(&pyplot::gca().yaxis())
    })
}

fn switch_backend(args: Args<'_>) -> Outcome {
    let backends = args
        .json("backends")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("backends is not a list".into()))?;
    let previous = pyplot::get_backend();
    let outcome = fresh(|| {
        pyplot::figure();
        pyplot::figure();
        let mut seen = vec![count(pyplot::get_fignums().len())];
        for backend in backends {
            let backend = backend
                .as_str()
                .ok_or_else(|| ProbeError::BadArgs(format!("{backend} is not a name")))?
                .parse()
                .map_err(ProbeError::rejected)?;
            pyplot::switch_backend(backend);
            seen.push(count(pyplot::get_fignums().len()));
        }
        Ok(Json::Array(seen))
    });
    pyplot::switch_backend(previous);
    outcome
}

fn matshow(args: Args<'_>) -> Outcome {
    let existing = args.bool("existing")?;
    fresh(|| {
        let fignum = existing.then(|| {
            pyplot::figure();
            current_number()
        });
        let data = Grid::from_vec(2, 2, vec![0.0, 1.0, 1.0, 2.0]).map_err(ProbeError::rejected)?;
        pyplot::matshow(data, fignum, ImshowOptions::default()).map_err(ProbeError::rejected)?;
        let fig = pyplot::gcf();
        Ok(Json::Array(vec![
            pair(fig.size_inches()),
            count(fig.axes().len()),
            fignums(),
        ]))
    })
}

/// Runs `make` on figure `num` (made first when argument `existing` is
/// true), with argument `clear`, recording whether it returned the figure
/// made first and its number of axes.
fn on_numbered_figure(
    args: Args<'_>,
    num: u32,
    make: impl FnOnce(bool) -> Result<Figure, pyplot::PyplotError>,
) -> Outcome {
    let (existing, clear) = (args.bool("existing")?, args.bool("clear")?);
    fresh(|| {
        let first = existing.then(|| pyplot::figure_num(num));
        let fig = make(clear).map_err(ProbeError::rejected)?;
        Ok(Json::Array(vec![
            yes(first.is_some_and(|first| first == fig)),
            count(fig.axes().len()),
        ]))
    })
}

fn subplots(args: Args<'_>) -> Outcome {
    on_numbered_figure(args, 1, |clear| {
        pyplot::subplots_num(1, clear, 1, 1).map(|(fig, _)| fig)
    })
}

fn subplot_mosaic(args: Args<'_>) -> Outcome {
    let mosaic = "AB".parse::<Mosaic>().map_err(ProbeError::rejected)?;
    on_numbered_figure(args, 2, |clear| {
        pyplot::subplot_mosaic_num(2, clear, &mosaic, MosaicOptions::default()).map(|(fig, _)| fig)
    })
}
