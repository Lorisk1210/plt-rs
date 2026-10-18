//! The gallery's "Tick locators" and "Tick formatters" strips.

use plt::axes::{TickLevel, TickParams, TickPosition};
use plt::layout::TightLayout;
use plt::prelude::*;
use plt::spines::SpineSide;
use plt::ticker::{
    AutoLocator, AutoMinorLocator, FixedFormatter, FixedLocator, FormatStrFormatter, FuncFormatter,
    IndexLocator, LinearLocator, LogLocator, MaxNLocator, MultipleLocator, Nbins, NullFormatter,
    NullLocator, NumTicks, PercentFormatter, ScalarFormatter, StrMethodFormatter, pyfmt,
};

use crate::figures::CaseResult;

/// `_strip(ax, title)`: only the bottom spine and the x ticks are shown,
/// over `[0, 5]`, with `title` written above the axis.
fn strip(ax: &Axes, title: &str) -> CaseResult {
    ax.yaxis().set_major_locator(NullLocator);
    let spines = ax.spines();
    for side in [SpineSide::Left, SpineSide::Right, SpineSide::Top] {
        spines.get(side).set_visible(false);
    }
    ax.xaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Bottom))?;
    for (which, width, length) in [(TickLevel::Major, 1.0, 5.0), (TickLevel::Minor, 0.75, 2.5)] {
        ax.tick_params(&TickParams {
            which,
            width: Some(width),
            length: Some(length),
            ..TickParams::default()
        });
    }
    ax.set_xlim(0.0, 5.0).set_ylim(0.0, 1.0);
    ax.set_patch_visible(false);
    ax.text(0.0, 0.2, title)
        .set_transform(ax.trans_axes())
        .set_fontsize(12.0)
        .set_color(Color::from("tab:blue"));
    Ok(())
}

pub(super) fn locators_gallery(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(8, 1)?;
    let ax = |i: usize| &axs[(i, 0)];
    strip(ax(0), "NullLocator()")?;
    ax(0)
        .xaxis()
        .set_major_locator(NullLocator)
        .set_minor_locator(NullLocator);
    strip(ax(1), "MultipleLocator(0.5, offset=0.2)")?;
    ax(1)
        .xaxis()
        .set_major_locator(MultipleLocator::new(0.5)?.with_offset(0.2))
        .set_minor_locator(MultipleLocator::new(0.1)?);
    strip(ax(2), "FixedLocator([0, 1, 5])")?;
    ax(2)
        .xaxis()
        .set_major_locator(FixedLocator::new(vec![0.0, 1.0, 5.0]))
        .set_minor_locator(FixedLocator::new(linspace(0.2, 0.8, 4)));
    strip(ax(3), "LinearLocator(numticks=3)")?;
    ax(3)
        .xaxis()
        .set_major_locator(LinearLocator::new(Some(3)))
        .set_minor_locator(LinearLocator::new(Some(31)));
    strip(ax(4), "IndexLocator(base=0.5, offset=0.25)")?;
    ax(4).plot_with(
        [0.0, 1.0, 2.0, 3.0, 4.0],
        [0.0; 5],
        LineProps::default().color("white"),
    )?;
    ax(4)
        .xaxis()
        .set_major_locator(IndexLocator::new(0.5, 0.25));
    strip(ax(5), "AutoLocator()")?;
    ax(5)
        .xaxis()
        .set_major_locator(AutoLocator::new())
        .set_minor_locator(AutoMinorLocator::new(None));
    strip(ax(6), "MaxNLocator(n=4)")?;
    ax(6)
        .xaxis()
        .set_major_locator(MaxNLocator::new(Nbins::Fixed(4)))
        .set_minor_locator(MaxNLocator::new(Nbins::Fixed(40)));
    strip(ax(7), "LogLocator(base=10, numticks=15)")?;
    ax(7).set_xlim(1e3, 1e10);
    ax(7).set_xscale("log")?;
    ax(7)
        .xaxis()
        .set_major_locator(LogLocator::new(10.0)?.with_numticks(NumTicks::Fixed(15)));
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

pub(super) fn formatters_gallery(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(9, 1)?;
    for ax in axs.as_slice() {
        strip(ax, "")?;
        ax.xaxis()
            .set_major_locator(MultipleLocator::new(1.0)?)
            .set_minor_locator(MultipleLocator::new(0.25)?);
        ax.tick_params(&TickParams {
            which: TickLevel::Minor,
            labelsize: Some(plt::fonts::FontSize::Points(10.0)),
            ..TickParams::default()
        });
    }
    fig.suptitle("Tick formatters");
    let axis = |i: usize| axs[(i, 0)].xaxis();
    axis(0).set_major_formatter(StrMethodFormatter::new("{x} km")?);
    axis(1).set_major_formatter(FuncFormatter::new(|x, _| pyfmt::repr(x - 5.0)));
    axis(2).set_major_formatter(NullFormatter);
    axis(3).set_major_formatter(StrMethodFormatter::new("{x:.3f}")?);
    axis(4).set_major_formatter(FuncFormatter::new(|x, _| {
        format!("[{}]", pyfmt::fixed(x, 2))
    }));
    axis(5)
        .set_major_locator(FixedLocator::new(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]))
        .set_major_formatter(FixedFormatter::new(
            ["A", "B", "C", "D", "E", "F"].map(String::from).to_vec(),
        ));
    axis(6).set_major_formatter(ScalarFormatter::new().with_use_mathtext(true));
    axis(7).set_major_formatter(FormatStrFormatter::new("#%d")?);
    axis(8).set_major_formatter(PercentFormatter::new(5.0));
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}
