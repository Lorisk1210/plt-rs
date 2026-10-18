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
