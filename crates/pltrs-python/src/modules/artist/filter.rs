//! Image filters (matplotlib's `Artist.set_agg_filter`): a Python function
//! taking what an artist drew, as an `(H, W, 4)` array of fractions, and the
//! resolution in dpi, and returning the image to draw in its place with
//! the offset of its bottom-left corner in pixels, `(image, ox, oy)`.

use pltrs::artist::{AggFilter, ArtistHandle, FilterImage, Filtered, RawHandle};

use crate::binding::prelude::*;

/// The filter running the Python function `function`. When the function
/// raises, its exception raises from the native call drawing the figure,
/// and the artist is drawn unfiltered meanwhile.
fn python_filter(function: Callback) -> AggFilter {
    AggFilter::new(move |image, dpi| {
        let unfiltered = || Filtered {
            image: image.clone(),
            offset: (0.0, 0.0),
        };
        let Ok(array) = image_array(image) else {
            return unfiltered();
        };
        function
            .call::<(NdArray, f64, f64)>((array, dpi))
            .and_then(|(array, ox, oy)| {
                Some(Filtered {
                    image: filter_image(&array).ok()?,
                    offset: (ox, oy),
                })
            })
            .unwrap_or_else(unfiltered)
    })
}

/// The image as an `(H, W, 4)` array, top row first.
fn image_array(image: &FilterImage) -> PyResult<NdArray> {
    let data = image.pixels().iter().flatten().copied().collect();
    NdArray::with_shape(data, vec![image.height(), image.width(), 4])
}

/// The image of an `(H, W, 4)` array, top row first.
fn filter_image(array: &NdArray) -> PyResult<FilterImage> {
    let &[height, width, 4] = &array.shape[..] else {
        return Err(PyErr::value_error(format!(
            "an agg filter returns an (H, W, 4) image, not one of shape {:?}",
            array.shape
        )));
    };
    let pixels = array.data.as_chunks::<4>().0.to_vec();
    FilterImage::new(width, height, pixels).map_err(|err| PyErr::value_error(err.to_string()))
}

/// Filters what `artist` draws through the Python function `function`, or
/// through none.
fn set_agg_filter(artist: RawHandle, function: Option<Callback>) -> PyResult<()> {
    artist.set_agg_filter(function.map(python_filter));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_agg_filter", set_agg_filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::with_python;

    #[test]
    fn images_cross_as_arrays_and_back() {
        with_python(|_| {
            let image = FilterImage::new(2, 1, vec![[0.0, 0.25, 0.5, 1.0], [1.0; 4]]).unwrap();
            let array = image_array(&image).unwrap();
            assert_eq!(array.shape, [1, 2, 4]);
            assert_eq!(filter_image(&array).unwrap(), image);
            let flat = NdArray::with_shape(vec![0.0; 8], vec![2, 4]).unwrap();
            assert!(filter_image(&flat).is_err());
        });
    }
}
