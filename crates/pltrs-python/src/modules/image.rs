//! `_native.image`: images (matplotlib's `image` and its native `_image`).
//!
//! An image's data crosses as `(kind, values, mask)`: the kind of numbers
//! (`"double"`, `"single"`, `"integer"`, `"boolean"` scalars to be colour
//! mapped, or `"float128"` and the like for floats wider than double
//! precision, narrowed to it by Python; `"rgb"`/`"rgba"` colours as
//! fractions, `"rgb32"`/`"rgba32"` as single-precision fractions,
//! `"rgb8"`/`"rgba8"` as
//! bytes), the values as an array of shape `(M, N)` or `(M, N, 3|4)`, and
//! the mask of a scalar array. The Python image keeps its colorizer and
//! hands pltrs (Rust) a copy of its colormap and norm (see
//! `colors::mapping`) whenever they change; pltrs (Rust) resamples and
//! colour-maps the image as matplotlib's `_make_image` does. Agg's
//! resampler itself, matplotlib's `_image.resample`, is in [`resample`].

mod bbox;
mod composite;
mod figure;
mod kinds;
mod resample;

use pltrs::artist::{RawHandle, TypedHandle};
use pltrs::axes::{Axes, ImshowOptions};
use pltrs::colors::{ColorizerOptions, Colormap, Norm, NormSpec};
use pltrs::image::{
    AnyImage, AxesImage, BboxImage, Extent, FigureImage, ImageAlpha, ImageData, ImageError,
    Interpolation, InterpolationStage, NonUniformImage, Origin, PcolorImage, ScalarField,
    VectorImsaveError,
};
use pltrs::numeric::Grid;
use pltrs::rc::Aspect;
use pltrs::ticker::AxisName;
use pltrs::transforms::Transform;

use super::backends::DocumentMetadata;
use super::transforms::matrix;
use crate::binding::prelude::*;
use crate::binding::{Element, dict_value};

artist_handle!(AnyImage, AxesImage, BboxImage, FigureImage);

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
fn grid<T: Element>(array: NdArray<T>) -> PyResult<Grid<T>> {
    let (rows, cols) = rows_cols(&array, None)?;
    Grid::from_vec(rows, cols, array.data).map_err(|err| PyErr::value_error(err.to_string()))
}

/// The pixels of a 3-D array of depth `N` as a grid of `[T; N]`.
fn pixels<T: Element, const N: usize>(array: &NdArray<T>) -> PyResult<Grid<[T; N]>> {
    let (rows, cols) = rows_cols(array, Some(N))?;
    let cells = array.data.as_chunks::<N>().0.to_vec();
    Grid::from_vec(rows, cols, cells).map_err(|err| PyErr::value_error(err.to_string()))
}

/// The pixels of a 3-D array of single-precision data, which numpy
/// widened to double precision, as a grid of `[f32; N]`.
fn single_pixels<const N: usize>(array: &NdArray) -> PyResult<Grid<[f32; N]>> {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "single-precision data, widened by numpy, narrows back exactly"
    )]
    let narrow = |v: f64| v as f32;
    Ok(pixels::<f64, N>(array)?.map(|pixel| pixel.map(narrow)))
}

/// Image data as Python hands it over (see the module documentation).
pub(super) struct ImageArray(pub(super) ImageData);

impl FromPy<'_> for ImageArray {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (kind, values, mask): (String, Bound<'_>, Option<NdArray<bool>>) = object.extract()?;
        let mask = mask.map(grid).transpose()?;
        let scalars = |field: ScalarField| -> PyResult<ImageData> {
            let field = match &mask {
                Some(mask) => field.with_mask(mask)?,
                None => field,
            };
            Ok(ImageData::Scalar(field))
        };
        let data = match kind.as_str() {
            "double" => scalars(ScalarField::new(grid(values.extract::<NdArray>()?)?))?,
            "single" => {
                let values = grid(values.extract::<NdArray>()?)?;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "single-precision data, widened by numpy, narrows back exactly"
                )]
                let single = values.map(|&v| v as f32);
                scalars(ScalarField::single(&single))?
            }
            "integer" => scalars(ScalarField::integers(&grid(
                values.extract::<NdArray<i64>>()?,
            )?))?,
            "boolean" => scalars(ScalarField::booleans(&grid(
                values.extract::<NdArray<bool>>()?,
            )?))?,
            wide if wide.starts_with("float") => {
                let bits = wide["float".len()..]
                    .parse()
                    .map_err(|_| PyErr::value_error(format!("no image data kind {wide:?}")))?;
                scalars(ScalarField::narrowed(
                    grid(values.extract::<NdArray>()?)?,
                    bits,
                ))?
            }
            "rgb" => ImageData::Rgb(pixels(&values.extract::<NdArray>()?)?),
            "rgba" => ImageData::Rgba(pixels(&values.extract::<NdArray>()?)?),
            "rgb32" => ImageData::RgbF32(single_pixels(&values.extract::<NdArray>()?)?),
            "rgba32" => ImageData::RgbaF32(single_pixels(&values.extract::<NdArray>()?)?),
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
        Ok(Self(ImageAlpha::Each(grid(object.extract::<NdArray>()?)?)))
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
        transform: dict_value(spec, "transform")?,
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

/// The images of an axes in data coordinates, which have an extent
/// (matplotlib's `AxesImage` and its subclasses).
enum DataImage {
    Axes(AxesImage),
    Pcolor(PcolorImage),
    NonUniform(NonUniformImage),
}

impl FromPy<'_> for DataImage {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let raw: RawHandle = object.extract()?;
        AxesImage::from_raw_handle(&raw)
            .map(Self::Axes)
            .or_else(|| PcolorImage::from_raw_handle(&raw).map(Self::Pcolor))
            .or_else(|| NonUniformImage::from_raw_handle(&raw).map(Self::NonUniform))
            .ok_or_else(|| PyErr::type_error("expected an image of an axes"))
    }
}

/// The extent `(left, right, bottom, top)` of the image in data
/// coordinates (a non-uniform image's is between its first and last
/// centres).
fn get_extent(image: DataImage) -> PyResult<(f64, f64, f64, f64)> {
    let Extent {
        left,
        right,
        bottom,
        top,
    } = match image {
        DataImage::Axes(image) => image.extent(),
        DataImage::Pcolor(image) => image.extent(),
        DataImage::NonUniform(image) => image
            .extent()
            .ok_or_else(|| PyErr::runtime_error("Must set data first"))?,
    };
    Ok((left, right, bottom, top))
}

/// Places the image at `(left, right, bottom, top)`.
fn set_extent(image: DataImage, value: (f64, f64, f64, f64)) -> PyResult<()> {
    let value = extent(value);
    match image {
        DataImage::Axes(image) => {
            image.set_extent(value);
        }
        DataImage::Pcolor(image) => {
            image.set_extent(value);
        }
        DataImage::NonUniform(image) => {
            image.set_extent(value);
        }
    }
    Ok(())
}

/// The interpolation's name.
fn get_interpolation(image: AnyImage) -> PyResult<&'static str> {
    Ok(image.interpolation().name())
}

/// Sets the interpolation by name.
fn set_interpolation(image: AnyImage, name: String) -> PyResult<()> {
    image.set_interpolation(parse(&name)?);
    Ok(())
}

/// The interpolation stage's name.
fn get_interpolation_stage(image: AnyImage) -> PyResult<String> {
    Ok(image.interpolation_stage().to_string())
}

/// Sets the interpolation stage by name.
fn set_interpolation_stage(image: AnyImage, name: String) -> PyResult<()> {
    image.set_interpolation_stage(parse(&name)?);
    Ok(())
}

/// `'upper'` or `'lower'`.
fn get_origin(image: AnyImage) -> PyResult<&'static str> {
    Ok(image.origin().name())
}

/// Whether the resampling filter is normalised.
fn get_filternorm(image: AnyImage) -> PyResult<bool> {
    Ok(image.filternorm())
}

/// Sets whether the resampling filter is normalised.
fn set_filternorm(image: AnyImage, filternorm: bool) -> PyResult<()> {
    image.set_filternorm(filternorm);
    Ok(())
}

/// The radius of the resampling filter.
fn get_filterrad(image: AnyImage) -> PyResult<f64> {
    Ok(image.filterrad())
}

/// Sets the radius of the resampling filter.
fn set_filterrad(image: AnyImage, radius: f64) -> PyResult<()> {
    image.set_filterrad(radius)?;
    Ok(())
}

/// Whether the image is resampled when drawn larger or smaller.
fn get_resample(image: AnyImage) -> PyResult<bool> {
    Ok(image.resample())
}

/// Sets whether the image is resampled.
fn set_resample(image: AnyImage, resample: bool) -> PyResult<()> {
    image.set_resample(resample);
    Ok(())
}

/// An image of `data` colour-mapped through `cmap` and `norm`, with the
/// options of the dict `spec` (`interpolation`, `interpolation_stage`,
/// `origin`, `filternorm`, `filterrad`, `resample`), outside any figure
/// until an offset box shows it (the `BboxImage` of an `OffsetImage`) or
/// an axes or figure adopts it (`BboxImage(bbox)`).
fn new_bbox_image(
    data: ImageArray,
    cmap: Colormap,
    norm: Box<dyn Norm>,
    spec: Owned,
) -> PyResult<BboxImage> {
    let options = Python::with_gil(|py| kinds::image_options(cmap, norm, &spec.bind(py)))?;
    Ok(BboxImage::loose(data.0, options)?)
}

/// The `(row, column)` of the array element under the display point
/// `(x, y)`, `None` outside the image (matplotlib's
/// `AxesImage.get_cursor_data`).
fn cursor_element(image: AxesImage, x: f64, y: f64) -> PyResult<Option<(usize, usize)>> {
    Ok(image.element_at(pltrs::transforms::Point::new(x, y)))
}

/// The image resampled for display at `magnification`, or colour-mapped
/// at its own resolution when `unsampled` (matplotlib's `make_image`):
/// `(pixels, x, y, matrix)`, the pixels an `(M, N, 4)` byte array with the
/// display's bottom row first (as matplotlib's resampler writes it, and
/// as its unsampled array is), `(x, y)` their bottom-left display corner
/// and the 3×3 matrix of the map from array indices to them (`None` when
/// not affine); `None` when nothing is visible. An axes image is
/// stretched over `over`, `((left, right, bottom, top), transform)`, when
/// given: the extent and transform a subclass of the program's gives
/// (`get_extent`, `get_transform`).
fn make_image(
    image: RawHandle,
    magnification: f64,
    unsampled: bool,
    over: Option<((f64, f64, f64, f64), Transform)>,
) -> PyResult<Option<MadeImageParts>> {
    let made = if let Some(image) = image.downcast::<AxesImage>() {
        if let Some((bounds, transform)) = over {
            image.make_image_over(magnification, unsampled, extent(bounds), &transform)?
        } else if unsampled {
            image.make_image_unsampled(magnification)?
        } else {
            image.make_image(magnification)?
        }
    } else if let Some(image) = image.downcast::<FigureImage>() {
        if unsampled {
            image.make_image_unsampled(magnification)?
        } else {
            image.make_image(magnification)?
        }
    } else if let Some(image) = image.downcast::<BboxImage>() {
        if unsampled {
            image.make_image_unsampled(magnification)?
        } else {
            image.make_image(magnification)?
        }
    } else if let Some(image) = image.downcast::<PcolorImage>() {
        image.make_image(magnification)?
    } else if let Some(image) = image.downcast::<NonUniformImage>() {
        image.make_image(magnification)?
    } else {
        return Err(PyErr::type_error("the artist is no image"));
    };
    let Some(made) = made else {
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
/// top row first, at `dpi`, with the document `metadata` (checked by the
/// format's writer in Python, as `savefig` checks it).
fn imsave_vector(
    rgba: NdArray<u8>,
    format: String,
    dpi: f64,
    metadata: Option<DocumentMetadata>,
) -> PyResult<Bytes> {
    let pixels = pixels::<u8, 4>(&rgba)?;
    Ok(Bytes(pltrs::image::imsave_vector(
        pixels,
        format.parse()?,
        dpi,
        metadata.map(|metadata| metadata.0),
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
fn set_image_alpha(image: AnyImage, alpha: Option<Alpha>) -> PyResult<()> {
    image.set_image_alpha(alpha.map(|Alpha(alpha)| alpha));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    resample::register(module)?;
    bbox::register(module)?;
    composite::register(module)?;
    kinds::register(module)?;
    module.add("imshow", imshow)?;
    module.add("get_images", get_images)?;
    module.add("autoscale_on", autoscale_on)?;
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
    module.add("cursor_element", cursor_element)?;
    module.add("new_bbox_image", new_bbox_image)?;
    module.add("make_image", make_image)?;
    module.add("imsave_vector", imsave_vector)?;
    figure::register(module)
}
