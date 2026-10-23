//! `axes_ticks`: manual major and minor ticks, the tick locators, tick
//! parameters and the style of a single tick.

use plt::axes::{TickDirection, TickLabelProps, TickLineProps, TickParams};
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::ticker::{
    AutoLocator, AutoMinorLocator, FixedLocator, IndexLocator, LinearLocator, LogLocator,
    MaxNLocator, MultipleLocator, Nbins, NullLocator, NumTicks, pyfmt,
};

use super::SEED;
use crate::figures::cases::common::{map, named_tick_strip};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Two constrained axes plotting `x = y` over `[0, 100)`, the top one
/// titled "Automatic ticks", the bottom one "Manual ticks"; returns the
/// bottom one.
fn automatic_and_manual(fig: &Figure) -> Result<Axes, Box<dyn std::error::Error>> {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(2, 1)?;
    let x = arange(0.0, 100.0, 1.0);
    for (row, title) in ["Automatic ticks", "Manual ticks"].into_iter().enumerate() {
        let ax = &axs[(row, 0)];
        ax.plot(&x, &x)?;
        ax.set_title(title);
    }
    Ok(axs[(1, 0)].clone())
}

fn axes_ticks_1(fig: &Figure) -> CaseResult {
    let ax = automatic_and_manual(fig)?;
    ax.set_yticks(arange(0.0, 100.1, 100.0 / 3.0));
    let xticks = arange(0.50, 101.0, 20.0);
    let xlabels: Vec<String> = xticks
        .iter()
        .map(|&x| format!("\\${}", pyfmt::fixed(x, 2)))
        .collect();
    ax.set_xticks(xticks).set_xticklabels(xlabels)?;
    Ok(())
}

fn axes_ticks_2(fig: &Figure) -> CaseResult {
    let ax = automatic_and_manual(fig)?;
    ax.set_yticks(arange(0.0, 100.1, 100.0 / 3.0));
    ax.yaxis().set_minor_ticks(arange(0.0, 100.1, 100.0 / 30.0));
    Ok(())
}

fn axes_ticks_3(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(8, 1)?;
    let ax = |i: usize| &axs[(i, 0)];
    named_tick_strip(ax(0), "NullLocator()")?;
    ax(0)
        .xaxis()
        .set_major_locator(NullLocator)
        .set_minor_locator(NullLocator);
    named_tick_strip(ax(1), "MultipleLocator(0.5)")?;
    ax(1)
        .xaxis()
        .set_major_locator(MultipleLocator::new(0.5)?)
        .set_minor_locator(MultipleLocator::new(0.1)?);
    named_tick_strip(ax(2), "FixedLocator([0, 1, 5])")?;
    ax(2)
        .xaxis()
        .set_major_locator(FixedLocator::new(vec![0.0, 1.0, 5.0]))
        .set_minor_locator(FixedLocator::new(linspace(0.2, 0.8, 4)));
    named_tick_strip(ax(3), "LinearLocator(numticks=3)")?;
    ax(3)
        .xaxis()
        .set_major_locator(LinearLocator::new(Some(3)))
        .set_minor_locator(LinearLocator::new(Some(31)));
    named_tick_strip(ax(4), "IndexLocator(base=0.5, offset=0.25)")?;
    ax(4).plot_with(
        [0.0, 1.0, 2.0, 3.0, 4.0],
        [0.0; 5],
        LineProps::default().color("white"),
    )?;
    ax(4)
        .xaxis()
        .set_major_locator(IndexLocator::new(0.5, 0.25));
    named_tick_strip(ax(5), "AutoLocator()")?;
    ax(5)
        .xaxis()
        .set_major_locator(AutoLocator::new())
        .set_minor_locator(AutoMinorLocator::new(None));
    named_tick_strip(ax(6), "MaxNLocator(n=4)")?;
    ax(6)
        .xaxis()
        .set_major_locator(MaxNLocator::new(Nbins::Fixed(4)))
        .set_minor_locator(MaxNLocator::new(Nbins::Fixed(40)));
    named_tick_strip(ax(7), "LogLocator(base=10, numticks=15)")?;
    ax(7).set_xlim(1e3, 1e10);
    ax(7).set_xscale("log")?;
    ax(7)
        .xaxis()
        .set_major_locator(LogLocator::new(10.0)?.with_numticks(NumTicks::Fixed(15)));
    Ok(())
}

fn axes_ticks_5(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(1, 2)?;
    for col in 0..2 {
        axs[(0, col)].plot_y(arange(0.0, 100.0, 1.0));
    }
    let ax = &axs[(0, 1)];
    ax.grid(true);
    ax.tick_params(&TickParams {
        axis: Some(plt::ticker::AxisName::Y),
        right: Some(true),
        left: Some(false),
        color: Some("r".into()),
        length: Some(16.0),
        grid_color: Some("none".into()),
        ..TickParams::default()
    });
    ax.tick_params(&TickParams {
        axis: Some(plt::ticker::AxisName::X),
        color: Some("m".into()),
        length: Some(4.0),
        direction: Some(TickDirection::In),
        width: Some(4.0),
        labelcolor: Some("g".into()),
        grid_color: Some("b".into()),
        ..TickParams::default()
    });
    Ok(())
}

fn axes_ticks_6(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let x = map(&rng.uniform(500, 0.0, 1.0), |u| 1.1 * (-3.0 * u).exp());
    let y = rng.uniform(500, 0.0, 1.0);
    let ax = fig.subplot();
    ax.plot_fmt(x, y, "o")?;
    ax.grid(true);
    ax.set_xticks([0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2]);
    let red = || Some(Color::from("red"));
    let xaxis = ax.xaxis();
    xaxis.set_major_ticklabel_props(
        5,
        &TickLabelProps {
            color: red(),
            ..TickLabelProps::default()
        },
    );
    xaxis.set_major_tick_line_props(
        5,
        &TickLineProps {
            markeredgecolor: red(),
            grid_visible: Some(true),
            grid_linestyle: Some(LineStyle::Dotted),
            grid_linewidth: Some(1.5),
            grid_color: red(),
        },
    );
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_axes/axes_ticks_1", axes_ticks_1).sized(5.4, 5.4, 100.0),
    FigureCase::new("gallery_guides_axes/axes_ticks_2", axes_ticks_2).sized(5.4, 5.4, 100.0),
    FigureCase::new("gallery_guides_axes/axes_ticks_3", axes_ticks_3),
    FigureCase::new("gallery_guides_axes/axes_ticks_5", axes_ticks_5).sized(6.4, 3.2, 100.0),
    FigureCase::new("gallery_guides_axes/axes_ticks_6", axes_ticks_6),
];
