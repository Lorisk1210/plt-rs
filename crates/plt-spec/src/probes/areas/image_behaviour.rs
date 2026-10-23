//! Probe operations for the `image_behaviour` area: cursor data, extents,
//! limits and aspect, compositing and the colours at `vmin` of images,
//! ported from matplotlib's `test_image.py` (see
//! `spec/reference/probes/test_image_behaviour.py`). Every op runs under
//! the reference configuration.

use plt::artist::ArtistHandle;
use plt::axes::{Axes, ImshowOptions, NonUniformOptions};
use plt::figure::Figure;
use plt::image::{ColorizerOptions, Extent, ImageData, InterpolationStage, Origin, ScalarField};
use plt::navigation::Navigation;
use plt::numeric::Grid;
use plt::rc::{self, Aspect};
use plt::transforms::Transform;
use plt_core::{Affine2D, Bbox};

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::{bbox, object, parse_points};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("image.cursor", |args| with_rc(args, || cursor(args))),
    Op::new("image.window_extent", |args| {
        with_rc(args, || window_extent(args))
    }),
    Op::new("image.limits", |args| with_rc(args, || limits(args))),
    Op::new("image.composite", |args| with_rc(args, || composite(args))),
    Op::new("image.exact_vmin", |args| with_rc(args, exact_vmin)),
    Op::new("image.draw", |args| with_rc(args, || draw(args))),
    Op::new("image.settings", |args| with_rc(args, settings)),
];

/// The resampling settings, limits, extent and shape of an image as made
/// and after they and the data changed, and whether a zero filter radius
/// is rejected.
fn settings() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let image = made(ax.imshow(
        Grid::from_fn(3, 4, |r, c| (4 * r + c) as f64),
        ImshowOptions::default(),
    ))?;
    let read = |image: &plt::image::AxesImage| {
        let (vmin, vmax) = image.clim();
        let Extent {
            left,
            right,
            bottom,
            top,
        } = image.extent();
        let (rows, columns) = image.array().shape();
        object([
            ("filternorm", image.filternorm().into()),
            ("filterrad", image.filterrad().into()),
            ("resample", image.resample().into()),
            (
                "interpolation_stage",
                match image.interpolation_stage() {
                    InterpolationStage::Auto => "auto",
                    InterpolationStage::Data => "data",
                    InterpolationStage::Rgba => "rgba",
                }
                .into(),
            ),
            (
                "clim",
                Json::from(vec![vmin.unwrap_or(f64::NAN), vmax.unwrap_or(f64::NAN)]),
            ),
            ("extent", Json::from(vec![left, right, bottom, top])),
            ("shape", Json::from(vec![rows as f64, columns as f64])),
        ])
    };
    let before = read(&image);
    image
        .set_filternorm(false)
        .set_resample(true)
        .set_interpolation_stage(InterpolationStage::Rgba)
        .set_data(Grid::from_fn(4, 2, |r, c| (2 * r + c) as f64 * 10.0));
    made(image.set_filterrad(2.0))?;
    let after = read(&image);
    Ok(object([
        ("before", before),
        ("after", after),
        (
            "zero_radius_rejected",
            image.set_filterrad(0.0).is_err().into(),
        ),
    ]))
}

fn made<T>(result: Result<T, impl std::fmt::Display>) -> Result<T, ProbeError> {
    result.map_err(ProbeError::rejected)
}

/// `arange(100).reshape(10, 10)` (integers).
fn grid100() -> ImageData {
    let values = Grid::from_fn(10, 10, |r, c| i64::try_from(10 * r + c).expect("small"));
    ScalarField::integers(&values).into()
}

/// The images of `test_format_cursor_data`: integers when every value is
/// one, as numpy infers from the Python lists.
fn named_data(name: &str) -> Result<ImageData, ProbeError> {
    let integers = |row: Vec<i64>| -> Result<ImageData, ProbeError> {
        Ok(ScalarField::integers(&made(Grid::from_rows(vec![row]))?).into())
    };
    let floats = |row: Vec<f64>| -> Result<ImageData, ProbeError> {
        Ok(made(Grid::from_rows(vec![row]))?.into())
    };
    match name {
        "big" => integers(vec![10001, 10000]),
        "small" => floats(vec![0.123, 0.987]),
        "nan" => floats(vec![f64::NAN, 1.0, 2.0]),
        "tiny_range" => floats(vec![1.0, 1.0 + 1e-15]),
        "negative" => integers(vec![-1, -1]),
        "zeros" => integers(vec![0, 0]),
        other => Err(ProbeError::BadArgs(format!("unknown data {other:?}"))),
    }
}

fn imshow(ax: &Axes, data: ImageData, options: ImshowOptions) -> Result<(), ProbeError> {
    made(ax.imshow(data, options))?;
    Ok(())
}

fn build(ax: &Axes, scene: &str) -> Result<(), ProbeError> {
    let plain = ImshowOptions::default;
    match scene {
        "upper" => {
            imshow(ax, grid100(), plain())?;
            ax.set_xlim(-1.0, 11.0).set_ylim(11.0, -1.0);
        }
        "lower" => {
            let options = ImshowOptions {
                origin: Some(Origin::Lower),
                ..plain()
            };
            imshow(ax, grid100(), options)?;
        }
        "extent" => {
            let options = ImshowOptions {
                extent: Some(Extent::new(0.0, 0.5, 0.0, 0.5)),
                ..plain()
            };
            imshow(ax, grid100(), options)?;
            ax.set_xlim(-0.1, 1.0).set_ylim(-0.1, 0.6);
        }
        "transformed" => {
            let turn = Affine2D::scaling(2.0, 2.0).rotate(0.5);
            let transform = Transform::Affine(turn).then(ax.trans_data());
            imshow(ax, grid100(), placed(transform, None))?;
            ax.set_xlim(-20.0, 20.0).set_ylim(-5.0, 25.0);
        }
        "nonuniform" => {
            let x = [0.0, 1.0, 4.0, 9.0, 16.0];
            let y = [0.0, 1.0, 2.0, 3.0, 4.0];
            let z = Grid::from_fn(5, 5, |r, c| x[c] * x[c] + y[r] * y[r]);
            made(ax.nonuniform_image(&x, &y, z, NonUniformOptions::default()))?;
            ax.set_xlim(-2.0, 16.0).set_ylim(-2.0, 4.0);
        }
        other => match other.strip_prefix("data:") {
            Some(name) => imshow(ax, named_data(name)?, plain())?,
            None => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
        },
    }
    Ok(())
}

/// The status bar message at each display pixel.
fn cursor(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    build(&fig.subplot(), args.str("scene")?)?;
    fig.layout();
    let navigation = Navigation::new();
    let messages = parse_points(args.json("pixels")?)?
        .into_iter()
        .map(|pixel| Json::String(navigation.readout(&fig, pixel)))
        .collect();
    Ok(Json::Array(messages))
}

/// `get_window_extent()` of an image in data or axes coordinates.
fn window_extent(args: Args<'_>) -> Outcome {
    let in_axes = args.str("transform")? == "axes";
    let fig = Figure::with_size(10.0, 10.0, 100.0);
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    let x0 = if in_axes { 1.0 } else { 0.0 };
    ax.set_xlim(x0, x0 + 1.0).set_ylim(0.0, 1.0);
    let rows = vec![
        vec![0.25, 0.75, 1.0, 0.75],
        vec![0.1, 0.65, 0.5, 0.4],
        vec![0.6, 0.3, 0.0, 0.2],
        vec![0.7, 0.9, 0.4, 0.6],
    ];
    let options = ImshowOptions {
        extent: Some(Extent::new(0.4, 0.7, 0.2, 0.9)),
        interpolation: Some(made("nearest".parse())?),
        transform: in_axes.then(|| ax.trans_axes()),
        ..ImshowOptions::default()
    };
    let image = made(ax.imshow(made(Grid::from_rows(rows))?, options))?;
    Ok(bbox(&image.window_extent().unwrap_or(Bbox::NULL)))
}

/// Options placing an image in `transform`.
fn placed(transform: Transform, aspect: Option<Aspect>) -> ImshowOptions {
    ImshowOptions {
        transform: Some(transform),
        aspect,
        ..ImshowOptions::default()
    }
}

/// The limits and aspect an image leaves its axes with.
fn limits(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let grid = || ImageData::from(Grid::from_fn(2, 2, |r, c| (2 * r + c) as f64));
    match args.str("scene")? {
        "relim" => {
            let options = ImshowOptions {
                extent: Some(Extent::new(0.0, 1.0, 0.0, 1.0)),
                ..ImshowOptions::default()
            };
            imshow(&ax, Grid::filled(1, 1, 0.0).into(), options)?;
            ax.relim(false).autoscale(None, None);
        }
        "axes" => imshow(&ax, grid(), placed(ax.trans_axes(), None))?,
        "affine" => {
            let scaled = Transform::Affine(Affine2D::scaling(2.0, 2.0)).then(ax.trans_data());
            imshow(&ax, grid(), placed(scaled, None))?;
        }
        "axes_aspect" => imshow(
            &ax,
            grid(),
            placed(ax.trans_axes(), Some(Aspect::Ratio(2.0))),
        )?,
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }
    let aspect = match ax.aspect() {
        Aspect::Auto => Json::from("auto"),
        Aspect::Equal => Json::from(1.0),
        Aspect::Ratio(ratio) => Json::from(ratio),
    };
    let (x0, x1) = ax.xlim();
    let (y0, y1) = ax.ylim();
    Ok(object([
        ("xlim", Json::from(vec![x0, x1])),
        ("ylim", Json::from(vec![y0, y1])),
        ("aspect", aspect),
    ]))
}

/// How many images two images on one axes become in vector output.
fn composite(args: Args<'_>) -> Outcome {
    let format = args.str("fmt")?;
    let composite = if args.bool("composite")? {
        "True"
    } else {
        "False"
    };
    let z = Grid::from_fn(10, 10, |r, _| (r as f64 - 5.0).powi(2).sin());
    let flipped = Grid::from_fn(10, 10, |r, _| (4.0 - r as f64).powi(2).sin());
    let count = rc::context(|| {
        made(rc::set("image.composite_image", composite))?;
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 3.0);
        for (data, left) in [(z, 0.0), (flipped, 2.0)] {
            let options = ImshowOptions {
                extent: Some(Extent::new(left, left + 1.0, 0.0, 1.0)),
                ..ImshowOptions::default()
            };
            imshow(&ax, data.into(), options)?;
        }
        let (output, needle) = match format {
            "svg" => (made(fig.to_svg())?, "<image"),
            "ps" => (made(fig.to_ps())?, " colorimage"),
            other => return Err(ProbeError::BadArgs(format!("unknown format {other:?}"))),
        };
        Ok(output.matches(needle).count())
    })?;
    Ok(Json::Integer(i64::try_from(count).expect("few images")))
}

/// The middle pixel row of a one-row image whose values sit at `vmin`.
fn exact_vmin() -> Outcome {
    let cmap = made(made(plt::colors::colormaps::get("autumn_r"))?.with_under("lightgrey"))?;
    let fig = Figure::with_size(1.9, 0.1, 100.0);
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    let values = vec![
        -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 43.0, 79.0, 95.0, 66.0, 1.0, -1.0, -1.0, -1.0, 0.0,
        0.0, 0.0, 34.0,
    ];
    let options = ImshowOptions {
        colors: ColorizerOptions {
            cmap: Some(cmap),
            vmin: Some(0.0),
            vmax: Some(100.0),
            ..ColorizerOptions::default()
        },
        aspect: Some(Aspect::Auto),
        ..ImshowOptions::default()
    };
    imshow(&ax, made(Grid::from_rows(vec![values]))?.into(), options)?;
    ax.set_axis_off();
    let image = made(fig.render_raster())?.to_image();
    let row = (0..image.width())
        .map(|x| {
            let pixel = image.pixel(x, 5).unwrap_or_default();
            Json::Array(pixel.iter().map(|&v| Json::Integer(i64::from(v))).collect())
        })
        .collect();
    Ok(Json::Array(row))
}

/// Whether images of invalid values draw.
fn draw(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let data = match args.str("scene")? {
        "all_nan" => Grid::filled(2, 2, f64::NAN),
        "partly_nan" => made(Grid::from_rows(vec![vec![1.0, 2.0], vec![3.0, f64::NAN]]))?,
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    };
    imshow(&ax, data.into(), ImshowOptions::default())?;
    made(fig.render_raster())?;
    Ok("drawn".into())
}
