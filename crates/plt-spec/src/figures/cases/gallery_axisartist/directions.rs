//! Directions of axis artists: `axis_direction`, `demo_axis_direction`
//! and `simple_axis_pad`.

use plt::axes::{AxisName, GridOptions, TickDirection};
use plt::axisartist::{self, AxisArtist, AxislineStyle, LabelDirection, Toggle};
use plt::gridspec::SubplotOverrides;
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::spines::SpineSide;
use plt::text::{AnnotateOptions, HAlign, VAlign};
use plt_core::Point;

use crate::figures::cases::common::{polar_box, show_label};
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_axisartist/axis_direction", axis_direction)
        .sized(10.0, 4.0, 100.0)
        .with_rc(&[("axes.titlesize", "medium"), ("axes.titley", "1.1")]),
    FigureCase::new(
        "gallery_axisartist/demo_axis_direction",
        demo_axis_direction,
    )
    .sized(8.0, 4.0, 100.0),
    FigureCase::new("gallery_axisartist/simple_axis_pad", simple_axis_pad).sized(9.0, 3.0, 100.0),
];

/// Hides (or shows) the tick labels of `axis`.
fn toggle_ticklabels(axis: &AxisArtist, visible: bool) {
    axis.toggle(&Toggle {
        ticklabels: Some(visible),
        ..Toggle::default()
    });
}

/// `axis_direction`'s `setup_axes`: in cell `index` of the 2 × 5 grid,
/// only a vertical axis line `x` with an arrow, through x = 0.5; returns
/// the axes and that line.
fn arrow_axis(
    fig: &Figure,
    index: usize,
) -> Result<(Axes, AxisArtist), Box<dyn std::error::Error>> {
    let ax = axisartist::AxisArtistAxes::new(fig.add_subplot(2, 5, index)?);
    ax.ax().set_ylim(-0.1, 1.5)?.set_yticks([0.0, 1.0]);
    for axis in ax.axis_artists() {
        axis.set_visible(false);
    }
    let axis = ax.new_floating_axis("x", AxisName::Y, 0.5);
    axis.set_axisline_style(Some("->, size=1.5".parse::<AxislineStyle>()?));
    Ok((ax.ax().clone(), axis))
}

fn axis_direction(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        bottom: Some(0.1),
        top: Some(0.9),
        left: Some(0.05),
        right: Some(0.95),
        ..SubplotOverrides::default()
    })?;
    arrow_axis(fig, 1)?.1.set_axis_direction(SpineSide::Left);
    let sides = [(LabelDirection::Plus, "+"), (LabelDirection::Minus, "-")];
    for (index, (direction, sign)) in [2, 3].into_iter().zip(sides) {
        let (ax, axis) = arrow_axis(fig, index)?;
        axis.label().set_text("Label");
        toggle_ticklabels(&axis, false);
        axis.set_axislabel_direction(direction);
        ax.set_title(format!("label direction=${sign}$"));
    }
    for (index, (direction, sign)) in [4, 5].into_iter().zip(sides) {
        let (ax, axis) = arrow_axis(fig, index)?;
        axis.set_ticklabel_direction(direction);
        ax.set_title(format!("ticklabel direction=${sign}$"));
    }
    let (_, ax7) = arrow_axis(fig, 7)?;
    ax7.label().set_text("rotation=10").set_rotation(10.0);
    toggle_ticklabels(&ax7, false);
    let (_, ax8) = arrow_axis(fig, 8)?;
    ax8.set_axislabel_direction(LabelDirection::Minus);
    ax8.label().set_text("rotation=10").set_rotation(10.0);
    toggle_ticklabels(&ax8, false);
    Ok(())
}

/// `add_floating_axis1`: the labelled axis line θ = 30°.
fn floating_lat(ax: &axisartist::AxisArtistAxes) -> AxisArtist {
    let axis = ax.new_floating_axis("lat", AxisName::X, 30.0);
    show_label(&axis, r"$\theta = 30^{\circ}$");
    axis
}

/// `add_floating_axis2`: the labelled axis line r = 6.
fn floating_lon(ax: &axisartist::AxisArtistAxes) -> AxisArtist {
    let axis = ax.new_floating_axis("lon", AxisName::Y, 6.0);
    show_label(&axis, r"$r = 6$");
    axis
}

fn demo_axis_direction(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let rows: [fn(&axisartist::AxisArtistAxes) -> AxisArtist; 2] = [floating_lat, floating_lon];
    for (row, add_floating_axis) in rows.into_iter().enumerate() {
        for (i, direction) in [
            SpineSide::Bottom,
            SpineSide::Left,
            SpineSide::Top,
            SpineSide::Right,
        ]
        .into_iter()
        .enumerate()
        {
            let ax = polar_box(fig, (2, 4, 4 * row + i + 1))?;
            for axis in ax.axis_artists() {
                toggle_ticklabels(&axis, false);
            }
            ax.grid(
                true,
                GridOptions {
                    color: Some(".9".into()),
                    ..GridOptions::default()
                },
            );
            add_floating_axis(&ax).set_axis_direction(direction);
            ax.ax().set_title(direction.name());
        }
    }
    Ok(())
}

/// A change to the floating axis of `simple_axis_pad`.
type Adjustment = fn(&AxisArtist);

fn simple_axis_pad(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.01),
        right: Some(0.99),
        bottom: Some(0.01),
        top: Some(0.99),
        wspace: Some(0.01),
        hspace: Some(0.01),
    })?;
    let adjustments: [(&str, Adjustment); 6] = [
        ("default", |_| {}),
        ("ticklabels.set_pad(10)", |axis| {
            axis.major_ticklabels().set_pad(10.0);
        }),
        ("label.set_pad(20)", |axis| {
            axis.label().set_pad(20.0);
        }),
        (r#"ticks.set_tick_direction("in")"#, |axis| {
            axis.major_ticks().set_tick_direction(TickDirection::In);
        }),
        (r#"ticks.set_tick_direction("out")"#, |axis| {
            axis.major_ticks().set_tick_direction(TickDirection::Out);
        }),
        (r#"ticks.set_tick_direction("inout")"#, |axis| {
            axis.major_ticks().set_tick_direction(TickDirection::InOut);
        }),
    ];
    for (i, (note, adjust)) in adjustments.into_iter().enumerate() {
        let ax = polar_box(fig, (2, 3, i + 1))?;
        for axis in ax.axis_artists() {
            axis.set_visible(false);
        }
        adjust(&floating_lat(&ax));
        ax.ax()
            .annotate(
                note,
                (0.5, 1.0),
                AnnotateOptions {
                    xytext: Some(Point::new(5.0, -5.0)),
                    xycoords: "axes fraction".parse()?,
                    textcoords: Some("offset points".parse()?),
                    ..AnnotateOptions::default()
                },
            )?
            .set_valign(VAlign::Top)
            .set_halign(HAlign::Center);
    }
    Ok(())
}
