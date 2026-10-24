//! The `MultiCursor` op.

use plt::artist::ArtistHandle;
use plt::figure::{Figure, Share};
use plt::lines::Line2D;
use plt::widgets::{CursorOptions, MultiCursor};

use super::steps::motion;
use super::{count, numbers};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// `{vx, hy, vvisible, hvisible}`: the x data of the vertical lines, the y
/// data of the horizontal ones and how many of each are visible.
fn lines(multi: &MultiCursor) -> Json {
    let data = |lines: &[Line2D], values: fn(&Line2D) -> Vec<f64>| {
        Json::Array(lines.iter().map(|line| numbers(&values(line))).collect())
    };
    let visible = |lines: &[Line2D]| count(lines.iter().filter(|line| line.visible()).count());
    encode::object([
        ("vx", data(multi.vertical_lines(), Line2D::xdata)),
        ("hy", data(multi.horizontal_lines(), Line2D::ydata)),
        ("vvisible", visible(multi.vertical_lines())),
        ("hvisible", visible(multi.horizontal_lines())),
    ])
}

/// `test_MultiCursor`: a cursor over the first of two axes sharing x and
/// the axes of a second figure, moved, toggled, moved again and moved over
/// the axes it leaves out.
pub(super) fn multicursor(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let axs = fig
            .subplots_shared(2, 1, Share::All, Share::None)
            .map_err(ProbeError::rejected)?;
        let (ax1, ax3) = (axs[(0, 0)].clone(), axs[(1, 0)].clone());
        let ax2 = Figure::new().subplot();
        let options = CursorOptions {
            horizontal: args.bool("horizOn")?,
            vertical: args.bool("vertOn")?,
            ..CursorOptions::multicursor()
        };
        let multi = MultiCursor::new(&[ax1.clone(), ax2], options);
        let counts = numbers(&[
            multi.vertical_lines().len() as f64,
            multi.horizontal_lines().len() as f64,
        ]);
        motion(&ax1, (0.5, 0.25))?;
        fig.layout();
        let moved = lines(&multi);
        multi.set_horizontal(!multi.horizontal());
        multi.set_vertical(!multi.vertical());
        motion(&ax1, (0.5, 0.25))?;
        let toggled = lines(&multi);
        motion(&ax3, (0.75, 0.75))?;
        Ok(encode::object([
            ("counts", counts),
            ("moved", moved),
            ("toggled", toggled),
            ("elsewhere", lines(&multi)),
        ]))
    })
}
