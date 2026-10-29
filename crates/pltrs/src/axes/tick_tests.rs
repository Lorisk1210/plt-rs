//! Tick parameters, tick label properties (of a level and of single
//! ticks), minor tick labels, overlapping minor ticks, the rotation of
//! tick labels and tick labels out of the layout.

use pltrs_core::{Point, Rgba};
use pltrs_text::{HAlign, RotationMode};

use super::{AxesState, AxisName, GridOptions, GridStyle, TickLabelProps, TickLevel, TickParams};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::figure::Figure;
use crate::fonts::{FontSize, FontStyle, FontWeight, RelativeSize};
use crate::layout::LayoutEngine;
use crate::lines::LineStyle;
use crate::rc::AxisBelow;
use crate::testing::{Call, RecordingRenderer};
use crate::ticker::MultipleLocator;

fn axes() -> super::Axes {
    let ax = Figure::new()
        .add_axes([0.125, 0.11, 0.775, 0.77])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 1000.0))
        .expect("finite limits");
    ax
}

/// The `(major, minor)` grid styles of the x axis.
fn grids(ax: &super::Axes) -> [(bool, GridStyle); 2] {
    ax.raw().with::<AxesState, _>(|a| {
        let x = a.axis(AxisName::X);
        [
            (x.major_style.grid_on, x.major_style.grid.clone()),
            (x.minor_style.grid_on, x.minor_style.grid.clone()),
        ]
    })
}

#[test]
fn negative_tick_label_rotations_lay_out_like_their_positive_equivalents() {
    // matplotlib reduces text rotations modulo 360, which the `ytick`
    // rotation mode relies on to choose the vertical alignment.
    let extent = |rotation: f64| {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_ylim(0.0, 1000.0).expect("finite limits");
        ax.tick_params(TickParams {
            axis: Some(AxisName::Y),
            labelrotation: Some(rotation),
            labelrotation_mode: Some(RotationMode::YTick),
            ..TickParams::default()
        });
        fig.tight_bbox()
    };
    assert_eq!(extent(-25.0), extent(335.0));
}

#[test]
fn the_tick_space_follows_the_major_label_size() {
    let ax = axes();
    let count = |ax: &super::Axes| ax.ticks(AxisName::X).expect("ticks").major.len();
    let default = count(&ax);
    ax.tick_params(TickParams {
        axis: Some(AxisName::X),
        labelsize: Some(FontSize::Points(40.0)),
        ..TickParams::default()
    });
    assert!(
        count(&ax) < default,
        "large labels leave room for fewer ticks"
    );
    ax.tick_params(TickParams {
        axis: Some(AxisName::X),
        which: TickLevel::Minor,
        labelsize: Some(FontSize::Points(4.0)),
        ..TickParams::default()
    });
    assert!(count(&ax) < default, "minor label sizes do not count");
}

#[test]
fn the_tick_space_follows_the_rc_label_size_when_laid_out() {
    // matplotlib reads `xtick.labelsize` when it draws, not when the axes
    // is made: large labels at creation leave the default room.
    let count = |ax: &super::Axes| ax.ticks(AxisName::X).expect("ticks").major.len();
    let default = count(&axes());
    let made_large = crate::rc::context(|| {
        crate::rc::set("xtick.labelsize", "40").expect("valid size");
        axes()
    });
    assert_eq!(count(&made_large), default);
    let laid_out_large = crate::rc::context(|| {
        crate::rc::set("xtick.labelsize", "40").expect("valid size");
        count(&axes())
    });
    assert!(
        laid_out_large < default,
        "large labels leave room for fewer ticks"
    );
}

#[test]
fn grid_styles_survive_hiding_and_follow_tick_parameters() {
    let ax = axes();
    ax.grid_with(
        true,
        GridOptions {
            color: Some(Color::from("red")),
            ..GridOptions::default()
        },
    );
    ax.grid(false);
    ax.tick_params(TickParams {
        which: TickLevel::Both,
        grid_linewidth: Some(3.0),
        grid_linestyle: Some(LineStyle::Dotted),
        ..TickParams::default()
    });
    ax.grid(true);
    let [(major_on, major), (minor_on, minor)] = grids(&ax);
    assert!(major_on && !minor_on);
    assert_eq!(major.color, Color::from("red"));
    assert_eq!((major.linewidth, minor.linewidth), (3.0, 3.0));
    assert_eq!(minor.linestyle, LineStyle::Dotted);
}

#[test]
fn minor_ticks_on_major_ticks_can_be_kept() {
    let ax = axes();
    let xaxis = ax.xaxis();
    xaxis
        .set_major_locator(MultipleLocator::new(5.0).unwrap())
        .set_minor_locator(MultipleLocator::new(2.5).unwrap());
    let minor = |ax: &super::Axes| {
        let ticks = ax.ticks(AxisName::X).unwrap();
        ticks.minor.iter().map(|t| t.loc).collect::<Vec<_>>()
    };
    assert_eq!(minor(&ax), [2.5, 7.5]);
    xaxis.set_remove_overlapping_locs(false);
    assert_eq!(minor(&ax), [0.0, 2.5, 5.0, 7.5, 10.0]);
}

#[test]
fn minor_ticks_take_labels_and_label_properties() {
    let ax = axes();
    let xaxis = ax.xaxis();
    xaxis.set_minor_ticks([2.5, 7.5]);
    xaxis.set_minor_ticklabels(["a", "b"]).unwrap();
    assert!(xaxis.set_minor_ticklabels(["only one"]).is_err());
    xaxis.set_ticklabel_props(
        TickLevel::Minor,
        TickLabelProps {
            color: Some(Color::from("tab:red")),
            fontsize: Some(FontSize::Points(7.0)),
            multialign: Some(HAlign::Right),
            ..TickLabelProps::default()
        },
    );
    let labels: Vec<String> = ax
        .ticks(AxisName::X)
        .unwrap()
        .minor
        .into_iter()
        .map(|t| t.label)
        .collect();
    assert_eq!(labels, ["a", "b"]);
    let style = ax
        .raw()
        .with::<AxesState, _>(|a| a.xaxis.minor_style.clone());
    assert_eq!(style.label_color, Color::from("tab:red"));
    assert_eq!(style.label_size, 7.0);
    assert_eq!(style.label_multialign, Some(HAlign::Right));
}

#[test]
fn tick_line_colours_leave_the_mark_edges_alone() {
    let ax = axes();
    ax.xaxis().set_tick_line_color("blue");
    for minor in [false, true] {
        let style = ax.xaxis().tick_style(minor);
        assert_eq!(style.color, Color::from("blue"));
        assert_eq!(style.edgecolor, Some(Color::from("black")));
    }
    assert_eq!(ax.yaxis().tick_style(false).edgecolor, None);
}

#[test]
fn axis_labels_take_a_font_style() {
    let ax = axes();
    ax.set_xlabel("x");
    ax.xaxis().set_label_style(FontStyle::Italic);
    let style = |name| {
        ax.raw()
            .with::<AxesState, _>(|a| a.axis(name).label.font.style)
    };
    assert_eq!(style(AxisName::X), FontStyle::Italic);
    assert_eq!(style(AxisName::Y), FontStyle::Normal);
}

#[test]
fn tick_labels_take_a_font_family_weight_and_style() {
    // matplotlib's `tick_params(labelfontfamily=...)` and
    // `set_ticks(..., fontweight="bold", fontstyle="italic")`.
    let ax = axes();
    ax.tick_params(TickParams {
        axis: Some(AxisName::Y),
        labelfontfamily: Some("serif".into()),
        ..TickParams::default()
    });
    ax.xaxis().set_ticklabel_props(
        TickLevel::Major,
        TickLabelProps {
            fontweight: Some(FontWeight::BOLD),
            fontstyle: Some(FontStyle::Italic),
            ..TickLabelProps::default()
        },
    );
    let rc = crate::rc::params();
    let y_font = ax.yaxis().tick_style(false).label_font(&rc);
    assert_eq!(y_font.families, ["serif"]);
    assert_eq!(y_font.weight, FontWeight::NORMAL);
    assert_eq!(y_font.style, FontStyle::Normal);
    let x_font = ax.xaxis().tick_style(false).label_font(&rc);
    assert_eq!(x_font.families, rc.font_family);
    assert_eq!(x_font.weight, FontWeight::BOLD);
    assert_eq!(x_font.style, FontStyle::Italic);
    assert_eq!(x_font.size, ax.xaxis().tick_style(false).label_size);
}

#[test]
fn ticklocs_and_labels_reach_beyond_the_view() {
    // matplotlib's test_tick_label_update: labels of every located tick.
    let ax = axes();
    ax.set_xticks(vec![-1.0, 0.0, 1.0, 2.0, 3.0]);
    ax.set_xlim(-0.5, 2.5).expect("finite limits");
    ax.xaxis()
        .set_ticklabels(["", "", "unit value", "", ""])
        .unwrap();
    let axis = ax.xaxis();
    assert_eq!(axis.ticklocs(false).unwrap(), [-1.0, 0.0, 1.0, 2.0, 3.0]);
    assert_eq!(
        axis.ticklabels(false).unwrap(),
        ["", "", "unit value", "", ""]
    );
    axis.set_tick_params(TickParams {
        labeltop: Some(true),
        ..TickParams::default()
    });
    assert_eq!(axis.ticklabels(false).unwrap().len(), 10, "both sides");
    assert!(axis.ticklocs(true).unwrap().is_empty(), "no minor ticks");
}

#[test]
fn fixed_ticks_are_subsampled_by_locator_params() {
    // matplotlib's test_subsampled_ticklabels.
    let ax = axes();
    let ticks: Vec<f64> = (0..10).map(|i| f64::from(i) + 0.1).collect();
    ax.xaxis().set_ticks(ticks);
    ax.locator_params(super::LocatorParams {
        nbins: Some(crate::ticker::Nbins::Fixed(5)),
        ..super::LocatorParams::default()
    })
    .unwrap();
    let labels: Vec<String> = "bcdefghijk".chars().map(String::from).collect();
    ax.xaxis().set_ticklabels(labels).unwrap();
    assert_eq!(
        ax.xaxis().ticklocs(false).unwrap(),
        [0.1, 2.1, 4.1, 6.1, 8.1]
    );
    assert_eq!(
        ax.xaxis().ticklabels(false).unwrap(),
        ["b", "d", "f", "h", "j"]
    );
    let steps = super::LocatorParams {
        steps: Some(vec![1.0, 2.0]),
        ..super::LocatorParams::default()
    };
    assert!(
        ax.locator_params(steps).is_err(),
        "fixed ticks take only nbins"
    );
}

#[test]
fn tick_labels_without_text_keep_the_axis_label_beyond_their_pad() {
    // matplotlib counts empty tick labels as empty boxes at their anchors
    // when it places the axis label (remove_text=True with a label).
    let label_left = |blank: bool| {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_ylabel("y");
        if blank {
            ax.yaxis().set_major_formatter(crate::ticker::NullFormatter);
        } else {
            ax.tick_params(TickParams {
                labelleft: Some(false),
                ..TickParams::default()
            });
        }
        fig.tight_bbox().x0()
    };
    let pad_inches = 3.5 / 72.0;
    assert!((label_left(false) - label_left(true) - pad_inches).abs() < 1e-9);
}

#[test]
fn a_scale_autoscales_with_its_own_locator() {
    // matplotlib's test_log_scales_no_data: the log locator's default
    // limits stay when the locator is replaced afterwards.
    let ax = Figure::new().subplot();
    ax.set_xscale("log").unwrap();
    ax.xaxis()
        .set_major_locator(MultipleLocator::new(1.0).unwrap());
    let (low, high) = ax.xlim();
    assert!((low - 0.891_250_938_133_745_5).abs() < 1e-12, "{low}");
    assert!((high - 11.220_184_543_019_636).abs() < 1e-12, "{high}");
}

#[test]
fn a_single_major_tick_label_takes_its_own_properties() {
    // matplotlib's `ax.xaxis.majorTicks[2].label1.set_color("red")`: the
    // index counts every located tick, the one left of the view included.
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let ax = fig.subplot();
    ax.set_xticks(vec![-1.0, 0.0, 1.0, 2.0, 3.0]);
    ax.set_xlim(-0.5, 2.5).expect("finite limits");
    let red = Rgba::opaque(1.0, 0.0, 0.0);
    ax.xaxis().set_major_ticklabel_props(
        2,
        TickLabelProps {
            color: Some(Color::from(red)),
            ..TickLabelProps::default()
        },
    );
    let mut renderer = RecordingRenderer::new(400.0, 300.0, 100.0);
    fig.draw(&mut renderer).expect("drawable");
    let red_x: Vec<f64> = (renderer.calls.iter())
        .filter_map(|call| match call {
            Call::Path {
                points,
                fill: Some(fill),
                ..
            } if *fill == red => Some(points.iter().map(|p| p.x)),
            _ => None,
        })
        .flatten()
        .collect();
    let tick_x = (fig.resolve_transform(&ax.trans_data()))
        .expect("resolvable")
        .transform_point(Point::new(1.0, 0.0))
        .x;
    assert!(!red_x.is_empty(), "the label of 1.0 is drawn in red");
    assert!(
        red_x.iter().all(|x| (x - tick_x).abs() < 15.0),
        "only the label of 1.0 (at {tick_x}) is red: {red_x:?}"
    );
}

#[test]
fn single_tick_label_properties_accumulate_on_the_major_style() {
    let ax = axes();
    let axis = ax.xaxis();
    axis.set_major_ticklabel_props(
        1,
        TickLabelProps {
            color: Some(Color::from("red")),
            ..TickLabelProps::default()
        },
    );
    axis.set_major_ticklabel_props(
        1,
        TickLabelProps {
            rotation: Some(30.0),
            fontsize: Some(FontSize::Relative(RelativeSize::Large)),
            ..TickLabelProps::default()
        },
    );
    let (base, own, other) = ax.raw().with::<AxesState, _>(|a| {
        (
            a.xaxis.major_style.clone(),
            a.xaxis.major_tick_style(1).into_owned(),
            a.xaxis.major_tick_style(0).into_owned(),
        )
    });
    assert_eq!(own.label_color, Color::from("red"));
    assert_eq!(own.label_rotation, 30.0);
    assert_eq!(own.label_size, 12.0, "large against font.size 10");
    assert_eq!(other, base, "other ticks keep the major style");
}

#[test]
fn tick_labels_link_to_their_url() {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.set_xticks([0.0, 1.0]);
    let props = TickLabelProps {
        url: Some("https://example.com/one".to_owned()),
        ..TickLabelProps::default()
    };
    ax.xaxis().set_major_ticklabel_props(1, props);
    let svg = fig.to_svg().unwrap();
    assert_eq!(
        svg.matches("<a xlink:href=\"https://example.com/one\"")
            .count(),
        1,
        "only the label of the second tick is linked"
    );
}

/// The position of a constrained subplot whose x ticks at 0 and 1 are
/// labelled `labels`, the second label taken out of the layout after a
/// first layout when `out_of_layout`.
fn constrained_position(labels: [&str; 2], out_of_layout: bool) -> pltrs_core::Bbox {
    let fig = Figure::new();
    fig.set_layout_engine(LayoutEngine::constrained()).unwrap();
    let ax = fig.subplot();
    ax.set_xticks([0.0, 1.0]).set_xticklabels(labels).unwrap();
    fig.layout();
    if out_of_layout {
        let props = TickLabelProps {
            in_layout: Some(false),
            ..TickLabelProps::default()
        };
        ax.xaxis().set_major_ticklabel_props(1, props);
    }
    ax.position()
}

#[test]
fn a_tick_label_out_of_the_layout_takes_no_room() {
    // matplotlib's test_tick_not_in_layout: the layout is as if the label
    // were empty.
    let long = "a very, very long tick label";
    let ignored = constrained_position(["short", long], true);
    assert_eq!(ignored, constrained_position(["short", ""], false));
    assert_ne!(ignored, constrained_position(["short", long], false));
}

#[test]
fn a_tick_label_out_of_the_layout_leaves_the_axis_label_alone() {
    // matplotlib's `_get_ticklabel_bboxes` skips labels not in the layout
    // when it places the axis label beyond the tick labels.
    let figure_bottom = |out_of_layout: bool| {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xticks([0.0, 1.0])
            .set_xticklabels(["short", "tall\nlabel"])
            .unwrap();
        ax.set_xlabel("x");
        if out_of_layout {
            let props = TickLabelProps {
                in_layout: Some(false),
                ..TickLabelProps::default()
            };
            ax.xaxis().set_major_ticklabel_props(1, props);
        }
        fig.tight_bbox().y0()
    };
    assert!(figure_bottom(true) > figure_bottom(false));
}

#[test]
fn axisbelow_reads_back_and_survives_clearing() {
    let ax = Figure::new().subplot();
    assert_eq!(ax.axisbelow(), AxisBelow::Line, "axes.axisbelow");
    ax.set_axisbelow(AxisBelow::Above);
    assert_eq!(ax.axisbelow(), AxisBelow::Above);
    ax.cla();
    assert_eq!(ax.axisbelow(), AxisBelow::Above);
}

#[test]
fn absurd_tick_counts_are_reported_not_drawn() {
    // Found by fuzzing: a million tick labels took minutes to draw.
    let fig = Figure::with_size(2.0, 2.0, 50.0);
    let ax = fig.subplot();
    let locator = |nbins| crate::ticker::MaxNLocator::new(crate::ticker::Nbins::Fixed(nbins));
    ax.xaxis().set_major_locator(locator(1 << 20));
    let error = fig.to_png().expect_err("the x axis cannot be drawn");
    assert!(
        error.to_string().contains("refusing to generate"),
        "{error}"
    );
    ax.xaxis().set_major_locator(locator(10));
    assert!(fig.to_png().is_ok());
}
