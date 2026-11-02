//! The plot families of everyday charts beyond lines and vertical bars:
//! horizontal bars, pies, polar axes, error bars, stairs and event
//! plots, in small, medium and large sizes.

use std::f64::consts::PI;

use pltrs::axes::{
    BarLabelOptions, ErrorSize, ErrorbarFormat, ErrorbarOptions, EventplotOptions, PieOptions,
    StairsOptions,
};
use pltrs::prelude::*;
use pltrs_spec::rng::SpecRng;

use super::{Build, Tier, Workload, random_walk};

/// The workloads of this module.
pub(super) const WORKLOADS: &[Workload] = &[
    Workload::new("small/barh_labels_png", "barh_labels", barh_labels).in_tier(Tier::Small),
    Workload::new("small/pie_png", "pie", pie).in_tier(Tier::Small),
    Workload::new("small/polar_rose_png", "polar_rose", polar_rose).in_tier(Tier::Small),
    Workload::new("small/errorbar_20_png", "errorbar_20", errorbar_20).in_tier(Tier::Small),
    Workload::new("small/stairs_50_png", "stairs_50", stairs_50).in_tier(Tier::Small),
    Workload::new("medium/eventplot_50x200_png", "eventplot_50x200", eventplot)
        .in_tier(Tier::Medium),
    Workload::new(
        "medium/polar_scatter_1e4_png",
        "polar_scatter_1e4",
        polar_scatter,
    )
    .in_tier(Tier::Medium),
    Workload::new("lines/errorbar_1e4", "errorbar_1e4", errorbar_1e4),
    Workload::new("stats/stairs_1e5", "stairs_1e5", stairs_1e5),
];

/// Eight horizontal bars of whole values, labelled, the first on top.
fn barh_labels() -> Build {
    let days: Vec<f64> = SpecRng::new(40)
        .uniform(8, 10.0, 90.0)
        .into_iter()
        .map(f64::round)
        .collect();
    Box::new(move |fig| {
        let ax = fig.subplot();
        let cities = [
            "Oslo", "Lima", "Pune", "Kyiv", "Turin", "Quito", "Perth", "Graz",
        ];
        let bars = ax.barh(cities, &days, BarOptions::default())?;
        ax.bar_label(&bars, BarLabelOptions::default())?;
        ax.yaxis().set_inverted(true);
        ax.set_xlabel("rainy days per quarter");
        ax.set_title("Rain by city");
        Ok(())
    })
}

/// A pie of six shares with labels and percentages, starting at the top.
fn pie() -> Build {
    Box::new(|fig| {
        let ax = fig.subplot();
        let options = PieOptions {
            labels: ["rent", "food", "transport", "savings", "leisure", "other"]
                .map(str::to_owned)
                .to_vec(),
            autopct: Some("%1.1f%%".into()),
            startangle: 90.0,
            ..PieOptions::default()
        };
        ax.pie([31.0, 24.0, 17.0, 12.0, 9.0, 7.0], options)?;
        ax.set_title("Where the money goes");
        Ok(())
    })
}

/// A five-petalled rose of 500 points on polar axes.
fn polar_rose() -> Build {
    let theta = linspace(0.0, 2.0 * PI, 500);
    let r: Vec<f64> = theta.iter().map(|t| 1.0 + 0.5 * (5.0 * t).sin()).collect();
    Box::new(move |fig| {
        let ax = fig.polar_subplot();
        ax.plot(&theta, &r)?;
        ax.set_title("A rose");
        Ok(())
    })
}

/// Twenty measurements with error bars and caps.
fn errorbar_20() -> Build {
    let mut rng = SpecRng::new(41);
    let x: Vec<f64> = (0..20).map(f64::from).collect();
    let y: Vec<f64> = x
        .iter()
        .zip(rng.normal(20, 0.0, 1.0))
        .map(|(x, noise)| 0.5 * x + noise)
        .collect();
    let yerr = rng.uniform(20, 0.3, 1.5);
    Box::new(move |fig| {
        let ax = fig.subplot();
        let options = ErrorbarOptions {
            yerr: Some(ErrorSize::Symmetric(yerr.clone().into())),
            fmt: "o".parse::<ErrorbarFormat>()?,
            capsize: Some(3.0),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, &y, options)?;
        ax.set_xlabel("trial").set_ylabel("response");
        Ok(())
    })
}

/// The 50 values of a histogram drawn as stairs.
fn stairs_50() -> Build {
    let values = SpecRng::new(42).uniform(50, 1.0, 10.0);
    Box::new(move |fig| {
        let ax = fig.subplot();
        ax.stairs(values.iter().copied(), StairsOptions::default())?;
        ax.set_title("Counts per bin");
        Ok(())
    })
}

/// 50 rows of 200 events each, one row per dataset.
fn eventplot() -> Build {
    let mut rng = SpecRng::new(43);
    let spikes: Vec<Vec<f64>> = (0..50).map(|_| rng.uniform(200, 0.0, 10.0)).collect();
    Box::new(move |fig| {
        let ax = fig.subplot();
        let options = EventplotOptions {
            linelengths: vec![0.8],
            ..EventplotOptions::default()
        };
        ax.eventplot(&spikes, options)?;
        ax.set_xlabel("time (s)").set_ylabel("neuron");
        Ok(())
    })
}

/// 10⁴ points scattered on polar axes, coloured by angle.
fn polar_scatter() -> Build {
    let mut rng = SpecRng::new(47);
    let theta = rng.uniform(10_000, 0.0, 2.0 * PI);
    let r: Vec<f64> = rng
        .uniform(10_000, 0.0, 1.0)
        .into_iter()
        .map(f64::sqrt)
        .collect();
    Box::new(move |fig| {
        let options = ScatterOptions {
            sizes: Some(vec![4.0]),
            colors: MappedColors::new(theta.clone()).into(),
            ..ScatterOptions::default()
        };
        fig.polar_subplot().scatter_with(&theta, &r, options)?;
        Ok(())
    })
}

/// 10⁴ points of a random walk with error bars in both directions.
fn errorbar_1e4() -> Build {
    let (x, y) = random_walk(10_000, 44);
    let mut rng = SpecRng::new(45);
    let xerr = rng.uniform(10_000, 0.1, 0.5);
    let yerr = rng.uniform(10_000, 0.5, 2.0);
    Box::new(move |fig| {
        let options = ErrorbarOptions {
            xerr: Some(ErrorSize::Symmetric(xerr.clone().into())),
            yerr: Some(ErrorSize::Symmetric(yerr.clone().into())),
            elinewidth: Some(0.5),
            ..ErrorbarOptions::default()
        };
        fig.subplot().errorbar(&x, &y, options)?;
        Ok(())
    })
}

/// 10⁵ steps of a random walk drawn as filled stairs.
fn stairs_1e5() -> Build {
    let (_, walk) = random_walk(100_000, 46);
    let lowest = walk.iter().copied().fold(f64::INFINITY, f64::min);
    let levels: Vec<f64> = walk.iter().map(|level| level - lowest).collect();
    Box::new(move |fig| {
        let options = StairsOptions {
            fill: true,
            ..StairsOptions::default()
        };
        fig.subplot().stairs(levels.iter().copied(), options)?;
        Ok(())
    })
}
