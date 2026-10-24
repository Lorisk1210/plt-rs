//! The gallery's "Tick locators" and "Tick formatters" strips.

use plt::axes::{TickLevel, TickParams, TickPosition};
use plt::layout::TightLayout;
use plt::prelude::*;
use plt::spines::SpineSide;
use plt::ticker::{
    FixedFormatter, FixedLocator, FormatStrFormatter, FuncFormatter, MultipleLocator,
    NullFormatter, PercentFormatter, ScalarFormatter, StrMethodFormatter, pyfmt,
};

use crate::figures::CaseResult;
use crate::figures::cases::common::{locator_strips, tick_strip};

/// `_strip(ax, title)`: only the bottom spine and the x ticks are shown,
/// over `[0, 5]`, with `title` written above the axis.
fn strip(ax: &Axes, title: &str) -> CaseResult {
    tick_strip(ax);
    ax.xaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Bottom))?;
    ax.set_xlim(0.0, 5.0)?.set_ylim(0.0, 1.0)?;
    ax.set_patch_visible(false);
    ax.text(0.0, 0.2, title)
        .set_transform(ax.trans_axes())
        .set_fontsize(12.0)
        .set_color(Color::from("tab:blue"));
    Ok(())
}

pub(super) fn locators_gallery(fig: &Figure) -> CaseResult {
    locator_strips(fig, strip)
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
