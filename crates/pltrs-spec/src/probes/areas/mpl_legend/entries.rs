//! What legends list: handles deduplicated by label, labelled artists no
//! handler draws, the parasites of a host axes, subfigure legends, and
//! whether a legend is draggable.

use pltrs::artist::ArtistHandle;
use pltrs::axes::{Mesh, PcolorOptions, ScatterColors, ScatterOptions};
use pltrs::axes_grid1::host_subplot;
use pltrs::colors::Color;
use pltrs::figure::{Figure, SubfigureOptions};
use pltrs::legend::{BboxToAnchor, LegendHandle, LegendOptions};
use pltrs::lines::LineProps;
use pltrs::markers::MarkerStyle;
use pltrs::numeric::Grid;
use pltrs::offsetbox::AnchorLoc;
use pltrs::pyplot;

use super::super::mpl_tests::in_test_style;
use super::{texts, window_extents};
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};
use crate::rng::SpecRng;

/// `test_legend_ordereddict`: the legend of ten single-point scatters
/// made from an ordered dict of their handles by label, which keeps the
/// first place of every label and its last handle.
pub(super) fn ordered_handles(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let seed = u64::try_from(args.i64("seed")?).map_err(ProbeError::rejected)?;
        let mut rng = SpecRng::new(seed);
        let (xs, ys) = (rng.normal(10, 0.0, 1.0), rng.normal(10, 0.0, 1.0));
        let fig = Figure::new();
        let ax = fig.subplot();
        for (i, (&x, &y)) in xs.iter().zip(&ys).enumerate() {
            let (label, color) = if i < 5 { ("a", "r") } else { ("b", "g") };
            let options = ScatterOptions {
                colors: ScatterColors::Single(Color::from(color)),
                ..ScatterOptions::default()
            };
            ax.scatter_with(vec![x], vec![y], options)
                .map_err(ProbeError::rejected)?
                .set_label(label);
        }
        let (handles, labels) = ax.legend_handles_labels();
        let mut ordered: Vec<(String, LegendHandle)> = Vec::new();
        for (handle, label) in handles.into_iter().zip(labels) {
            match ordered.iter_mut().find(|(known, _)| *known == label) {
                Some(entry) => entry.1 = handle,
                None => ordered.push((label, handle)),
            }
        }
        let (labels, handles): (Vec<_>, Vec<_>) = ordered.into_iter().unzip();
        let legend = ax
            .legend(
                LegendOptions::default()
                    .handles(handles)
                    .labels(labels)
                    .loc(AnchorLoc::CenterLeft)
                    .bbox_to_anchor(BboxToAnchor::Point(1.0, 0.5)),
            )
            .map_err(ProbeError::rejected)?;
        fig.canvas().draw().map_err(ProbeError::rejected)?;
        Ok(object([
            ("texts", texts(&legend)),
            ("extent", window_extents(&legend)?),
        ]))
    })
}

/// `test_legend_repeatcheckok`: the labels of two scatters both labelled
/// `test`, with markers `o` and `v` in `colors`.
pub(super) fn repeated_labels(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let colors = args
            .json("colors")?
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs("colors is not a list".into()))?;
        let ax = Figure::new().subplot();
        for ((x, y), (color, marker)) in [(0.0, 1.0), (0.5, 0.0)]
            .into_iter()
            .zip(colors.iter().zip(["o", "v"]))
        {
            let color = color
                .as_str()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad colour {color}")))?;
            let marker: MarkerStyle = marker.parse().map_err(ProbeError::rejected)?;
            let options = ScatterOptions {
                colors: ScatterColors::Single(Color::from(color)),
                marker: Some(marker),
                ..ScatterOptions::default()
            };
            ax.scatter_with(vec![x], vec![y], options)
                .map_err(ProbeError::rejected)?
                .set_label("test");
        }
        ax.legend(LegendOptions::default())
            .map_err(ProbeError::rejected)?;
        Ok(Json::from(ax.legend_handles_labels().1))
    })
}

/// `test_text_nohandler_warning`: the labels of `ax.legend()` and of
/// `get_legend_handles_labels` beside a labelled text, or the latter
/// beside a mesh.
pub(super) fn unlisted_artists(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let ax = Figure::new().subplot();
        let legend = match args.str("artist")? {
            "text" => {
                ax.plot_with([0.0], [0.0], LineProps::default().label("mock data"))
                    .map_err(ProbeError::rejected)?;
                ax.text(0.0, 0.0, "text").set_label("label");
                let legend = ax
                    .legend(LegendOptions::default())
                    .map_err(ProbeError::rejected)?;
                texts(&legend)
            }
            "pcolormesh" => {
                let values = SpecRng::new(0).uniform(100, 0.0, 1.0);
                let grid = Grid::from_vec(10, 10, values).map_err(ProbeError::rejected)?;
                ax.pcolormesh(Mesh::new(grid), PcolorOptions::default())
                    .map_err(ProbeError::rejected)?;
                Json::Null
            }
            other => return Err(ProbeError::BadArgs(format!("unknown artist {other}"))),
        };
        Ok(object([
            ("legend", legend),
            ("labels", Json::from(ax.legend_handles_labels().1)),
        ]))
    })
}

/// `test_parasite`: `plt.legend()` for a host axes and its `twinx`
/// parasite, each with a labelled line.
pub(super) fn parasite_legend(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let host = host_subplot(&pyplot::gcf(), 1, 1, 1).map_err(ProbeError::rejected)?;
        let parasite = host.twinx();
        host.axes()
            .plot_with(
                [0.0, 1.0, 2.0],
                [0.0, 1.0, 2.0],
                LineProps::default().label("Density"),
            )
            .map_err(ProbeError::rejected)?;
        parasite
            .axes()
            .plot_with(
                [0.0, 1.0, 2.0],
                [0.0, 3.0, 2.0],
                LineProps::default().label("Temperature"),
            )
            .map_err(ProbeError::rejected)?;
        let legend = pyplot::legend(LegendOptions::default()).map_err(ProbeError::rejected)?;
        Ok(texts(&legend))
    })
}

/// `test_subfigure_legend`: the legends of a subfigure and of its figure
/// after `subfig.legend()`.
pub(super) fn subfigure_legend(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let subfigures = fig
            .subfigures(1, 1, SubfigureOptions::default())
            .map_err(ProbeError::rejected)?;
        let subfig = &subfigures[(0, 0)];
        subfig
            .subplot()
            .plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("line"))
            .map_err(ProbeError::rejected)?;
        let legend = subfig
            .legend(LegendOptions::default())
            .map_err(ProbeError::rejected)?;
        let count = |fig: &Figure| Json::from(fig.legends().len() as f64);
        Ok(object([
            ("subfigure", count(subfig)),
            ("figure", count(&fig)),
            ("texts", texts(&legend)),
        ]))
    })
}

/// `test_legend_draggable` (`draggable` given) and
/// `test_get_set_draggable`.
pub(super) fn draggable(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        if let Some(draggable) = args.opt_bool("draggable")? {
            let ax = Figure::new().subplot();
            ax.plot_with(
                (0..10).map(f64::from).collect::<Vec<_>>(),
                (0..10).map(f64::from).collect::<Vec<_>>(),
                LineProps::default().label("shabnams"),
            )
            .map_err(ProbeError::rejected)?;
            let options = LegendOptions {
                draggable,
                ..LegendOptions::default()
            };
            let legend = ax.legend(options).map_err(ProbeError::rejected)?;
            return Ok(Json::from(legend.draggable()));
        }
        let legend = pyplot::legend(LegendOptions::default()).map_err(ProbeError::rejected)?;
        let mut states = vec![legend.draggable()];
        legend.set_draggable(true);
        states.push(legend.draggable());
        legend.set_draggable(false);
        states.push(legend.draggable());
        Ok(Json::Array(states.into_iter().map(Json::from).collect()))
    })
}
