//! The current image and colormaps: what `colorbar`, `clim` and the
//! colormap functions act on (matplotlib's `gci`, `sci`, `colorbar`,
//! `clim`, `get_cmap`, `set_cmap` and `viridis()`, `gray()`, ...).

use super::error::PyplotError;
use super::registry::{gca, gcf};
use crate::axes::{Axes, ColorizingArtist, PlotError};
use crate::colorbar::{Colorbar, ColorbarOptions, Mappable};
use crate::colors::{Colormap, ColormapError, colormaps};

/// Makes `image`, just made by a plotting method of `ax`, the current
/// image of `ax` (the `sci(__ret)` of matplotlib's generated wrappers).
pub(super) fn make_current(ax: &Axes, image: impl Into<ColorizingArtist>) {
    ax.set_current_image(image)
        .expect("plotting methods add their artists to the axes they are called on");
}

/// The current image of the current figure: the colour-mapped artist made
/// last by `imshow`, `pcolormesh`, `contourf`, `scatter`, ... in the
/// current axes or else in the latest axes (matplotlib's `gci`).
pub fn gci() -> Option<ColorizingArtist> {
    gcf().current_image()
}

/// Makes `image` the current image of the current axes (matplotlib's
/// `sci`).
///
/// # Errors
///
/// [`PlotError::NotInAxes`] when the artist is not drawn in the current
/// axes.
pub fn sci(image: impl Into<ColorizingArtist>) -> Result<(), PlotError> {
    gca().set_current_image(image).map(|_| ())
}

/// Sets the data values mapped to the ends of the colormap of the current
/// image; `None` leaves a limit unchanged (matplotlib's `clim`). Its
/// colorbars follow.
///
/// # Errors
///
/// [`PyplotError::NoCurrentImage`] without a current image.
pub fn clim(vmin: Option<f64>, vmax: Option<f64>) -> Result<(), PyplotError> {
    gci()
        .ok_or(PyplotError::NoCurrentImage)?
        .set_clim(vmin, vmax);
    Ok(())
}

/// Adds a colorbar for `mappable`, or for the current image when `None`,
/// to the current figure (matplotlib's `colorbar`).
///
/// # Errors
///
/// [`PyplotError::NoCurrentImage`] without a mappable, or the colorbar's
/// own errors.
pub fn colorbar(
    mappable: Option<Mappable<'_>>,
    options: ColorbarOptions,
) -> Result<Colorbar, PyplotError> {
    let current;
    let mappable = if let Some(mappable) = mappable {
        mappable
    } else {
        current = gci().ok_or(PyplotError::NoCurrentImage)?;
        Mappable::from(&current)
    };
    Ok(gcf().colorbar(mappable, options)?)
}

/// The colormap `name`, or `image.cmap` when `None`, resampled to `lut`
/// entries if given (matplotlib's `get_cmap`).
///
/// # Errors
///
/// When no colormap has that name or `lut` is zero.
pub fn get_cmap(name: Option<&str>, lut: Option<usize>) -> Result<Colormap, ColormapError> {
    let cmap = colormaps::get_cmap(name)?;
    match lut {
        Some(lut) => cmap.resampled(lut),
        None => Ok(cmap),
    }
}

/// Makes `name` the default colormap (`image.cmap`) and the colormap of
/// the current image, if any (matplotlib's `set_cmap`); its colorbars
/// follow.
///
/// # Errors
///
/// [`PyplotError::Colormap`] when no colormap has that name.
pub fn set_cmap(name: &str) -> Result<(), PyplotError> {
    let cmap = colormaps::get(name)?;
    crate::rc::update(|rc| name.clone_into(&mut rc.image_cmap));
    if let Some(image) = gci() {
        image.set_cmap(cmap);
    }
    Ok(())
}

/// Defines one `set_cmap` shortcut per colormap name.
macro_rules! colormap_shortcuts {
    ($($name:ident),* $(,)?) => {$(
        #[doc = concat!(
            "Makes `", stringify!($name), "` the default colormap and that of the current ",
            "image (matplotlib's `", stringify!($name), "()`, which is `set_cmap(\"",
            stringify!($name), "\")`).",
        )]
        ///
        /// # Errors
        ///
        /// As [`set_cmap`], when the colormap was unregistered.
        pub fn $name() -> Result<(), PyplotError> {
            set_cmap(stringify!($name))
        }
    )*};
}

colormap_shortcuts!(
    autumn,
    bone,
    cool,
    copper,
    flag,
    gray,
    hot,
    hsv,
    inferno,
    jet,
    magma,
    nipy_spectral,
    pink,
    plasma,
    prism,
    spring,
    summer,
    viridis,
    winter,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::{ImshowOptions, MappedColors, ScatterOptions};
    use crate::contour::{ContourOptions, QuadField};
    use crate::numeric::Grid;
    use crate::pyplot::{close_all, contourf, imshow, scatter_with};

    fn ramp() -> Grid<f64> {
        Grid::from_fn(4, 4, |row, col| (row + col) as f64)
    }

    #[test]
    fn colorbars_and_limits_need_a_current_image() {
        close_all();
        assert!(gci().is_none());
        assert_eq!(clim(Some(0.0), None), Err(PyplotError::NoCurrentImage));
        assert!(matches!(
            colorbar(None, ColorbarOptions::default()),
            Err(PyplotError::NoCurrentImage)
        ));
        let image = imshow(ramp(), ImshowOptions::default()).unwrap();
        assert!(matches!(gci(), Some(ColorizingArtist::Image(current)) if current == image));
        clim(Some(1.0), Some(5.0)).unwrap();
        assert_eq!(image.clim(), (Some(1.0), Some(5.0)));
        crate::rc::context(|| {
            hot().unwrap();
            assert_eq!(image.cmap().name(), "hot");
        });
        assert!(colorbar(None, ColorbarOptions::default()).is_ok());
        close_all();
    }

    #[test]
    fn colormaps_change_the_default_and_the_current_image() {
        close_all();
        crate::rc::context(|| {
            let points = scatter_with(
                vec![0.0, 1.0],
                vec![1.0, 0.0],
                ScatterOptions {
                    colors: MappedColors::new(vec![0.0, 1.0]).into(),
                    ..ScatterOptions::default()
                },
            )
            .unwrap();
            gray().unwrap();
            assert_eq!(crate::rc::params().image_cmap, "gray");
            clim(None, Some(4.0)).unwrap();
            assert_eq!(points.clim(), (Some(0.0), Some(4.0)));
            assert!(matches!(
                set_cmap("no such map"),
                Err(PyplotError::Colormap(_))
            ));
            let field = QuadField::new(ramp());
            let contours = contourf(&field, ContourOptions::default()).unwrap();
            viridis().unwrap();
            assert_eq!(crate::rc::params().image_cmap, "viridis");
            assert_eq!(
                contours.cmap().map(|c| c.name().to_owned()),
                Some("viridis".to_owned()),
                "contour sets are recoloured"
            );
            clim(Some(-1.0), None).unwrap();
            assert_eq!(contours.norm_limits().0, Some(-1.0));
            sci(&points).unwrap();
            assert!(matches!(gci(), Some(ColorizingArtist::Collection(c)) if c == points));
        });
        close_all();
    }

    #[test]
    fn get_cmap_resamples() {
        assert_eq!(get_cmap(Some("viridis"), Some(8)).unwrap().n(), 8);
        assert_eq!(
            get_cmap(None, None).unwrap().name(),
            crate::rc::params().image_cmap
        );
        assert!(get_cmap(Some("nope"), None).is_err());
    }
}
