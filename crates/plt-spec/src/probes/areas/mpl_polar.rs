//! Probe operations for the `mpl_polar` area: matplotlib's own tests of
//! polar axes that the `polar_behaviour` area leaves out (see
//! `spec/reference/probes/test_mpl_polar.py`).

use std::rc::Rc;

use plt::artist::ArtistHandle;
use plt::axes::{Adjustable, Axes, AxisName, GridOptions, ProjectionKind};
use plt::figure::{Figure, Mosaic, MosaicOptions, Share};
use plt::polar::{Compass, PolarAxes, ThetaDirection};
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::ticker::{FuncFormatter, MaxNLocator, Nbins, TickerError};

use super::scenario::axis_mode;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("axes.Axes.set_adjustable", set_adjustable),
    Op::new("axes.Axes.axis", axis),
    Op::new("axes.Axes.grid", grid_alpha),
    Op::new("spines.Spine.get_tightbbox", spine_extents),
    Op::new("axes.Axes.axvspan._interpolation_steps", span_steps),
    Op::new("figure.Figure.subplots.get_ticklabels", shared_ticklabels),
    Op::new("axes.Axes.format_coord", format_coord),
    Op::new("axis.RadialAxis.get_majorticklocs", radial_locators),
    Op::new("axis.RadialAxis.set_ticklabels", radial_ticklabels),
];

/// Draws `fig`, bringing its ticks up to date.
fn draw(fig: &Figure) -> Result<(), ProbeError> {
    fig.render_raster()
        .map(|_| ())
        .map_err(ProbeError::rejected)
}

/// Tick labels as recorded.
fn labels(labels: Result<Vec<String>, TickerError>) -> Outcome {
    Ok(Json::from(labels.map_err(ProbeError::rejected)?))
}

/// An axes of the probe's `projection` on `fig`.
fn projected(fig: &Figure, args: Args<'_>) -> Result<Axes, ProbeError> {
    match args.opt_str("projection")? {
        None => Ok(fig.subplot()),
        Some("polar") => Ok(Axes::clone(&fig.polar_subplot())),
        Some(other) => Err(ProbeError::BadArgs(format!("unknown projection {other:?}"))),
    }
}

/// The name of an adjustable.
fn adjustable_name(ax: &Axes) -> Json {
    Json::from(match ax.adjustable() {
        Adjustable::Box => "box",
        Adjustable::Datalim => "datalim",
    })
}

fn set_adjustable(args: Args<'_>) -> Outcome {
    let ax = projected(&Figure::new(), args)?;
    let adjustable = match args.str("adjustable")? {
        "box" => Adjustable::Box,
        "datalim" => Adjustable::Datalim,
        other => return Err(ProbeError::BadArgs(format!("unknown adjustable {other:?}"))),
    };
    ax.set_adjustable(adjustable)
        .map_err(ProbeError::rejected)?;
    Ok(adjustable_name(&ax))
}

/// The adjustable and aspect after `ax.axis(arg)`.
fn axis(args: Args<'_>) -> Outcome {
    let ax = projected(&Figure::new(), args)?;
    ax.axis_mode(axis_mode(args.str("arg")?)?)
        .map_err(ProbeError::rejected)?;
    let aspect = match ax.aspect() {
        Aspect::Equal => Json::from(1.0),
        Aspect::Auto => Json::from("auto"),
        Aspect::Ratio(ratio) => Json::from(ratio),
    };
    Ok(Json::Array(vec![adjustable_name(&ax), aspect]))
}

/// The alpha of the first major x and y gridlines after `ax.grid(alpha=)`.
fn grid_alpha(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.polar_subplot();
    let options = GridOptions {
        alpha: Some(args.f64("alpha")?),
        ..GridOptions::default()
    };
    ax.grid_with(true, options);
    draw(&fig)?;
    let first_alpha = |name| -> Result<Json, ProbeError> {
        let lines = ax.axis_of(name).gridlines().map_err(ProbeError::rejected)?;
        let first = lines
            .first()
            .ok_or_else(|| ProbeError::rejected("no gridlines"))?;
        Ok(Json::from(first.style.alpha))
    };
    Ok(Json::Array(vec![
        first_alpha(AxisName::X)?,
        first_alpha(AxisName::Y)?,
    ]))
}

/// A compass point by matplotlib's name.
fn compass(name: &str) -> Result<Compass, ProbeError> {
    Ok(match name {
        "N" => Compass::N,
        "NW" => Compass::NW,
        "W" => Compass::W,
        "SW" => Compass::SW,
        "S" => Compass::S,
        "SE" => Compass::SE,
        "E" => Compass::E,
        "NE" => Compass::NE,
        other => return Err(ProbeError::BadArgs(format!("unknown location {other:?}"))),
    })
}

/// The extents of the outer polar spine of turned polar axes.
fn spine_extents(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.polar_subplot();
    match args.opt_i64("theta_direction")? {
        None => {}
        Some(-1) => {
            ax.set_theta_direction(ThetaDirection::Clockwise);
        }
        Some(1) => {
            ax.set_theta_direction(ThetaDirection::Counterclockwise);
        }
        Some(other) => return Err(ProbeError::BadArgs(format!("bad direction {other}"))),
    }
    if let Some(location) = args.get("theta_zero_location") {
        let Some([name, offset]) = location.as_array() else {
            return Err(ProbeError::BadArgs(format!("bad location {location}")));
        };
        let name = name
            .as_str()
            .ok_or_else(|| ProbeError::BadArgs(format!("bad location {location}")))?;
        let offset = offset
            .as_f64()
            .ok_or_else(|| ProbeError::BadArgs(format!("bad location {location}")))?;
        ax.set_theta_zero_location(compass(name)?, offset);
    }
    if let Some(offset) = args.opt_f64("theta_offset")? {
        ax.set_theta_offset(offset);
    }
    draw(&fig)?;
    let extent = ax
        .spines()
        .get(SpineSide::Top)
        .window_extent()
        .ok_or_else(|| ProbeError::rejected("the outer spine is not drawn"))?;
    Ok(Json::from(vec![
        extent.x0(),
        extent.y0(),
        extent.x1(),
        extent.y1(),
    ]))
}

fn span_steps(args: Args<'_>) -> Outcome {
    let ax = Figure::new().polar_subplot();
    let span = ax.axvspan(args.f64("xmin")?, args.f64("xmax")?);
    Ok(Json::from(span.interpolation_steps() as f64))
}

/// The upper right axes of a 2 × 2 grid sharing both axes.
fn upper_right(fig: &Figure, how: &str, polar: bool) -> Result<Axes, ProbeError> {
    match how {
        "subplots" if polar => {
            let axs = fig
                .polar_subplots(2, 2, Share::All, Share::All)
                .map_err(ProbeError::rejected)?;
            Ok(Axes::clone(&axs[(0, 1)]))
        }
        "subplots" => {
            let axs = fig
                .subplots_shared(2, 2, Share::All, Share::All)
                .map_err(ProbeError::rejected)?;
            Ok(axs[(0, 1)].clone())
        }
        "mosaic" => {
            let projection = if polar {
                ProjectionKind::Polar
            } else {
                ProjectionKind::Rectilinear
            };
            let options = MosaicOptions {
                sharex: true,
                sharey: true,
                projection: projection.into(),
                ..MosaicOptions::default()
            };
            let mosaic = "ab\ncd".parse::<Mosaic>().map_err(ProbeError::rejected)?;
            let axs = fig
                .subplot_mosaic(&mosaic, options)
                .map_err(ProbeError::rejected)?;
            axs.get("b")
                .cloned()
                .ok_or_else(|| ProbeError::rejected("no axes b"))
        }
        other => Err(ProbeError::BadArgs(format!("unknown grid {other:?}"))),
    }
}

fn shared_ticklabels(args: Args<'_>) -> Outcome {
    let polar = match args.opt_str("projection")? {
        None => false,
        Some("polar") => true,
        Some(other) => return Err(ProbeError::BadArgs(format!("unknown projection {other:?}"))),
    };
    let fig = Figure::new();
    let ax = upper_right(&fig, args.str("how")?, polar)?;
    draw(&fig)?;
    Ok(Json::Array(vec![
        labels(ax.xaxis().ticklabels(false))?,
        labels(ax.yaxis().ticklabels(false))?,
    ]))
}

/// The test's `millions` formatter, `'$%1.1fM' % (x * 1e-6)`, or none.
fn formatter(args: Args<'_>, key: &str) -> Result<Option<Rc<FuncFormatter>>, ProbeError> {
    match args.opt_str(key)? {
        None => Ok(None),
        Some("millions") => Ok(Some(Rc::new(FuncFormatter::new(|x, _| {
            format!("${:.1}M", x * 1e-6)
        })))),
        Some(other) => Err(ProbeError::BadArgs(format!("unknown formatter {other:?}"))),
    }
}

fn format_coord(args: Args<'_>) -> Outcome {
    let ax = Figure::new().polar_subplot();
    for (name, key) in [(AxisName::X, "fmt_xdata"), (AxisName::Y, "fmt_ydata")] {
        let custom = formatter(args, key)?;
        ax.set_fmt_data(name, custom.map(|f| f as Rc<dyn plt::ticker::Formatter>));
    }
    Ok(Json::from(
        ax.format_coord(args.f64("theta")?, args.f64("r")?),
    ))
}

/// `[isDefault_majloc, majorticklocs]` of the radial axis.
fn radial_state(ax: &PolarAxes) -> Outcome {
    let yaxis = ax.yaxis();
    Ok(Json::Array(vec![
        Json::from(yaxis.is_default_major_locator()),
        Json::from(yaxis.ticklocs(false).map_err(ProbeError::rejected)?),
    ]))
}

fn radial_locators(_: Args<'_>) -> Outcome {
    let ax = Figure::new().polar_subplot();
    let ticks = [0.0, 1.0, 2.0, 3.0];
    let mut states = vec![radial_state(&ax)?];
    ax.yaxis()
        .set_major_locator(MaxNLocator::new(Nbins::Fixed(3)));
    states.push(radial_state(&ax)?);
    ax.clear();
    states.push(radial_state(&ax)?);
    ax.set_rticks(&ticks);
    states.push(radial_state(&ax)?);
    ax.clear();
    states.push(radial_state(&ax)?);
    ax.set_rgrids(&ticks, None);
    states.push(radial_state(&ax)?);
    ax.clear();
    states.push(radial_state(&ax)?);
    ax.set_yscale("log").map_err(ProbeError::rejected)?;
    states.push(radial_state(&ax)?);
    Ok(Json::Array(states))
}

fn radial_ticklabels(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.polar_subplot();
    ax.set_rticks(&args.f64_vec("ticks")?);
    let texts: Vec<String> = args
        .json("labels")?
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|label| {
            label
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| ProbeError::BadArgs(format!("bad label {label}")))
        })
        .collect::<Result<_, _>>()?;
    ax.yaxis()
        .set_ticklabels(texts)
        .map_err(ProbeError::rejected)?;
    draw(&fig)?;
    labels(ax.yaxis().ticklabels(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compass_points_by_name() {
        assert_eq!(compass("N"), Ok(Compass::N));
        assert!(compass("up").is_err());
    }

    #[test]
    fn spine_extent_of_a_default_polar_axes() {
        let args = Json::parse("{}").expect("valid JSON");
        let extents = spine_extents(Args::new(&args)).expect("drawn spine");
        assert_eq!(extents.as_f64_vec().map(|v| v.len()), Some(4));
    }
}
