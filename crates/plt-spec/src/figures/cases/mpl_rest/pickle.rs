//! The `fig_ref` side of `test_pickle.py`: a figure of nine kinds of
//! plots, two subfigures on top and aligned y labels.

use plt::axes::{ErrorbarOptions, ImshowOptions, Mesh, PcolorOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::contour::{ContourOptions, QuadField};
use plt::figure::SubfigureOptions;
use plt::legend::{LegendLoc, LegendOptions};
use plt::lines::LineProps;
use plt::numeric::{Grid, arange, linspace};
use plt::prelude::*;
use plt::quiver::{QuiverOptions, VectorField};
use plt::streamplot::{StreamField, StreamplotOptions};

use crate::figures::{CaseResult, FigureCase};

/// `_generate_complete_test_figure`: a line, hatched filled contours with
/// a colorbar, a mesh, an image, a polygon mesh, streamlines, arrows,
/// scatter and error bar plots with legends, then two subfigures of two
/// subplots each over them all.
fn complete_ref(fig: &Figure) -> CaseResult {
    fig.suptitle("Can you fit any more in a figure?");
    let x = arange(0.0, 8.0, 1.0);
    let y = arange(0.0, 10.0, 1.0);
    let data = Grid::from_vec(10, 8, linspace(0.0, 10.0, 80))?;
    let v = data.map(|value| (value * -0.6).sin());

    let ax = fig.add_subplot(3, 3, 1)?;
    ax.plot_y((0..10).map(f64::from).collect::<Vec<_>>());
    ax.set_ylabel("hello");

    let ax = fig.add_subplot(3, 3, 2)?;
    let hatched = ContourOptions {
        hatches: vec![Some("//".to_owned()), Some("ooo".to_owned())],
        ..ContourOptions::default()
    };
    let contours = ax.contourf(&QuadField::new(data.clone()), hatched)?;
    fig.colorbar(
        &contours,
        ColorbarOptions {
            axes: ColorbarAxes::Parent(ax),
            ..ColorbarOptions::default()
        },
    )?;

    let ax = fig.add_subplot(3, 3, 3)?;
    ax.pcolormesh(Mesh::new(data.clone()), PcolorOptions::default())?;

    let ax = fig.add_subplot(3, 3, 4)?;
    ax.imshow(data.clone(), ImshowOptions::default())?;
    ax.set_ylabel("hello\nworld!");

    let ax = fig.add_subplot(3, 3, 5)?;
    ax.pcolor(Mesh::new(data.clone()), PcolorOptions::default())?;

    let ax = fig.add_subplot(3, 3, 6)?;
    ax.set_xlim(0.0, 7.0)?.set_ylim(0.0, 9.0)?;
    let field = StreamField::new(x.clone(), y.clone(), data.clone(), v.clone());
    ax.streamplot(&field, StreamplotOptions::default())?;

    let ax = fig.add_subplot(3, 3, 7)?;
    ax.set_xlim(0.0, 7.0)?.set_ylim(0.0, 9.0)?;
    ax.quiver(
        VectorField::on_axes(&x, &y, &data, &v),
        QuiverOptions::default(),
    )?;

    let ax = fig.add_subplot(3, 3, 8)?;
    let squares: Vec<f64> = x.iter().map(|x| x * x).collect();
    ax.scatter(x.clone(), squares)?.set_label("$x^2$");
    ax.legend(LegendOptions {
        loc: Some(LegendLoc::from("upper left")),
        ..LegendOptions::default()
    })?;

    let ax = fig.add_subplot(3, 3, 9)?;
    let halves: Vec<f64> = x.iter().map(|x| x * -0.5).collect();
    ax.errorbar(
        &x,
        halves,
        ErrorbarOptions {
            xerr: Some(0.2.into()),
            yerr: Some(0.4.into()),
            line: LineProps::default().label("$-.5 x$"),
            ..ErrorbarOptions::default()
        },
    )?;
    ax.legend(LegendOptions::default())?.set_draggable(true);

    for subfig in fig
        .subfigures(2, 1, SubfigureOptions::default())?
        .as_slice()
    {
        subfig.subplots(1, 2)?;
    }
    fig.align_ylabels(None);
    Ok(())
}

/// The group's figure from `test_pickle.py`.
pub(super) const CASES: &[FigureCase] =
    &[FigureCase::new("mpl_rest/complete_ref", complete_ref).sized(10.0, 6.0, 100.0)];
