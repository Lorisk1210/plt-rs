//! Orientation, sector and radial-origin cases of
//! `figures/cases_polar.py`.

use std::f64::consts::TAU;

use plt::axes::{BarOptions, FillBetweenOptions};
use plt::collections::CollectionProps;
use plt::colors::Color;
use plt::figure::Figure;
use plt::gridspec::SubplotOverrides;
use plt::lines::LineProps;
use plt::numeric::linspace;
use plt::patches::PatchProps;
use plt::polar::{Compass, ThetaDirection};
use plt_render::CapStyle;

use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("polar/theta_zero_locations_directions", zero_locations).sized(6.0, 6.0, 100.0),
    FigureCase::new("polar/sector_gauge_clockwise", gauge).sized(5.0, 4.0, 100.0),
    FigureCase::new("polar/sector_narrow_thetalim", narrow).sized(5.0, 4.0, 100.0),
    FigureCase::new("polar/sector_bars_rorigin_west", bars_west).sized(5.0, 3.5, 100.0),
    FigureCase::new("polar/negative_radii_autoscale", negative_radii).sized(4.0, 4.0, 100.0),
    FigureCase::new("polar/rlim_negative_rorigin", rlim_negative).sized(4.0, 4.0, 100.0),
    FigureCase::new("polar/radial_data_offset", radial_offset).sized(4.0, 4.0, 100.0),
];

/// `np.radians(values)`.
fn radians(degrees: &[f64]) -> Vec<f64> {
    degrees.iter().map(|d| d.to_radians()).collect()
}

fn zero_locations(fig: &Figure) -> CaseResult {
    let settings = [
        (Compass::NE, "NE", ThetaDirection::Counterclockwise, 0.0),
        (Compass::SW, "SW", ThetaDirection::Clockwise, 0.0),
        (Compass::E, "E", ThetaDirection::Clockwise, 30.0),
        (Compass::S, "S", ThetaDirection::Counterclockwise, -15.0),
    ];
    let theta: Vec<f64> = (0..8).map(|i| f64::from(i) * TAU / 8.0).collect();
    let heights: Vec<f64> = (0..8).map(|i| f64::from(1 + i % 3)).collect();
    for (i, (location, name, direction, offset)) in settings.into_iter().enumerate() {
        let ax = fig.add_polar_subplot(2, 2, i + 1)?;
        let options = BarOptions {
            thickness: vec![0.5],
            color: vec![Color::from("tab:blue")],
            props: PatchProps::default().alpha(0.6),
            ..BarOptions::default()
        };
        ax.bar(theta.clone(), heights.clone(), options)?;
        let props = LineProps::default()
            .color(Color::from("tab:red"))
            .linewidth(2.0);
        ax.plot_with([0.0, 0.0], [0.0, 3.0], props)?;
        ax.set_theta_zero_location(location, offset)
            .set_theta_direction(direction);
        let sign = if direction == ThetaDirection::Clockwise {
            "-1"
        } else {
            "+1"
        };
        ax.set_title(format!("{name}, {sign}, {offset}°"))
            .set_fontsize(9.0);
    }
    fig.subplots_adjust(&SubplotOverrides {
        hspace: Some(0.4),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

fn gauge(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    ax.set_theta_zero_location(Compass::N, 0.0)
        .set_theta_direction(ThetaDirection::Clockwise)
        .set_thetamin(-135.0)
        .set_thetamax(135.0);
    let theta = radians(&linspace(-135.0, 60.0, 50));
    let props = LineProps::default()
        .linewidth(8.0)
        .color(Color::from("tab:green"))
        .solid_capstyle(CapStyle::Butt);
    ax.plot_with(theta, vec![0.8; 50], props)?;
    ax.set_rorigin(Some(-1.0))
        .set_rlim(0.0, 1.0)?
        .set_rticks(&[]);
    let angles: Vec<f64> = (0..7).map(|i| -135.0 + 45.0 * f64::from(i)).collect();
    ax.set_thetagrids(&angles);
    Ok(())
}

fn narrow(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta = radians(&linspace(10.0, 40.0, 30));
    let r: Vec<f64> = theta.iter().map(|t| 5.0 + (t * 20.0).sin()).collect();
    ax.plot_with(theta, r, LineProps::fmt("o-")?.markersize(3.0))?;
    ax.set_thetalim(10f64.to_radians(), 40f64.to_radians())?;
    ax.set_rlim(3.0, 7.0)?;
    Ok(())
}

fn bars_west(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    ax.set_theta_zero_location(Compass::W, 0.0)
        .set_theta_direction(ThetaDirection::Clockwise)
        .set_thetamin(0.0)
        .set_thetamax(180.0);
    let theta: Vec<f64> = (0..9)
        .map(|i| f64::from(10 + 20 * i).to_radians())
        .collect();
    let options = BarOptions {
        thickness: vec![15f64.to_radians()],
        base: vec![1.0],
        color: ["C0", "C1", "C2"]
            .repeat(3)
            .into_iter()
            .map(Color::from)
            .collect(),
        edgecolor: vec![Color::from("k")],
        linewidth: vec![0.5],
        ..BarOptions::default()
    };
    ax.bar(
        theta,
        [3.0, 5.0, 2.0, 6.0, 4.0, 7.0, 1.0, 5.0, 3.0],
        options,
    )?;
    ax.set_rorigin(Some(-2.0)).set_rlabel_position(90.0);
    Ok(())
}

fn negative_radii(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta = linspace(0.0, TAU, 200);
    let r: Vec<f64> = theta.iter().map(|t| (3.0 * t).cos() + 0.5).collect();
    ax.plot(&theta, r)?;
    ax.plot_with(
        &theta,
        vec![-0.5; 200],
        LineProps::fmt("--")?.color(Color::from("0.5")),
    )?;
    Ok(())
}

fn rlim_negative(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta = linspace(0.0, TAU, 200);
    let r: Vec<f64> = theta.iter().map(|t| 2.0 * (2.0 * t).sin()).collect();
    ax.plot(theta, r)?;
    ax.set_rlim(-2.0, 2.0)?.set_rorigin(Some(-3.0));
    Ok(())
}

fn radial_offset(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta = linspace(0.0, TAU, 100);
    let r: Vec<f64> = theta.iter().map(|t| 7.5 + 2.5 * (5.0 * t).sin()).collect();
    ax.plot(&theta, &r)?;
    let band = FillBetweenOptions {
        props: CollectionProps::default().alpha(0.3),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(&theta, [5.0], r, band)?;
    Ok(())
}
