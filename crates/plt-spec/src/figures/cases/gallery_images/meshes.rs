//! Quadrilateral meshes: `pcolor_demo`, `pcolormesh_grids`,
//! `pcolormesh_levels` and `quadmesh_demo`.

use plt::axes::{
    Axes, ImshowOptions, Mesh, MeshCoords, PcolorFast, PcolorOptions, PcolorfastOptions, Shading,
};
use plt::collections::{CollectionProps, EdgeColors};
use plt::colorbar::{ColorbarOptions, Mappable};
use plt::colors::{BoundaryNorm, Extend, LogNorm, colormaps};
use plt::contour::{ContourOptions, Levels, QuadCoords, QuadField};
use plt::figure::{Figure, Share};
use plt::image::{ColorizerOptions, Extent, Interpolation, Origin, ScalarField};
use plt::layout::{LayoutEngine, TightLayout};
use plt::lines::LineProps;
use plt::numeric::{Grid, arange, linspace, meshgrid};
use plt::rc::Aspect;
use plt::ticker::{Locator, MaxNLocator, Nbins, TickContext};

use super::data::{
    SEED, extremes, func3, mesh_field, mgrid, mgrid_step, on_mesh, spike, uniform_grid,
};
use crate::figures::{CaseResult, FigureCase};

/// `a[:-1, :-1]`.
fn trimmed(a: &Grid<f64>) -> Grid<f64> {
    Grid::from_fn(a.rows() - 1, a.cols() - 1, |r, c| a[(r, c)])
}

/// 6 × 10 uniform samples (`_random_6x10`).
fn random_6x10() -> Grid<f64> {
    uniform_grid(SEED, 6, 10)
}

fn pcolor_demo_1(fig: &Figure) -> CaseResult {
    let z = random_6x10();
    let axs = fig.subplots(2, 1)?;
    axs[(0, 0)].pcolor(Mesh::new(z.clone()), PcolorOptions::default())?;
    axs[(0, 0)].set_title("default: no edges");
    let edged = PcolorOptions {
        props: CollectionProps {
            edgecolors: Some(EdgeColors::Colors(vec!["k".into()])),
            linewidths: Some(vec![4.0]),
            ..CollectionProps::default()
        },
        ..PcolorOptions::default()
    };
    axs[(1, 0)].pcolor(Mesh::new(z), edged)?;
    axs[(1, 0)].set_title("thick edges");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn pcolor_demo_2(fig: &Figure) -> CaseResult {
    let (dx, dy) = (0.15, 0.05);
    let (y, x) = mgrid(
        &mgrid_step(-3.0, 3.0 + dy, dy),
        &mgrid_step(-3.0, 3.0 + dx, dx),
    );
    let z = trimmed(&on_mesh(&x, &y, func3));
    let top = extremes(&z).1.max(-extremes(&z).0);
    let colors = || -> Result<ColorizerOptions, plt::colors::ColormapError> {
        Ok(ColorizerOptions::default()
            .cmap(colormaps::get("RdBu")?)
            .limits(Some(-top), Some(top)))
    };
    let mesh = || Mesh::with_coords(x.clone(), y.clone(), z.clone());
    let axs = fig.subplots(2, 2)?;

    let options = PcolorOptions {
        colors: colors()?,
        ..PcolorOptions::default()
    };
    let c = axs[(0, 0)].pcolor(mesh(), options)?;
    axs[(0, 0)].set_title("pcolor");
    fig.colorbar(&c, ColorbarOptions::beside(&axs[(0, 0)]))?;

    let options = PcolorOptions {
        colors: colors()?,
        ..PcolorOptions::default()
    };
    let c = axs[(0, 1)].pcolormesh(mesh(), options)?;
    axs[(0, 1)].set_title("pcolormesh");
    fig.colorbar(&c, ColorbarOptions::beside(&axs[(0, 1)]))?;

    let (x_lo, x_hi) = extremes(&x);
    let (y_lo, y_hi) = extremes(&y);
    let image = ImshowOptions {
        colors: colors()?,
        extent: Some(Extent::new(x_lo, x_hi, y_lo, y_hi)),
        interpolation: Some(Interpolation::Nearest),
        origin: Some(Origin::Lower),
        aspect: Some(Aspect::Auto),
        ..ImshowOptions::default()
    };
    let c = axs[(1, 0)].imshow(z.clone(), image)?;
    axs[(1, 0)].set_title("image (nearest, aspect=\"auto\")");
    fig.colorbar(&c, ColorbarOptions::beside(&axs[(1, 0)]))?;

    let options = PcolorfastOptions {
        colors: colors()?,
        ..PcolorfastOptions::default()
    };
    let coords = (MeshCoords::Grid(x.clone()), MeshCoords::Grid(y.clone()));
    let c = axs[(1, 1)].pcolorfast(Some(coords), ScalarField::new(z.clone()), options)?;
    axs[(1, 1)].set_title("pcolorfast");
    let PcolorFast::QuadMesh(quads) = &c else {
        return Err("pcolorfast draws a quadrilateral grid as a mesh".into());
    };
    fig.colorbar(
        Mappable::Collection(quads),
        ColorbarOptions::beside(&axs[(1, 1)]),
    )?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn pcolor_demo_3(fig: &Figure) -> CaseResult {
    let (x, y, z) = mesh_field(&linspace(-3.0, 3.0, 100), &linspace(-2.0, 2.0, 100), spike);
    let (lo, hi) = extremes(&z);
    let axs = fig.subplots(2, 1)?;
    for (ax, colors) in [
        (
            &axs[(0, 0)],
            ColorizerOptions::default().norm(LogNorm::new(lo, hi)),
        ),
        (&axs[(1, 0)], ColorizerOptions::default()),
    ] {
        let options = PcolorOptions {
            colors: colors.cmap(colormaps::get("PuBu_r")?),
            shading: Some(Shading::Auto),
            ..PcolorOptions::default()
        };
        let c = ax.pcolor(Mesh::with_coords(x.clone(), y.clone(), z.clone()), options)?;
        fig.colorbar(&c, ColorbarOptions::beside(ax))?;
    }
    Ok(())
}

/// `pcolormesh_grids`' values: 0 to 14 in 3 rows of 5.
fn grid_values() -> Grid<f64> {
    Grid::from_fn(3, 5, |r, c| (r * 5 + c) as f64)
}

/// `_grid_mesh(ax, x, y, Z, shading, title)`: the mesh coloured over the
/// full range of the values, its points marked in magenta.
fn grid_mesh(
    ax: &Axes,
    cols: usize,
    rows: usize,
    z: Grid<f64>,
    shading: Shading,
    title: &str,
) -> CaseResult {
    let x = arange(0.0, cols as f64, 1.0);
    let y = arange(0.0, rows as f64, 1.0);
    let options = PcolorOptions {
        colors: ColorizerOptions::default().limits(Some(0.0), Some(14.0)),
        shading: Some(shading),
        ..PcolorOptions::default()
    };
    ax.pcolormesh(Mesh::with_coords(x.clone(), y.clone(), z), options)?;
    let (xx, yy) = meshgrid(&x, &y);
    ax.plot_with(
        xx.as_slice(),
        yy.as_slice(),
        LineProps::fmt("o")?.color("m"),
    )?;
    ax.set_xlim(-0.7, 5.2);
    ax.set_ylim(-0.7, 3.2);
    ax.set_title(title);
    Ok(())
}

fn pcolormesh_grids_1(fig: &Figure) -> CaseResult {
    grid_mesh(
        &fig.subplot(),
        6,
        4,
        grid_values(),
        Shading::Flat,
        "shading='flat'",
    )
}

fn pcolormesh_grids_2(fig: &Figure) -> CaseResult {
    grid_mesh(
        &fig.subplot(),
        5,
        3,
        trimmed(&grid_values()),
        Shading::Flat,
        "shading='flat': X, Y, C same shape",
    )
}

fn pcolormesh_grids_3(fig: &Figure) -> CaseResult {
    grid_mesh(
        &fig.subplot(),
        5,
        3,
        grid_values(),
        Shading::Nearest,
        "shading='nearest'",
    )
}

fn pcolormesh_grids_4(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()));
    let axs = fig.subplots(2, 1)?;
    grid_mesh(
        &axs[(0, 0)],
        5,
        3,
        grid_values(),
        Shading::Auto,
        "shading='auto'; X, Y, Z: same shape (nearest)",
    )?;
    grid_mesh(
        &axs[(1, 0)],
        6,
        4,
        grid_values(),
        Shading::Auto,
        "shading='auto'; X, Y one larger than Z (flat)",
    )
}

fn pcolormesh_grids_5(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()));
    grid_mesh(
        &fig.subplot(),
        5,
        3,
        grid_values(),
        Shading::Gouraud,
        "shading='gouraud'; X, Y same shape as Z",
    )
}

fn pcolormesh_levels_1(fig: &Figure) -> CaseResult {
    let mesh = Mesh::with_coords(
        arange(-0.5, 10.0, 1.0),
        arange(4.5, 11.0, 1.0),
        random_6x10(),
    );
    fig.subplot().pcolormesh(mesh, PcolorOptions::default())?;
    Ok(())
}

fn pcolormesh_levels_2(fig: &Figure) -> CaseResult {
    let (x, y) = meshgrid(&arange(-0.5, 10.0, 1.0), &arange(4.5, 11.0, 1.0));
    let tilted_x = on_mesh(&x, &y, |x, y| x + 0.2 * y);
    let tilted_y = on_mesh(&tilted_x, &y, |x, y| y + 0.3 * x);
    fig.subplot().pcolormesh(
        Mesh::with_coords(tilted_x, tilted_y, random_6x10()),
        PcolorOptions::default(),
    )?;
    Ok(())
}

fn pcolormesh_levels_3(fig: &Figure) -> CaseResult {
    let z = random_6x10();
    let (lo, hi) = extremes(&z);
    let (x, y) = meshgrid(&arange(0.0, 10.0, 1.0), &arange(0.0, 6.0, 1.0));
    let axs = fig.subplots_shared(2, 1, Share::All, Share::All)?;
    for (ax, values, shading, title) in [
        (
            &axs[(0, 0)],
            z.clone(),
            Shading::Auto,
            "shading='auto' = 'nearest'",
        ),
        (&axs[(1, 0)], trimmed(&z), Shading::Flat, "shading='flat'"),
    ] {
        let options = PcolorOptions {
            colors: ColorizerOptions::default().limits(Some(lo), Some(hi)),
            shading: Some(shading),
            ..PcolorOptions::default()
        };
        ax.pcolormesh(Mesh::with_coords(x.clone(), y.clone(), values), options)?;
        ax.set_title(title);
    }
    Ok(())
}

fn pcolormesh_levels_4(fig: &Figure) -> CaseResult {
    let (dx, dy) = (0.05, 0.05);
    let (y, x) = mgrid(
        &mgrid_step(1.0, 5.0 + dy, dy),
        &mgrid_step(1.0, 5.0 + dx, dx),
    );
    let z = trimmed(&on_mesh(&x, &y, |x, y| {
        x.sin().powf(10.0) + (10.0 + y * x).cos() * x.cos()
    }));
    let (lo, hi) = extremes(&z);
    let levels = MaxNLocator::new(Nbins::Fixed(15)).tick_values(lo, hi, &TickContext::default())?;
    let cmap = colormaps::get("PiYG")?;
    let norm = BoundaryNorm::new(levels.clone(), cmap.n(), Extend::Neither)?.with_clip(true)?;
    let axs = fig.subplots(2, 1)?;
    let (ax0, ax1) = (&axs[(0, 0)], &axs[(1, 0)]);
    let options = PcolorOptions {
        colors: ColorizerOptions::default().cmap(cmap.clone()).norm(norm),
        ..PcolorOptions::default()
    };
    let im = ax0.pcolormesh(Mesh::with_coords(x.clone(), y.clone(), z.clone()), options)?;
    fig.colorbar(&im, ColorbarOptions::beside(ax0))?;
    ax0.set_title("pcolormesh with levels");
    let centres = |a: &Grid<f64>, half: f64| trimmed(a).map(|v| v + half);
    let field = QuadField::new(z).with_coords(QuadCoords::Mesh {
        x: centres(&x, dx / 2.0),
        y: centres(&y, dy / 2.0),
    });
    let options = ContourOptions {
        levels: Levels::Values(levels),
        cmap: Some(cmap),
        ..ContourOptions::default()
    };
    let cf = ax1.contourf(&field, options)?;
    fig.colorbar(&cf, ColorbarOptions::beside(ax1))?;
    ax1.set_title("contourf with levels");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn quadmesh_demo(fig: &Figure) -> CaseResult {
    let n = 12;
    let (x, y) = meshgrid(&linspace(-1.5, 1.5, n), &linspace(-1.5, 1.5, n * 2));
    let qx = on_mesh(&x, &y, |x, y| y.cos() - x.cos());
    let qz = on_mesh(&x, &y, |x, y| y.sin() + x.sin());
    let radius = on_mesh(&x, &y, |x, y| (x * x + y * y).sqrt() / 5.0);
    let (lo, hi) = extremes(&radius);
    let z = radius.map(|r| (r - lo) / (hi - lo));
    let qz_max = extremes(&qz).1;
    let mask = qz.map(|q| q.abs() < 0.5 * qz_max);
    let axs = fig.subplots(1, 3)?;
    let gouraud = |colors: ColorizerOptions| PcolorOptions {
        colors,
        shading: Some(Shading::Gouraud),
        ..PcolorOptions::default()
    };
    let mesh = |values: ScalarField| Mesh::with_coords(qx.clone(), qz.clone(), values);
    axs[(0, 0)].pcolormesh(
        mesh(ScalarField::new(z.clone())),
        gouraud(ColorizerOptions::default()),
    )?;
    axs[(0, 0)].set_title("Without masked values");
    let cmap = colormaps::get("viridis")?.with_bad("y")?;
    axs[(0, 1)].pcolormesh(
        mesh(ScalarField::masked(z.clone(), &mask)?),
        gouraud(ColorizerOptions::default().cmap(cmap)),
    )?;
    axs[(0, 1)].set_title("With masked values");
    axs[(0, 2)].pcolormesh(
        mesh(ScalarField::masked(z, &mask)?),
        gouraud(ColorizerOptions::default()),
    )?;
    axs[(0, 2)].set_title("With masked values");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_images/pcolor_demo_1", pcolor_demo_1),
    FigureCase::new("gallery_images/pcolor_demo_2", pcolor_demo_2),
    FigureCase::new("gallery_images/pcolor_demo_3", pcolor_demo_3),
    FigureCase::new("gallery_images/pcolormesh_grids_1", pcolormesh_grids_1),
    FigureCase::new("gallery_images/pcolormesh_grids_2", pcolormesh_grids_2),
    FigureCase::new("gallery_images/pcolormesh_grids_3", pcolormesh_grids_3),
    FigureCase::new("gallery_images/pcolormesh_grids_4", pcolormesh_grids_4),
    FigureCase::new("gallery_images/pcolormesh_grids_5", pcolormesh_grids_5),
    FigureCase::new("gallery_images/pcolormesh_levels_1", pcolormesh_levels_1),
    FigureCase::new("gallery_images/pcolormesh_levels_2", pcolormesh_levels_2),
    FigureCase::new("gallery_images/pcolormesh_levels_3", pcolormesh_levels_3),
    FigureCase::new("gallery_images/pcolormesh_levels_4", pcolormesh_levels_4),
    FigureCase::new("gallery_images/quadmesh_demo", quadmesh_demo),
];
