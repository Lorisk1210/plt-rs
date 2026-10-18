//! The `xtick.*`, `ytick.*` and `grid.*` cases.

use plt::figure::Figure;
use plt::ticker::{FixedLocator, StrMethodFormatter};

use super::{SMALL, curve, map, rc_cases, t};
use crate::figures::{CaseResult, FigureCase};

/// `_minor_labels`: a curve with labelled minor ticks.
fn minor_labels(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = t();
    ax.plot(&t, map(&t, f64::sin))?;
    ax.xaxis()
        .set_minor_locator(FixedLocator::new(vec![1.5, 4.5]))
        .set_minor_formatter(StrMethodFormatter::new("{x:.1f}")?);
    ax.yaxis()
        .set_minor_locator(FixedLocator::new(vec![-0.25, 0.25]))
        .set_minor_formatter(StrMethodFormatter::new("{x:.2f}")?);
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    curve, SMALL => {
        "xtick_top_labeltop": [
            ("xtick.top", "True"),
            ("xtick.bottom", "False"),
            ("xtick.labeltop", "True"),
            ("xtick.labelbottom", "False"),
        ],
        "ytick_right_labelright": [
            ("ytick.left", "False"),
            ("ytick.right", "True"),
            ("ytick.labelleft", "False"),
            ("ytick.labelright", "True"),
        ],
        "xtick_sizes": [
            ("xtick.minor.visible", "True"),
            ("xtick.major.size", "8"),
            ("xtick.minor.size", "4"),
            ("xtick.major.width", "2"),
            ("xtick.minor.width", "1.5"),
        ],
        "ytick_sizes": [
            ("ytick.minor.visible", "True"),
            ("ytick.major.size", "8"),
            ("ytick.minor.size", "4"),
            ("ytick.major.width", "2"),
            ("ytick.minor.width", "1.5"),
        ],
        "tick_colors": [
            ("xtick.color", "tab:red"),
            ("xtick.labelcolor", "tab:blue"),
            ("ytick.color", "tab:green"),
            ("ytick.labelcolor", "tab:purple"),
        ],
        "tick_colors_inherited": [("xtick.color", "tab:red"), ("ytick.color", "tab:green")],
        "xtick_major_minor_sides": [
            ("xtick.top", "True"),
            ("xtick.minor.visible", "True"),
            ("xtick.major.top", "False"),
            ("xtick.minor.bottom", "False"),
        ],
        "xtick_major_minor_sides_swapped": [
            ("xtick.top", "True"),
            ("xtick.minor.visible", "True"),
            ("xtick.major.bottom", "False"),
            ("xtick.minor.top", "False"),
        ],
        "ytick_major_minor_sides": [
            ("ytick.right", "True"),
            ("ytick.minor.visible", "True"),
            ("ytick.major.right", "False"),
            ("ytick.minor.left", "False"),
        ],
        "ytick_major_minor_sides_swapped": [
            ("ytick.right", "True"),
            ("ytick.minor.visible", "True"),
            ("ytick.major.left", "False"),
            ("ytick.minor.right", "False"),
        ],
        "tick_minor_ndivs": [
            ("xtick.minor.visible", "True"),
            ("ytick.minor.visible", "True"),
            ("xtick.minor.ndivs", "2"),
            ("ytick.minor.ndivs", "3"),
        ],
        "tick_labelsize": [("xtick.labelsize", "small"), ("ytick.labelsize", "13")],
        "tick_direction": [("xtick.direction", "in"), ("ytick.direction", "inout")],
        "tick_alignment": [("xtick.alignment", "left"), ("ytick.alignment", "top")],
        "grid_style": [
            ("axes.grid", "True"),
            ("grid.color", "tab:orange"),
            ("grid.linestyle", "--"),
            ("grid.linewidth", "1.5"),
            ("grid.alpha", "0.5"),
        ],
        "grid_major": [
            ("axes.grid", "True"),
            ("axes.grid.which", "both"),
            ("xtick.minor.visible", "True"),
            ("ytick.minor.visible", "True"),
            ("grid.major.color", "tab:blue"),
            ("grid.major.linestyle", "-."),
            ("grid.major.linewidth", "2"),
            ("grid.major.alpha", "0.6"),
        ],
        "grid_minor": [
            ("axes.grid", "True"),
            ("axes.grid.which", "both"),
            ("xtick.minor.visible", "True"),
            ("ytick.minor.visible", "True"),
            ("grid.minor.color", "tab:red"),
            ("grid.minor.linestyle", ":"),
            ("grid.minor.linewidth", "1.2"),
            ("grid.minor.alpha", "0.7"),
        ],
    }
    minor_labels, SMALL => {
        "xtick_pads": [("xtick.major.pad", "9"), ("xtick.minor.pad", "1")],
        "ytick_pads": [("ytick.major.pad", "9"), ("ytick.minor.pad", "1")],
    }
};
