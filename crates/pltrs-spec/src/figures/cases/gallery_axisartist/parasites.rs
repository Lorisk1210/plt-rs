//! Parasite axes with axis artists of their own: `demo_parasite_axes`
//! and `demo_parasite_axes2`.

use pltrs::axes_grid1::ParasiteLink;
use pltrs::axisartist::{self, Toggle};
use pltrs::gridspec::SubplotOverrides;
use pltrs::legend::LegendOptions;
use pltrs::prelude::*;
use pltrs::spines::SpineSide;

use crate::figures::cases::common::axis_artist;
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_axisartist/demo_parasite_axes", demo_parasite_axes),
    FigureCase::new(
        "gallery_axisartist/demo_parasite_axes2",
        demo_parasite_axes2,
    ),
];

/// The plots, limits, labels and legend both demos share; returns the
/// colours of the host's line and of each parasite's.
fn three_datasets(
    host: &Axes,
    par1: &Axes,
    par2: &Axes,
) -> Result<[Color; 3], Box<dyn std::error::Error>> {
    let x = [0.0, 1.0, 2.0];
    let mut colors = Vec::with_capacity(3);
    for (ax, y, label) in [
        (host, [0.0, 1.0, 2.0], "Density"),
        (par1, [0.0, 3.0, 2.0], "Temperature"),
        (par2, [50.0, 30.0, 15.0], "Velocity"),
    ] {
        let line = ax.plot(x, y)?;
        line.set_label(label);
        colors.push(line.color());
    }
    host.set_xlim(0.0, 2.0)?
        .set_ylim(0.0, 2.0)?
        .set_xlabel("Distance")
        .set_ylabel("Density");
    par1.set_ylim(0.0, 4.0)?.set_ylabel("Temperature");
    par2.set_ylim(1.0, 65.0)?.set_ylabel("Velocity");
    host.legend(LegendOptions::default())?;
    Ok(colors
        .try_into()
        .expect("one colour for each of the three lines"))
}

fn demo_parasite_axes(fig: &Figure) -> CaseResult {
    let host = axisartist::AxisArtistAxes::new(fig.add_axes([0.15, 0.1, 0.65, 0.8])?);
    let par1 = host.add_parasite_axes(ParasiteLink::Independent)?;
    par1.ax().share_x(host.ax())?;
    let par2 = host.add_parasite_axes(ParasiteLink::Independent)?;
    par2.ax().share_x(host.ax())?;
    axis_artist(&host, "right")?.set_visible(false);
    let right = axis_artist(&par1, "right")?;
    right.set_visible(true);
    right.major_ticklabels().set_visible(true);
    right.label().set_visible(true);
    let right2 = par2.new_fixed_axis("right2", SpineSide::Right, (60.0, 0.0));
    let [c1, c2, c3] = three_datasets(host.ax(), par1.ax(), par2.ax())?;
    axis_artist(&host, "left")?.label().set_color(Some(c1));
    right.label().set_color(Some(c2));
    right2.label().set_color(Some(c3));
    Ok(())
}

fn demo_parasite_axes2(fig: &Figure) -> CaseResult {
    let host = axisartist::AxisArtistAxes::new(fig.subplot());
    fig.subplots_adjust(SubplotOverrides {
        right: Some(0.75),
        ..SubplotOverrides::default()
    })?;
    let par1 = host.twinx();
    let par2 = host.twinx();
    par2.new_fixed_axis("right", SpineSide::Right, (60.0, 0.0));
    let everything = Toggle {
        all: Some(true),
        ..Toggle::default()
    };
    let right1 = axis_artist(&par1, "right")?;
    right1.toggle(everything);
    let right2 = axis_artist(&par2, "right")?;
    right2.toggle(everything);
    let [c1, c2, c3] = three_datasets(host.ax(), par1.ax(), par2.ax())?;
    axis_artist(&host, "left")?.label().set_color(Some(c1));
    right1.label().set_color(Some(c2));
    right2.label().set_color(Some(c3));
    Ok(())
}
