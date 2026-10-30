//! `_native.image`: images (matplotlib's `image` and its native `_image`).
//!
//! An image's data crosses as `(kind, values, mask)`: the kind of numbers
//! (`"double"`, `"single"`, `"integer"`, `"boolean"` scalars to be colour
//! mapped, or `"float128"` and the like for floats wider than double
//! precision, narrowed to it by Python; `"rgb"`/`"rgba"` colours as
//! fractions, `"rgb8"`/`"rgba8"` as
//! bytes), the values as an array of shape `(M, N)` or `(M, N, 3|4)`, and
//! the mask of a scalar array. The Python image keeps its colorizer and
//! hands pltrs (Rust) a copy of its colormap and norm (see
//! `colors::mapping`) whenever they change; pltrs (Rust) resamples and
//! colour-maps the image as matplotlib's `_make_image` does. Agg's
//! resampler itself, matplotlib's `_image.resample`, is in [`resample`].

mod resample;

use pltrs::axes::{Axes, ImshowOptions};
use pltrs::colors::{ColorizerOptions, Colormap, Norm, NormSpec};
use pltrs::image::{
    AxesImage, Extent, ImageAlpha, ImageData, ImageError, Interpolation, InterpolationStage,
    Origin, ScalarField, VectorImsaveError,
};
use pltrs::numeric::Grid;
use pltrs::rc::Aspect;
use pltrs::ticker::AxisName;

use super::transforms::matrix;
use crate::binding::prelude::*;
use crate::binding::{Element, dict_value};

artist_handle!(AxesImage);

impl From<ImageError> for PyErr {
    /// matplotlib's `ValueError` for images it cannot make or draw; its
    /// `TypeError`s and messages for the meshes `pcolor` and `pcolormesh`
    /// cannot fit to their colours (`_pcolorargs`).
    fn from(err: ImageError) -> Self {
        match err {
            ImageError::MeshShape {
                shading: "flat",
                values,
                nx,
                ny,
            } => PyErr::type_error(format!(
                "Dimensions of C {values:?} should be one smaller than X({nx}) and Y({ny}) \
                 while using shading='flat'"
            )),
            ImageError::MeshShape { values, nx, ny, .. } => PyErr::type_error(format!(
                "Dimensions of C {values:?} are incompatible with X ({nx}) and/or Y ({ny})"
            )),
            ImageError::IncompatibleCoordinates => PyErr::type_error("Incompatible X, Y inputs"),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// The `(rows, cols)` of an array of shape `(rows, cols, ...)`.
fn rows_cols<T: Element>(array: &NdArray<T>, depth: Option<usize>) -> PyResult<(usize, usize)> {
    match (&array.shape[..], depth) {
        ([rows, cols], None) => Ok((*rows, *cols)),
        ([rows, cols, d], Some(depth)) if *d == depth => Ok((*rows, *cols)),
        (shape, _) => Err(PyErr::type_error(format!(
            "Invalid shape {shape:?} for image data"
        ))),
    }
}

/// The cells of a 2-D array as a grid.
fn grid<T: Element>(array: &NdArray<T>) -> PyResult<Grid<T>> {
    let (rows, cols) = rows_cols(array, None)?;
    Grid::from_vec(rows, cols, array.data.clone())
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// The pixels of a 3-D array of depth `N` as a grid of `[T; N]`.
fn pixels<T: Element, const N: usize>(array: &NdArray<T>) -> PyResult<Grid<[T; N]>> {
    let (rows, cols) = rows_cols(array, Some(N))?;
    let cells = array.data.as_chunks::<N>().0.to_vec();
    Grid::from_vec(rows, cols, cells).map_err(|err| PyErr::value_error(err.to_string()))
}

/// Image data as Python hands it over (see the module documentation).
pub(super) struct ImageArray(pub(super) ImageData);

impl FromPy<'_> for ImageArray {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (kind, values, mask): (String, Bound<'_>, Option<NdArray<bool>>) = object.extract()?;
        let scalars = |field: ScalarField| -> PyResult<ImageData> {
            let field = match &mask {
                Some(mask) => field.with_mask(&grid(mask)?)?,
                None => field,
            };
            Ok(ImageData::Scalar(field))
        };
        let data = match kind.as_str() {
            "double" => scalars(ScalarField::new(grid(&values.extract::<NdArray>()?)?))?,
            "single" => {
                let values = grid(&values.extract::<NdArray>()?)?;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "single-precision data, widened by numpy, narrows back exactly"
                )]
                let single = values.map(|&v| v as f32);
                scalars(ScalarField::single(&single))?
            }
            "integer" => scalars(ScalarField::integers(&grid(
                &values.extract::<NdArray<i64>>()?,
            )?))?,
            "boolean" => scalars(ScalarField::booleans(&grid(
                &values.extract::<NdArray<bool>>()?,
            )?))?,
            wide if wide.starts_with("float") => {
                let bits = wide["float".len()..]
                    .parse()
                    .map_err(|_| PyErr::value_error(format!("no image data kind {wide:?}")))?;
                scalars(ScalarField::narrowed(
                    grid(&values.extract::<NdArray>()?)?,
                    bits,
                ))?
            }
            "rgb" => ImageData::Rgb(pixels(&values.extract::<NdArray>()?)?),
            "rgba" => ImageData::Rgba(pixels(&values.extract::<NdArray>()?)?),
            "rgb8" => ImageData::RgbBytes(pixels(&values.extract::<NdArray<u8>>()?)?),
            "rgba8" => ImageData::RgbaBytes(pixels(&values.extract::<NdArray<u8>>()?)?),
            other => {
                return Err(PyErr::value_error(format!("no image data kind {other:?}")));
            }
        };
        Ok(Self(data))
    }
}

/// An image's opacity: one value, or one per pixel.
struct Alpha(ImageAlpha);

impl FromPy<'_> for Alpha {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(alpha) = object.extract::<f64>() {
            return Ok(Self(ImageAlpha::Uniform(alpha)));
        }
        Ok(Self(ImageAlpha::Each(grid(&object.extract::<NdArray>()?)?)))
    }
}

/// A name matplotlib gives an interpolation, origin or stage.
fn parse<T: std::str::FromStr<Err: std::fmt::Display>>(name: &str) -> PyResult<T> {
    name.parse()
        .map_err(|err: T::Err| PyErr::value_error(err.to_string()))
}

/// The aspect `set_aspect` takes: `'equal'`, `'auto'` or a ratio.
fn aspect(value: &Bound<'_>) -> PyResult<Aspect> {
    if let Ok(name) = value.extract::<String>() {
        return match name.as_str() {
            "equal" => Ok(Aspect::Equal),
            "auto" => Ok(Aspect::Auto),
            _ => Err(PyErr::value_error(format!(
                "aspect must be finite and positive, 'equal' or 'auto', not {name:?}"
            ))),
        };
    }
    Ok(Aspect::Ratio(value.extract()?))
}

/// An extent `(left, right, bottom, top)`.
fn extent((left, right, bottom, top): (f64, f64, f64, f64)) -> Extent {
    Extent::new(left, right, bottom, top)
}

/// The options of `imshow` given as a dict of matplotlib's keywords.
fn imshow_options(
    cmap: Colormap,
    norm: Box<dyn Norm>,
    spec: &Bound<'_>,
) -> PyResult<ImshowOptions> {
    let named = |key: &str| -> PyResult<Option<String>> { dict_value(spec, key) };
    let aspect_value: Bound<'_> = dict_value(spec, "aspect")?;
    Ok(ImshowOptions {
        colorizer: ColorizerOptions {
            cmap: Some(cmap),
            norm: Some(NormSpec::Norm(norm)),
            ..ColorizerOptions::default()
        },
        aspect: if aspect_value.is_none() {
            None
        } else {
            Some(aspect(&aspect_value)?)
        },
        interpolation: named("interpolation")?
            .as_deref()
            .map(parse::<Interpolation>)
            .transpose()?,
        interpolation_stage: named("interpolation_stage")?
            .as_deref()
            .map(parse::<InterpolationStage>)
            .transpose()?,
        origin: named("origin")?
            .as_deref()
            .map(parse::<Origin>)
            .transpose()?,
        extent: dict_value::<Option<(f64, f64, f64, f64)>>(spec, "extent")?.map(extent),
        filternorm: dict_value(spec, "filternorm")?,
        filterrad: dict_value(spec, "filterrad")?,
        resample: dict_value(spec, "resample")?,
        ..ImshowOptions::default()
    })
}

/// `imshow`: `data` shown on a regular grid in `axes`, colour-mapped
/// through `cmap` and `norm`, with the options of the dict `spec`.
fn imshow(
    axes: Axes,
    data: ImageArray,
    cmap: Colormap,
    norm: Box<dyn Norm>,
    spec: Owned,
) -> PyResult<AxesImage> {
    let options = Python::with_gil(|py| imshow_options(cmap, norm, &spec.bind(py)))?;
    Ok(axes.imshow(data.0, options)?)
}

/// Whether the x and y views of `axes` follow their data, so that an
/// image's extent moves them (`get_autoscalex_on`, `get_autoscaley_on`).
fn autoscale_on(axes: Axes) -> PyResult<(bool, bool)> {
    Ok((
        axes.autoscale_on(Some(AxisName::X)),
        axes.autoscale_on(Some(AxisName::Y)),
    ))
}

/// The images of `axes`, in the order they were added.
fn get_images(axes: Axes) -> PyResult<Vec<AxesImage>> {
    Ok(axes.images())
}

/// Replaces the image's data.
fn set_data(image: AxesImage, data: ImageArray) -> PyResult<()> {
    image.set_data(data.0);
    Ok(())
}

/// Maps the image through `cmap` and `norm` from now on.
fn set_mapping(image: AxesImage, cmap: Colormap, norm: Box<dyn Norm>) -> PyResult<()> {
    image.set_cmap(cmap);
    image.set_norm(norm);
    Ok(())
}

/// The extent `(left, right, bottom, top)` of the image in data
/// coordinates.
fn get_extent(image: AxesImage) -> PyResult<(f64, f64, f64, f64)> {
    let Extent {
        left,
        right,
        bottom,
        top,
    } = image.extent();
    Ok((left, right, bottom, top))
}

/// Places the image at `(left, right, bottom, top)`.
fn set_extent(image: AxesImage, value: (f64, f64, f64, f64)) -> PyResult<()> {
    image.set_extent(extent(value));
    Ok(())
}

/// The interpolation's name.
fn get_interpolation(image: AxesImage) -> PyResult<&'static str> {
    Ok(image.interpolation().name())
}

/// Sets the interpolation by name.
fn set_interpolation(image: AxesImage, name: String) -> PyResult<()> {
    image.set_interpolation(parse(&name)?);
    Ok(())
}

/// The interpolation stage's name.
fn get_interpolation_stage(image: AxesImage) -> PyResult<String> {
    Ok(image.interpolation_stage().to_string())
}

/// Sets the interpolation stage by name.
fn set_interpolation_stage(image: AxesImage, name: String) -> PyResult<()> {
    image.set_interpolation_stage(parse(&name)?);
    Ok(())
}

/// `'upper'` or `'lower'`.
fn get_origin(image: AxesImage) -> PyResult<&'static str> {
    Ok(image.origin().name())
}

/// Whether the resampling filter is normalised.
fn get_filternorm(image: AxesImage) -> PyResult<bool> {
    Ok(image.filternorm())
}

/// Sets whether the resampling filter is normalised.
fn set_filternorm(image: AxesImage, filternorm: bool) -> PyResult<()> {
    image.set_filternorm(filternorm);
    Ok(())
}

/// The radius of the resampling filter.
fn get_filterrad(image: AxesImage) -> PyResult<f64> {
    Ok(image.filterrad())
}

/// Sets the radius of the resampling filter.
fn set_filterrad(image: AxesImage, radius: f64) -> PyResult<()> {
    image.set_filterrad(radius)?;
    Ok(())
}

/// Whether the image is resampled when drawn larger or smaller.
fn get_resample(image: AxesImage) -> PyResult<bool> {
    Ok(image.resample())
}

/// Sets whether the image is resampled.
fn set_resample(image: AxesImage, resample: bool) -> PyResult<()> {
    image.set_resample(resample);
    Ok(())
}

/// The image resampled for display at `magnification` (matplotlib's
/// `make_image`): `(pixels, x, y, matrix)`, the pixels an `(M, N, 4)` byte
/// array with the display's bottom row first (as matplotlib's resampler
/// writes it), `(x, y)` their bottom-left display corner and the 3×3
/// matrix of the map from array indices to them (`None` when not affine);
/// `None` when nothing is visible.
fn make_image(image: AxesImage, magnification: f64) -> PyResult<Option<MadeImageParts>> {
    let Some(made) = image.make_image(magnification)? else {
        return Ok(None);
    };
    let (width, height) = (made.pixels.width(), made.pixels.height());
    let bottom_up = made
        .pixels
        .data()
        .chunks_exact(width * 4)
        .rev()
        .flatten()
        .copied()
        .collect();
    let pixels = NdArray::with_shape(bottom_up, vec![height, width, 4])?;
    let matrix = made.transform.map(matrix).transpose()?;
    Ok(Some((pixels, made.origin.x, made.origin.y, matrix)))
}

/// `imsave` to a vector format: the document of `format` (`'pdf'`,
/// `'ps'`, `'eps'` or `'svg'`) showing the `(M, N, 4)` byte array `rgba`,
/// top row first, at `dpi`.
fn imsave_vector(rgba: NdArray<u8>, format: String, dpi: f64) -> PyResult<Bytes> {
    let pixels = pixels::<u8, 4>(&rgba)?;
    Ok(Bytes(pltrs::image::imsave_vector(
        pixels,
        format.parse()?,
        dpi,
        None,
    )?))
}

impl From<VectorImsaveError> for PyErr {
    /// matplotlib's `ValueError` for formats it cannot write and images it
    /// cannot place, and the errors of `savefig` otherwise.
    fn from(err: VectorImsaveError) -> Self {
        match err {
            VectorImsaveError::Save(err) => err.into(),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// What [`make_image`] returns.
type MadeImageParts = (NdArray<u8>, f64, f64, Option<NdArray>);

/// Sets the image's opacity: one value, one per pixel, or `None`.
fn set_image_alpha(image: AxesImage, alpha: Option<Alpha>) -> PyResult<()> {
    image.set_image_alpha(alpha.map(|Alpha(alpha)| alpha));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    resample::register(module)?;
    module.add("imshow", imshow)?;
    module.add("get_images", get_images)?;
    module.add("autoscale_on", autoscale_on)?;
    module.add("set_data", set_data)?;
    module.add("set_mapping", set_mapping)?;
    module.add("get_extent", get_extent)?;
    module.add("set_extent", set_extent)?;
    module.add("get_interpolation", get_interpolation)?;
    module.add("set_interpolation", set_interpolation)?;
    module.add("get_interpolation_stage", get_interpolation_stage)?;
    module.add("set_interpolation_stage", set_interpolation_stage)?;
    module.add("get_origin", get_origin)?;
    module.add("get_filternorm", get_filternorm)?;
    module.add("set_filternorm", set_filternorm)?;
    module.add("get_filterrad", get_filterrad)?;
    module.add("set_filterrad", set_filterrad)?;
    module.add("get_resample", get_resample)?;
    module.add("set_resample", set_resample)?;
    module.add("set_image_alpha", set_image_alpha)?;
    module.add("make_image", make_image)?;
    module.add("imsave_vector", imsave_vector)
}
