//! Grid and tick-formatting cases of `figures/cases_geo.py`, on every
//! projection.

use std::f64::consts::PI;

use plt::axes::{AxisName, GridOptions, TickParams};
use plt::colors::Color;
use plt::figure::Figure;
use plt::fonts::FontSize;
use plt::geo::GeoProjection;
use plt::lines::{LineProps, LineStyle};
use plt::ticker::FuncFormatter;

use super::track;
use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("geo/four_projections_grids", four_projections).sized(8.0, 6.0, 100.0),
    FigureCase::new("geo/hammer_coarse_grid_styled_ticks", hammer_coarse),
    FigureCase::new("geo/mollweide_fine_grid_ends", mollweide_fine),
    FigureCase::new("geo/lambert_south_center_grid", lambert_south).sized(5.0, 5.0, 100.0),
    FigureCase::new("geo/aitoff_custom_ticklabels", custom_ticklabels),
    FigureCase::new("geo/mollweide_hour_formatter", hour_formatter),
];

/// `ax.tick_params(labelsize=size)` for both axes.
fn label_size(size: f64) -> TickParams {
    TickParams {
        labelsize: Some(FontSize::Points(size)),
        ..TickParams::default()
    }
}

fn four_projections(fig: &Figure) -> CaseResult {
    let projections = [
        ("aitoff", GeoProjection::Aitoff),
        ("hammer", GeoProjection::Hammer),
        ("lambert", GeoProjection::LAMBERT),
        ("mollweide", GeoProjection::Mollweide),
    ];
    for (i, (name, projection)) in projections.into_iter().enumerate() {
        let ax = fig.add_geo_subplot(projection, 2, 2, i + 1)?;
        let (lon, lat) = track();
        ax.plot_with(lon, lat, LineProps::default().color("tab:red"))?;
        ax.grid(true);
        ax.set_title(name).set_fontsize(10.0);
        ax.tick_params(&label_size(7.0));
    }
    Ok(())
}

fn hammer_coarse(fig: &Figure) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Hammer);
    ax.set_longitude_grid(60.0)?.set_latitude_grid(20.0)?;
    ax.grid_with(
        true,
        GridOptions {
            color: Some(Color::from("0.6")),
            linestyle: Some(LineStyle::Dotted),
            ..GridOptions::default()
        },
    );
    ax.tick_params(&TickParams {
        axis: Some(AxisName::X),
        labelcolor: Some(Color::from("tab:blue")),
        labelsize: Some(FontSize::Points(8.0)),
        ..TickParams::default()
    });
    ax.tick_params(&TickParams {
        axis: Some(AxisName::Y),
        labelcolor: Some(Color::from("tab:green")),
        ..TickParams::default()
    });
    Ok(())
}

fn mollweide_fine(fig: &Figure) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Mollweide);
    ax.set_longitude_grid(20.0)?
        .set_latitude_grid(10.0)?
        .set_longitude_grid_ends(80.0);
    ax.grid_with(
        true,
        GridOptions {
            linewidth: Some(0.5),
            ..GridOptions::default()
        },
    );
    ax.tick_params(&label_size(6.0));
    Ok(())
}

fn lambert_south(fig: &Figure) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Lambert {
        center_longitude: -PI / 3.0,
        center_latitude: -PI / 4.0,
    });
    ax.set_longitude_grid(45.0)?.set_latitude_grid(15.0)?;
    let (lon, lat) = track();
    ax.plot_with(lon, lat, LineProps::default().color("tab:purple"))?;
    ax.grid(true);
    Ok(())
}

/// `f"{abs(d)}°{hemisphere}"` for `d` in `range(start, stop + 1, step)`.
fn degree_labels(
    start: i32,
    stop: i32,
    step: usize,
    negative: &str,
    positive: &str,
) -> Vec<String> {
    (start..=stop)
        .step_by(step)
        .map(|d| {
            let hemisphere = match d.signum() {
                -1 => negative,
                1 => positive,
                _ => "",
            };
            format!("{}°{hemisphere}", d.abs())
        })
        .collect()
}

fn custom_ticklabels(fig: &Figure) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Aitoff);
    ax.set_xticklabels(degree_labels(-150, 150, 30, "W", "E"))?;
    ax.set_yticklabels(degree_labels(-75, 75, 15, "S", "N"))?;
    ax.grid(true);
    Ok(())
}

fn hour_formatter(fig: &Figure) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Mollweide);
    ax.xaxis().set_major_formatter(FuncFormatter::new(|x, _| {
        let hours = (x.to_degrees() / 15.0).round_ties_even();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Python's int() of a small value"
        )]
        let hours = hours as i64;
        format!("{}h", hours.rem_euclid(24))
    }));
    let (lon, lat) = track();
    ax.plot(lon, lat)?;
    ax.grid(true);
    Ok(())
}
