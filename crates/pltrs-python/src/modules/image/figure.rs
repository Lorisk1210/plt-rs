//! Images drawn in figure pixels (matplotlib's `Figure.figimage` and
//! `FigureImage`).

use pltrs::colors::{ColorizerOptions, Colormap, Norm, NormSpec};
use pltrs::figure::Figure;
use pltrs::image::{FigimageOptions, FigureImage};

use super::{ImageArray, parse};
use crate::binding::prelude::*;

/// `figimage`: `data` drawn unresampled in `figure`'s pixels with its
/// bottom-left corner at `(xo, yo)`, colour-mapped through `cmap` and
/// `norm`, row 0 at the `origin` (`"upper"`, `"lower"`, or
/// `image.origin` when `None`); the Python side resizes the figure first
/// when asked to, as matplotlib's `figimage` does.
fn figimage(
    figure: Figure,
    data: ImageArray,
    cmap: Colormap,
    norm: Box<dyn Norm>,
    (xo, yo): (f64, f64),
    origin: Option<String>,
) -> PyResult<FigureImage> {
    let options = FigimageOptions {
        xo,
        yo,
        colorizer: ColorizerOptions {
            cmap: Some(cmap),
            norm: Some(NormSpec::Norm(norm)),
            ..ColorizerOptions::default()
        },
        origin: origin.as_deref().map(parse).transpose()?,
        alpha: None,
        resize: false,
    };
    Ok(figure.figimage(data.0, options)?)
}

/// The figure pixels the image covers, `(left, right, bottom, top)`
/// (matplotlib's `FigureImage.get_extent`).
fn figure_image_extent(image: FigureImage) -> PyResult<(f64, f64, f64, f64)> {
    let extent = image.extent();
    Ok((extent.left, extent.right, extent.bottom, extent.top))
}

/// The images `figimage` drew in `figure`, in the order they were made
/// (matplotlib's `Figure.images`).
fn figure_images(figure: Figure) -> PyResult<Vec<FigureImage>> {
    Ok(figure.images())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("figimage", figimage)?;
    module.add("figure_image_extent", figure_image_extent)?;
    module.add("figure_images", figure_images)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::colors::Normalize;
    use pltrs::image::ImageData;
    use pltrs::numeric::Grid;

    #[test]
    fn figure_images_are_placed_in_pixels() {
        let fig = Figure::new();
        let cmap = pltrs::colors::colormaps::get("viridis").expect("a colormap");
        let image = figimage(
            fig.clone(),
            ImageArray(ImageData::from(Grid::filled(2, 3, 1.0))),
            cmap,
            Box::new(Normalize::new(0.0, 1.0)),
            (10.0, 20.0),
            Some("lower".into()),
        )
        .expect("valid options");
        assert_eq!(
            figure_image_extent(image.clone()).expect("an extent"),
            (9.5, 12.5, 19.5, 21.5)
        );
        assert_eq!(figure_images(fig).expect("images"), [image]);
    }
}
