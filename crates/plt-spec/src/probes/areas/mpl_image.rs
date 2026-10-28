//! Probe operations for the `mpl_image` area: matplotlib's own non-image
//! tests of images (see `spec/reference/probes/test_mpl_image.py`).

use std::collections::BTreeSet;
use std::rc::Rc;

use plt::axes::ImshowOptions;
use plt::colors::{Colorize, ColorizerOptions, NoNorm};
use plt::figure::{Figure, SaveFormat, SaveOptions};
use plt::image::{
    FigimageOptions, ImageData, ImsaveOptions, Interpolation, ResampleOptions, ScalarField,
    imread_bytes, imsave_bytes, imsave_format, resample,
};
use plt::navigation::Navigation;
use plt::numeric::{Grid, linspace};
use plt::transforms::{Affine2D, Point, PointMap, ResolvedTransform, Step};

use super::mpl_agg::{chunks_and_text, png_metadata};
use super::rcsetup::validate;
use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::count;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};
use crate::rng::SpecRng;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("image.AxesImage.format_cursor_data", |args| {
        with_rc(args, || format_cursor_data(args))
    }),
    Op::new(
        "image.AxesImage.get_interpolation_stage",
        interpolation_stage,
    ),
    Op::new("rc.validate", validate),
    Op::new("image.AxesImage.get_array", get_array),
    Op::new("image.imsave.imread", round_trip),
    Op::new("image.imsave.png_text", imsave_text),
    Op::new("pyplot.savefig.jpg", |args| with_rc(args, jpeg_alpha)),
    Op::new("image.resample", resample_row),
];

/// A pair of numbers.
fn pair(args: Args<'_>, key: &str) -> Result<(f64, f64), ProbeError> {
    match args.f64_vec(key)?[..] {
        [a, b] => Ok((a, b)),
        _ => Err(ProbeError::BadArgs(format!("{key} needs two values"))),
    }
}

/// `im.format_cursor_data(im.get_cursor_data(event))` at a data point,
/// read from the status bar the event writes.
fn format_cursor_data(args: Args<'_>) -> Outcome {
    if args.str("dtype")? != "uint8" {
        return Err(ProbeError::BadArgs("only uint8 data is probed".into()));
    }
    let values = Grid::from_rows(
        args.rows("data")?
            .into_iter()
            .map(|row| row.into_iter().map(|v| v as i64).collect::<Vec<_>>()),
    )
    .map_err(ProbeError::rejected)?;
    let colors = match args.opt_str("norm")? {
        None => ColorizerOptions::default(),
        Some("NoNorm") => ColorizerOptions::default().norm(NoNorm::default()),
        Some(other) => return Err(ProbeError::BadArgs(format!("unknown norm {other:?}"))),
    };
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.imshow(
        ScalarField::integers(&values),
        ImshowOptions::default().colorizer(colors),
    )
    .map_err(ProbeError::rejected)?;
    let (x, y) = pair(args, "point")?;
    let data = fig
        .resolve_transform(&ax.trans_data())
        .map_err(ProbeError::rejected)?;
    let message = Navigation::new().readout(&fig, data.transform_point(Point::new(x, y)));
    // The second line of the message is the image's cursor data.
    Ok(Json::from(message.lines().last().unwrap_or_default()))
}

fn interpolation_stage(args: Args<'_>) -> Outcome {
    plt::rc::context(|| {
        plt::rc::set("image.interpolation_stage", args.str("rc")?).map_err(ProbeError::rejected)?;
        let image = Figure::new()
            .subplot()
            .imshow(
                Grid::from_vec(1, 2, vec![1.0, 2.0]).map_err(ProbeError::rejected)?,
                ImshowOptions::default(),
            )
            .map_err(ProbeError::rejected)?;
        Ok(Json::from(image.interpolation_stage().to_string()))
    })
}

/// `arange(300).reshape((10, 10, 3)) - 10` divided by 255: RGB colours
/// below 0 and above 1.
fn out_of_range(index: usize) -> f64 {
    (index as f64 - 10.0) / 255.0
}

/// Colours as `[[[r, g, b], ...], ...]`.
fn colors<T: Copy + Into<f64>>(grid: &Grid<[T; 3]>) -> Json {
    Json::Array(
        grid.iter_rows()
            .map(|row| {
                Json::Array(
                    row.iter()
                        .map(|rgb| Json::from(rgb.map(Into::into).to_vec()))
                        .collect(),
                )
            })
            .collect(),
    )
}

fn get_array(args: Args<'_>) -> Outcome {
    let data: ImageData = match args.str("dtype")? {
        "float32" => Grid::from_fn(10, 10, |r, c| {
            let base = 3 * (10 * r + c);
            // numpy divides float32 data in single precision.
            [0, 1, 2].map(|k| ((base + k) as f32 - 10.0) / 255.0)
        })
        .into(),
        "float64" => Grid::from_fn(10, 10, |r, c| {
            let base = 3 * (10 * r + c);
            [0, 1, 2].map(|k| out_of_range(base + k))
        })
        .into(),
        other => return Err(ProbeError::BadArgs(format!("unprobed dtype {other:?}"))),
    };
    let image = Figure::new()
        .subplot()
        .imshow(data, ImshowOptions::default())
        .map_err(ProbeError::rejected)?;
    match image.array() {
        ImageData::Rgb(grid) => Ok(colors(&grid)),
        ImageData::RgbF32(grid) => Ok(colors(&grid)),
        other => Err(ProbeError::rejected(format!(
            "the image keeps {:?} data",
            other.shape()
        ))),
    }
}

/// The image data of a `data` argument.
fn spec_data(spec: &Json) -> Result<ImageData, ProbeError> {
    let spec = Args::new(spec);
    let dims = |key| -> Result<Vec<usize>, ProbeError> {
        spec.f64_vec(key)
            .map(|dims| dims.into_iter().map(|d| d as usize).collect())
    };
    if spec.contains("zeros") {
        let [rows, cols] = dims("zeros")?[..] else {
            return Err(ProbeError::BadArgs("zeros needs two dimensions".into()));
        };
        return Ok(Grid::filled(rows, cols, 0.0).into());
    }
    let seed = u64::try_from(spec.i64("uniform")?).map_err(ProbeError::rejected)?;
    let shape = dims("shape")?;
    let values = SpecRng::new(seed).uniform(shape.iter().product(), 0.0, 1.0);
    match shape[..] {
        [rows, cols] => Ok(Grid::from_vec(rows, cols, values)
            .map_err(ProbeError::rejected)?
            .into()),
        [rows, cols, 4] => {
            let pixels = values.as_chunks::<4>().0.to_vec();
            Ok(Grid::from_vec(rows, cols, pixels)
                .map_err(ProbeError::rejected)?
                .into())
        }
        _ => Err(ProbeError::BadArgs(format!("unprobed shape {shape:?}"))),
    }
}

/// The values of image data as bytes (floats times 255, rounded), row by
/// row with the channels last, and the number of channels.
fn image_bytes(data: &ImageData) -> Result<(Vec<i64>, usize), ProbeError> {
    fn flat<T: Copy, const N: usize>(
        grid: &Grid<[T; N]>,
        byte: impl Fn(T) -> i64,
    ) -> (Vec<i64>, usize) {
        let values = grid
            .as_slice()
            .iter()
            .flat_map(|pixel| pixel.map(&byte))
            .collect();
        (values, N)
    }
    let scaled = |v: f64| (v * 255.0).round() as i64;
    Ok(match data {
        ImageData::Rgb(grid) => flat(grid, scaled),
        ImageData::Rgba(grid) => flat(grid, scaled),
        ImageData::RgbF32(grid) => flat(grid, |v| scaled(f64::from(v))),
        ImageData::RgbaF32(grid) => flat(grid, |v| scaled(f64::from(v))),
        ImageData::RgbBytes(grid) => flat(grid, i64::from),
        ImageData::RgbaBytes(grid) => flat(grid, i64::from),
        ImageData::Scalar(_) => return Err(ProbeError::rejected("grey images are not probed")),
    })
}

fn round_trip(args: Args<'_>) -> Outcome {
    let defaults = ImsaveOptions::default();
    let options = ImsaveOptions {
        dpi: args.opt_f64("dpi")?.unwrap_or(defaults.dpi),
        origin: args
            .opt_str("origin")?
            .map(str::parse)
            .transpose()
            .map_err(ProbeError::rejected)?,
        ..defaults
    };
    let format = imsave_format(args.str("format")?).map_err(ProbeError::rejected)?;
    let written = imsave_bytes(spec_data(args.json("data")?)?, format, options)
        .map_err(ProbeError::rejected)?;
    let read = imread_bytes(&written).map_err(ProbeError::rejected)?;
    let (rows, cols) = read.shape();
    let (values, channels) = image_bytes(&read)?;
    let sum: i64 = values.iter().sum();
    let weighted: i64 = (1..).zip(&values).map(|(i, v)| i * v).sum();
    Ok(encode::object([
        (
            "shape",
            Json::Array(vec![count(rows), count(cols), count(channels)]),
        ),
        ("sum", Json::from(sum as f64)),
        ("weighted", Json::from(weighted as f64)),
    ]))
}

/// The chunk types and text entries of the 2×2 array `[[0, 1], [2, 3]]`
/// saved by `imsave` in `format` with the probe's metadata.
fn imsave_text(args: Args<'_>) -> Outcome {
    let format = imsave_format(args.str("format")?).map_err(ProbeError::rejected)?;
    let options = ImsaveOptions {
        metadata: png_metadata(args)?,
        ..ImsaveOptions::default()
    };
    let data = Grid::from_rows([[0.0, 1.0], [2.0, 3.0]]).map_err(ProbeError::rejected)?;
    let png = imsave_bytes(data, format, options).map_err(ProbeError::rejected)?;
    chunks_and_text(&png)
}

/// `test_jpeg_alpha`'s JPEG: the number of its colours and its top left
/// pixel.
fn jpeg_alpha() -> Outcome {
    let fig = Figure::with_size(1.0, 1.0, 300.0);
    let alphas = linspace(0.0, 1.0, 300);
    let ramp = Grid::from_fn(300, 300, |_, c| [0.0, 0.0, 0.0, alphas[c]]);
    fig.figimage(ramp, FigimageOptions::default())
        .map_err(ProbeError::rejected)?;
    let options = SaveOptions {
        facecolor: Some("red".into()),
        dpi: Some(300.0),
        ..SaveOptions::default()
    };
    let jpeg = fig
        .to_format(SaveFormat::Jpeg, options)
        .map_err(ProbeError::rejected)?;
    let ImageData::RgbBytes(pixels) = imread_bytes(&jpeg).map_err(ProbeError::rejected)? else {
        return Err(ProbeError::rejected("the JPEG is not RGB"));
    };
    let colors: BTreeSet<[u8; 3]> = pixels.as_slice().iter().copied().collect();
    let corner = pixels.as_slice().first().copied().unwrap_or_default();
    Ok(encode::object([
        ("num_colors", count(colors.len())),
        ("corner", Json::from(corner.map(f64::from).to_vec())),
    ]))
}

/// The test's `nonaffine_identity`: the identity as a non-affine map.
#[derive(Debug)]
struct NonAffineIdentity;

impl PointMap for NonAffineIdentity {
    fn forward(&self, point: Point) -> Point {
        point
    }

    fn inverse(&self, point: Point) -> Point {
        point
    }
}

fn resample_row(args: Args<'_>) -> Outcome {
    let row = args.f64_vec("data")?;
    let width = usize::try_from(args.i64("width")?).map_err(ProbeError::rejected)?;
    let stretch = Affine2D::scaling(width as f64 / row.len() as f64, 1.0);
    let transform = if args.bool("nonaffine")? {
        ResolvedTransform::with_step(Step::Plane(Rc::new(NonAffineIdentity)), stretch)
    } else {
        ResolvedTransform::affine(stretch)
    };
    let interpolation: Interpolation = args
        .str("interpolation")?
        .parse()
        .map_err(ProbeError::rejected)?;
    let data = Grid::from_vec(1, row.len(), row).map_err(ProbeError::rejected)?;
    let mut out = Grid::filled(1, width, 0.0);
    let options = ResampleOptions {
        interpolation,
        ..ResampleOptions::default()
    };
    resample(&data, &mut out, &transform, options).map_err(ProbeError::rejected)?;
    Ok(Json::from(out.row(0).to_vec()))
}
