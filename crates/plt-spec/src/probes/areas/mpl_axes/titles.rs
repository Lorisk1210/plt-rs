//! Where titles go: their padding, manual positions, and how they are
//! lifted above tick labels, offset texts and inset axes on top.

use plt::artist::ArtistHandle;
use plt::axes::{Axes, AxisName, TickParams, TickPosition, TitleLocation};
use plt::figure::{Figure, Share};
use plt::layout::LayoutEngine;
use plt::spines::SpineSide;
use plt::text::Text;
use plt_core::Point;

use super::{made, numbers};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError};

/// The height of a title in axes fractions.
fn title_y(title: &Text) -> Json {
    Json::from(title.position().y)
}

/// Two subplots, sharing x or not, titled with a pad of -40 points.
fn location_shared(share: Share) -> Outcome {
    let fig = Figure::new();
    let axs = made(fig.subplots_shared(2, 1, share, Share::None))?;
    for (ax, title) in axs.as_slice().iter().zip(["A", "B"]) {
        ax.set_title(title);
        ax.set_title_pad(-40.0);
    }
    fig.layout();
    Ok(Json::Array(
        axs.as_slice()
            .iter()
            .map(|ax| title_y(&ax.title_text(TitleLocation::Center)))
            .collect(),
    ))
}

/// A title placed by hand under a twin's top axis label.
fn set_position() -> Outcome {
    let fig = Figure::new();
    made(fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
        top: Some(0.8),
        ..plt::gridspec::SubplotOverrides::default()
    }))?;
    let ax = fig.subplot();
    let ax2 = ax.twiny();
    ax.set_xlabel("Xlabel");
    ax2.set_xlabel("Xlabel2");
    let title = ax.set_title("Title");
    title.set_position(Point::new(0.5, 1.11));
    fig.layout();
    let at = title.position();
    Ok(numbers([at.x, at.y]))
}

/// A title over x tick labels on top: `ticks_top_only` moves the ticks
/// there, otherwise they are on both sides.
fn xticks_top(ticks_top_only: bool) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    if ticks_top_only {
        made(
            ax.xaxis()
                .set_ticks_position(TickPosition::Side(SpineSide::Top)),
        )?;
    } else {
        ax.tick_params(&TickParams {
            axis: Some(AxisName::X),
            bottom: Some(true),
            top: Some(true),
            labelbottom: Some(true),
            labeltop: Some(true),
            ..TickParams::default()
        });
    }
    let title = ax.set_title("xlabel top");
    fig.layout();
    Ok(title_y(&title))
}

/// Left and centre titles over a y offset text.
fn above_offset(left: &str, center: &str) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let (_, top) = ax.ylim();
    made(ax.set_ylim(1e11, top))?;
    ax.set_title_at(TitleLocation::Left, left);
    ax.set_title(center);
    fig.layout();
    Ok(Json::Array(vec![
        title_y(&ax.title_text(TitleLocation::Left)),
        title_y(&ax.title_text(TitleLocation::Center)),
    ]))
}

/// Titles of subplots sharing y, whose inner offset text is hidden, in a
/// tight layout: whether each title and position stay finite.
fn above_hidden_offset() -> Outcome {
    let fig = Figure::new();
    let axs = made(fig.subplots_shared(1, 2, Share::None, Share::All))?;
    fig.set_layout_engine(LayoutEngine::tight())
        .map_err(ProbeError::rejected)?;
    for (i, ax) in axs.as_slice().iter().enumerate() {
        ax.set_title(format!("Subplot {i}"));
        made(ax.plot((0..10).map(f64::from).collect::<Vec<_>>(), vec![1e53; 10]))?;
    }
    fig.layout();
    axs.as_slice()
        .iter()
        .map(|ax| {
            let title = ax.title_text(TitleLocation::Center).window_extent();
            let finite_title = title.is_some_and(|extent| extent.y0().is_finite());
            let position = ax.position();
            let finite_position = [
                position.x0(),
                position.y0(),
                position.width(),
                position.height(),
            ]
            .iter()
            .all(|v| v.is_finite());
            Ok(Json::from(finite_title && finite_position))
        })
        .collect::<Result<Vec<_>, ProbeError>>()
        .map(Json::Array)
}

/// The title of axes reaching below the figure, with x tick labels on
/// both sides.
fn no_move_off_page() -> Outcome {
    let fig = Figure::new();
    let ax = made(fig.add_axes([0.1, -0.5, 0.8, 0.2]))?;
    ax.tick_params(&TickParams {
        axis: Some(AxisName::X),
        bottom: Some(true),
        top: Some(true),
        labelbottom: Some(true),
        labeltop: Some(true),
        ..TickParams::default()
    });
    let title = ax.set_title("Boo");
    fig.layout();
    Ok(title_y(&title))
}

/// The title before and after an inset axes on top of the axes.
fn inset_ax() -> Outcome {
    let fig = Figure::new();
    let ax: Axes = fig.subplot();
    let title = ax.set_title("Title");
    fig.layout();
    let before = title_y(&title);
    let _inset = ax.inset_axes([0.0, 1.0, 1.0, 0.1]);
    fig.layout();
    Ok(Json::Array(vec![before, title_y(&title)]))
}

pub(super) fn title_positions(args: Args<'_>) -> Outcome {
    styled(args, || match args.str("case")? {
        "location_shared" => location_shared(Share::All),
        "location_unshared" => location_shared(Share::None),
        "setpos" => set_position(),
        "xticks_top" => xticks_top(true),
        "xticks_top_both" => xticks_top(false),
        "above_offset_left" => above_offset("left", ""),
        "above_offset_center" => above_offset("", "center"),
        "above_offset_both" => above_offset("left", "center"),
        "above_hidden_offset" => above_hidden_offset(),
        "no_move_off_page" => no_move_off_page(),
        "inset_ax" => inset_ax(),
        other => Err(ProbeError::BadArgs(format!("unknown case {other}"))),
    })
}

pub(super) fn twiny_title_gap(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        fig.set_dpi(72.0);
        let ax = fig.subplot();
        let ax2 = ax.twiny();
        ax2.set_xlabel("Xlabel2");
        let title = ax.set_title("Title");
        fig.layout();
        let title_box = title
            .window_extent()
            .ok_or_else(|| ProbeError::rejected("the title has no extent"))?;
        let label_box = ax2
            .xaxis()
            .label_extent()
            .ok_or_else(|| ProbeError::rejected("the label has no extent"))?;
        Ok(Json::from(title_box.y0() - label_box.y1()))
    })
}
