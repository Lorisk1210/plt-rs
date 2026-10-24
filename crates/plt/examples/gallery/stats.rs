//! Statistics: histograms, box and violin plots, pies, empirical
//! distributions, asymmetric error bars and a bootstrapped band.

use plt::axes::{
    BoxplotOptions, EcdfOptions, ErrorSize, FillBetweenOptions, HistPlotOptions, PieOptions,
    ViolinDrawOptions, ViolinplotOptions,
};
use plt::collections::CollectionProps;
use plt::figure::Share;
use plt::legend::LegendOptions;
use plt::offsetbox::AnchorLoc;
use plt::prelude::*;
use plt::stats::{Bins, HistOptions, HistType};
use plt::ticker::LabelFormat;

use super::data::Samples;
use super::lines::map;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure(
        "histograms",
        "Overlapping histograms (step and filled)",
        histograms,
    ),
    Entry::figure(
        "box_and_violin",
        "Box plots next to violin plots",
        box_and_violin,
    ),
    Entry::figure("ecdf", "Empirical cumulative distributions (ecdf)", ecdf),
    Entry::figure(
        "asymmetric_errorbars",
        "Error bars with asymmetric errors",
        asymmetric_errorbars,
    ),
    Entry::figure(
        "bootstrap_band",
        "A bootstrapped confidence band around a fit",
        bootstrap_band,
    ),
    Entry::figure("pie", "Pie chart with percentages", pie),
];

/// Two normal samples, as filled and step histograms.
fn histograms() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let mut samples = Samples::new(7);
    let a = samples.normals(2000, 0.0, 1.0);
    let b = samples.normals(2000, 1.5, 0.7);
    let layout = HistOptions {
        bins: Some(Bins::Count(40)),
        density: true,
        ..HistOptions::default()
    };
    ax.hist(
        &[a],
        HistPlotOptions {
            layout: layout.clone(),
            props: PatchProps::default().alpha(0.6).label("N(0, 1)"),
            ..HistPlotOptions::default()
        },
    )?;
    ax.hist(
        &[b],
        HistPlotOptions {
            layout: HistOptions {
                histtype: HistType::Step,
                ..layout
            },
            props: PatchProps::default().linewidth(2.0).label("N(1.5, 0.7)"),
            ..HistPlotOptions::default()
        },
    )?;
    ax.set_title("Two samples");
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}

/// The same four samples as box plots and as violins, sharing the y
/// axis so that the two summaries line up.
fn box_and_violin() -> Built {
    let fig = Figure::with_size(8.0, 4.0, 100.0);
    let grid = fig.subplots_shared(1, 2, Share::None, Share::All)?;
    let mut samples = Samples::new(11);
    let data: Vec<Vec<f64>> = (0..4)
        .map(|i| samples.normals(200, f64::from(i), 0.5 + 0.3 * f64::from(i)))
        .collect();
    grid[(0, 0)].boxplot(&data, BoxplotOptions::default())?;
    grid[(0, 0)].set_title("boxplot");
    let violins = ViolinplotOptions {
        draw: ViolinDrawOptions {
            showmedians: true,
            ..ViolinDrawOptions::default()
        },
        ..ViolinplotOptions::default()
    };
    grid[(0, 1)].violinplot(&data, violins)?;
    grid[(0, 1)].set_xticks([1.0, 2.0, 3.0, 4.0]);
    grid[(0, 1)].set_title("violinplot");
    Ok(fig)
}

/// The fraction of a sample at or below each value, for three spreads,
/// and the complementary function of the widest one.
fn ecdf() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let mut samples = Samples::new(23);
    for sd in [0.5, 1.0, 2.0] {
        let options = EcdfOptions {
            line: LineProps::default().label(format!("σ = {sd}")),
            ..EcdfOptions::default()
        };
        ax.ecdf(&samples.normals(300, 0.0, sd), options)?;
    }
    let options = EcdfOptions {
        complementary: true,
        line: LineProps::fmt("--")?.color("0.5").label("1 − F, σ = 2"),
        ..EcdfOptions::default()
    };
    ax.ecdf(&samples.normals(300, 0.0, 2.0), options)?;
    ax.set_title("Empirical CDFs");
    ax.set_ylabel("fraction of the sample");
    ax.legend(LegendOptions::default().loc(AnchorLoc::CenterRight))?;
    Ok(fig)
}

/// Measurements whose uncertainty is larger above than below: each bar
/// takes a lower and an upper length.
fn asymmetric_errorbars() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let dose = linspace(1.0, 10.0, 10);
    let response = map(&dose, |d| 10.0 * (1.0 - (-d / 3.0).exp()));
    let options = ErrorbarOptions {
        yerr: Some(ErrorSize::Asymmetric {
            lower: map(&dose, |d| 0.3 + 0.03 * d).into(),
            upper: map(&dose, |d| 0.5 + 0.12 * d).into(),
        }),
        xerr: Some(vec![0.25].into()),
        capsize: Some(3.0),
        fmt: "o".parse()?,
        ..ErrorbarOptions::default()
    };
    ax.errorbar(&dose, &response, options)?;
    ax.set_title("Dose response (asymmetric errors)");
    ax.set_xlabel("dose (mg)").set_ylabel("response");
    Ok(fig)
}

/// The least-squares line through noisy points, and the band holding
/// 90 % of the lines fitted to 400 bootstrap resamples of the points.
fn bootstrap_band() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let mut samples = Samples::new(5);
    let x: Vec<f64> = (0..40).map(|_| 10.0 * samples.uniform()).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|x| 1.0 + 0.6 * x + samples.normal(0.0, 1.2))
        .collect();
    let grid = linspace(0.0, 10.0, 50);
    let fits: Vec<Vec<f64>> = (0..400)
        .map(|_| {
            let picks: Vec<usize> = (0..x.len())
                .map(|_| (samples.uniform() * 40.0) as usize)
                .collect();
            let (a, b) = fit(&picks.iter().map(|&i| (x[i], y[i])).collect::<Vec<_>>());
            map(&grid, |g| a + b * g)
        })
        .collect();
    let quantile = |q: f64| -> Vec<f64> {
        (0..grid.len())
            .map(|i| {
                let mut at: Vec<f64> = fits.iter().map(|fit| fit[i]).collect();
                at.sort_by(f64::total_cmp);
                at[(q * (at.len() - 1) as f64).round() as usize]
            })
            .collect()
    };
    let band = FillBetweenOptions {
        props: CollectionProps::default()
            .alpha(0.25)
            .label("90 % bootstrap band"),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(&grid, quantile(0.05), quantile(0.95), band)?;
    let (a, b) = fit(&x.iter().copied().zip(y.iter().copied()).collect::<Vec<_>>());
    ax.plot_with(
        &grid,
        map(&grid, |g| a + b * g),
        LineProps::default().label("least squares"),
    )?;
    ax.plot_with(
        &x,
        &y,
        LineProps::fmt("o")?
            .color("0.5")
            .markersize(4.0)
            .label("data"),
    )?;
    ax.set_title("Bootstrapped regression");
    ax.legend(LegendOptions::default().loc(AnchorLoc::LowerRight))?;
    Ok(fig)
}

/// The intercept and slope of the least-squares line through `points`.
fn fit(points: &[(f64, f64)]) -> (f64, f64) {
    let n = points.len() as f64;
    let (mx, my) = points
        .iter()
        .fold((0.0, 0.0), |(sx, sy), (x, y)| (sx + x / n, sy + y / n));
    let sxy: f64 = points.iter().map(|(x, y)| (x - mx) * (y - my)).sum();
    let sxx: f64 = points.iter().map(|(x, _)| (x - mx).powi(2)).sum();
    let slope = sxy / sxx;
    (my - slope * mx, slope)
}

/// Shares of a whole, labelled with their percentages.
fn pie() -> Built {
    let fig = Figure::with_size(5.0, 5.0, 100.0);
    let ax = fig.subplot();
    let options = PieOptions {
        labels: ["Frogs", "Hogs", "Dogs", "Logs"].map(String::from).to_vec(),
        autopct: Some(LabelFormat::Template("%1.1f%%".into())),
        explode: vec![0.0, 0.1, 0.0, 0.0],
        startangle: 90.0,
        ..PieOptions::default()
    };
    ax.pie(&[15.0, 30.0, 45.0, 10.0], options)?;
    ax.set_title("Pets");
    Ok(fig)
}
