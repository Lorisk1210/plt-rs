//! Cases from the second half of `test_axes.py`: inset indicators on
//! inverted axes (`test_indicate_inset_inverted`), offset texts following
//! their ticks (`test_move_offsetlabel`) and animated artists a draw
//! leaves out (`test_animated_artists_not_drawn_by_default`), in the test
//! suite's classic style.

use plt::artist::ArtistHandle;
use plt::axes::ImshowOptions;
use plt::figure::Figure;
use plt::numeric::Grid;
use plt::patches::PatchProps;
use plt::spines::SpineSide;

use super::super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Every case of this part of the group.
pub(super) fn cases() -> Vec<FigureCase> {
    vec![
        classic("mpl_tests_axes/indicate_inset_inverted_nox_noy", |fig| {
            indicate_inset_inverted(fig, false, false)
        }),
        classic("mpl_tests_axes/indicate_inset_inverted_nox_y", |fig| {
            indicate_inset_inverted(fig, false, true)
        }),
        classic("mpl_tests_axes/indicate_inset_inverted_x_noy", |fig| {
            indicate_inset_inverted(fig, true, false)
        }),
        classic("mpl_tests_axes/indicate_inset_inverted_x_y", |fig| {
            indicate_inset_inverted(fig, true, true)
        }),
        classic("mpl_tests_axes/move_offsetlabel_y", |fig| {
            move_offsetlabel(fig, SpineSide::Right)
        }),
        classic("mpl_tests_axes/move_offsetlabel_x", |fig| {
            move_offsetlabel(fig, SpineSide::Top)
        }),
        classic(
            "mpl_tests_axes/animated_artists_not_drawn",
            animated_artists_not_drawn,
        ),
    ]
}

/// An inset indicator from the first subplot to the second, the first
/// subplot's axes inverted as asked.
fn indicate_inset_inverted(fig: &Figure, x_inverted: bool, y_inverted: bool) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let x: Vec<f64> = (0..10).map(f64::from).collect();
    ax1.plot_fmt(x.clone(), x, "o")?;
    ax1.xaxis().set_inverted(x_inverted);
    ax1.yaxis().set_inverted(y_inverted);
    ax1.indicate_inset([2.0, 2.0, 5.0, 4.0], Some(ax2), PatchProps::default());
    Ok(())
}

/// Tiny values, whose offset text follows the ticks to `side`.
fn move_offsetlabel(fig: &Figure, side: SpineSide) -> CaseResult {
    let ax = fig.subplot();
    let data: Vec<f64> = SpecRng::new(7244)
        .uniform(10, 0.0, 1.0)
        .into_iter()
        .map(|v| v * 1e-22)
        .collect();
    let index: Vec<f64> = (0..data.len()).map(|i| i as f64).collect();
    ax.plot(index, data)?;
    ax.axis_of(side.axis()).tick_to(side)?;
    Ok(())
}

/// An animated image and an animated line, which drawing the figure
/// leaves out.
fn animated_artists_not_drawn(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let imdata = SpecRng::new(10416).uniform(400, 0.0, 1.0);
    let rows: Vec<Vec<f64>> = imdata.chunks(20).map(<[f64]>::to_vec).collect();
    let first_row = rows[0].clone();
    axs[(0, 0)]
        .imshow(Grid::from_rows(rows)?, ImshowOptions::default())?
        .set_animated(true);
    let index: Vec<f64> = (0..first_row.len()).map(|i| i as f64).collect();
    axs[(0, 1)].plot(index, first_row)?.set_animated(true);
    Ok(())
}
