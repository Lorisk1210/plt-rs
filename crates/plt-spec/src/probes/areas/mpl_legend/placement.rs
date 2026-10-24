//! Where legends go: the best location among patches, extents at other
//! resolutions, figure legends outside the axes, `set_loc` and the
//! alignment of the title.

use plt::artist::ArtistHandle;
use plt::axes::Axes;
use plt::figure::Figure;
use plt::layout::LayoutEngine;
use plt::legend::{LegendAlignment, LegendLoc, LegendOptions};
use plt::lines::LineProps;
use plt::offsetbox::AnchorLoc;
use plt::patches::{Ellipse, PatchProps, Polygon, Wedge};
use plt_core::{Bbox, Point};

use super::super::mpl_tests::in_test_style;
use super::{legend, window_extents};
use crate::json::Json;
use crate::probes::encode::{numbers, object};
use crate::probes::{Args, Outcome, ProbeError};

/// `range(10)` as plotted data.
fn range10() -> Vec<f64> {
    (0..10).map(f64::from).collect()
}

/// Plots `range(10)` labelled `label` on `ax`.
fn labelled_range(ax: &Axes, label: &str) -> Result<(), ProbeError> {
    ax.plot_with(range10(), range10(), LineProps::default().label(label))
        .map(drop)
        .map_err(ProbeError::rejected)
}

/// The window extent of an artist, laid out.
fn extent(artist: &impl ArtistHandle) -> Result<Bbox, ProbeError> {
    artist
        .window_extent()
        .ok_or_else(|| ProbeError::rejected("the artist has no extent"))
}

/// `Legend.get_window_extent` in the scene `scene`.
pub(super) fn window_extent(args: Args<'_>) -> Outcome {
    in_test_style(args, || match args.str("scene")? {
        "patches" => among_patches(),
        "dpi" => {
            let rc = plt::rc::params();
            let [width, height] = rc.figure_figsize;
            let ax = Figure::with_size(width, height, args.f64("dpi")?).subplot();
            labelled_range(&ax, "Aardvark")?;
            let legend = legend(&ax, LegendOptions::default())?;
            Ok(Json::from(extent(&legend)?.x0()))
        }
        "cached" => {
            let rc = plt::rc::params();
            let [width, height] = rc.figure_figsize;
            let fig = Figure::with_size(width, height, 100.0);
            let ax = fig.subplot();
            labelled_range(&ax, "Aardvark")?;
            let axes_legend = legend(&ax, LegendOptions::default())?;
            let figure_legend = fig
                .legend(LegendOptions::default())
                .map_err(ProbeError::rejected)?;
            fig.canvas().draw().map_err(ProbeError::rejected)?;
            Ok(Json::Array(vec![
                window_extents(&axes_legend)?,
                window_extents(&figure_legend)?,
            ]))
        }
        other => Err(ProbeError::BadArgs(format!("unknown scene {other}"))),
    })
}

/// `test_legend_auto5`: the bounds in axes fractions of the legends at
/// `"center"` and `"best"` among an ellipse, a U-shaped polygon and a
/// ring.
fn among_patches() -> Outcome {
    let fig = Figure::with_size(9.6, 4.8, plt::rc::params().figure_dpi);
    let axes = (1..=2)
        .map(|index| fig.add_subplot(1, 2, index))
        .collect::<Result<Vec<_>, _>>()
        .map_err(ProbeError::rejected)?;
    let mut bounds = Vec::new();
    for (ax, loc) in axes
        .into_iter()
        .zip([LegendLoc::Anchored(AnchorLoc::Center), LegendLoc::Best])
    {
        let c1 = PatchProps::default().facecolor("C1");
        ax.add_patch_with(Ellipse::new((0.5, 0.9), 0.8, 0.2), c1.clone());
        let u_shape = [
            [0.0, 1.0],
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.9, 1.0],
            [0.9, 0.1],
            [0.1, 0.1],
            [0.1, 1.0],
            [0.1, 1.0],
        ]
        .map(|[x, y]| Point::new(x, y));
        ax.add_patch_with(Polygon::new(u_shape, true), c1);
        let mut ring = Wedge::new((0.5, 0.5), 0.5, 0.0, 360.0);
        ring.width = Some(0.05);
        ax.add_patch_with(ring, PatchProps::default().facecolor("C0"));
        ax.plot_with(
            [0.1, 0.9],
            [0.9, 0.9],
            LineProps::default().label("A segment"),
        )
        .map_err(ProbeError::rejected)?;
        let legend = legend(&ax, LegendOptions::default().loc(loc))?;
        fig.canvas().draw().map_err(ProbeError::rejected)?;
        let (frame, axes) = (extent(&legend)?, extent(&ax)?);
        bounds.push(numbers([
            (frame.x0() - axes.x0()) / axes.width(),
            (frame.y0() - axes.y0()) / axes.height(),
            frame.width() / axes.width(),
            frame.height() / axes.height(),
        ]));
    }
    Ok(Json::Array(bounds))
}

/// `test_figure_legend_outside`: the extents of the axes and of the
/// figure legend at `"outside <loc>"` of a constrained figure at 100 dpi.
pub(super) fn outside(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let [width, height] = plt::rc::params().figure_figsize;
        let fig = Figure::with_size(width, height, 100.0);
        fig.set_layout_engine(LayoutEngine::constrained())
            .map_err(ProbeError::rejected)?;
        let ax = fig.subplot();
        labelled_range(&ax, "Boo1")?;
        let loc: LegendLoc = format!("outside {}", args.str("loc")?)
            .parse()
            .map_err(ProbeError::rejected)?;
        let legend = fig
            .legend(LegendOptions::default().loc(loc))
            .map_err(ProbeError::rejected)?;
        fig.layout();
        Ok(object([
            ("axes", window_extents(&ax)?),
            ("legend", window_extents(&legend)?),
        ]))
    })
}

/// `test_ax_legend_set_loc`: `_get_loc()` of an axes legend moved to
/// `loc`.
pub(super) fn set_loc(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let ax = Figure::new().subplot();
        labelled_range(&ax, "test")?;
        let legend = legend(&ax, LegendOptions::default())?;
        let loc: LegendLoc = args.str("loc")?.parse().map_err(ProbeError::rejected)?;
        legend.set_loc(loc).map_err(ProbeError::rejected)?;
        let loc = legend.loc();
        loc.code()
            .map(|code| Json::Integer(i64::from(code)))
            .ok_or_else(|| ProbeError::rejected(format!("no location code: {loc}")))
    })
}

/// `test_legend_alignment` and `test_legend_set_alignment`: the alignment
/// of a titled legend and the extents of its title and frame, drawn.
pub(super) fn alignment(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let alignment = match args.str("alignment")? {
            "center" => LegendAlignment::Center,
            "left" => LegendAlignment::Left,
            "right" => LegendAlignment::Right,
            other => return Err(ProbeError::BadArgs(format!("bad alignment {other}"))),
        };
        let fig = Figure::new();
        let ax = fig.subplot();
        labelled_range(&ax, "test")?;
        let options = LegendOptions::default().title("Aardvark");
        let legend = if args.opt_bool("set")?.unwrap_or(false) {
            let legend = legend(&ax, options)?;
            legend.set_alignment(alignment);
            legend
        } else {
            legend(&ax, options.alignment(alignment))?
        };
        fig.canvas().draw().map_err(ProbeError::rejected)?;
        let name = match legend.alignment() {
            LegendAlignment::Center => "center",
            LegendAlignment::Left => "left",
            LegendAlignment::Right => "right",
        };
        Ok(object([
            ("alignment", Json::from(name)),
            ("title", window_extents(&legend.title())?),
            ("legend", window_extents(&legend.frame())?),
        ]))
    })
}
