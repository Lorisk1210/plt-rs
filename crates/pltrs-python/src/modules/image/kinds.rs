//! The image classes made by their constructors (matplotlib's
//! `AxesImage(ax)`, `NonUniformImage(ax)`, `PcolorImage(ax)`,
//! `FigureImage(fig)` and `BboxImage(bbox)`): loose images without data
//! until `set_data`, which an axes or a figure adopts; and what every kind
//! of image shares, its colour mapping.

use pltrs::artist::{RawHandle, TypedHandle};
use pltrs::axes::NonUniformOptions;
use pltrs::colors::{ColorizerOptions, Colormap, Norm, NormSpec};
use pltrs::image::{
    AxesImage, BboxImage, BboxImageOptions, FigimageOptions, FigureImage, ImageData,
    NonUniformImage, NonUniformInterpolation, Origin, PcolorImage,
};
use pltrs::transforms::{Bbox, Transform};

use super::{ImageArray, extent, parse};
use crate::binding::dict_value;
use crate::binding::prelude::*;

artist_handle!(NonUniformImage);

/// The colorizer options of a mapping through `cmap` and `norm`.
fn colorizer(cmap: Colormap, norm: Box<dyn Norm>) -> ColorizerOptions {
    ColorizerOptions {
        cmap: Some(cmap),
        norm: Some(NormSpec::Norm(norm)),
        ..ColorizerOptions::default()
    }
}

/// The keyword arguments the image classes share, given as a dict of
/// matplotlib's keywords: `interpolation`, `interpolation_stage`, `origin`,
/// `filternorm`, `filterrad` and `resample`.
pub(super) fn image_options(
    cmap: Colormap,
    norm: Box<dyn Norm>,
    spec: &Bound<'_>,
) -> PyResult<BboxImageOptions> {
    let named = |key: &str| -> PyResult<Option<String>> { dict_value(spec, key) };
    Ok(BboxImageOptions {
        colorizer: colorizer(cmap, norm),
        interpolation: named("interpolation")?.as_deref().map(parse).transpose()?,
        interpolation_stage: named("interpolation_stage")?
            .as_deref()
            .map(parse)
            .transpose()?,
        origin: named("origin")?.as_deref().map(parse).transpose()?,
        filternorm: dict_value(spec, "filternorm")?,
        filterrad: dict_value(spec, "filterrad")?,
        resample: dict_value(spec, "resample")?,
        alpha: None,
    })
}

/// An image without data for an axes, mapped through `cmap` and `norm`,
/// with the options of the dict `spec` (see [`image_options`]) and its
/// `extent` (matplotlib's `AxesImage(ax, ...)`).
fn new_axes_image(cmap: Colormap, norm: Box<dyn Norm>, spec: Owned) -> PyResult<AxesImage> {
    let (options, placed) = Python::with_gil(|py| {
        let spec = spec.bind(py);
        let placed: Option<(f64, f64, f64, f64)> = dict_value(&spec, "extent")?;
        PyResult::Ok((image_options(cmap, norm, &spec)?, placed.map(extent)))
    })?;
    Ok(AxesImage::loose(ImageData::empty(), options, placed)?)
}

/// The interpolation of a non-uniform image by name.
fn nonuniform_interpolation(name: &str) -> PyResult<NonUniformInterpolation> {
    match name {
        "nearest" => Ok(NonUniformInterpolation::Nearest),
        "bilinear" => Ok(NonUniformInterpolation::Bilinear),
        _ => Err(PyErr::new(
            ExceptionKind::NotImplementedError,
            "Only nearest neighbor and bilinear interpolations are supported",
        )),
    }
}

/// An image without data on a non-uniform grid, mapped through `cmap`
/// and `norm` and interpolated by name (matplotlib's
/// `NonUniformImage(ax, ...)`).
fn new_nonuniform_image(
    cmap: Colormap,
    norm: Box<dyn Norm>,
    interpolation: String,
) -> PyResult<NonUniformImage> {
    Ok(NonUniformImage::loose(NonUniformOptions {
        colorizer: colorizer(cmap, norm),
        interpolation: nonuniform_interpolation(&interpolation)?,
    })?)
}

/// An image without cells on a non-uniform rectangular grid, mapped
/// through `cmap` and `norm` (matplotlib's `PcolorImage(ax)`).
fn new_pcolor_image(cmap: Colormap, norm: Box<dyn Norm>) -> PyResult<PcolorImage> {
    Ok(PcolorImage::loose(colorizer(cmap, norm))?)
}

/// An image without data for a figure, mapped through `cmap` and `norm`,
/// drawn with row 0 at `origin` (`None` for `image.origin`) and its
/// bottom-left corner `offset` pixels from the figure's (matplotlib's
/// `FigureImage(fig, ...)`).
fn new_figure_image(
    cmap: Colormap,
    norm: Box<dyn Norm>,
    origin: Option<String>,
    (xo, yo): (f64, f64),
) -> PyResult<FigureImage> {
    let options = FigimageOptions {
        xo,
        yo,
        colorizer: colorizer(cmap, norm),
        origin: origin.as_deref().map(parse::<Origin>).transpose()?,
        ..FigimageOptions::default()
    };
    Ok(FigureImage::loose(ImageData::empty(), options)?)
}

/// Stretches `image` over the box `(x0, y0, x1, y1)` in the coordinates of
/// `coords`, display pixels when `None` (matplotlib's
/// `BboxImage(TransformedBbox(bbox, coords))` and `BboxImage(bbox)`).
fn set_bbox(
    image: BboxImage,
    (x0, y0, x1, y1): (f64, f64, f64, f64),
    coords: Option<Transform>,
) -> PyResult<()> {
    image.set_bbox(
        Bbox::from_extents(x0, y0, x1, y1),
        coords.unwrap_or(Transform::Display),
    );
    Ok(())
}

/// The images whose data is one array.
enum ArrayImage {
    Axes(AxesImage),
    Figure(FigureImage),
    Bbox(BboxImage),
}

impl FromPy<'_> for ArrayImage {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let raw: RawHandle = object.extract()?;
        AxesImage::from_raw_handle(&raw)
            .map(Self::Axes)
            .or_else(|| FigureImage::from_raw_handle(&raw).map(Self::Figure))
            .or_else(|| BboxImage::from_raw_handle(&raw).map(Self::Bbox))
            .ok_or_else(|| PyErr::type_error("expected an image of one array"))
    }
}

/// Replaces the image's data.
fn set_data(image: ArrayImage, data: ImageArray) -> PyResult<()> {
    match image {
        ArrayImage::Axes(image) => {
            image.set_data(data.0);
        }
        ArrayImage::Figure(image) => {
            image.set_data(data.0);
        }
        ArrayImage::Bbox(image) => {
            image.set_data(data.0);
        }
    }
    Ok(())
}

/// The images on grids of their own.
enum GridImage {
    NonUniform(NonUniformImage),
    Pcolor(PcolorImage),
}

impl FromPy<'_> for GridImage {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let raw: RawHandle = object.extract()?;
        NonUniformImage::from_raw_handle(&raw)
            .map(Self::NonUniform)
            .or_else(|| PcolorImage::from_raw_handle(&raw).map(Self::Pcolor))
            .ok_or_else(|| PyErr::type_error("expected an image on a grid"))
    }
}

/// Replaces the grid and the data of a non-uniform image (pixel centres
/// `x` and `y`) or of a pcolor image (cell edges `x` and `y`).
fn set_grid_data(image: GridImage, x: Vec<f64>, y: Vec<f64>, data: ImageArray) -> PyResult<()> {
    match image {
        GridImage::NonUniform(image) => {
            image.set_data(&x, &y, data.0)?;
        }
        GridImage::Pcolor(image) => {
            image.set_data(x, y, data.0)?;
        }
    }
    Ok(())
}

/// Any image, by the kind of its data.
enum MappedImage {
    Array(ArrayImage),
    Grid(GridImage),
}

impl FromPy<'_> for MappedImage {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        match object.extract() {
            Ok(image) => Ok(Self::Array(image)),
            Err(_) => Ok(Self::Grid(object.extract()?)),
        }
    }
}

/// Applies the same change of colour mapping to whichever image kind.
macro_rules! on_image {
    ($image:expr, $method:ident($($arg:expr),*)) => {
        match $image {
            MappedImage::Array(ArrayImage::Axes(image)) => { image.$method($($arg),*); }
            MappedImage::Array(ArrayImage::Figure(image)) => { image.$method($($arg),*); }
            MappedImage::Array(ArrayImage::Bbox(image)) => { image.$method($($arg),*); }
            MappedImage::Grid(GridImage::NonUniform(image)) => { image.$method($($arg),*); }
            MappedImage::Grid(GridImage::Pcolor(image)) => { image.$method($($arg),*); }
        }
    };
}

/// Maps the image through `cmap` and `norm` from now on: a new norm when
/// `replaced`, else the image's norm with new parameters.
fn set_mapping(
    image: MappedImage,
    cmap: Colormap,
    norm: Box<dyn Norm>,
    replaced: bool,
) -> PyResult<()> {
    on_image!(&image, set_cmap(cmap));
    if replaced {
        on_image!(&image, set_norm(norm));
    } else {
        on_image!(&image, retune_norm(norm));
    }
    Ok(())
}

/// The limits of the image's norm in pltrs (Rust), which its draws scale
/// when they were unset; `(None, None)` for the kinds of images whose
/// norm the Python side keeps scaled.
fn mapped_clim(image: MappedImage) -> PyResult<(Option<f64>, Option<f64>)> {
    Ok(match image {
        MappedImage::Array(ArrayImage::Axes(image)) => image.clim(),
        MappedImage::Array(ArrayImage::Figure(image)) => image.clim(),
        MappedImage::Array(ArrayImage::Bbox(image)) => image.clim(),
        _ => (None, None),
    })
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new_axes_image", new_axes_image)?;
    module.add("new_nonuniform_image", new_nonuniform_image)?;
    module.add("new_pcolor_image", new_pcolor_image)?;
    module.add("new_figure_image", new_figure_image)?;
    module.add("set_bbox", set_bbox)?;
    module.add("set_data", set_data)?;
    module.add("set_grid_data", set_grid_data)?;
    module.add("set_mapping", set_mapping)?;
    module.add("mapped_clim", mapped_clim)
}
