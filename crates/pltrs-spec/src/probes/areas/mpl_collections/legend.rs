//! The `PathCollection.legend_elements` op: the lines a legend draws for
//! the handles `legend_elements` makes of a scatter's colours or sizes.

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes::{Axes, MappedColors, ScatterOptions};
use pltrs::collections::{ElementCount, LegendElementsOptions, LegendProperty, LegendValueFn};
use pltrs::colors::{Color, Colorize, colormaps};
use pltrs::figure::Figure;
use pltrs::legend::{LegendHandle, LegendOptions};
use pltrs::lines::Line2D;
use pltrs::markers::{FillStyle, Marker, MarkerStyle};
use pltrs::ticker::{MaxNLocator, Nbins, StrMethodFormatter};

use super::{colors, made, numbers, unknown};
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};
use crate::rng::SpecRng;

/// The labels and the look of the lines a legend of `handles` draws.
fn legend_looks(ax: &Axes, handles: Vec<LegendHandle>, labels: Vec<String>) -> Outcome {
    let legend = made(
        ax.legend(
            LegendOptions::default()
                .handles(handles)
                .labels(labels.clone()),
        ),
    )?;
    let lines = legend
        .handles()
        .iter()
        .map(|handle| {
            handle
                .as_ref()
                .and_then(RawHandle::downcast::<Line2D>)
                .ok_or_else(|| ProbeError::rejected("a legend handle is no line"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let resolve = |color: Color| made(color.resolve());
    Ok(object([
        ("labels", Json::from(labels)),
        (
            "colors",
            colors(
                lines
                    .iter()
                    .map(|l| resolve(l.color()))
                    .collect::<Result<_, _>>()?,
            ),
        ),
        (
            "alphas",
            Json::Array(lines.iter().map(|l| Json::from(l.alpha())).collect()),
        ),
        ("markersizes", numbers(lines.iter().map(Line2D::markersize))),
        (
            "markerfacecolors",
            colors(
                lines
                    .iter()
                    .map(|l| resolve(l.effective_markerfacecolor()))
                    .collect::<Result<_, _>>()?,
            ),
        ),
    ]))
}

pub(super) fn legend_elements(args: Args<'_>) -> Outcome {
    let scene = args.str("scene")?;
    let ax = Figure::new().subplot();
    let mut stream = SpecRng::new(19_680_801);
    if scene == "inverse" {
        let (x, y) = (stream.uniform(50, 0.0, 1.0), stream.uniform(50, 0.0, 1.0));
        let sizes = stream
            .uniform(50, 0.0, 1.0)
            .into_iter()
            .map(|u| 5.0 / (1.0 - u))
            .collect();
        let options = ScatterOptions {
            sizes: Some(sizes),
            ..ScatterOptions::default()
        };
        let scatter = made(ax.scatter_with(x, y, options))?;
        let (handles, labels) = made(scatter.legend_elements(LegendElementsOptions {
            prop: LegendProperty::Sizes,
            num: ElementCount::Values(vec![0.2, 0.4, 0.6, 0.8]),
            func: LegendValueFn::new(|s| 5.0 / s),
            ..LegendElementsOptions::default()
        }))?;
        return legend_looks(&ax, handles, labels);
    }
    let (x, y) = (stream.uniform(10, 0.0, 1.0), stream.uniform(10, 0.0, 1.0));
    let values: Vec<f64> = stream
        .integers(10, 0, 5)
        .into_iter()
        .map(|v| v as f64)
        .collect();
    let sizes = stream
        .integers(10, 10, 300)
        .into_iter()
        .map(|v| v as f64)
        .collect();
    let options = ScatterOptions {
        sizes: Some(sizes),
        colors: MappedColors::new(values)
            .cmap(made(colormaps::get("jet"))?)
            .into(),
        marker: Some(made(MarkerStyle::new(
            made("o".parse::<Marker>())?,
            FillStyle::Full,
        ))?),
        linewidths: Some(vec![0.0]),
        ..ScatterOptions::default()
    };
    let scatter = made(ax.scatter_with(x, y, options))?;
    let options = match scene {
        "colors" => LegendElementsOptions {
            fmt: Some(Box::new(made(StrMethodFormatter::new("{x:g}"))?)),
            ..LegendElementsOptions::default()
        },
        "sizes" => LegendElementsOptions {
            prop: LegendProperty::Sizes,
            alpha: Some(0.5),
            color: Some(Color::from("red")),
            ..LegendElementsOptions::default()
        },
        "func" => LegendElementsOptions {
            prop: LegendProperty::Sizes,
            num: ElementCount::Target(4),
            fmt: Some(Box::new(made(StrMethodFormatter::new("{x:.2f}"))?)),
            func: LegendValueFn::new(|x| 2.0 * x),
            ..LegendElementsOptions::default()
        },
        "locator" => {
            let locator = made(
                MaxNLocator::new(Nbins::Fixed(9))
                    .with_min_n_ticks(8)
                    .with_steps(&[1.0, 2.0, 2.5, 3.0, 5.0, 6.0, 8.0, 10.0]),
            )?;
            LegendElementsOptions {
                num: ElementCount::Locator(std::rc::Rc::new(locator)),
                ..LegendElementsOptions::default()
            }
        }
        other => return Err(unknown(other)),
    };
    let (handles, labels) = made(scatter.legend_elements(options))?;
    legend_looks(&ax, handles, labels)
}
