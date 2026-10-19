//! Tick parameters, tick label properties, minor tick labels, overlapping
//! minor ticks and the rotation of tick labels.

use plt_text::{HAlign, RotationMode};

use super::{AxesState, AxisName, GridOptions, GridStyle, TickLabelProps, TickLevel, TickParams};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::figure::Figure;
use crate::fonts::FontSize;
use crate::lines::LineStyle;
use crate::ticker::MultipleLocator;

fn axes() -> super::Axes {
    let ax = Figure::new().add_axes([0.125, 0.11, 0.775, 0.77]);
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 1000.0);
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
        ax.set_ylim(0.0, 1000.0);
        ax.tick_params(&TickParams {
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
    let count = |ax: &super::Axes| ax.ticks(AxisName::X).map(|t| t.major.len());
    let default = count(&ax);
    ax.tick_params(&TickParams {
        axis: Some(AxisName::X),
        labelsize: Some(FontSize::Points(40.0)),
        ..TickParams::default()
    });
    assert!(
        count(&ax) < default,
        "large labels leave room for fewer ticks"
    );
    ax.tick_params(&TickParams {
        axis: Some(AxisName::X),
        which: TickLevel::Minor,
        labelsize: Some(FontSize::Points(4.0)),
        ..TickParams::default()
    });
    assert!(count(&ax) < default, "minor label sizes do not count");
}

#[test]
fn grid_styles_survive_hiding_and_follow_tick_parameters() {
    let ax = axes();
    ax.grid_with(
        true,
        &GridOptions {
            color: Some(Color::from("red")),
            ..GridOptions::default()
        },
    );
    ax.grid(false);
    ax.tick_params(&TickParams {
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
        &TickLabelProps {
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
fn ticklocs_and_labels_reach_beyond_the_view() {
    // matplotlib's test_tick_label_update: labels of every located tick.
    let ax = axes();
    ax.set_xticks(vec![-1.0, 0.0, 1.0, 2.0, 3.0]);
    ax.set_xlim(-0.5, 2.5);
    ax.xaxis()
        .set_ticklabels(["", "", "unit value", "", ""])
        .unwrap();
    let axis = ax.xaxis();
    assert_eq!(axis.ticklocs(false).unwrap(), [-1.0, 0.0, 1.0, 2.0, 3.0]);
    assert_eq!(
        axis.ticklabels(false).unwrap(),
        ["", "", "unit value", "", ""]
    );
    axis.set_tick_params(&TickParams {
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
    ax.locator_params(&super::LocatorParams {
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
        ax.locator_params(&steps).is_err(),
        "fixed ticks take only nbins"
    );
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
