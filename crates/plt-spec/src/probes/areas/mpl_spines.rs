//! Probe operations for the `mpl_spines` area: matplotlib's own non-image
//! tests of spines (see `spec/reference/probes/test_mpl_spines.py`).

use plt::artist::ArtistHandle;
use plt::axes::TickPosition;
use plt::figure::Figure;
use plt::gridspec::SubplotOverrides;
use plt::numeric::linspace;
use plt::spines::{SpinePosition, SpineSide};
use plt_core::Bbox;

use super::mpl_tests::{in_test_style, numbers};
use crate::json::Json;
use crate::probes::encode::bbox;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Spine.set_bounds", set_bounds),
    Op::new("Axis.label", label_without_ticks),
];

/// The bounds of spines given a missing end, and the view limits after a
/// sine plot (`test_spine_set_bounds_with_none`).
fn set_bounds(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let x = linspace(0.0, 10.0, 100);
        let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
        ax.plot(x, y).map_err(ProbeError::rejected)?;
        let (xlim, ylim) = (ax.xlim(), ax.ylim());
        let spines = ax.spines();
        spines.bottom.set_bound_ends(Some(2.0), None);
        spines.left.set_bound_ends(None, None);
        let pair = |bounds: Option<(f64, f64)>| -> Result<Json, ProbeError> {
            let (low, high) = bounds.ok_or_else(|| ProbeError::rejected("no bounds"))?;
            Ok(numbers([low, high]))
        };
        Ok(Json::Object(vec![
            ("bottom".into(), pair(spines.bottom.bounds())?),
            ("left".into(), pair(spines.left.bounds())?),
            ("xlim".into(), numbers([xlim.0, xlim.1])),
            ("ylim".into(), numbers([ylim.0, ylim.1])),
        ]))
    })
}

/// The axis labels' extents and the outward spines' lines of an axes
/// without ticks (`test_label_without_ticks`).
fn label_without_ticks(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.subplots_adjust(&SubplotOverrides {
            left: Some(0.3),
            bottom: Some(0.3),
            ..SubplotOverrides::default()
        })
        .map_err(ProbeError::rejected)?;
        let ax = fig.subplot();
        ax.plot_y((0..10).map(f64::from).collect::<Vec<_>>());
        let spines = ax.spines();
        ax.yaxis()
            .set_ticks_position(TickPosition::Side(SpineSide::Left))
            .map_err(ProbeError::rejected)?;
        spines.left.set_position(SpinePosition::Outward(30.0));
        spines.right.set_visible(false);
        ax.set_ylabel("y label");
        ax.xaxis()
            .set_ticks_position(TickPosition::Side(SpineSide::Bottom))
            .map_err(ProbeError::rejected)?;
        spines.bottom.set_position(SpinePosition::Outward(30.0));
        spines.top.set_visible(false);
        ax.set_xlabel("x label");
        ax.set_xticks(Vec::<f64>::new())
            .set_yticks(Vec::<f64>::new());
        fig.canvas().draw().map_err(ProbeError::rejected)?;
        let extent = |b: Option<Bbox>| b.ok_or_else(|| ProbeError::rejected("not drawn"));
        Ok(Json::Object(vec![
            ("ylabel".into(), bbox(&extent(ax.yaxis().label_extent())?)),
            ("left".into(), bbox(&extent(spines.left.line_extent())?)),
            ("xlabel".into(), bbox(&extent(ax.xaxis().label_extent())?)),
            ("bottom".into(), bbox(&extent(spines.bottom.line_extent())?)),
        ]))
    })
}
