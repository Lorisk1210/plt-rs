//! The artists each handler makes, checked against matplotlib's handler
//! arithmetic for a 10 pt legend.

use std::any::Any;

use plt_core::Point;

use super::*;
use crate::artist::ArtistHandle;
use crate::axes::{ErrorbarOptions, ScatterOptions, StemOptions};
use crate::figure::Figure;
use crate::legend::{LegendHandle, LegendOptions};
use crate::lines::{LineProps, LineStyle, MarkEvery};
use crate::patches::{FancyBboxPatch, PatchArtist, Rectangle};

/// matplotlib's default handle box: 20 × 7 points, no descent.
const AREA: HandleBox = HandleBox {
    xdescent: 0.0,
    ydescent: 0.0,
    width: 20.0,
    height: 7.0,
};

/// Runs the handler of `source` with the legend settings of `options`.
fn handle(fig: &Figure, source: &Source, options: &LegendOptions) -> Option<Vec<HandlerArtist>> {
    let rc = crate::rc::params();
    let settings = Settings::resolve(options, &rc).unwrap();
    let state = fig.raw().figure_state().borrow();
    let handlers = HandlerMap::builtin();
    let ctx = HandlerContext {
        store: &state.store,
        settings: &settings,
        rc: &rc,
        handlers: &handlers,
    };
    create(&ctx, source, &AREA).unwrap()
}

fn artist_source(artist: &impl ArtistHandle) -> Source {
    Source::Artist(artist.id())
}

fn as_line(artist: &HandlerArtist) -> &LineArtist {
    let any: &dyn Any = artist.artist.as_ref();
    any.downcast_ref().expect("a line")
}

#[test]
fn single_point_lines_mark_only_their_middle() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let props = LineProps::fmt("o--").unwrap().markersize(4.0).label("l");
    let line = ax.plot_with([0.0, 1.0], [0.0, 1.0], props).unwrap();
    let made = handle(&fig, &artist_source(&line), &LegendOptions::default()).unwrap();
    let legline = as_line(&made[0]);
    assert_eq!(legline.xdata, [0.0, 10.0, 20.0]);
    assert_eq!(legline.ydata, [3.5; 3]);
    assert_eq!(legline.markevery, Some(MarkEvery::Indices(vec![1])));
    assert_eq!(legline.markersize, 4.0);
    assert_eq!(legline.linestyle, crate::lines::LineStyle::Dashed);
    assert_eq!(legline.base().label, "l");

    let scaled = LegendOptions::default().numpoints(3_usize).markerscale(2.0);
    let made = handle(&fig, &artist_source(&line), &scaled).unwrap();
    let legline = as_line(&made[0]);
    assert_eq!(legline.xdata, [3.0, 10.0, 17.0], "a 0.3 font size pad");
    assert_eq!(legline.markevery, None);
    assert_eq!(legline.markersize, 8.0);
}

#[test]
fn a_handle_can_carry_its_own_number_of_markers() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let line = ax
        .plot_with([0.0, 1.0], [0.0, 1.0], LineProps::fmt("o-").unwrap())
        .unwrap();
    let four = LegendHandle::from(&line).with_numpoints(4);
    let made = handle(&fig, &four.0, &LegendOptions::default()).unwrap();
    let legline = as_line(&made[0]);
    assert_eq!(
        legline.xdata,
        linspace(3.0, 17.0, 4),
        "four markers in the pads"
    );
    assert_eq!(legline.markevery, None);
    let legend = ax
        .legend(
            LegendOptions::default()
                .handles([LegendHandle::from(&line).with_numpoints(4)])
                .labels(["four"]),
        )
        .unwrap();
    assert_eq!(legend.labels(), ["four"]);
}

#[test]
fn scatter_markers_span_the_plotted_sizes() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let options = ScatterOptions {
        sizes: Some(vec![10.0, 30.0, 50.0]),
        ..ScatterOptions::default()
    };
    let points = ax
        .scatter_with([0.0, 1.0, 2.0], [0.0, 1.0, 2.0], options)
        .unwrap();
    let sizes = |count: usize| {
        let options = LegendOptions::default().scatterpoints(count);
        let made = handle(&fig, &artist_source(&points), &options).unwrap();
        assert_eq!(made[0].attach, Attach::Offsets);
        let any: &dyn Any = made[0].artist.as_ref();
        let markers: &CollectionArtist = any.downcast_ref().expect("a collection");
        (markers.sizes.clone().unwrap(), markers.offsets.len())
    };
    assert_eq!(sizes(1), (vec![30.0], 1));
    assert_eq!(sizes(3), (vec![30.0, 50.0, 10.0], 3));
    let (many, offsets) = sizes(5);
    assert_eq!(many, [10.0, 20.0, 30.0, 40.0, 50.0]);
    assert_eq!(offsets, 5);
}

#[test]
fn error_bars_come_first_then_caps_then_the_line() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let options = ErrorbarOptions {
        xerr: Some(0.1.into()),
        yerr: Some(0.2.into()),
        capsize: Some(3.0),
        fmt: "o".into(),
        ..ErrorbarOptions::default()
    };
    let errors = ax.errorbar([0.0, 1.0], [0.0, 1.0], options).unwrap();
    let source = LegendHandle::from(&errors).0;
    let made = handle(&fig, &source, &LegendOptions::default()).unwrap();
    assert_eq!(made.len(), 2 + 4 + 2);
    let any: &dyn Any = made[0].artist.as_ref();
    let bars: &CollectionArtist = any.downcast_ref().expect("bars first");
    assert_eq!(
        bars.paths[0].vertices(),
        [Point::new(5.0, 3.5), Point::new(15.0, 3.5)]
    );
    let markers = as_line(&made[7]);
    assert_eq!(markers.xdata, [10.0]);
    assert_eq!(markers.linestyle, crate::lines::LineStyle::None);

    let hidden = ErrorbarOptions {
        yerr: Some(0.2.into()),
        fmt: "none".into(),
        ..ErrorbarOptions::default()
    };
    let bars_only = ax.errorbar([0.0], [0.0], hidden).unwrap();
    let made = handle(
        &fig,
        &LegendHandle::from(&bars_only).0,
        &LegendOptions::default(),
    )
    .unwrap();
    assert_eq!(made.len(), 3);
    assert!(!made[2].artist.base().visible, "no data line to show");
}

#[test]
fn stems_rise_to_the_scatter_height() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let stems = ax
        .stem([0.0, 1.0], [1.0, 2.0], &StemOptions::default())
        .unwrap();
    let made = handle(
        &fig,
        &LegendHandle::from(&stems).0,
        &LegendOptions::default(),
    )
    .unwrap();
    assert_eq!(made.len(), 3);
    let stem = as_line(&made[0]);
    assert_eq!(stem.xdata, [10.0, 10.0]);
    assert_eq!(stem.ydata, [0.0, 7.0 * (0.5 * 0.375 + 0.5)]);
    assert!(
        stem.linestyle.is_dashed(),
        "matplotlib's explicit dash pattern"
    );
    assert_eq!(as_line(&made[1]).xdata, [0.0, 20.0]);
}

#[test]
fn patches_become_rectangles_in_their_resolved_style() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let boxed = ax.add_patch(FancyBboxPatch::round((0.0, 0.0), 1.0, 1.0));
    boxed.set_facecolor("yellow").set_alpha(Some(0.5));
    let made = handle(&fig, &artist_source(&boxed), &LegendOptions::default()).unwrap();
    let any: &dyn Any = made[0].artist.as_ref();
    let rect: &PatchArtist<Rectangle> = any.downcast_ref().expect("a rectangle");
    assert_eq!(rect.shape, Rectangle::new((0.0, 0.0), 20.0, 7.0));
    assert_eq!(rect.style.facecolor, Some(Color::from("yellow")));
    assert_eq!(
        rect.style.edgecolor,
        Some(crate::rc::params().patch_edgecolor.clone()),
        "fancy boxes draw an edge by default"
    );
    assert_eq!(rect.base().alpha, Some(0.5));
}

#[test]
fn texts_have_no_handler() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let text = ax.text(0.0, 0.0, "note");
    text.set_label("note");
    assert!(handle(&fig, &artist_source(&text), &LegendOptions::default()).is_none());
}

#[test]
fn line_collections_hand_over_their_dash_pattern() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let solid = ax.vlines([1.0, 2.0], [0.0], [1.0]).unwrap();
    let made = handle(&fig, &artist_source(&solid), &LegendOptions::default()).unwrap();
    let legline = as_line(&made[0]);
    assert_eq!(
        legline.xdata,
        [0.0, 20.0],
        "one scatter point: the whole width"
    );
    assert_eq!(legline.linestyle, LineStyle::dashes(0.0, Vec::new()));
    assert!(
        legline.linestyle.is_dashed(),
        "drawn with the dash cap style"
    );
    let dashed = ax.hlines([1.0], [0.0], [1.0]).unwrap();
    dashed.set_linestyles(vec![LineStyle::Dashed]);
    let made = handle(&fig, &artist_source(&dashed), &LegendOptions::default()).unwrap();
    let pattern = crate::rc::params().lines_dashed_pattern.clone();
    assert_eq!(as_line(&made[0]).linestyle, LineStyle::dashes(0.0, pattern));
}

#[test]
fn proxy_lines_are_drawn_like_lines() {
    let fig = Figure::new();
    let mut proxy = LineArtist::new(vec![0.0], vec![0.0], crate::rc::params());
    proxy.marker = crate::markers::Marker::Symbol(crate::markers::MarkerSymbol::Square);
    proxy.markersize = 9.0;
    proxy.linestyle = crate::lines::LineStyle::None;
    let LegendHandle(source) = LegendHandle::proxy(proxy);
    let made = handle(&fig, &source, &LegendOptions::default()).unwrap();
    let legline = as_line(&made[0]);
    assert_eq!(legline.markersize, 9.0);
    assert_eq!(legline.markevery, Some(MarkEvery::Indices(vec![1])));
}

#[test]
fn handlers_of_other_kinds_of_handles_fail() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let line = ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).unwrap();
    let as_box = LegendHandle::from(&line).with_handler(HandlerPatch);
    assert_eq!(
        ax.legend(LegendOptions::default().handles([as_box]).labels(["box"]))
            .unwrap_err(),
        LegendError::UnsupportedHandle("HandlerPatch")
    );
    let patch = ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
    let as_errorbar = LegendHandle::from(&patch).with_handler(HandlerErrorbar);
    assert_eq!(
        ax.legend(
            LegendOptions::default()
                .handles([as_errorbar])
                .labels(["bars"])
        )
        .unwrap_err(),
        LegendError::UnsupportedHandle("HandlerErrorbar")
    );
    // A handle whose artist is gone draws nothing instead.
    let removed = Source::Handled(
        Box::new(artist_source(&line)),
        crate::legend::handle::OwnHandler(std::rc::Rc::new(HandlerPatch)),
    );
    line.remove();
    assert!(handle(&fig, &removed, &LegendOptions::default()).is_none());
}
