//! Axes placed by locators, collections and images changed after they are
//! made, and date axes without dates.

use pltrs::axes::{
    AxesLocatorFn, FillBetweenOptions, ImshowOptions, ScatterColors, ScatterOptions,
};
use pltrs::collections::CollectionProps;
use pltrs::image::Interpolation;
use pltrs::markers::MarkerStyle;
use pltrs::numeric::Grid;
use pltrs::prelude::*;
use pltrs::rc::Aspect;
use pltrs::transforms::{Bbox, Point};

use crate::figures::CaseResult;

pub(super) fn axes_locator(fig: &Figure) -> CaseResult {
    let fixed = fig.add_subplot(1, 2, 1)?;
    fixed.plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.0])?;
    fixed.set_axes_locator(Some(AxesLocatorFn::new(|_| {
        Bbox::from_bounds(0.1, 0.5, 0.3, 0.4)
    })));
    let halved = fig.add_subplot(1, 2, 2)?;
    halved.plot([0.0, 1.0, 2.0], [2.0, 0.0, 1.0])?;
    halved.set_axes_locator(Some(AxesLocatorFn::new(|original: Bbox| {
        let (x0, y0, width, height) = original.bounds();
        Bbox::from_bounds(x0, y0, width, height / 2.0)
    })));
    halved.set_aspect(Aspect::Equal)?;
    Ok(())
}

pub(super) fn collection_setters(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 1.0, 6);
    let squares = ax.scatter_with(
        x.clone(),
        x.iter().map(|v| v * v).collect::<Vec<_>>(),
        ScatterOptions {
            sizes: Some(vec![400.0]),
            marker: Some(MarkerStyle::new("s".parse::<Marker>()?, FillStyle::Full)?),
            ..ScatterOptions::default()
        },
    )?;
    squares.set_hatch("//");
    squares.set_array(linspace(0.0, 10.0, 6).into_iter().map(Some).collect())?;
    squares.set_hatch_linewidth(3.0);
    let band = ax.fill_between(
        &x,
        [-0.5],
        x.iter().map(|v| v / 2.0 - 0.5).collect::<Vec<_>>(),
        FillBetweenOptions {
            props: CollectionProps::default().facecolor("none").edgecolor("k"),
            ..FillBetweenOptions::default()
        },
    )?;
    band.set_hatch("x");
    band.set_hatch_linewidth(0.3);
    let corner = ax.scatter_with(
        vec![0.0],
        vec![0.0],
        ScatterOptions {
            sizes: Some(vec![200.0]),
            colors: ScatterColors::Single("k".into()),
            ..ScatterOptions::default()
        },
    )?;
    corner.set_offset_transform(Some(ax.trans_axes()));
    corner.set_offsets(vec![Point::new(0.1, 0.9)]);
    Ok(())
}

pub(super) fn image_set_data(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let first = axs[(0, 0)].imshow(
        Grid::from_fn(3, 4, |r, c| (4 * r + c) as f64),
        ImshowOptions::default(),
    )?;
    first.set_data(Grid::from_fn(4, 2, |r, c| ((2 * r + c) as f64).powi(2)));
    let sinc = axs[(0, 1)].imshow(
        Grid::from_fn(4, 4, |r, c| ((4 * r + c) % 3) as f64),
        ImshowOptions {
            interpolation: Some(Interpolation::Sinc),
            ..ImshowOptions::default()
        },
    )?;
    sinc.set_filterrad(1.5)?.set_filternorm(false);
    Ok(())
}

pub(super) fn date_axes(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    let (dated, empty) = (&axs[(0, 0)], &axs[(1, 0)]);
    dated.plot([19_000.0, 19_003.0, 19_010.0], [0.0, 2.0, 1.0])?;
    dated.xaxis_date(None);
    empty.yaxis_date(None);
    Ok(())
}
