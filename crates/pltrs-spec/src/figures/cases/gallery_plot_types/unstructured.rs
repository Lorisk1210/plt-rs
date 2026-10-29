//! The plots of unstructured triangular grids: contour lines and bands,
//! pseudocolour and the triangulation itself, over random points.

use pltrs::axes::{PlotError, TriValues, TripcolorOptions};
use pltrs::prelude::*;
use pltrs::tri::{TriError, Triangulation};

use super::{extrema_levels, plain};
use crate::figures::cases::common::func3;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The data of every thumbnail of the section: 256 random points of
/// `[-3, 3]²`, their Delaunay triangulation and `func3` at each point.
struct Scattered {
    x: Vec<f64>,
    y: Vec<f64>,
    tri: Triangulation,
    z: Vec<f64>,
}

impl Scattered {
    fn new() -> Result<Self, TriError> {
        let mut rng = SpecRng::new(1);
        let x = rng.uniform(256, -3.0, 3.0);
        let y = rng.uniform(256, -3.0, 3.0);
        let tri = Triangulation::delaunay(&x, &y)?;
        let z = x.iter().zip(&y).map(|(&x, &y)| func3(x, y)).collect();
        Ok(Self { x, y, tri, z })
    }

    /// Marks the points in `color`.
    fn mark(&self, ax: &Axes, color: &str) -> CaseResult {
        ax.plot_with(
            &self.x,
            &self.y,
            LineProps::fmt("o")?.markersize(2.0).color(color),
        )?;
        Ok(())
    }
}

/// The view of every thumbnail of the section.
fn view(ax: &Axes) -> Result<(), PlotError> {
    ax.set_xlim(-3.0, 3.0)?.set_ylim(-3.0, 3.0)?;
    Ok(())
}

fn tricontour(fig: &Figure) -> CaseResult {
    let data = Scattered::new()?;
    let ax = fig.subplot();
    data.mark(&ax, "lightgrey")?;
    ax.tricontour(&data.tri, &data.z, extrema_levels(&data.z))?;
    view(&ax)?;
    Ok(())
}

fn tricontourf(fig: &Figure) -> CaseResult {
    let data = Scattered::new()?;
    let ax = fig.subplot();
    data.mark(&ax, "grey")?;
    ax.tricontourf(&data.tri, &data.z, extrema_levels(&data.z))?;
    view(&ax)?;
    Ok(())
}

fn tripcolor(fig: &Figure) -> CaseResult {
    let data = Scattered::new()?;
    let ax = fig.subplot();
    data.mark(&ax, "grey")?;
    ax.tripcolor(
        &data.tri,
        TriValues::Auto(data.z),
        TripcolorOptions::default(),
    )?;
    view(&ax)?;
    Ok(())
}

fn triplot(fig: &Figure) -> CaseResult {
    let data = Scattered::new()?;
    let ax = fig.subplot();
    ax.triplot(&data.tri, "", LineProps::default())?;
    view(&ax)?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    plain("gallery_plot_types/tricontour", tricontour),
    plain("gallery_plot_types/tricontourf", tricontourf),
    plain("gallery_plot_types/tripcolor", tripcolor),
    plain("gallery_plot_types/triplot", triplot),
];
