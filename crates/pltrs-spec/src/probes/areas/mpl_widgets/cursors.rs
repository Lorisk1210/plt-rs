//! The `MultiCursor` and `Cursor.useblit` ops.

use pltrs::artist::ArtistHandle;
use pltrs::figure::{Figure, Share};
use pltrs::lines::Line2D;
use pltrs::widgets::{Cursor, CursorOptions, MultiCursor};

use super::steps::motion;
use super::{count, numbers};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode, malformed};

/// `{vx, hy, vvisible, hvisible}`: the x data of the vertical lines, the y
/// data of the horizontal ones and how many of each are visible.
fn lines(multi: &MultiCursor) -> Json {
    let data = |lines: &[Line2D], values: fn(&Line2D) -> Vec<f64>| {
        Json::Array(lines.iter().map(|line| numbers(values(line))).collect())
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
            useblit: args.bool("useblit")?,
            ..CursorOptions::multicursor()
        };
        let multi = MultiCursor::new(&[ax1.clone(), ax2], options);
        let lines_of = || {
            multi
                .vertical_lines()
                .iter()
                .chain(multi.horizontal_lines())
        };
        let counts = Json::Array(vec![
            count(multi.vertical_lines().len()),
            count(multi.horizontal_lines().len()),
            count(lines_of().filter(|line| line.animated()).count()),
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

/// `test_cursor_overlapping_axes_blitting_warning`: whether a cursor asked
/// to blit over the first of axes at the probe's `rects` does, and whether
/// its vertical line is animated.
pub(super) fn cursor_useblit(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let rects = args
            .json("rects")?
            .as_array()
            .ok_or_else(|| malformed("rects"))?;
        let mut axes = Vec::new();
        for rect in rects {
            let &[left, bottom, width, height] = rect.as_f64_vec().unwrap_or_default().as_slice()
            else {
                return Err(malformed("rects"));
            };
            axes.push(
                fig.add_axes([left, bottom, width, height])
                    .map_err(ProbeError::rejected)?,
            );
        }
        let ax = axes.first().ok_or_else(|| malformed("rects"))?;
        let options = CursorOptions {
            useblit: true,
            ..CursorOptions::default()
        };
        let cursor = Cursor::new(ax, options);
        Ok(Json::Array(vec![
            Json::Bool(cursor.useblit()),
            Json::Bool(cursor.vertical_line().animated()),
        ]))
    })
}
