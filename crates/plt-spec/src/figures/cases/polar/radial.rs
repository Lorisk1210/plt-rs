//! Radial scale, grid, spine and annotation cases of
//! `figures/cases_polar.py`.

use std::f64::consts::{PI, TAU};

use plt::axes::{AxisName, ErrorbarOptions, StemOptions, TickParams};
use plt::colors::Color;
use plt::figure::Figure;
use plt::fonts::FontSize;
use plt::lines::{LineProps, LineStyle};
use plt::numeric::linspace;
use plt::patches::ArrowStyle;
use plt::polar::ThetaDirection;
use plt::prelude::ArtistHandle;
use plt::scale::ScaleOptions;
use plt::text::{
    AnnotateOptions, ArrowProps, FancyArrowProps, HorizontalAlignment, VerticalAlignment,
};
use plt::ticker::FormatStrFormatter;
use plt::transforms::Point;

use super::spiral;
use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("polar/rscale_log_rlim_rgrids", log_rgrids).sized(4.0, 4.0, 100.0),
    FigureCase::new("polar/rscale_symlog", symlog).sized(4.0, 4.0, 100.0),
    FigureCase::new("polar/rgrids_labels_thetagrids_fmt", labelled_grids).sized(4.0, 4.0, 100.0),
    FigureCase::new("polar/spines_grid_off_sector", spines).sized(5.0, 4.0, 100.0),
    FigureCase::new("polar/errorbar_annotate_text", annotated).sized(4.0, 4.0, 100.0),
    FigureCase::new("polar/theta_offset_radians_stem", stems).sized(4.0, 4.0, 100.0),
];

fn log_rgrids(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta = linspace(0.0, 2.0 * TAU, 300);
    let r: Vec<f64> = theta
        .iter()
        .map(|t| 10f64.powf(t / (2.0 * TAU) * 3.0))
        .collect();
    ax.plot(theta, r)?;
    ax.set_rscale("log")?;
    ax.set_rlim(1.0, 1000.0)
        .set_rgrids(&[1.0, 10.0, 100.0, 1000.0], Some(135.0));
    Ok(())
}

fn symlog(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta = linspace(0.0, TAU, 300);
    let r: Vec<f64> = theta
        .iter()
        .map(|t| 50.0 * (1.0 + (4.0 * t).sin()))
        .collect();
    ax.plot(theta, r)?;
    let options = ScaleOptions {
        linthresh: Some(1.0),
        ..ScaleOptions::default()
    };
    ax.set_scale_with(AxisName::Y, "symlog", &options)?;
    Ok(())
}

fn labelled_grids(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let (theta, r) = spiral();
    ax.plot_with(
        theta,
        r,
        LineProps::default().color(Color::from("tab:purple")),
    )?;
    ax.set_rgrids_labeled(&[0.5, 1.0, 1.5], ["low", "mid", "high"], Some(200.0))?;
    let angles: Vec<f64> = (0..12).map(|i| 30.0 * f64::from(i)).collect();
    ax.set_thetagrids(&angles);
    // matplotlib's `fmt` formats the tick angles in radians.
    ax.xaxis()
        .set_major_formatter(FormatStrFormatter::new("%d°")?);
    ax.tick_params(&TickParams {
        axis: Some(AxisName::Y),
        labelcolor: Some(Color::from("tab:purple")),
        labelsize: Some(FontSize::Points(8.0)),
        ..TickParams::default()
    });
    Ok(())
}

fn spines(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta = linspace(0.0, 3.0 * PI / 2.0, 60);
    let r: Vec<f64> = theta.iter().map(|t| 1.0 + t).collect();
    ax.plot(theta, r)?;
    ax.set_thetamax(270.0).set_rorigin(Some(-1.0));
    ax.grid(false);
    let spines = ax.spines();
    // The polar spines: `polar` (outer arc), `inner`, `start` and `end`.
    spines.top.set_color("tab:red").set_linewidth(2.0);
    spines.bottom.set_color("tab:blue");
    spines.left.set_linestyle(LineStyle::Dashed);
    spines.right.set_visible(false);
    Ok(())
}

fn annotated(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta: Vec<f64> = (0..8).map(|i| f64::from(i) * TAU / 8.0).collect();
    let r: Vec<f64> = theta.iter().map(|t| 2.0 + (2.0 * t).cos()).collect();
    let options = ErrorbarOptions {
        yerr: Some(0.3.into()),
        xerr: Some(0.1.into()),
        fmt: "o".into(),
        capsize: Some(3.0),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(theta, r, options)?;
    let arrow = FancyArrowProps::new("->".parse::<ArrowStyle>()?);
    ax.annotate(
        "peak",
        (0.0, 3.0),
        AnnotateOptions {
            xytext: Some(Point::new(PI / 4.0, 3.6)),
            arrowprops: Some(ArrowProps::Fancy(arrow)),
            ..AnnotateOptions::default()
        },
    )?
    .set_halign(HorizontalAlignment::Center);
    ax.text(PI, 1.5, "west")
        .set_halign(HorizontalAlignment::Center)
        .set_valign(VerticalAlignment::Center)
        .set_color("tab:red");
    ax.set_rmax(4.0);
    Ok(())
}

fn stems(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta: Vec<f64> = (0..12).map(|i| f64::from(i) * TAU / 12.0).collect();
    let r: Vec<f64> = theta.iter().map(|t| 1.0 + t / PI).collect();
    let options = StemOptions {
        basefmt: " ".into(),
        ..StemOptions::default()
    };
    ax.stem(&theta, &r, &options)?;
    ax.set_theta_offset(PI / 3.0)
        .set_theta_direction(ThetaDirection::Clockwise);
    Ok(())
}
