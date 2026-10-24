//! The mesh ops of the `mpl_collections` area: cursor data, vertex grids,
//! arrays and face colours of `pcolor` and `pcolormesh`.

use plt::artist::ArtistHandle;
use plt::axes::{Axes, Mesh, PcolorOptions, Shading};
use plt::collections::{Collection, CollectionProps, EdgeColors};
use plt::colors::{Color, ColorizerOptions, NormSpec, Normalize, colormaps};
use plt::figure::Figure;
use plt::image::ScalarField;
use plt::numeric::{Grid, meshgrid};
use plt_core::Point;

use super::{colors, made, numbers, unknown};
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};

/// `test_quadmesh_cursor_data`: the values under the cell `(0.5, 0.5)`
/// of `pcolormesh(x * x.T)`, without an array or with ones.
pub(super) fn cursor_data(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let values = Grid::from_fn(4, 4, |r, c| (r * c) as f64);
    let mesh = made(ax.pcolormesh(Mesh::new(values), PcolorOptions::default()))?;
    match args.str("array")? {
        "none" => {
            mesh.clear_array();
        }
        "ones" => {
            made(mesh.set_array(vec![Some(1.0); 16]))?;
        }
        other => return Err(unknown(other)),
    }
    let at = made(fig.resolve_transform(&ax.trans_data()))?.transform_point(Point::new(0.5, 0.5));
    Ok(match mesh.cursor_data(at) {
        None => Json::Null,
        Some(values) => Json::Array(values.into_iter().map(Json::from).collect()),
    })
}

/// `test_quadmesh_get_coordinates`: the vertex grid of a mesh on the
/// points of `[0, 1, 2]` x `[2, 4, 6]`.
pub(super) fn coordinates() -> Outcome {
    let (x, y) = meshgrid(&[0.0, 1.0, 2.0], &[2.0, 4.0, 6.0]);
    let mesh = made(Figure::new().subplot().pcolormesh(
        Mesh::from_coords(x, y, Grid::filled(2, 2, 1.0)),
        PcolorOptions::default(),
    ))?;
    let grid = mesh
        .mesh_coordinates()
        .ok_or_else(|| ProbeError::rejected("not a mesh"))?;
    Ok(Json::Array(
        grid.iter_rows()
            .map(|row| Json::Array(row.iter().map(|p| numbers([p.x, p.y])).collect()))
            .collect(),
    ))
}

/// `test_quadmesh_set_array` and the flat-array case of
/// `test_quadmesh_set_array_validation`.
pub(super) fn set_array(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let shading = args.str("shading")?;
    if shading == "wrong_size" {
        let coords: (Vec<f64>, Vec<f64>) = (
            (0..11).map(f64::from).collect(),
            (0..8).map(f64::from).collect(),
        );
        let mesh = made(ax.pcolormesh(
            Mesh::from_coords(coords.0, coords.1, Grid::filled(7, 10, 0.0)),
            PcolorOptions::default(),
        ))?;
        made(mesh.set_array((0..54).map(|i| Some(f64::from(i))).collect()))?;
        return Ok(Json::Null);
    }
    let (cells, shading) = match shading {
        "flat" => (3, None),
        "gouraud" => (4, Some(Shading::Gouraud)),
        other => return Err(unknown(other)),
    };
    let axis: Vec<f64> = (0..4).map(f64::from).collect();
    let options = PcolorOptions {
        shading,
        ..PcolorOptions::default()
    };
    let mesh = made(ax.pcolormesh(
        Mesh::from_coords(axis.clone(), axis, Grid::filled(cells, cells, 1.0)),
        options,
    ))?;
    let count = cells * cells;
    let mut reads = Vec::new();
    for values in [
        (0..count).map(|i| Some(i as f64)).collect::<Vec<_>>(),
        vec![Some(1.0); count],
    ] {
        made(mesh.set_array(values))?;
        made(fig.to_png())?;
        let array = mesh
            .array()
            .ok_or_else(|| ProbeError::rejected("no array"))?;
        reads.push(Json::Array(array.into_iter().map(Json::from).collect()));
    }
    Ok(Json::Array(reads))
}

/// `pcolor` or `pcolormesh` on the probe's axes.
fn pcolor(
    func: &str,
    fig: &Figure,
    mesh: Mesh,
    options: PcolorOptions,
) -> Result<Collection, ProbeError> {
    let ax = fig.subplot();
    made(match func {
        "pcolor" => ax.pcolor(mesh, options),
        "pcolormesh" => ax.pcolormesh(mesh, options),
        other => return Err(ProbeError::BadArgs(format!("unknown func {other:?}"))),
    })
}

fn faces(collection: &Collection) -> Result<Json, ProbeError> {
    Ok(colors(made(collection.colors())?.faces))
}

/// The face colours of `test_quadmesh_vmin_vmax`,
/// `test_quadmesh_alpha_array` and `test_alpha_validation`.
pub(super) fn facecolors(args: Args<'_>) -> Outcome {
    let func = args.str("func")?;
    let fig = Figure::new();
    match args.str("scene")? {
        "vmin_vmax" => {
            let options = PcolorOptions {
                colorizer: ColorizerOptions {
                    cmap: Some(made(colormaps::get("plasma"))?),
                    norm: Some(NormSpec::Norm(Box::new(Normalize::new(0.0, 1.0)))),
                    ..ColorizerOptions::default()
                },
                ..PcolorOptions::default()
            };
            let mesh = pcolor(func, &fig, Mesh::new(Grid::filled(1, 1, 1.0)), options)?;
            let before = faces(&mesh)?;
            mesh.set_clim(Some(1.0), Some(2.0));
            Ok(Json::Array(vec![before, faces(&mesh)?]))
        }
        "alpha_array" => {
            let axis: Vec<f64> = (0..4).map(f64::from).collect();
            let z = Grid::from_fn(3, 3, |r, c| (3 * r + c) as f64);
            let alphas: Vec<f64> = z.as_slice().iter().map(|v| v / 8.0).collect();
            let data = || Mesh::from_coords(axis.clone(), axis.clone(), z.clone());
            let mut given = PcolorOptions::default();
            given.props.alphas = Some(alphas.clone());
            let first = pcolor(func, &fig, data(), given)?;
            let second = pcolor(func, &fig, data(), PcolorOptions::default())?;
            second.set_alphas(alphas);
            Ok(Json::Array(vec![faces(&first)?, faces(&second)?]))
        }
        "alpha_validation" => {
            let z = Grid::from_fn(3, 4, |r, c| (4 * r + c) as f64);
            let mesh = pcolor(func, &fig, Mesh::new(z), PcolorOptions::default())?;
            mesh.set_alphas(vec![0.5, 0.6]);
            faces(&mesh)
        }
        other => Err(unknown(other)),
    }
}

/// `pcolor` or `pcolormesh` of `values` with `props` on `ax`.
fn mesh_on(
    ax: &Axes,
    func: &str,
    mesh: Mesh,
    props: CollectionProps,
) -> Result<Collection, ProbeError> {
    let options = PcolorOptions {
        props,
        ..PcolorOptions::default()
    };
    made(match func {
        "pcolor" => ax.pcolor(mesh, options),
        "pcolormesh" => ax.pcolormesh(mesh, options),
        other => return Err(ProbeError::BadArgs(format!("unknown func {other:?}"))),
    })
}

/// `{"faces", "edges"}` of a collection.
fn faces_and_edges(collection: &Collection) -> Result<Json, ProbeError> {
    let resolved = made(collection.colors())?;
    Ok(object([
        ("faces", colors(resolved.faces)),
        ("edges", colors(resolved.edges)),
    ]))
}

/// `test_color_logic`: the colours after each step.
pub(super) fn color_logic(args: Args<'_>) -> Outcome {
    let func = args.str("func")?;
    let ax = Figure::new().subplot();
    let z = Grid::from_fn(3, 4, |r, c| (4 * r + c) as f64);
    let make = |props| mesh_on(&ax, func, Mesh::new(z.clone()), props);
    let red_edges = || Some(EdgeColors::Colors(vec![Color::from((1.0, 0.0, 0.0))]));
    let mut out = Vec::new();
    let unfilled = CollectionProps {
        edgecolors: Some(EdgeColors::Colors(vec![Color::from("red")])),
        facecolors: Some(vec![Color::none()]),
        ..CollectionProps::default()
    };
    out.push(faces_and_edges(&make(unfilled)?)?);
    let pc = make(CollectionProps::default())?;
    pc.set_facecolor(Color::none()).set_edgecolor("red");
    out.push(faces_and_edges(&pc)?);
    pc.set_alpha(Some(0.5));
    out.push(faces_and_edges(&pc)?);
    pc.set_alpha(None);
    out.push(faces_and_edges(&pc)?);
    pc.set_edgecolors(EdgeColors::Default);
    out.push(faces_and_edges(&pc)?);
    pc.reset_facecolor();
    out.push(faces_and_edges(&pc)?);
    pc.clear_array();
    out.push(faces_and_edges(&pc)?);
    made(pc.set_array(z.as_slice().iter().copied().map(Some).collect()))?;
    out.push(faces_and_edges(&pc)?);
    let green = CollectionProps {
        edgecolors: red_edges(),
        facecolors: Some(vec![Color::from((0.0, 1.0, 0.0))]),
        ..CollectionProps::default()
    };
    out.push(faces_and_edges(&make(green)?)?);
    for white in [
        Color::from((1.0, 1.0, 1.0)),
        Color::from((1.0, 1.0, 1.0, 1.0)),
    ] {
        let rows = CollectionProps {
            edgecolors: red_edges(),
            facecolors: Some(vec![white; 12]),
            ..CollectionProps::default()
        };
        let pc = make(rows)?;
        out.push(faces_and_edges(&pc)?);
        pc.clear_array();
        out.push(faces_and_edges(&pc)?);
    }
    Ok(Json::Array(out))
}

/// `test_polyquadmesh_masked_vertices_array`: the cells `pcolor` keeps
/// where a corner or a value of the first cell is masked.
pub(super) fn masked_vertices(args: Args<'_>) -> Outcome {
    let (xx, yy) = meshgrid(&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0, 3.0]);
    let zz = Grid::from_fn(3, 2, |r, c| xx[(r, c)] * yy[(r, c)]);
    let ax = Figure::new().subplot();
    let quadmesh = mesh_on(
        &ax,
        "pcolormesh",
        Mesh::from_coords(xx.clone(), yy.clone(), zz.clone()),
        CollectionProps::default(),
    )?;
    let corner = |grid: &Grid<f64>| {
        let mut grid = grid.clone();
        grid[(0, 0)] = f64::NAN;
        grid
    };
    let mesh = match args.str("scene")? {
        "x" => Mesh::from_coords(corner(&xx), yy, zz),
        "y" => Mesh::from_coords(xx, corner(&yy), zz),
        "z" => {
            let mask = Grid::from_fn(3, 2, |r, c| (r, c) == (0, 0));
            Mesh::new(made(ScalarField::from(zz).with_mask(&mask))?)
        }
        other => return Err(unknown(other)),
    };
    let polymesh = mesh_on(&ax, "pcolor", mesh, CollectionProps::default())?;
    let mesh_faces = made(quadmesh.colors())?.faces;
    Ok(object([
        (
            "paths",
            Json::Integer(i64::try_from(polymesh.paths().len()).expect("few cells")),
        ),
        ("faces", colors(made(polymesh.colors())?.faces)),
        (
            "mesh_faces",
            colors(mesh_faces.into_iter().skip(1).collect()),
        ),
    ]))
}
