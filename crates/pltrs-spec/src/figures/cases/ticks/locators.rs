//! Linear locators: `MaxNLocator` options, minor locators and minor tick
//! parameters.

use pltrs::axes::{AxisName, TickDirection, TickLevel, TickParams};
use pltrs::fonts::FontSize;
use pltrs::layout::TightLayout;
use pltrs::prelude::*;
use pltrs::ticker::{
    AutoMinorLocator, FixedLocator, FormatStrFormatter, MaxNLocator, MinorDivisions,
    MultipleLocator, Nbins, Prune,
};

use crate::figures::CaseResult;

pub(super) fn maxn_integer_symmetric_prune(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    let [ax1, ax2, ax3] = [&axs[(0, 0)], &axs[(0, 1)], &axs[(0, 2)]];
    ax1.plot_fmt([0.2, 3.7], [0.1, 2.6], "o-")?;
    for axis in [ax1.xaxis(), ax1.yaxis()] {
        axis.set_major_locator(MaxNLocator::new(Nbins::Auto).with_integer(true));
    }
    ax2.plot([-2.0, 5.0], [-0.3, 1.2])?;
    ax2.xaxis()
        .set_major_locator(MaxNLocator::new(Nbins::Fixed(4)).with_symmetric(true));
    ax2.yaxis()
        .set_major_locator(MaxNLocator::new(Nbins::Fixed(5)).with_prune(Some(Prune::Both)));
    ax3.plot([0.0, 100.0], [0.0, 1e-3])?;
    ax3.xaxis().set_major_locator(
        MaxNLocator::new(Nbins::Fixed(3))
            .with_steps(&[1.0, 3.0, 10.0])?
            .with_prune(Some(Prune::Lower)),
    );
    ax3.yaxis().set_major_locator(
        MaxNLocator::new(Nbins::Fixed(6))
            .with_min_n_ticks(4)
            .with_prune(Some(Prune::Upper)),
    );
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

pub(super) fn minor_locators_and_params(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let x = linspace(0.0, 7.0, 50);
    let y: Vec<f64> = x.iter().map(|x| x.cos() * 3.0).collect();
    for ax in axs.as_slice() {
        ax.plot(&x, &y)?;
    }
    axs[(0, 0)]
        .xaxis()
        .set_minor_locator(AutoMinorLocator::new(Some(MinorDivisions::Count(4))));
    axs[(0, 0)]
        .yaxis()
        .set_minor_locator(AutoMinorLocator::new(None));
    axs[(0, 1)]
        .xaxis()
        .set_major_locator(MultipleLocator::new(2.0)?)
        .set_minor_locator(MultipleLocator::new(0.5)?);
    axs[(0, 1)].tick_params(TickParams {
        which: TickLevel::Minor,
        length: Some(6.0),
        color: Some(Color::from("tab:red")),
        width: Some(1.5),
        ..TickParams::default()
    });
    axs[(1, 0)].minorticks_on();
    axs[(1, 0)].tick_params(TickParams {
        which: TickLevel::Minor,
        direction: Some(TickDirection::In),
        top: Some(true),
        right: Some(true),
        length: Some(3.0),
        ..TickParams::default()
    });
    axs[(1, 0)].tick_params(TickParams {
        direction: Some(TickDirection::InOut),
        length: Some(8.0),
        ..TickParams::default()
    });
    let ax = &axs[(1, 1)];
    ax.xaxis()
        .set_major_locator(MultipleLocator::new(1.0)?)
        .set_minor_locator(MultipleLocator::new(0.5)?)
        .set_remove_overlapping_locs(false);
    ax.tick_params(TickParams {
        which: TickLevel::Minor,
        length: Some(10.0),
        color: Some(Color::from("tab:green")),
        ..TickParams::default()
    });
    ax.yaxis()
        .set_minor_locator(FixedLocator::new(vec![-2.5, -0.5, 0.5, 2.5]))
        .set_minor_formatter(FormatStrFormatter::new("%.1f")?);
    ax.tick_params(TickParams {
        axis: Some(AxisName::Y),
        which: TickLevel::Minor,
        labelsize: Some(FontSize::Points(7.0)),
        labelcolor: Some(Color::from("tab:green")),
        ..TickParams::default()
    });
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

pub(super) fn minor_ndivs_rc(fig: &Figure) -> CaseResult {
    fig.subplot().plot([0.0, 9.0], [0.0, 0.9])?;
    Ok(())
}
