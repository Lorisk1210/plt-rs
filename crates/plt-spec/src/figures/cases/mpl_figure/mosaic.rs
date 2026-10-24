//! `test_figure.py`, `TestSubplotMosaic`: `subplot_mosaic` from nested
//! lists and strings, with nested mosaics, empty cells, polar axes and
//! per-axes face colours.

use plt::axes::ProjectionKind;
use plt::figure::{Mosaic, MosaicAxes, MosaicCell, MosaicOptions};
use plt::layout::LayoutEngine;
use plt::prelude::*;

use super::{Fallible, classic};
use crate::figures::cases::common::mosaic as grid;
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_figure/subplot_mosaic_basic_str", |fig| {
        titled(fig, &grid(&[&["A", "A", "B"], &["C", "D", "B"]])?)
    }),
    classic("mpl_figure/subplot_mosaic_basic_int", |fig| {
        titled(fig, &grid(&[&["1", "1", "2"], &["3", "4", "2"]])?)
    }),
    classic("mpl_figure/subplot_mosaic_all_nested", |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        let x = grid(&[&["A", "B"], &["C", "D"]])?;
        let y = grid(&[&["E", "F"], &["G", "H"]])?;
        titled(
            fig,
            &Mosaic::new(vec![vec![MosaicCell::Nested(x), MosaicCell::Nested(y)]])?,
        )
    }),
    classic("mpl_figure/subplot_mosaic_nested", |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        titled(fig, &f_over(grid(&[&["A", "B"], &["C", "D"]])?)?)
    }),
    classic("mpl_figure/subplot_mosaic_nested_tuple", |fig| {
        plain(fig, &f_over(grid(&[&["A", "B", "B"], &["C", "C", "D"]])?)?)
    }),
    classic("mpl_figure/subplot_mosaic_empty_str", |fig| {
        titled(fig, &grid(&[&["A", "."], &[".", "B"]])?)
    }),
    classic("mpl_figure/subplot_mosaic_empty_int", |fig| {
        titled(fig, &grid(&[&["1", "."], &[".", "2"]])?)
    }),
    classic("mpl_figure/subplot_mosaic_subplot_kw", |fig| {
        plain(fig, &grid(&[&["1", "2"]])?)
    }),
    classic("mpl_figure/subplot_mosaic_subplot_kw_polar", |fig| {
        let options = MosaicOptions {
            projection: ProjectionKind::Polar.into(),
            ..MosaicOptions::default()
        };
        fig.subplot_mosaic(&grid(&[&["1", "2"]])?, options)?;
        Ok(())
    }),
    classic("mpl_figure/subplot_mosaic_per_subplot_kw", |fig| {
        let axes = fig.subplot_mosaic(&"AB;CD".parse::<Mosaic>()?, MosaicOptions::default())?;
        for (label, ax) in axes.iter() {
            let color = match label {
                "D" => "blue",
                "B" | "C" => "green",
                _ => "red",
            };
            ax.set_facecolor(color);
        }
        Ok(())
    }),
    classic("mpl_figure/subplot_mosaic_single_str_input_1", |fig| {
        plain(fig, &"AAA\nBBB".parse::<Mosaic>()?)
    }),
    classic("mpl_figure/subplot_mosaic_single_str_input_2", |fig| {
        plain(fig, &"ABC\nDEF".parse::<Mosaic>()?)
    }),
    // Two distinct `object()` keys.
    classic("mpl_figure/subplot_mosaic_hashable_keys", |fig| {
        plain(fig, &grid(&[&["A", "B"]])?)
    }),
];

/// `[["F"], [inner]]`: an axes above a nested mosaic.
fn f_over(inner: Mosaic) -> Fallible<Mosaic> {
    Ok(Mosaic::new(vec![
        vec![MosaicCell::Label("F".to_owned())],
        vec![MosaicCell::Nested(inner)],
    ])?)
}

/// `fig.subplot_mosaic(mosaic)`.
fn mosaic_axes(fig: &Figure, mosaic: &Mosaic) -> Fallible<MosaicAxes> {
    Ok(fig.subplot_mosaic(mosaic, MosaicOptions::default())?)
}

/// The axes of `mosaic`, untitled.
fn plain(fig: &Figure, mosaic: &Mosaic) -> CaseResult {
    mosaic_axes(fig, mosaic)?;
    Ok(())
}

/// The axes of `mosaic`, each titled with its label.
fn titled(fig: &Figure, mosaic: &Mosaic) -> CaseResult {
    for (label, ax) in mosaic_axes(fig, mosaic)?.iter() {
        ax.set_title(label);
    }
    Ok(())
}
