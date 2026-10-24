//! Legends end to end: which entries are listed and where they go.

use plt_core::Bbox;

use super::*;
use crate::axes::{BarOptions, ErrorbarOptions};
use crate::colors::Color;
use crate::lines::LineProps;
use crate::offsetbox::AnchorLoc;
use crate::patches::{PatchProps, Shadow};

fn labelled(label: &str) -> LineProps {
    LineProps::default().label(label)
}

/// An axes filling a 400 × 300 px figure at 72 dpi (one point per pixel).
fn canvas() -> (Figure, Axes) {
    let fig = Figure::with_size(400.0 / 72.0, 300.0 / 72.0, 72.0);
    let ax = fig
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    (fig, ax)
}

/// The display extent of the legend's frame after a layout pass.
fn frame_extent(fig: &Figure, legend: &Legend) -> Bbox {
    fig.layout();
    let frame = legend.frame().shape();
    Bbox::from_bounds(frame.xy.x, frame.xy.y, frame.width, frame.height)
}

#[test]
fn automatic_entries_skip_hidden_labels_and_list_containers_last() {
    let (_, ax) = canvas();
    let bars = BarOptions {
        label: Some("bars".into()),
        ..BarOptions::default()
    };
    ax.bar([1.0, 2.0], [1.0, 2.0], bars).unwrap();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled("shown"))
        .unwrap();
    ax.plot_with([0.0, 1.0], [1.0, 0.0], labelled("_hidden"))
        .unwrap();
    ax.plot([0.0, 1.0], [0.5, 0.5]).unwrap();
    let errors = ErrorbarOptions {
        yerr: Some(0.1.into()),
        line: labelled("errors"),
        ..ErrorbarOptions::default()
    };
    ax.errorbar([0.0, 1.0], [0.2, 0.3], errors).unwrap();
    let (_, labels) = ax.legend_handles_labels();
    assert_eq!(labels, ["shown", "bars", "errors"]);
    let legend = ax.legend(LegendOptions::default()).unwrap();
    assert_eq!(legend.labels(), ["shown", "bars", "errors"]);
    assert_eq!(ax.current_legend(), Some(legend));
}

#[test]
fn explicit_handles_and_labels_pair_up_to_the_shorter_list() {
    let (fig, ax) = canvas();
    let first = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
    let second = ax
        .plot_with([0.0, 1.0], [1.0, 0.0], labelled("own"))
        .unwrap();
    let paired = LegendOptions::default()
        .handles([&first, &second])
        .labels(["a"]);
    assert_eq!(ax.legend(paired).unwrap().labels(), ["a"]);
    let own_labels = LegendOptions::default().handles([&second]);
    assert_eq!(ax.legend(own_labels).unwrap().labels(), ["own"]);
    // Labels alone go to the first candidates, labelled or not.
    let renamed = LegendOptions::default().labels(["x", "y", "z"]);
    assert_eq!(fig.legend(renamed).unwrap().labels(), ["x", "y"]);
}

#[test]
fn a_new_axes_legend_replaces_the_previous_one_and_takes_its_parts_along() {
    let (fig, ax) = canvas();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled("line"))
        .unwrap();
    let artists = || fig.raw().figure_state().borrow().store.len();
    let before = artists();
    let first = ax.legend(LegendOptions::default()).unwrap();
    let with_one = artists();
    assert!(with_one > before);
    let second = ax.legend(LegendOptions::default().title("Again")).unwrap();
    assert!(!first.raw().is_attached());
    assert_eq!(artists(), with_one, "the old legend's parts are gone");
    assert_eq!(ax.current_legend(), Some(second.clone()));
    second.remove();
    assert_eq!(artists(), before);
    assert_eq!(ax.current_legend(), None);
}

#[test]
fn figure_legends_default_to_upper_right_and_reject_best() {
    let (fig, ax) = canvas();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled("line"))
        .unwrap();
    assert_eq!(
        fig.legend(LegendOptions::default().loc(LegendLoc::Best)),
        Err(LegendError::BestForFigure)
    );
    let legend = fig.legend(LegendOptions::default()).unwrap();
    let extent = frame_extent(&fig, &legend);
    // borderaxespad 0.5 font sizes of 10 points.
    assert!((extent.x1() - 395.0).abs() < 1e-9);
    assert!((extent.y1() - 295.0).abs() < 1e-9);
    assert_eq!(fig.legends(), [legend]);
}

#[test]
fn anchored_locations_keep_the_border_pad() {
    let (fig, ax) = canvas();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled("line"))
        .unwrap();
    let legend = ax
        .legend(
            LegendOptions::default()
                .loc(AnchorLoc::LowerLeft)
                .borderaxespad(2.0),
        )
        .unwrap();
    let extent = frame_extent(&fig, &legend);
    assert!((extent.x0() - 20.0).abs() < 1e-9);
    assert!((extent.y0() - 20.0).abs() < 1e-9);
    legend.set_loc((0.5, 0.5)).unwrap();
    let centred = frame_extent(&fig, &legend);
    assert!((centred.x0() - 200.0).abs() < 1e-9);
    assert!((centred.y0() - 150.0).abs() < 1e-9);
}

#[test]
fn best_avoids_the_data() {
    let (fig, ax) = canvas();
    let x: Vec<f64> = (0..50).map(f64::from).collect();
    ax.plot_with(&x, vec![10.0; 50], labelled("top")).unwrap();
    ax.plot([0.0, 49.0], [0.0, 10.5]).unwrap();
    ax.set_ylim(0.0, 10.5).expect("finite limits");
    let legend = ax.legend(LegendOptions::default()).unwrap();
    let extent = frame_extent(&fig, &legend);
    assert!(
        extent.y1() < 150.0,
        "moved below the crowded top: {extent:?}"
    );
}

#[test]
fn expanded_legends_span_the_anchor_box() {
    let (fig, ax) = canvas();
    for label in ["a", "b"] {
        ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled(label))
            .unwrap();
    }
    let options = LegendOptions::default()
        .loc(AnchorLoc::UpperLeft)
        .bbox_to_anchor((0.0, 0.0, 1.0, 0.5))
        .mode(LegendMode::Expand)
        .ncols(2);
    let legend = ax.legend(options).unwrap();
    let extent = frame_extent(&fig, &legend);
    assert!((extent.x0() - 5.0).abs() < 1e-9);
    assert!((extent.x1() - 395.0).abs() < 1e-9);
    assert!((extent.y1() - 145.0).abs() < 1e-9);
}

#[test]
fn titles_show_only_when_set() {
    let (_, ax) = canvas();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled("line"))
        .unwrap();
    let legend = ax.legend(LegendOptions::default()).unwrap();
    assert!(!legend.title().visible());
    legend.set_title("Title");
    assert!(legend.title().visible());
    assert_eq!(legend.title().text(), "Title");
    legend.set_frame_on(false);
    assert!(!legend.frame_on());
}

#[test]
fn line_colours_become_label_colours() {
    let (_, ax) = canvas();
    let line = LineProps::default().color("tab:red").label("red");
    ax.plot_with([0.0, 1.0], [0.0, 1.0], line).unwrap();
    let legend = ax
        .legend(LegendOptions::default().labelcolor("linecolor"))
        .unwrap();
    assert_eq!(legend.texts()[0].color(), Color::from("tab:red"));
    let plain = ax
        .legend(LegendOptions::default().labelcolor("blue"))
        .unwrap();
    assert_eq!(
        plain.texts()[0].color(),
        Color::from(plt_core::Rgba::opaque(0.0, 0.0, 1.0))
    );
}

#[test]
fn invalid_options_are_errors() {
    let (_, ax) = canvas();
    let line = ax
        .plot_with([0.0, 1.0], [0.0, 1.0], labelled("line"))
        .unwrap();
    let invalid = |options: LegendOptions| ax.legend(options).unwrap_err();
    assert!(matches!(
        "middle".parse::<LegendLoc>(),
        Err(LegendError::InvalidLocation(_))
    ));
    assert_eq!(
        invalid(LegendOptions::default().numpoints(0_usize)),
        LegendError::NoPoints
    );
    let two_entries = LegendOptions::default().handles([&line, &line]).ncols(0);
    assert_eq!(invalid(two_entries), LegendError::NoColumns);
    let both = LegendOptions::default()
        .title_fontsize(12.0)
        .title_fontproperties(crate::fonts::FontProperties::from_rc(&crate::rc::params()));
    assert_eq!(invalid(both), LegendError::TitleFontConflict);
    assert!(matches!(
        invalid(LegendOptions::default().facecolor("not a colour")),
        LegendError::Color(_)
    ));
    assert_eq!(
        ax.current_legend(),
        None,
        "failed legends leave nothing behind"
    );
}

#[test]
fn shadow_styles_move_and_recolour_the_shadow() {
    let (fig, ax) = canvas();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled("line"))
        .unwrap();
    let style = LegendShadowStyle {
        ox: 10.0,
        oy: -5.0,
        props: PatchProps::default().color("red").alpha(0.1),
        ..LegendShadowStyle::default()
    };
    let legend = ax.legend(LegendOptions::default().shadow(style)).unwrap();
    let frame = frame_extent(&fig, &legend);
    let id = legend.read(LegendArtist::shadow_id).expect("a shadow");
    let shadow = Patch::<Shadow<FancyBboxPatch>>::from_raw(legend.handle(id));
    let red = Color::from("red").resolve().unwrap();
    assert_eq!(shadow.colors().unwrap().face, red.with_alpha(0.1));
    let moved = shadow.window_extent().unwrap();
    assert!((moved.x0() - frame.x0() - 10.0).abs() < 1e-9);
    assert!((moved.y0() - frame.y0() + 5.0).abs() < 1e-9);
    assert_eq!(
        legend.frame().alpha(),
        Some(1.0),
        "a shadow makes the frame opaque"
    );
}

#[test]
fn shadows_follow_the_option_over_the_rc_parameter() {
    let (_, ax) = canvas();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled("line"))
        .unwrap();
    let shadowless = |options: LegendOptions| {
        let legend = ax.legend(options).unwrap();
        legend.read(LegendArtist::shadow_id).is_none()
    };
    assert!(shadowless(LegendOptions::default()));
    assert!(!shadowless(LegendOptions::default().shadow(true)));
    crate::rc::context(|| {
        crate::rc::set("legend.shadow", "True").unwrap();
        assert!(!shadowless(LegendOptions::default()));
        assert!(shadowless(
            LegendOptions::default().shadow(LegendShadow::Off)
        ));
    });
}

#[test]
fn kept_legends_stay_beside_the_next_one() {
    let (_, ax) = canvas();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled("line"))
        .unwrap();
    let first = ax
        .legend(LegendOptions::default().loc(AnchorLoc::LowerLeft))
        .unwrap();
    ax.keep_legend(&first);
    let second = ax
        .legend(LegendOptions::default().loc(AnchorLoc::UpperRight))
        .unwrap();
    assert!(first.raw().is_attached(), "a kept legend is not replaced");
    assert_eq!(ax.current_legend(), Some(second.clone()));
    let third = ax.legend(LegendOptions::default()).unwrap();
    assert!(!second.raw().is_attached(), "the axes legend still is");
    assert_eq!(ax.current_legend(), Some(third));
}

#[test]
fn location_and_anchor_read_back_and_move_the_legend() {
    let (fig, ax) = canvas();
    ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled("line"))
        .unwrap();
    let legend = ax
        .legend(LegendOptions::default().loc(AnchorLoc::UpperLeft))
        .unwrap();
    assert_eq!(legend.loc(), LegendLoc::Anchored(AnchorLoc::UpperLeft));
    assert_eq!(legend.loc().code(), Some(2));
    assert_eq!(legend.bbox_to_anchor(), None);
    let inside = frame_extent(&fig, &legend);
    legend.set_bbox_to_anchor(Some(BboxToAnchor::Point(1.0, 1.0)), None);
    assert_eq!(
        legend.bbox_to_anchor(),
        Some((
            Bbox::from_bounds(1.0, 1.0, 0.0, 0.0),
            Transform::Axes(ax.id())
        ))
    );
    let outside = frame_extent(&fig, &legend);
    assert!(outside.x0() > inside.x0(), "past the right edge");
    legend.set_bbox_to_anchor(None, None);
    assert_eq!(frame_extent(&fig, &legend), inside);
    legend.set_loc((0.25, 0.5)).unwrap();
    assert_eq!(legend.loc(), LegendLoc::Point(0.25, 0.5));
    legend.set_loc(LegendLoc::Best).unwrap();
    assert_eq!(legend.loc(), LegendLoc::Best);
}

#[test]
fn alignment_moves_the_title_across_the_frame() {
    let (fig, ax) = canvas();
    let wide = labelled("a label much wider than the title");
    ax.plot_with([0.0, 1.0], [0.0, 1.0], wide).unwrap();
    let legend = ax.legend(LegendOptions::default().title("T")).unwrap();
    assert_eq!(legend.alignment(), LegendAlignment::Center);
    let title_x0 = |legend: &Legend| {
        fig.layout();
        legend.title().window_extent().unwrap().x0()
    };
    let centred = title_x0(&legend);
    legend.set_alignment(LegendAlignment::Left);
    assert_eq!(legend.alignment(), LegendAlignment::Left);
    let left = title_x0(&legend);
    legend.set_alignment(LegendAlignment::Right);
    let right = title_x0(&legend);
    assert!(
        left < centred && centred < right,
        "{left} {centred} {right}"
    );
    let options = LegendOptions::default()
        .title("T")
        .alignment(LegendAlignment::Right);
    assert_eq!(title_x0(&ax.legend(options).unwrap()), right);
}
