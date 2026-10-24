//! Probe operations for the `mpl_tightlayout` area: tight layout, ported
//! from matplotlib's `tests/test_tightlayout.py` (see
//! `spec/reference/probes/test_mpl_tightlayout.py`, which documents every
//! op).

use plt::artist::ArtistHandle;
use plt::figure::Figure;
use plt::layout::{LayoutEngine, TightLayout};
use plt::pyplot;
use plt::text::AnnotateOptions;

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::{numbers, object};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Figure.tight_layout", tight_layout),
    Op::new("Text.get_window_extent", suptitle),
    Op::new("Figure.get_tight_layout", get_tight_layout),
];

/// Whether tight layout declined a figure, and what it left.
fn tight_layout(args: Args<'_>) -> Outcome {
    in_test_style(args, || match args.str("scene")? {
        "empty" => empty_layout(),
        "badsubplotgrid" => bad_subplot_grid(),
        "collapsed" => collapsed(),
        other => Err(ProbeError::BadArgs(format!("unknown scene {other}"))),
    })
}

/// `test_empty_layout`: tight layout of a figure without axes.
fn empty_layout() -> Outcome {
    let fig = pyplot::gcf();
    let warned = fig.tight_layout(&TightLayout::default()).is_err();
    let params = fig.subplot_params();
    Ok(object([
        ("warned", Json::from(warned)),
        (
            "params",
            numbers([
                params.left,
                params.bottom,
                params.right,
                params.top,
                params.wspace,
                params.hspace,
            ]),
        ),
    ]))
}

/// `test_badsubplotgrid`: axes on grids of 4 and 5 rows, which tight
/// layout cannot combine.
fn bad_subplot_grid() -> Outcome {
    let fig = pyplot::figure();
    pyplot::subplot2grid((4, 5), (0, 0), 1, 1).map_err(ProbeError::rejected)?;
    pyplot::subplot2grid((5, 5), (0, 3), 5, 3).map_err(ProbeError::rejected)?;
    let warned = pyplot::tight_layout(&TightLayout::default()).is_err();
    let positions = fig
        .axes()
        .iter()
        .map(|ax| {
            let p = ax.position();
            numbers([p.x0(), p.y0(), p.x1(), p.y1()])
        })
        .collect();
    Ok(object([
        ("warned", Json::from(warned)),
        ("positions", Json::Array(positions)),
    ]))
}

/// `test_collapsed`: an annotation far right of its axes leaves the axes
/// no width under tight layout.
fn collapsed() -> Outcome {
    let fig = Figure::new();
    fig.set_layout_engine(LayoutEngine::tight())
        .map_err(ProbeError::rejected)?;
    let ax = fig.subplot();
    ax.set_xlim(0.0, 1.0)
        .and_then(|ax| ax.set_ylim(0.0, 1.0))
        .map_err(ProbeError::rejected)?;
    let options = AnnotateOptions {
        xytext: Some((10.5, 1.75).into()),
        annotation_clip: Some(false),
        ..AnnotateOptions::default()
    };
    ax.annotate("BIG LONG STRING", (1.25, 2.0), options)
        .map_err(ProbeError::rejected)?;
    let width = || ax.position().width();
    let mut widths = vec![width()];
    let mut warned = Vec::new();
    for rect in [None, Some([0.0, 0.0, 0.8, 0.8])] {
        let params = TightLayout {
            rect,
            ..TightLayout::default()
        };
        warned.push(Json::from(fig.tight_layout(&params).is_err()));
        widths.push(width());
    }
    Ok(object([
        ("warned", Json::Array(warned)),
        ("widths", numbers(widths)),
    ]))
}

/// `test_suptitle`: the bottom of the figure title and the top of the
/// axes title under the tight engine, after a draw.
fn suptitle(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.set_layout_engine(LayoutEngine::tight())
            .map_err(ProbeError::rejected)?;
        let ax = fig.subplot();
        let st = fig.suptitle("foo");
        let t = ax.set_title("bar");
        fig.layout();
        let extent = |text: &plt::text::Text| {
            text.window_extent()
                .ok_or_else(|| ProbeError::rejected("no extent"))
        };
        Ok(numbers([extent(&st)?.y0(), extent(&t)?.y1()]))
    })
}

/// Whether the figure is laid out by the tight engine after each
/// `set_tight_layout(value)`.
fn get_tight_layout(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.subplot();
        let mut out = Vec::new();
        for value in args.json("calls")?.as_array().unwrap_or(&[]) {
            let on = value
                .as_bool()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad value {value}")))?;
            fig.set_layout_engine(on.then(LayoutEngine::tight))
                .map_err(ProbeError::rejected)?;
            out.push(Json::from(matches!(
                fig.layout_engine(),
                Some(LayoutEngine::Tight(_))
            )));
        }
        Ok(Json::Array(out))
    })
}
