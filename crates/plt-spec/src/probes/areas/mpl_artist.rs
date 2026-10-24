//! Probe operations for the `mpl_artist` area: matplotlib's own non-image
//! tests of `Artist` that the `artist_behaviour` area leaves out (see
//! `spec/reference/probes/test_mpl_artist.py`).

use plt::artist::ArtistHandle;
use plt::figure::{Figure, SubfigureOptions};

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("Artist.get_figure", get_figure)];

/// The only subfigure of a one-by-one grid of subfigures in `parent`.
fn subfigure(parent: &Figure) -> Result<Figure, ProbeError> {
    let grid = parent
        .subfigures(1, 1, SubfigureOptions::default())
        .map_err(ProbeError::rejected)?;
    Ok(grid.as_slice()[0].clone())
}

/// Whether the root and parent figures of the artists of nested
/// subfigures are the expected ones (`test_get_figure`).
fn get_figure(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let sfig1 = subfigure(&fig)?;
        let sfig2 = subfigure(&sfig1)?;
        let ax = sfig2.subplot();
        let line = ax
            .plot([0.0, 1.0], [0.0, 1.0])
            .map_err(ProbeError::rejected)?;
        let is = |found: Figure, expected: &Figure| Json::Bool(found.id() == expected.id());
        Ok(Json::Object(vec![
            ("figure_root".into(), is(fig.figure(), &fig)),
            ("figure_parent".into(), is(fig.parent_figure(), &fig)),
            ("axes_root".into(), is(ax.figure(), &fig)),
            ("axes_parent".into(), is(ax.parent_figure(), &sfig2)),
            ("line_root".into(), is(line.figure(), &fig)),
            ("line_parent".into(), is(line.parent_figure(), &sfig2)),
            ("sfig2_root".into(), is(sfig2.figure(), &fig)),
            ("sfig2_parent".into(), is(sfig2.parent_figure(), &sfig1)),
            ("sfig1_root".into(), is(sfig1.figure(), &fig)),
            ("sfig1_parent".into(), is(sfig1.parent_figure(), &fig)),
        ]))
    })
}
