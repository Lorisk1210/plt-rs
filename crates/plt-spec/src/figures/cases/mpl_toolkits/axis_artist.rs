//! The tests of `axisartist/tests/test_axis_artist.py`: tick marks drawn
//! on their own, and axis artists on the edges of a plain axes.

use plt::axes::{AxisName, TickDirection};
use plt::axisartist::{AxisArtist, TickMarks};
use plt::figure::Figure;
use plt::spines::SpineSide;
use plt_core::Point;

use crate::figures::{CaseResult, FigureCase};

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_toolkits/axis_artist_ticks", axis_artist_ticks),
    FigureCase::new("mpl_toolkits/axis_artist", axis_artist),
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn axis_artist_ticks(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.xaxis().set_visible(false);
    ax.yaxis().set_visible(false);
    let locs_angles: Vec<(Point, f64)> = (-1..12)
        .map(|i| (Point::new(f64::from(i) / 10.0, 0.0), f64::from(i) * 30.0))
        .collect();
    ax.add_tick_marks(TickMarks::new(10.0, AxisName::X, locs_angles.clone()));
    ax.add_tick_marks(TickMarks {
        direction: TickDirection::Out,
        color: Some("C3".into()),
        ..TickMarks::new(10.0, AxisName::X, locs_angles)
    });
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn axis_artist(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.xaxis().set_visible(false);
    ax.yaxis().set_visible(false);
    let [_, _, bottom] = [
        (SpineSide::Left, TickDirection::In),
        (SpineSide::Right, TickDirection::Out),
        (SpineSide::Bottom, TickDirection::InOut),
    ]
    .map(|(loc, direction)| {
        let axisline = AxisArtist::add_fixed(&ax, loc);
        axisline.major_ticks().set_tick_direction(direction);
        axisline
    });
    bottom.label().set_text("TTT").set_pad(5.0);
    ax.set_ylabel("Test");
    Ok(())
}
