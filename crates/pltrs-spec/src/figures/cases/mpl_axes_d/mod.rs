//! `figures/cases_mpl_axes_d.py`: matplotlib's own image tests from the
//! last part of `test_axes.py`, split by topic: infinite and segment
//! lines, spectral plots, twins and tick parameters, pies, tick labels and
//! log axes, insets and axis options, and the rest (colours, clipping,
//! `ecdf`, orientation keywords and colorizers).

mod axlines;
mod insets;
mod misc;
mod pies;
mod spectral;
mod ticks;
mod twins;

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        axlines::CASES,
        &spectral::cases(),
        twins::CASES,
        pies::CASES,
        ticks::CASES,
        insets::CASES,
        misc::CASES,
    ]
    .concat()
}
