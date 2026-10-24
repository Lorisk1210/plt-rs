//! The frame and the title of a legend: the frame's opacity and edge
//! width from the options and rc parameters, and the title's font size.

use plt::artist::ArtistHandle;
use plt::figure::Figure;
use plt::fonts::FontSize;
use plt::legend::{LegendOptions, LegendShadow};
use plt::lines::LineProps;

use super::super::mpl_tests::in_test_style;
use super::super::text_metrics::font_properties;
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};

/// `test_shadow_framealpha` and `test_legend_linewidth`: the opacity and
/// edge width of the frame of `ax.legend(**legend)` for a labelled line.
pub(super) fn frame(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let ax = Figure::new().subplot();
        ax.plot_with(
            [0.0, 1.0, 2.0],
            [1.0, 2.0, 3.0],
            LineProps::default().label("data"),
        )
        .map_err(ProbeError::rejected)?;
        let mut options = LegendOptions::default();
        let keywords = args
            .json("legend")?
            .as_object()
            .ok_or_else(|| ProbeError::BadArgs("legend is not a dict".into()))?;
        for (key, value) in keywords {
            let bad = || ProbeError::BadArgs(format!("bad {key}: {value}"));
            match key.as_str() {
                "shadow" => {
                    options.shadow = Some(LegendShadow::from(value.as_bool().ok_or_else(bad)?));
                }
                "facecolor" => options.facecolor = Some(value.as_str().ok_or_else(bad)?.into()),
                "linewidth" => options.linewidth = Some(value.as_f64().ok_or_else(bad)?),
                _ => return Err(bad()),
            }
        }
        let frame = ax.legend(options).map_err(ProbeError::rejected)?.frame();
        Ok(object([
            ("alpha", frame.alpha().map_or(Json::Null, Json::from)),
            ("linewidth", Json::from(frame.linewidth())),
        ]))
    })
}

/// `test_legend_title_fontprop_fontsize`: the title's font size of
/// `legend(title="Aardvark", title_fontsize=..., title_fontproperties=...)`.
pub(super) fn title(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let ax = Figure::new().subplot();
        ax.plot_with(
            (0..10).map(f64::from).collect::<Vec<_>>(),
            (0..10).map(f64::from).collect::<Vec<_>>(),
            LineProps::default().label("mock data"),
        )
        .map_err(ProbeError::rejected)?;
        let mut options = LegendOptions::default().title("Aardvark");
        options.title_fontsize = args.opt_f64("title_fontsize")?.map(FontSize::Points);
        // A dict (or, with `as_object`, a `FontProperties` made from it)
        // that names a size gives the same font.
        options.title_fontproperties = args
            .get("title_fontproperties")
            .map(font_properties)
            .transpose()?;
        let legend = ax.legend(options).map_err(ProbeError::rejected)?;
        Ok(Json::from(legend.title().font().size))
    })
}
