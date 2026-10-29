//! `figures/cases_gallery_misc.py`: the gallery's miscellaneous, specialty
//! plot, showcase, spine and pyplot sections, one case per figure. Spines
//! and plain plots, hand-built artists, diagrams, Sankey flows, path
//! effects, agg filters, images, computed pictures, polar axes, the two
//! projections written as user code and the showcase each have a
//! submodule.

mod artists;
mod basics;
mod custom_projection;
mod diagrams;
mod effects;
mod filters;
mod images;
mod pictures;
mod projections;
mod sankey;
mod showcase;
mod skewt;

use pltrs::axes::Axes;
use pltrs::figure::{Mosaic, MosaicAxes, MosaicCell, MosaicError};
use pltrs::gridspec::{GridSpec, GridSpecError, SubplotOverrides};

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        basics::CASES,
        artists::CASES,
        custom_projection::CASES,
        diagrams::CASES,
        effects::CASES,
        filters::CASES,
        images::CASES,
        pictures::CASES,
        projections::CASES,
        sankey::CASES,
        skewt::CASES,
        showcase::CASES,
    ]
    .concat()
}

/// A mosaic of labelled cells (`[['a', 'b'], ['c', 'd']]`).
fn named_mosaic(rows: &[&[&str]]) -> Result<Mosaic, MosaicError> {
    Mosaic::new(
        rows.iter()
            .map(|row| {
                row.iter()
                    .map(|&name| MosaicCell::Label(name.into()))
                    .collect()
            })
            .collect(),
    )
}

/// The axes of `axd` labelled `label`.
fn panel<'a>(axd: &'a MosaicAxes, label: &str) -> Result<&'a Axes, String> {
    axd.get(label)
        .ok_or_else(|| format!("no axes labelled {label:?}"))
}

/// An `nrows × ncols` grid with its own spacing
/// (`gridspec_kw={'wspace': wspace, 'hspace': hspace}`).
fn spaced_grid(
    nrows: usize,
    ncols: usize,
    wspace: f64,
    hspace: f64,
) -> Result<GridSpec, GridSpecError> {
    GridSpec::with_ratios(
        nrows,
        ncols,
        None,
        None,
        SubplotOverrides {
            wspace: Some(wspace),
            hspace: Some(hspace),
            ..SubplotOverrides::default()
        },
    )
}
