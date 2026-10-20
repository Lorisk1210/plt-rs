//! Locators and formatters on ordinary plots: automatic tick placement
//! with round-number limits, labels from functions and format strings,
//! engineering notation, major and minor ticks, the scalar formatter's
//! options, labels looked up from values, and colorbar tick labels
//! (`auto_ticks`, `custom_ticker1`, `dollar_ticks`, `engformatter_offset`,
//! `engineering_formatter`, `major_minor_demo`, `scalarformatter`,
//! `tick_labels_from_values` and `colorbar_tick_labelling_demo`).

use plt::axes::{
    AxisName, ImshowOptions, MappedColors, TickLabelProps, TickLevel, TickParams, TicklabelFormat,
};
use plt::colorbar::{ColorbarFormat, ColorbarOptions, ColorbarTicks};
use plt::colors::{Extend, colormaps};
use plt::figure::Share;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::image::ColorizerOptions;
use plt::layout::{LayoutEngine, TightLayout};
use plt::numeric::Grid;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::text::VerticalAlignment as VAlign;
use plt::ticker::{
    AutoMinorLocator, EngFormatter, FixedFormatter, FuncFormatter, MaxNLocator, MultipleLocator,
    Nbins, OffsetMode, StrMethodFormatter, pyfmt,
};

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The seed of the examples' random data.
const SEED: u64 = 19_680_801;

/// `auto_ticks`' scatter of a 10 × 10 grid over `[0.3, 1.2]`, coloured by
/// `x + y`.
fn auto_ticks_scatter(fig: &Figure) -> Result<Axes, Box<dyn std::error::Error>> {
    let ax = fig.subplot();
    let dots = linspace(0.3, 1.2, 10);
    let (x, y): (Vec<f64>, Vec<f64>) = dots
        .iter()
        .flat_map(|&y| dots.iter().map(move |&x| (x, y)))
        .unzip();
    let sums: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x + y).collect();
    ax.scatter_with(
        x,
        y,
        ScatterOptions {
            colors: MappedColors::new(sums).into(),
            ..ScatterOptions::default()
        },
    )?;
    Ok(ax)
}

fn auto_ticks(fig: &Figure) -> CaseResult {
    auto_ticks_scatter(fig)?;
    Ok(())
}

fn auto_ticks_margin(fig: &Figure) -> CaseResult {
    auto_ticks_scatter(fig)?.set_margin(AxisName::X, 0.8)?;
    Ok(())
}

fn custom_ticker1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.yaxis().set_major_formatter(FuncFormatter::new(|x, _| {
        format!("${}M", pyfmt::fixed(x * 1e-6, 1))
    }));
    let money = [1.5e5, 2.5e6, 5.5e6, 2.0e7];
    ax.bar(
        ["Bill", "Fred", "Mary", "Sue"],
        money,
        BarOptions::default(),
    )?;
    Ok(())
}

fn dollar_ticks(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_y(map(&SpecRng::new(SEED).uniform(20, 0.0, 1.0), |u| {
        100.0 * u
    }));
    ax.yaxis()
        .set_major_formatter(StrMethodFormatter::new("${x:1.2f}")?)
        .set_tick_params(&TickParams {
            which: TickLevel::Major,
            labelcolor: Some("green".into()),
            labelleft: Some(false),
            labelright: Some(true),
            ..TickParams::default()
        });
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn engformatter_offset(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.yaxis().set_major_formatter(
        EngFormatter::new()
            .with_use_offset(OffsetMode::Auto)
            .with_unit("Hz"),
    );
    let noise = SpecRng::new(SEED).uniform(100, -2e3, 2e3);
    ax.plot_y(map(&noise, |n| 1e9 + n));
    Ok(())
}

fn engineering_formatter(fig: &Figure) -> CaseResult {
    let xs = logspace(1.0, 9.0, 100, 10.0);
    let ys: Vec<f64> = SpecRng::new(SEED)
        .uniform(100, 0.0, 1.0)
        .iter()
        .zip(&xs)
        .map(|(u, x)| (0.8 + 0.4 * u) * x.log10().powi(2))
        .collect();
    let axs = fig.subplots(2, 1)?;
    let (ax0, ax1) = (&axs[(0, 0)], &axs[(1, 0)]);
    for ax in [ax0, ax1] {
        ax.set_xscale("log")?;
    }
    ax0.set_title("Full unit ticklabels, w/ default precision & space separator");
    ax0.xaxis()
        .set_major_formatter(EngFormatter::new().with_unit("Hz"));
    ax0.plot(xs.clone(), ys.clone())?;
    ax0.set_xlabel("Frequency");
    ax1.set_title("SI-prefix only ticklabels, 1-digit precision & thin space separator");
    ax1.xaxis().set_major_formatter(
        EngFormatter::new()
            .with_places(Some(1))
            .with_sep("\u{2009}"),
    );
    ax1.plot(xs, ys)?;
    ax1.set_xlabel("Frequency [Hz]");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// `major_minor_demo`'s damped sine `sin(frequency·π·t)·exp(-t/100)`
/// over `[0, 100)`, sampled every `step`.
fn damped_sine(
    fig: &Figure,
    step: f64,
    frequency: f64,
) -> Result<Axes, Box<dyn std::error::Error>> {
    let t = arange(0.0, 100.0, step);
    let s = map(&t, |t| {
        (frequency * std::f64::consts::PI * t).sin() * (-t * 0.01).exp()
    });
    let ax = fig.subplot();
    ax.plot(t, s)?;
    Ok(ax)
}

fn major_minor_demo_1(fig: &Figure) -> CaseResult {
    let ax = damped_sine(fig, 0.1, 0.1)?;
    ax.xaxis()
        .set_major_locator(MultipleLocator::new(20.0)?)
        .set_major_formatter(StrMethodFormatter::new("{x:.0f}")?)
        .set_minor_locator(MultipleLocator::new(5.0)?);
    Ok(())
}

fn major_minor_demo_2(fig: &Figure) -> CaseResult {
    let ax = damped_sine(fig, 0.01, 2.0)?;
    ax.xaxis().set_minor_locator(AutoMinorLocator::new(None));
    ax.tick_params(&TickParams {
        which: TickLevel::Both,
        width: Some(2.0),
        ..TickParams::default()
    });
    ax.tick_params(&TickParams {
        which: TickLevel::Major,
        length: Some(7.0),
        ..TickParams::default()
    });
    ax.tick_params(&TickParams {
        which: TickLevel::Minor,
        length: Some(4.0),
        color: Some("r".into()),
        ..TickParams::default()
    });
    Ok(())
}

fn scalarformatter(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 1.0, 0.01);
    fig.set_layout_engine(LayoutEngine::constrained());
    let gs = GridSpec::with_ratios(
        3,
        3,
        None,
        None,
        SubplotOverrides {
            hspace: Some(0.1),
            ..SubplotOverrides::default()
        },
    )?;
    let axs = fig.subplots_on(&gs, Share::None, Share::None)?;
    for col in 0..3 {
        axs[(0, col)].plot(map(&x, |x| x * 1e5 + 1e10), map(&x, |x| x * 1e-10 + 1e-5))?;
        axs[(1, col)].plot(map(&x, |x| x * 1e5), map(&x, |x| x * 1e-4))?;
        axs[(2, col)].plot(map(&x, |x| -x * 1e5 - 1e10), map(&x, |x| -x * 1e-5 - 1e-10))?;
    }
    for row in 0..3 {
        axs[(row, 1)].ticklabel_format(&TicklabelFormat {
            use_mathtext: Some(true),
            ..TicklabelFormat::default()
        })?;
        axs[(row, 2)].ticklabel_format(&TicklabelFormat {
            use_offset: Some(OffsetMode::Off),
            ..TicklabelFormat::default()
        })?;
    }
    plt::rc::set("axes.titleweight", "bold")?;
    plt::rc::set("axes.titley", "1.1")?;
    axs[(0, 0)].set_title("default settings");
    axs[(0, 1)].set_title("useMathText=True");
    axs[(0, 2)].set_title("useOffset=False");
    Ok(())
}

fn tick_labels_from_values(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let xs: Vec<f64> = (0..26).map(f64::from).collect();
    ax.xaxis()
        .set_major_formatter(FuncFormatter::new(|value, _| {
            // `int(tick_val)` truncates; letters exist for 0..=25 only.
            let index = value.trunc();
            if (0.0..26.0).contains(&index) {
                char::from(b'a' + index as u8).to_string()
            } else {
                String::new()
            }
        }))
        .set_major_locator(MaxNLocator::new(Nbins::Fixed(10)).with_integer(true));
    ax.plot(xs.clone(), xs)?;
    Ok(())
}

/// The 250 × 250 standard normal noise of `colorbar_tick_labelling_demo`.
fn noise() -> Grid<f64> {
    let values = SpecRng::new(SEED).normal(250 * 250, 0.0, 1.0);
    Grid::from_fn(250, 250, |row, col| values[row * 250 + col])
}

fn colorbar_tick_labelling_demo_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let image = ax.imshow(
        noise(),
        ImshowOptions::default().colors(ColorizerOptions {
            cmap: Some(colormaps::get("coolwarm")?),
            vmin: Some(-1.0),
            vmax: Some(1.0),
            ..ColorizerOptions::default()
        }),
    )?;
    ax.set_title("Gaussian noise with vertical colorbar");
    let cbar = fig.colorbar(
        &image,
        ColorbarOptions {
            ticks: Some(ColorbarTicks::Values(vec![-1.0, 0.0, 1.0])),
            format: Some(ColorbarFormat::Formatter(std::rc::Rc::new(
                FixedFormatter::new(["< -1", "0", "> 1"].map(String::from).to_vec()),
            ))),
            extend: Some(Extend::Both),
            ..ColorbarOptions::default()
        },
    )?;
    let aligned = |valign| TickLabelProps {
        valign: Some(valign),
        ..TickLabelProps::default()
    };
    let axis = cbar.ax().yaxis();
    axis.set_major_ticklabel_props(0, &aligned(VAlign::Top));
    axis.set_major_ticklabel_props(2, &aligned(VAlign::Bottom));
    Ok(())
}

fn colorbar_tick_labelling_demo_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let clipped = noise().map(|v| v.clamp(-1.0, 1.0));
    let image = ax.imshow(
        clipped,
        ImshowOptions::default().cmap(colormaps::get("afmhot")?),
    )?;
    ax.set_title("Gaussian noise with horizontal colorbar");
    let cbar = fig.colorbar(
        &image,
        ColorbarOptions {
            orientation: Some(Orientation::Horizontal),
            ..ColorbarOptions::default()
        },
    )?;
    cbar.set_ticks(vec![-1.0, 0.0, 1.0]);
    cbar.long_axis().set_ticklabels(["Low", "Medium", "High"])?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_ticks/auto_ticks_1", auto_ticks),
    FigureCase::new("gallery_ticks/auto_ticks_2", auto_ticks)
        .with_rc(&[("axes.autolimit_mode", "round_numbers")]),
    FigureCase::new("gallery_ticks/auto_ticks_3", auto_ticks_margin)
        .with_rc(&[("axes.autolimit_mode", "round_numbers")]),
    FigureCase::new("gallery_ticks/custom_ticker1", custom_ticker1),
    FigureCase::new("gallery_ticks/dollar_ticks", dollar_ticks),
    FigureCase::new("gallery_ticks/engformatter_offset", engformatter_offset),
    FigureCase::new("gallery_ticks/engineering_formatter", engineering_formatter)
        .sized(7.0, 9.6, 100.0),
    FigureCase::new("gallery_ticks/major_minor_demo_1", major_minor_demo_1),
    FigureCase::new("gallery_ticks/major_minor_demo_2", major_minor_demo_2),
    FigureCase::new("gallery_ticks/scalarformatter", scalarformatter).sized(9.0, 9.0, 100.0),
    FigureCase::new(
        "gallery_ticks/tick_labels_from_values",
        tick_labels_from_values,
    ),
    FigureCase::new(
        "gallery_ticks/colorbar_tick_labelling_demo_1",
        colorbar_tick_labelling_demo_1,
    ),
    FigureCase::new(
        "gallery_ticks/colorbar_tick_labelling_demo_2",
        colorbar_tick_labelling_demo_2,
    ),
];
