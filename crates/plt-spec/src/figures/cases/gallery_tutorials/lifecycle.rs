//! The lifecycle of a plot: companies' revenue as horizontal bars, first
//! plain, then in the `fivethirtyeight` style with slanted tick labels,
//! an automatic layout, labels and a title, on larger figures, with a
//! currency formatter, and finally with a line at the mean and notes.

use plt::axes::{Axes, PlotError, TickLabelProps, TickLevel};
use plt::gridspec::SubplotOverrides;
use plt::prelude::*;
use plt::text::{HAlign, VAlign};
use plt::ticker::FuncFormatter;

use crate::figures::{CaseResult, FigureCase};

/// The companies and their revenue.
const REVENUE: [(&str, f64); 10] = [
    ("Barton LLC", 109_438.50),
    ("Frami, Hills and Schmidt", 103_569.59),
    ("Fritsch, Russel and Anderson", 112_214.71),
    ("Jerde-Hilpert", 112_591.43),
    ("Keeling LLC", 100_934.30),
    ("Koepp Ltd", 103_660.54),
    ("Kulas Inc", 137_351.96),
    ("Trantow-Barrows", 123_381.38),
    ("White-Trantow", 135_841.99),
    ("Will LLC", 104_437.60),
];

/// The style the tutorial switches to after its second figure, and the
/// automatic layout it turns on after its fifth.
const STYLE: &str = "fivethirtyeight";
const AUTOLAYOUT: &[(&str, &str)] = &[("figure.autolayout", "True")];

/// The revenue as horizontal bars (`_revenue`).
fn revenue(fig: &Figure) -> Result<Axes, PlotError> {
    let ax = fig.subplot();
    let (names, values): (Vec<&str>, Vec<f64>) = REVENUE.into_iter().unzip();
    ax.barh(names, values, BarOptions::default())?;
    Ok(ax)
}

/// [`revenue`] with its x tick labels slanted (`_slanted_revenue`).
fn slanted_revenue(fig: &Figure) -> Result<Axes, PlotError> {
    let ax = revenue(fig)?;
    let slanted = TickLabelProps {
        rotation: Some(45.0),
        halign: Some(HAlign::Right),
        ..TickLabelProps::default()
    };
    ax.xaxis().set_ticklabel_props(TickLevel::Major, slanted);
    Ok(ax)
}

/// Limits, labels and a title.
fn titled(ax: &Axes) -> Result<(), PlotError> {
    ax.set_xlim(-10_000.0, 140_000.0)?;
    ax.set_xlabel("Total Revenue").set_ylabel("Company");
    ax.set_title("Company Revenue");
    Ok(())
}

/// Thousands and millions of dollars (`_currency`).
fn currency() -> FuncFormatter {
    FuncFormatter::new(|x, _| {
        if x >= 1e6 {
            format!("${:.1}M", x * 1e-6)
        } else {
            format!("${:.0}K", x * 1e-3)
        }
    })
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn lifecycle_1(fig: &Figure) -> CaseResult {
    fig.subplot();
    Ok(())
}

fn bars(fig: &Figure) -> CaseResult {
    revenue(fig)?;
    Ok(())
}

fn slanted_bars(fig: &Figure) -> CaseResult {
    slanted_revenue(fig)?;
    Ok(())
}

fn titled_bars(fig: &Figure) -> CaseResult {
    titled(&slanted_revenue(fig)?)?;
    Ok(())
}

fn lifecycle_9(fig: &Figure) -> CaseResult {
    let ax = slanted_revenue(fig)?;
    titled(&ax)?;
    ax.xaxis().set_major_formatter(currency());
    Ok(())
}

fn lifecycle_10(fig: &Figure) -> CaseResult {
    let ax = slanted_revenue(fig)?;
    let mean = REVENUE.iter().map(|(_, v)| v).sum::<f64>() / REVENUE.len() as f64;
    ax.axvline(
        mean,
        LineProps::default().linestyle(LineStyle::Dashed).color("r"),
    );
    for group in [3.0, 5.0, 8.0] {
        ax.text(145_000.0, group, "New Company")
            .set_fontsize(10.0)
            .set_valign(VAlign::Center);
    }
    // The example's `ax.title.set(y=1.05)` is undone by the `set(title=...)`
    // that follows it, which puts the title back at y = 1.
    titled(&ax)?;
    ax.xaxis().set_major_formatter(currency());
    ax.set_xticks([0.0, 25e3, 50e3, 75e3, 100e3, 125e3]);
    fig.subplots_adjust(&SubplotOverrides {
        right: Some(0.1),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_tutorials/lifecycle_1", lifecycle_1),
    FigureCase::new("gallery_tutorials/lifecycle_2", bars),
    FigureCase::new("gallery_tutorials/lifecycle_3", bars).with_style(STYLE),
    FigureCase::new("gallery_tutorials/lifecycle_4", bars).with_style(STYLE),
    FigureCase::new("gallery_tutorials/lifecycle_5", slanted_bars).with_style(STYLE),
    FigureCase::new("gallery_tutorials/lifecycle_6", slanted_bars)
        .with_style(STYLE)
        .with_rc(AUTOLAYOUT),
    FigureCase::new("gallery_tutorials/lifecycle_7", titled_bars)
        .with_style(STYLE)
        .with_rc(AUTOLAYOUT),
    FigureCase::new("gallery_tutorials/lifecycle_8", titled_bars)
        .sized(8.0, 4.0, 100.0)
        .with_style(STYLE)
        .with_rc(AUTOLAYOUT),
    FigureCase::new("gallery_tutorials/lifecycle_9", lifecycle_9)
        .sized(6.0, 8.0, 100.0)
        .with_style(STYLE)
        .with_rc(AUTOLAYOUT),
    FigureCase::new("gallery_tutorials/lifecycle_10", lifecycle_10)
        .sized(8.0, 8.0, 100.0)
        .with_style(STYLE)
        .with_rc(AUTOLAYOUT),
];
