//! The proxy artists legends draw for their handles (matplotlib's
//! `legend_handles`, `get_lines` and `get_patches`): their markers, dash
//! patterns, colours and data.

use plt::artist::{ArtistHandle, RawHandle};
use plt::axes::{Axes, HistArtists, HistPlotOptions, ScatterOptions};
use plt::collections::{Collection, CollectionPatch, CollectionProps, CollectionSpec, EdgeColors};
use plt::colors::Color;
use plt::figure::Figure;
use plt::legend::{Legend, LegendHandle, LegendOptions};
use plt::lines::{Line2D, LineProps, LineStyle};
use plt::markers::Marker;
use plt::patches::{Circle, Patch, PatchProps, Rectangle};
use plt_core::{Point, Rgba};
use plt_render::Dashes;

use super::super::canon::Canon;
use super::super::mpl_tests::{in_test_style, numbers};
use super::{maybe_rgba, texts};
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};

/// `Legend.legend_handles` in the scene `scene`.
pub(super) fn legend_handles(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let ax = Figure::new().subplot();
        match args.str("scene")? {
            "reverse" => reversed_markers(&ax, args.bool("reverse")?),
            "handlerline2d" => line_handle_marker(&ax),
            "linecollection" => line_collection_dashes(&ax),
            "alpha" => opaque_histogram_proxy(&ax),
            "patchcollection" => patch_collection(&ax, args.bool("empty")?),
            other => Err(ProbeError::BadArgs(format!("unknown scene {other}"))),
        }
    })
}

/// The proxy of entry `index` as a `T`.
fn proxy<T: plt::artist::TypedHandle>(legend: &Legend, index: usize) -> Result<T, ProbeError> {
    legend
        .handles()
        .get(index)
        .and_then(Option::as_ref)
        .and_then(RawHandle::downcast)
        .ok_or_else(|| ProbeError::rejected(format!("no proxy of that type at {index}")))
}

/// The legend of `ax` made with `options`.
fn legend(ax: &Axes, options: LegendOptions) -> Result<Legend, ProbeError> {
    ax.legend(options).map_err(ProbeError::rejected)
}

/// `test_reverse_legend_handles_and_labels`: the labels and proxy
/// markers of three marked lines.
fn reversed_markers(ax: &Axes, reverse: bool) -> Outcome {
    for (marker, label) in [
        (".", "First label"),
        (",", "Second label"),
        ("o", "Third label"),
    ] {
        let props = LineProps::fmt(marker)
            .map_err(ProbeError::rejected)?
            .label(label);
        ax.plot_with([1.0], [1.0], props)
            .map_err(ProbeError::rejected)?;
    }
    let legend = legend(ax, LegendOptions::default().reverse(reverse))?;
    let markers = (0..legend.handles().len())
        .map(|i| proxy::<Line2D>(&legend, i).map(|line| line.marker().canon()))
        .collect::<Result<_, _>>()?;
    Ok(object([
        ("texts", texts(&legend)),
        ("markers", Json::Array(markers)),
    ]))
}

/// `test_handlerline2d`: the marker of a `Line2D` handle beside a scatter
/// and that of its proxy.
fn line_handle_marker(ax: &Axes) -> Outcome {
    let marker: Marker = "v".parse().map_err(ProbeError::rejected)?;
    let options = ScatterOptions {
        marker: Some("v".parse().map_err(ProbeError::rejected)?),
        ..ScatterOptions::default()
    };
    ax.scatter_with(vec![0.0, 1.0], vec![0.0, 1.0], options)
        .map_err(ProbeError::rejected)?;
    let handle = LegendHandle::line(LineProps::default().marker(marker.clone()));
    let legend = legend(
        ax,
        LegendOptions::default()
            .handles([handle])
            .labels(["Aardvark"])
            .numpoints(1_usize),
    )?;
    let proxy: Line2D = proxy(&legend, 0)?;
    Ok(Json::Array(vec![marker.canon(), proxy.marker().canon()]))
}

/// `[offset, dashes]`, `dashes` being `null` for a solid line.
fn dashes(Dashes { offset, pattern }: Dashes) -> Json {
    let pattern = if pattern.is_empty() {
        Json::Null
    } else {
        numbers(pattern)
    };
    Json::Array(vec![Json::from(offset), pattern])
}

/// The scaled dash pattern of a collection's first line.
fn collection_dashes(collection: &Collection) -> Json {
    let style = collection.style();
    let width = collection.linewidths().first().copied().unwrap_or(0.0);
    let first = style
        .linestyles
        .first()
        .cloned()
        .unwrap_or(LineStyle::Solid);
    dashes(first.scaled_dashes(width, &plt::rc::params()))
}

/// `test_linecollection_scaled_dashes`: the first dash pattern of three
/// line collections and of their proxies.
fn line_collection_dashes(ax: &Axes) -> Outcome {
    let segments = |lines: [[(f64, f64); 2]; 2]| {
        lines
            .iter()
            .map(|line| line.iter().map(|&(x, y)| Point::new(x, y)).collect())
            .collect()
    };
    let styled = [
        (
            [[(0.0, 0.5), (0.5, 1.0)], [(0.3, 0.6), (0.2, 0.2)]],
            LineStyle::Dashed,
            Some(3.0),
        ),
        (
            [[(0.7, 0.2), (0.8, 0.4)], [(0.5, 0.7), (0.6, 0.1)]],
            LineStyle::DashDot,
            None,
        ),
        (
            [[(0.6, 0.2), (0.8, 0.4)], [(0.5, 0.7), (0.1, 0.1)]],
            LineStyle::Dotted,
            Some(0.5),
        ),
    ];
    let collections: Vec<Collection> = styled
        .into_iter()
        .map(|(lines, style, width)| {
            let props = CollectionProps {
                linestyles: Some(vec![style]),
                linewidths: width.map(|w| vec![w]),
                ..CollectionProps::default()
            };
            ax.add_collection(CollectionSpec::lines(segments(lines)).with_props(props))
        })
        .collect();
    let legend = legend(
        ax,
        LegendOptions::default()
            .handles(&collections)
            .labels(["line1", "line2", "line 3"]),
    )?;
    let rc = plt::rc::params();
    collections
        .iter()
        .enumerate()
        .map(|(i, collection)| {
            let line: Line2D = proxy(&legend, i)?;
            Ok(Json::Array(vec![
                collection_dashes(collection),
                dashes(line.linestyle().scaled_dashes(line.linewidth(), &rc)),
            ]))
        })
        .collect::<Result<_, _>>()
        .map(Json::Array)
}

/// `[r, g, b, a]`.
fn rgba(color: Rgba) -> Json {
    numbers(color.to_array())
}

/// The face and edge colours of a rectangle.
fn patch_colors(patch: &Patch<Rectangle>) -> Result<[Rgba; 2], ProbeError> {
    let colors = patch.colors().map_err(ProbeError::rejected)?;
    Ok([colors.face, colors.edge])
}

/// `test_alpha_handles`: the colours of the proxy of a histogram drawn at
/// alpha 0.25 once set to alpha 1, then those of its second bar.
fn opaque_histogram_proxy(ax: &Axes) -> Outcome {
    let mut options = HistPlotOptions {
        color: vec![Color::from("red")],
        label: vec!["data".to_owned()],
        ..HistPlotOptions::default()
    };
    options.props.base.alpha = Some(0.25);
    let hist = ax
        .hist(&[[1.0, 2.0, 3.0]], options)
        .map_err(ProbeError::rejected)?;
    let HistArtists::Bars(bars) = hist.artists else {
        return Err(ProbeError::rejected("a bar histogram has bars"));
    };
    let bar = bars
        .first()
        .and_then(|container| container.patches.get(1))
        .ok_or_else(|| ProbeError::rejected("the histogram has no second bar"))?;
    let legend = legend(ax, LegendOptions::default())?;
    for proxy in legend.handles().iter().flatten() {
        proxy.set_alpha(Some(1.0));
    }
    let last: Patch<Rectangle> = proxy(&legend, legend.handles().len() - 1)?;
    let [face, edge] = patch_colors(&last)?;
    let [bar_face, bar_edge] = patch_colors(bar)?;
    Ok(Json::Array(
        [face, edge, bar_face, bar_edge].map(rgba).to_vec(),
    ))
}

/// `test_patchcollection_legend` and `..._empty`: the labels, the number
/// of proxies and the look of the proxy and of the patch collection.
fn patch_collection(ax: &Axes, empty: bool) -> Outcome {
    let (patches, props, label) = if empty {
        (Vec::new(), CollectionProps::default(), "empty collection")
    } else {
        let circles = [(0.0, 0.0), (2.0, 0.0)]
            .map(|center| CollectionPatch::new(&Circle::new(center, 1.0), PatchProps::default()));
        let props = CollectionProps {
            facecolors: Some(vec![Color::from("red")]),
            edgecolors: Some(EdgeColors::Colors(vec![Color::from("blue")])),
            linewidths: Some(vec![3.0]),
            linestyles: Some(vec![LineStyle::Dashed]),
            ..CollectionProps::default()
        };
        (circles.to_vec(), props, "patch collection")
    };
    let spec = CollectionSpec::patches(&patches, false).map_err(ProbeError::rejected)?;
    let collection = ax.add_collection(spec.with_props(props));
    collection.set_label(label);
    if !empty {
        ax.autoscale_view(None);
    }
    let legend = legend(ax, LegendOptions::default())?;
    let mut out = vec![
        ("texts", texts(&legend)),
        ("handles", Json::from(legend.handles().len() as f64)),
    ];
    if !empty {
        let rc = plt::rc::params();
        let proxy: Patch<Rectangle> = proxy(&legend, 0)?;
        let [face, edge] = patch_colors(&proxy)?;
        let width = proxy.linewidth();
        out.push((
            "proxy",
            Json::Array(vec![
                rgba(face),
                rgba(edge),
                Json::from(width),
                dashes(proxy.style().linestyle.scaled_dashes(width, &rc)),
            ]),
        ));
        let colors = collection.colors().map_err(ProbeError::rejected)?;
        out.push((
            "collection",
            Json::Array(vec![
                maybe_rgba(colors.faces.first().copied()),
                maybe_rgba(colors.edges.first().copied()),
                Json::from(collection.linewidths().first().copied().unwrap_or(0.0)),
                collection_dashes(&collection),
            ]),
        ));
    }
    Ok(Json::Object(
        out.into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    ))
}

/// `Legend.get_lines` in the scene `scene`.
pub(super) fn lines(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let ax = Figure::new().subplot();
        match args.str("scene")? {
            "numpoints" => {
                let numpoints =
                    usize::try_from(args.i64("numpoints")?).map_err(ProbeError::rejected)?;
                ax.plot_with(
                    (0..5).map(f64::from).collect::<Vec<_>>(),
                    (0..5).map(f64::from).collect::<Vec<_>>(),
                    LineProps::default().label("test"),
                )
                .map_err(ProbeError::rejected)?;
                let legend = legend(&ax, LegendOptions::default().numpoints(numpoints))?;
                let line = legend
                    .lines()
                    .into_iter()
                    .next()
                    .ok_or_else(|| ProbeError::rejected("the legend has no line"))?;
                Ok(numbers(line.xdata()))
            }
            "markers" => {
                let handles = [".", "*", "v"]
                    .into_iter()
                    .map(|code| {
                        let marker: Marker = code.parse().map_err(ProbeError::rejected)?;
                        Ok(LegendHandle::line(
                            LineProps::default()
                                .linestyle(LineStyle::None)
                                .marker(marker),
                        ))
                    })
                    .collect::<Result<Vec<_>, ProbeError>>()?;
                let legend = legend(
                    &ax,
                    LegendOptions::default()
                        .handles(handles)
                        .labels(["foo", "bar", "xyzzy"]),
                )?;
                let markers = legend
                    .lines()
                    .iter()
                    .map(|line| line.marker().canon())
                    .collect();
                Ok(object([
                    ("markers", Json::Array(markers)),
                    ("texts", texts(&legend)),
                ]))
            }
            other => Err(ProbeError::BadArgs(format!("unknown scene {other}"))),
        }
    })
}

/// `test_setting_alpha_keeps_polycollection_color`: the colours of the
/// proxy of a `fill_between` once set to alpha 0.5, then the collection's.
pub(super) fn patches(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let ax = Figure::new().subplot();
        let mut options = plt::axes::FillBetweenOptions::default();
        options.props = options.props.color("#123456");
        options.props.base.label = Some("label".to_owned());
        let fill = ax
            .fill_between([0.0, 1.0], [2.0, 3.0], [0.0, 0.0], options)
            .map_err(ProbeError::rejected)?;
        let legend = legend(&ax, LegendOptions::default())?;
        let patch = legend
            .patches()
            .first()
            .and_then(RawHandle::downcast::<Patch<Rectangle>>)
            .ok_or_else(|| ProbeError::rejected("the legend has no patch"))?;
        patch.set_alpha(Some(0.5));
        let [face, edge] = patch_colors(&patch)?;
        let colors = fill.colors().map_err(ProbeError::rejected)?;
        Ok(Json::Array(vec![
            rgba(face),
            rgba(edge),
            maybe_rgba(colors.faces.first().copied()),
            maybe_rgba(colors.edges.first().copied()),
        ]))
    })
}
