//! Probe operations for the `mpl_contour` area: matplotlib's own non-image
//! tests of contour sets that the other contour areas leave out (see
//! `spec/reference/probes/test_mpl_contour.py`).

mod labels;

use plt::artist::{ArtistHandle, ClipPathSpec};
use plt::axes::Axes;
use plt::colors::{ColorizerOptions, LogNorm, NormSpec};
use plt::contour::{ClabelOptions, ContourOptions, ContourSet, Levels, QuadCoords, QuadField};
use plt::figure::Figure;
use plt::fonts::FontSize;
use plt::lines::LineStyle;
use plt::numeric::{Grid, arange, meshgrid};
use plt::rc;
use plt::tri::Triangulation;
use plt_core::{Affine2D, Path};

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::{numbers, object};
use crate::probes::{Args, Op, Outcome, ProbeError, made};
use crate::rng::SpecRng;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Axes.contour", |args| in_test_style(args, || contour(args))),
    Op::new("Axes.contourf", |args| {
        in_test_style(args, || contourf(args))
    }),
    Op::new("Axes.tricontour", |args| {
        in_test_style(args, || tri_bools(args, false))
    }),
    Op::new("Axes.tricontourf", |args| {
        in_test_style(args, || tri_bools(args, true))
    }),
    Op::new("ContourSet.clabel", |args| {
        in_test_style(args, || labels::clabel(args))
    }),
    Op::new("ContourSet.find_nearest_contour", |args| {
        in_test_style(args, || labels::find_nearest_contour(args))
    }),
    Op::new("ContourSet.remove", |args| {
        in_test_style(args, || remove(args))
    }),
    Op::new("ContourSet.get_clip_path", |args| {
        in_test_style(args, clip_path)
    }),
    Op::new("ContourSet.allsegs", |args| in_test_style(args, allsegs)),
    Op::new("ContourSet.negative_linestyles", |args| {
        in_test_style(args, || negative_linestyles(args))
    }),
];

fn unknown(scene: &str) -> ProbeError {
    ProbeError::BadArgs(format!("unknown scene {scene:?}"))
}

/// `{"levels": [...]}` of a contour set.
fn levels(cs: &ContourSet) -> Json {
    object([("levels", numbers(cs.levels()))])
}

/// `np.arange(n)` as floats.
fn indices(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64).collect()
}

/// The inputs of `test_contour_shape_error` a quadrilateral field holds.
fn shape_error_field(scene: &str) -> Option<QuadField> {
    let zeros = |rows, cols| Grid::filled(rows, cols, 0.0);
    let (z, coords) = match scene {
        "x_len" => (zeros(9, 10), axes_coords(indices(9), indices(9))),
        "y_len" => (zeros(9, 10), axes_coords(indices(10), indices(10))),
        "x2d_shape" => (
            zeros(9, 10),
            QuadCoords::Mesh {
                x: zeros(9, 9),
                y: zeros(9, 10),
            },
        ),
        "y2d_shape" => (
            zeros(9, 10),
            QuadCoords::Mesh {
                x: zeros(9, 10),
                y: zeros(9, 9),
            },
        ),
        "z_1x1" => return Some(QuadField::new(zeros(1, 1))),
        "xyz_1x1" => (zeros(1, 1), axes_coords(vec![0.0], vec![0.0])),
        _ => return None,
    };
    Some(QuadField::new(z).with_coords(coords))
}

fn axes_coords(x: Vec<f64>, y: Vec<f64>) -> QuadCoords {
    QuadCoords::Axes { x, y }
}

/// The checkerboard `arange(9) % 2` of `test_bool_autolevel`.
fn checkerboard() -> Vec<bool> {
    (0..9).map(|i| i % 2 == 1).collect()
}

/// The checkerboard as a 3 × 3 field, its centre masked for
/// `bool_masked`.
fn bool_field(scene: &str) -> Result<QuadField, ProbeError> {
    let field = QuadField::from_bools(&made(Grid::from_vec(3, 3, checkerboard()))?);
    match scene {
        "bool" => Ok(field),
        "bool_masked" => Ok(field.with_mask(Grid::from_fn(3, 3, |r, c| (r, c) == (1, 1)))),
        other => Err(unknown(other)),
    }
}

fn contour(args: Args<'_>) -> Outcome {
    let scene = args.str("scene")?;
    let ax = Figure::new().subplot();
    if let Some(field) = shape_error_field(scene) {
        return Ok(levels(&made(
            ax.contour(&field, ContourOptions::default()),
        )?));
    }
    let (field, options) = match scene {
        "valid_1d" | "valid_2d" => {
            let z = SpecRng::new(19_680_801).uniform(90, 0.0, 1.0);
            let (x, y) = (indices(10), indices(9));
            let coords = if scene == "valid_1d" {
                axes_coords(x, y)
            } else {
                let (x, y) = meshgrid(&x, &y);
                QuadCoords::Mesh { x, y }
            };
            let field = QuadField::new(made(Grid::from_vec(9, 10, z))?).with_coords(coords);
            (field, ContourOptions::default())
        }
        "empty_levels" => {
            let z = SpecRng::new(9).uniform(81, 0.0, 1.0);
            let options = ContourOptions {
                levels: Levels::Values(Vec::new()),
                ..ContourOptions::default()
            };
            (QuadField::new(made(Grid::from_vec(9, 9, z))?), options)
        }
        "uniform" => (
            QuadField::new(Grid::filled(9, 9, 1.0)),
            ContourOptions::default(),
        ),
        "bool" | "bool_masked" => (bool_field(scene)?, ContourOptions::default()),
        _ => return lognorm_levels(&ax, scene),
    };
    Ok(levels(&made(ax.contour(&field, options))?))
}

/// `test_lognorm_levels`: the levels of `|sin(x) e^y|` on
/// `np.mgrid[1:10:0.1, 1:10:0.1]` under a `LogNorm`, and how many lie
/// within the data.
fn lognorm_levels(ax: &Axes, scene: &str) -> Outcome {
    let count = scene
        .strip_prefix("lognorm:")
        .and_then(|n| n.parse::<i64>().ok())
        .ok_or_else(|| unknown(scene))?;
    let step = |i: usize| i as f64 * 0.1 + 1.0;
    let x = Grid::from_fn(90, 90, |r, _| step(r));
    let y = Grid::from_fn(90, 90, |_, c| step(c));
    let z = Grid::from_fn(90, 90, |r, c| (step(r).sin() * step(c).exp()).abs());
    let (lo, hi) = z
        .as_slice()
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    let options = ContourOptions {
        levels: Levels::Count(count),
        colorizer: ColorizerOptions {
            norm: Some(NormSpec::Norm(Box::new(LogNorm::default()))),
            ..ColorizerOptions::default()
        },
        ..ContourOptions::default()
    };
    let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x, y });
    let cs = made(ax.contour(&field, options))?;
    let all = cs.levels();
    let visible = all.iter().filter(|&&l| (lo..=hi).contains(&l)).count();
    Ok(object([
        ("levels", numbers(all)),
        (
            "visible",
            Json::Integer(i64::try_from(visible).expect("few levels")),
        ),
    ]))
}

fn contourf(args: Args<'_>) -> Outcome {
    let scene = args.str("scene")?;
    let ax = Figure::new().subplot();
    let field = match scene.split(':').collect::<Vec<_>>().as_slice() {
        ["bool" | "bool_masked"] => bool_field(scene)?,
        ["algorithm", name, corner @ ..] => {
            let mut field =
                QuadField::new(made(Grid::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]))?);
            field.algorithm = Some(made(name.parse())?);
            field.corner_mask = match corner {
                [] => None,
                [flag] => Some(*flag == "True"),
                _ => return Err(unknown(scene)),
            };
            field
        }
        _ => return Err(unknown(scene)),
    };
    Ok(levels(&made(
        ax.contourf(&field, ContourOptions::default()),
    )?))
}

/// The checkerboard flattened at nine random points, triangulated.
fn tri_bools(args: Args<'_>, filled: bool) -> Outcome {
    let scene = args.str("scene")?;
    if scene != "bool" {
        return Err(unknown(scene));
    }
    let mut stream = SpecRng::new(5);
    let (x, y) = (stream.uniform(9, 0.0, 1.0), stream.uniform(9, 0.0, 1.0));
    let tri = made(Triangulation::delaunay(&x, &y))?;
    let ax = Figure::new().subplot();
    let z = checkerboard();
    let cs = if filled {
        ax.tricontourf_bools(&tri, &z, ContourOptions::default())
    } else {
        ax.tricontour_bools(&tri, &z, ContourOptions::default())
    };
    Ok(levels(&made(cs)?))
}

/// What a probe can see of an axes' children.
fn children(ax: &Axes) -> impl PartialEq + use<> {
    (ax.collections(), ax.texts(), ax.artists())
}

/// `test_contour_remove` and `test_contour_remove_with_labels`.
fn remove(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let before = children(&ax);
    let z = Grid::from_fn(4, 4, |r, c| (4 * r + c) as f64);
    let cs = made(ax.contour(&QuadField::new(z), ContourOptions::default()))?;
    let labels = made(cs.clabel(ClabelOptions::default()))?;
    let changed = children(&ax) != before;
    if args.bool("labels_removed")? {
        labels
            .first()
            .ok_or_else(|| ProbeError::rejected("no labels"))?
            .remove();
    }
    let warned = cs.remove().is_some();
    Ok(object([
        ("changed", Json::Bool(changed)),
        ("restored", Json::Bool(children(&ax) == before)),
        ("warned", Json::Bool(warned)),
    ]))
}

/// `test_contour_clip_path`: contours clipped to a circle in axes
/// coordinates keep it.
fn clip_path() -> Outcome {
    let ax = Figure::new().subplot();
    let z = made(Grid::from_rows(vec![vec![0.0, 1.0], vec![1.0, 0.0]]))?;
    let cs = made(ax.contour(&QuadField::new(z), ContourOptions::default()))?;
    let circle =
        Path::unit_circle().transformed(&Affine2D::identity().scale(0.5, 0.5).translate(0.5, 0.5));
    cs.set_clip_path(Some(ClipPathSpec::Path {
        path: circle,
        transform: ax.trans_axes(),
    }));
    Ok(Json::Bool(cs.clip_path().is_some()))
}

/// `test_allsegs_allkinds`: the number of pieces of each level.
fn allsegs() -> Outcome {
    let x: Vec<f64> = (0..5).map(|i| f64::from(2 * i)).collect();
    let z = Grid::from_fn(5, 5, |r, c| x[c].sin() * x[r].cos());
    let field = QuadField::new(z).with_coords(axes_coords(x.clone(), x));
    let options = ContourOptions {
        levels: Levels::Values(vec![0.0, 0.5]),
        ..ContourOptions::default()
    };
    let cs = made(Figure::new().subplot().contour(&field, options))?;
    let lengths = |counts: Vec<usize>| {
        Json::Array(
            counts
                .into_iter()
                .map(|n| Json::Integer(i64::try_from(n).expect("few pieces")))
                .collect(),
        )
    };
    Ok(object([
        (
            "allsegs",
            lengths(cs.allsegs().iter().map(Vec::len).collect()),
        ),
        (
            "allkinds",
            lengths(cs.allkinds().iter().map(Vec::len).collect()),
        ),
    ]))
}

/// The two Gaussians of `test_linestyles` on its 0.025 grid.
fn two_gaussians() -> QuadField {
    let (x, y) = (arange(-3.0, 3.0, 0.025), arange(-2.0, 2.0, 0.025));
    let z = Grid::from_fn(y.len(), x.len(), |r, c| {
        let (x, y) = (x[c], y[r]);
        ((-x * x - y * y).exp() - (-(x - 1.0).powi(2) - (y - 1.0).powi(2)).exp()) * 2.0
    });
    QuadField::new(z).with_coords(axes_coords(x, y))
}

/// matplotlib's name of a named line style.
fn style_name(style: &LineStyle) -> Result<&'static str, ProbeError> {
    Ok(match style {
        LineStyle::Solid => "solid",
        LineStyle::Dashed => "dashed",
        LineStyle::DashDot => "dashdot",
        LineStyle::Dotted => "dotted",
        other => return Err(ProbeError::rejected(format!("unnamed style {other}"))),
    })
}

/// `test_negative_linestyles`: the style of negative levels from the rc
/// or the keywords.
fn negative_linestyles(args: Args<'_>) -> Outcome {
    let scene = args.str("scene")?;
    let (kind, style) = scene.split_once(':').unwrap_or((scene, ""));
    let mut options = ContourOptions {
        levels: Levels::Count(6),
        colors: Some(vec!["k".into()]),
        ..ContourOptions::default()
    };
    match kind {
        "default" => {}
        "rc" => made(rc::set("contour.negative_linestyle", style))?,
        "kwarg" => options.negative_linestyles = Some(made(style.parse())?),
        "kwarg_linestyles" => {
            options.linestyles = Some(vec![LineStyle::DashDot]);
            options.negative_linestyles = Some(made(style.parse())?);
        }
        _ => return Err(unknown(scene)),
    }
    let ax = Figure::new().subplot();
    let cs = made(ax.contour(&two_gaussians(), options))?;
    made(cs.clabel(ClabelOptions {
        fontsize: Some(FontSize::Points(9.0)),
        ..ClabelOptions::default()
    }))?;
    Ok(Json::from(style_name(&cs.negative_linestyle())?))
}
