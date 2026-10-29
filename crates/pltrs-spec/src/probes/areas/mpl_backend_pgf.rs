//! Probe operations for the `mpl_backend_pgf` area: matplotlib's own
//! tests of the PGF backend that check the `.pgf` text itself (see
//! `spec/reference/probes/test_mpl_backend_pgf.py`).

use pltrs::artist::{ArtistHandle, Sketch};
use pltrs::figure::Figure;

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("RendererPgf.draw_text", |args| {
        in_test_style(args, || draw_text(args))
    }),
    Op::new("FigureCanvasPgf.print_pgf", |args| {
        in_test_style(args, preamble)
    }),
    Op::new("RendererPgf.draw_path", |args| in_test_style(args, sketch)),
];

fn pgf(fig: &Figure) -> Result<String, ProbeError> {
    fig.to_pgf().map_err(ProbeError::rejected)
}

fn lines(lines: impl IntoIterator<Item = impl Into<String>>) -> Json {
    Json::Array(lines.into_iter().map(|l| Json::String(l.into())).collect())
}

/// The `\pgftext` lines of a figure text at its centre.
fn draw_text(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    fig.text(0.5, 0.5, args.str("text")?);
    Ok(lines(
        pgf(&fig)?
            .lines()
            .filter(|line| line.starts_with(r"\pgftext")),
    ))
}

/// The preamble listed in the header comments of an empty figure.
fn preamble() -> Outcome {
    let text = pgf(&Figure::new())?;
    let listed = text
        .lines()
        .skip_while(|line| !line.contains("used the following preamble"))
        .skip(1)
        .take_while(|&line| line != "%%")
        .map(|line| line.strip_prefix("%%   ").unwrap_or(line));
    Ok(lines(listed))
}

/// The lines of the sketched line of `test_sketch_params`, from its move
/// to its stroke.
fn sketch() -> Outcome {
    let fig = Figure::with_size(3.0, 3.0, pltrs::rc::params().figure_dpi);
    let ax = fig.subplot();
    ax.set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new())
        .set_frame_on(false);
    let line = ax.plot_y(vec![0.0, 1.0]);
    line.set_sketch_params(Some(Sketch {
        scale: 5.0,
        length: 30.0,
        randomness: 42.0,
    }));
    let text = pgf(&fig)?;
    let all: Vec<&str> = text.lines().collect();
    let seed = all
        .iter()
        .position(|&line| line == r"\pgfmathsetseed{42}%")
        .ok_or_else(|| ProbeError::rejected("no sketch seed"))?;
    let block = all
        .get(seed.saturating_sub(5)..seed + 3)
        .ok_or_else(|| ProbeError::rejected("a short document"))?;
    Ok(lines(block.iter().copied()))
}
