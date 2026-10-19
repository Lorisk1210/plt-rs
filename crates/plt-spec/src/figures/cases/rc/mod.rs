//! `figures/cases_rc.py`: every visual rcParam set to a non-default value
//! on a small scene that the value visibly changes, one module per family
//! of parameters.

mod axes;
mod dates;
mod figure;
mod images;
mod legend;
mod lines;
mod paths;
mod projections;
mod stats;
mod text;
mod ticks;

use plt::figure::Figure;
use plt::gridspec::SubplotOverrides;
use plt::numeric::linspace;

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        axes::CASES,
        dates::CASES,
        figure::CASES,
        images::CASES,
        legend::CASES,
        lines::CASES,
        paths::CASES,
        paths::EFFECTS,
        projections::CASES,
        stats::CASES,
        text::CASES,
        ticks::CASES,
    ]
    .concat()
}

/// `SMALL`: the canvas most cases use, in inches.
const SMALL: (f64, f64) = (3.0, 2.0);
/// `DATED`: room for date tick labels.
const DATED: (f64, f64) = (5.0, 2.0);
/// `LABELLED`: room for a title, axis labels or a legend.
const LABELLED: (f64, f64) = (3.2, 2.4);
/// `SQUARE`: polar and 3-D axes.
const SQUARE: (f64, f64) = (2.6, 2.6);
/// `PANELS`: a 2 × 2 grid of labelled axes.
const PANELS: (f64, f64) = (4.0, 3.0);

/// `_cases(scene, figsize, entries)`: one `rc/<name>` case per entry, each
/// drawing `scene` at 100 dpi under its rcParams.
macro_rules! rc_cases {
    ($(
        $scene:expr, $size:expr => {
            $($name:literal: [$(($key:literal, $value:literal $(,)?)),* $(,)?]),* $(,)?
        }
    )*) => {
        &[$($(
            crate::figures::FigureCase::new(concat!("rc/", $name), $scene)
                .sized($size.0, $size.1, 100.0)
                .with_rc(&[$(($key, $value)),*]),
        )*)*]
    };
}
use rc_cases;

/// `T = np.linspace(0, 6, 25)`.
fn t() -> Vec<f64> {
    linspace(0.0, 6.0, 25)
}

/// `_curve`: one curve.
fn curve(fig: &Figure) -> CaseResult {
    let t = t();
    fig.subplot().plot(&t, map(&t, f64::sin))?;
    Ok(())
}

/// `_curves`: two curves drawn with the line defaults.
fn curves(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = t();
    ax.plot(&t, map(&t, f64::sin))?;
    ax.plot(&t, map(&t, |x| 0.5 * x.cos()))?;
    Ok(())
}

/// `_labelled`: a curve with a title and axis labels.
fn labelled(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = t();
    ax.plot(&t, map(&t, f64::sin))?;
    ax.set_title("Title");
    ax.set_xlabel("time").set_ylabel("value");
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.2),
        bottom: Some(0.22),
        top: Some(0.85),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}
