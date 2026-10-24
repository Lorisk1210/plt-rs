//! Probe operations for the `line_behaviour` area: hit testing, window
//! extents, tight bounding boxes and drawing outcomes of `Line2D`, ported
//! from matplotlib's `test_lines.py` (see
//! `spec/reference/probes/test_line_behaviour.py`). Every op runs under the
//! reference configuration.

use plt::artist::ArtistHandle;
use plt::colors::Color;
use plt::figure::Figure;
use plt::lines::{AxLineThrough, DrawStyle, Line2D, LineProps, LineStyle, MarkEvery};
use plt::numeric::linspace;
use plt::transforms::Transform;
use plt_core::Point;

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::{bbox, object, parse_points};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("line.contains", |args| with_rc(args, || contains(args))),
    Op::new("line.window_extent", |args| {
        with_rc(args, || window_extent(args))
    }),
    Op::new("line.axline_small_slope", |args| {
        with_rc(args, axline_small_slope)
    }),
    Op::new("line.tightbbox", |args| with_rc(args, || tightbbox(args))),
    Op::new("line.draw", |args| with_rc(args, || draw(args))),
];

type Lines = Result<Vec<(&'static str, Line2D)>, ProbeError>;

fn plotted(result: Result<Line2D, impl std::fmt::Display>) -> Result<Line2D, ProbeError> {
    result.map_err(ProbeError::rejected)
}

fn segment_hits(fig: &Figure) -> Lines {
    let line = plotted(fig.add_line(&[553.0, 553.0], &[95.0, 947.0], LineProps::default()))?;
    line.set_transform(Transform::Display).set_pickradius(6.94);
    Ok(vec![("line", line)])
}

fn picking(fig: &Figure) -> Lines {
    let ax = fig.subplot();
    let plot = || plotted(ax.plot([0.0, 1.0], [0.0, 1.0]));
    let default = plot()?;
    let wide = plot()?;
    wide.set_pickradius(20.0);
    let later = plot()?;
    later.set_pickradius(20.0);
    Ok(vec![("default", default), ("wide", wide), ("later", later)])
}

fn markers_only(fig: &Figure) -> Lines {
    let line = plotted(
        fig.subplot()
            .plot_fmt([0.2, 0.5, 0.8], [0.3, 0.6, 0.4], "o"),
    )?;
    line.set_linestyle(LineStyle::None);
    Ok(vec![("markers", line)])
}

fn steps(fig: &Figure) -> Lines {
    let ax = fig.subplot();
    let line = plotted(ax.plot([0.0, 1.0, 2.0, 3.0], [0.0, 1.0, 0.0, 1.0]))?;
    line.set_drawstyle(DrawStyle::StepsMid);
    ax.set_xlim(-1.0, 4.0).map_err(ProbeError::rejected)?;
    Ok(vec![("steps", line)])
}

/// `line.contains(event)` per line and point.
fn contains(args: Args<'_>) -> Outcome {
    let scene = args.str("scene")?;
    let fig = if scene == "segment_hits" {
        Figure::with_size(10.0, 15.0, 72.0)
    } else {
        Figure::new()
    };
    let lines = match scene {
        "segment_hits" => segment_hits(&fig),
        "picking" => picking(&fig),
        "markers_only" => markers_only(&fig),
        "steps" => steps(&fig),
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }?;
    fig.layout();
    let mut points = parse_points(args.json("points")?)?;
    if args.str("coords")? == "data" {
        let data = fig
            .resolve_transform(&fig.axes()[0].trans_data())
            .map_err(ProbeError::rejected)?;
        points = points
            .into_iter()
            .map(|p| data.transform_point(p))
            .collect();
    }
    let hits = lines.iter().map(|(gid, line)| {
        let per_point = points
            .iter()
            .map(|&at| {
                line.contains(at).map_or(Json::Null, |indices| {
                    Json::Array(indices.into_iter().map(index).collect())
                })
            })
            .collect();
        ((*gid).to_owned(), Json::Array(per_point))
    });
    Ok(Json::Object(hits.collect()))
}

fn index(i: usize) -> Json {
    Json::Integer(i64::try_from(i).expect("indices fit in i64"))
}

fn plain(fig: &Figure) -> Result<Vec<Line2D>, ProbeError> {
    let ax = fig.subplot();
    let (x, y) = ([0.0, 1.0, 2.0], [0.0, 2.0, 1.0]);
    let steps = plotted(ax.plot(x, y))?;
    steps.set_drawstyle(DrawStyle::StepsPost);
    let lines = vec![
        plotted(ax.plot(x, y))?,
        plotted(ax.plot_fmt(x, y, "o"))?,
        plotted(
            ax.plot_with(
                x,
                y,
                LineProps::fmt("s")
                    .map_err(ProbeError::rejected)?
                    .markersize(12.0),
            ),
        )?,
        steps,
    ];
    Ok(lines)
}

fn nonfinite(fig: &Figure) -> Result<Vec<Line2D>, ProbeError> {
    let ax = fig.subplot();
    Ok(vec![
        plotted(ax.plot([0.0, 1.0, f64::NAN, 3.0], [1.0, 0.0, 2.0, 2.0]))?,
        plotted(ax.plot_fmt([0.0, 1.0, 2.0], [1.0, f64::INFINITY, 2.0], "o-"))?,
        plotted(ax.plot([f64::NAN, f64::NAN], [1.0, 2.0]))?,
    ])
}

fn axlines(fig: &Figure) -> Result<Vec<Line2D>, ProbeError> {
    let ax = fig.subplot();
    let axline = |xy1: (f64, f64), through| {
        ax.axline(xy1.into(), through, LineProps::default())
            .map_err(ProbeError::rejected)
    };
    let lines = vec![
        axline((0.2, 0.3), AxLineThrough::Point(Point::new(0.6, 0.9)))?,
        axline((0.5, 0.5), AxLineThrough::Slope(-2.0))?,
        axline((0.4, 0.0), AxLineThrough::Slope(f64::INFINITY))?,
        axline((0.0, 1.5), AxLineThrough::Slope(0.0))?,
    ];
    ax.set_xlim(0.0, 1.0)
        .and_then(|ax| ax.set_ylim(0.0, 2.0))
        .map_err(ProbeError::rejected)?;
    Ok(lines)
}

fn figure_line(fig: &Figure) -> Result<Vec<Line2D>, ProbeError> {
    let props = LineProps::fmt("o-").map_err(ProbeError::rejected)?;
    Ok(vec![plotted(fig.add_line(
        &[0.1, 0.9],
        &[0.2, 0.8],
        props,
    ))?])
}

/// `get_window_extent()` of every line of a scene.
fn window_extent(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let lines = match args.str("scene")? {
        "plain" => plain(&fig)?,
        "nonfinite" => nonfinite(&fig)?,
        "axlines" => axlines(&fig)?,
        "figure_line" => figure_line(&fig)?,
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    };
    let extents = lines
        .iter()
        .map(|line| bbox(&line.window_extent().unwrap_or(plt_core::Bbox::NULL)))
        .collect();
    Ok(Json::Array(extents))
}

/// Whether `axline((0, 0), slope=1e-14)` rises, by less than 4e-12 pixels.
fn axline_small_slope() -> Outcome {
    let fig = Figure::new();
    let line = fig
        .subplot()
        .axline(
            Point::ORIGIN,
            AxLineThrough::Slope(1e-14),
            LineProps::default(),
        )
        .map_err(ProbeError::rejected)?;
    let extent = line
        .window_extent()
        .ok_or_else(|| ProbeError::rejected("the line has no extent"))?;
    let rise = extent.height();
    Ok(object([
        ("rises", Json::Bool(rise > 0.0)),
        ("below_4e-12", Json::Bool(rise < 4e-12)),
    ]))
}

/// `fig.get_tightbbox()` with a clipped or unclipped line reaching
/// outside the axes.
fn tightbbox(args: Args<'_>) -> Outcome {
    let clip_on = match args.str("scene")? {
        "clipped" => true,
        "unclipped" => false,
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    };
    let fig = Figure::new();
    let ax = fig.subplot();
    let props = LineProps::fmt("o-")
        .map_err(ProbeError::rejected)?
        .markersize(20.0);
    plotted(ax.plot_with([-0.5, 0.5, 1.5], [0.5, 1.4, 0.5], props))?.set_clip_on(clip_on);
    ax.set_xlim(0.0, 1.0)
        .and_then(|ax| ax.set_ylim(0.0, 1.0))
        .map_err(ProbeError::rejected)?;
    Ok(bbox(&fig.tight_bbox()))
}

/// Whether a figure holding the scene draws.
fn draw(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    match args.str("scene")? {
        "zero_width_dashed" => {
            let props = LineProps::fmt("--")
                .map_err(ProbeError::rejected)?
                .linewidth(0.0);
            plotted(fig.subplot().plot_with([0.0, 1.0], [0.0, 1.0], props))?;
        }
        "figure_markevery_float" => {
            let props = LineProps::fmt("o-")
                .map_err(ProbeError::rejected)?
                .markevery(Some(MarkEvery::Distance {
                    start: 0.0,
                    step: 0.5,
                }));
            plotted(fig.add_line(&[0.0, 1.0], &[0.0, 1.0], props))?;
        }
        "extreme_zoom" => {
            let ax = fig.subplot();
            let x = linspace(0.0, 10.0, 200);
            let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
            let props = LineProps::fmt("o-")
                .map_err(ProbeError::rejected)?
                .markevery(Some(MarkEvery::Distance {
                    start: 0.0,
                    step: 0.05,
                }));
            plotted(ax.plot_with(x, y, props))?;
            ax.set_xlim(5.0, 5.000_001).map_err(ProbeError::rejected)?;
        }
        "color_variants" => {
            let ax = fig.subplot();
            let colors: [Color; 5] = [
                "none".into(),
                "r".into(),
                ".3".into(),
                (1.0, 0.0, 0.0, 1.0).into(),
                (1.0, 0.0, 0.0).into(),
            ];
            for color in colors {
                plotted(ax.plot_with(
                    linspace(0.0, 9.0, 10),
                    linspace(0.0, 9.0, 10),
                    LineProps::default().color(color),
                ))?;
            }
        }
        "nan_data" => {
            let y: Vec<f64> = (0..2000).map(f64::from).collect();
            plotted(fig.subplot().plot(vec![f64::NAN; 2000], y))?;
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }
    let outcome = if fig.render_raster().is_ok() {
        "drawn"
    } else {
        "error"
    };
    Ok(outcome.into())
}
