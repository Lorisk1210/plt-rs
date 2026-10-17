//! Showing images in an axes: `imshow` and `matshow`.

use std::rc::Rc;

use plt_core::Point;

use super::children::DataUpdate;
use super::{Axes, AxesState, AxisName, TickPosition};
use crate::artist::{Artist, ArtistHandle, RawHandle};
use crate::image::{
    AxesImage, AxesImageArtist, ColorizerOptions, CoreSettings, Extent, ImageAlpha, ImageCore,
    ImageData, ImageError, Interpolation, InterpolationStage, Origin,
};
use crate::rc::Aspect;
use crate::spines::SpineSide;
use crate::text::Text;
use crate::ticker::{MaxNLocator, Nbins};

/// Options of [`Axes::imshow`] (matplotlib's keyword arguments); unset
/// ones follow the `image.*` rc parameters.
#[derive(Debug, Default)]
pub struct ImshowOptions {
    /// Colormap, norm and limits for scalar data.
    pub colors: ColorizerOptions,
    /// The axes aspect to set (`None` = `image.aspect`).
    pub aspect: Option<Aspect>,
    /// Interpolation when resampling (`None` = `image.interpolation`).
    pub interpolation: Option<Interpolation>,
    /// Whether data or colours are resampled
    /// (`None` = `image.interpolation_stage`).
    pub interpolation_stage: Option<InterpolationStage>,
    /// Opacity: uniform or per pixel.
    pub alpha: Option<ImageAlpha>,
    /// Where row 0 of the array is drawn (`None` = `image.origin`).
    pub origin: Option<Origin>,
    /// The data rectangle covered (`None` = pixel centres on indices).
    pub extent: Option<Extent>,
    /// Normalise the kernel weights (`None` = `true`).
    pub filternorm: Option<bool>,
    /// Kernel radius of `sinc`, `lanczos` and `blackman` (`None` = 4).
    pub filterrad: Option<f64>,
    /// Agg's full resampling when downsampling (`None` = `image.resample`).
    pub resample: Option<bool>,
}

impl Axes {
    /// The images of the axes in the order they were added (matplotlib's
    /// `ax.images`).
    pub fn images(&self) -> Vec<AxesImage> {
        self.children_of::<AxesImageArtist>()
            .into_iter()
            .map(AxesImage::from_raw)
            .collect()
    }

    /// Shows `data` as an image on a regular grid (matplotlib's `imshow`):
    /// scalars are colour-mapped, RGB(A) colours drawn as they are. The
    /// axes aspect becomes `image.aspect` (equal) unless given, and the view
    /// limits fit the image when autoscaling.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::ImshowOptions;
    /// use plt::numeric::Grid;
    /// let ax = Figure::new().subplot();
    /// let data = Grid::from_fn(3, 4, |r, c| (r + c) as f64);
    /// let image = ax.imshow(data, ImshowOptions::default())?;
    /// assert_eq!(image.shape(), (3, 4));
    /// assert_eq!(ax.xlim(), (-0.5, 3.5));
    /// assert_eq!(ax.ylim(), (2.5, -0.5), "row 0 on top");
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the colormap, norm, aspect or filter radius is invalid.
    pub fn imshow(
        &self,
        data: impl Into<ImageData>,
        options: ImshowOptions,
    ) -> Result<AxesImage, ImageError> {
        let rc = crate::rc::params();
        let settings = CoreSettings {
            colors: options.colors,
            interpolation: options.interpolation,
            stage: options.interpolation_stage,
            origin: options.origin,
            filternorm: options.filternorm,
            filterrad: options.filterrad,
            resample: options.resample.unwrap_or(rc.image_resample),
            alpha: options.alpha,
        };
        let (core, uniform) = ImageCore::build(data, settings, &rc)?;
        self.set_aspect(options.aspect.unwrap_or(rc.image_aspect))?;
        let (rows, cols) = core.data.shape();
        let extent = options
            .extent
            .unwrap_or_else(|| Extent::default_for(rows, cols, core.resampling.origin));
        let mut artist = AxesImageArtist::new(core, extent);
        artist.base_mut().alpha = uniform;
        Ok(self.add_image(artist))
    }

    /// Shows a matrix with row 0 on top, square cells, nearest-neighbour
    /// sampling and the x axis on top (matplotlib's `matshow`); unset
    /// options default to those choices rather than the rc parameters.
    ///
    /// # Errors
    ///
    /// As [`Axes::imshow`].
    pub fn matshow(
        &self,
        data: impl Into<ImageData>,
        options: ImshowOptions,
    ) -> Result<AxesImage, ImageError> {
        let options = ImshowOptions {
            origin: options.origin.or(Some(Origin::Upper)),
            interpolation: options.interpolation.or(Some(Interpolation::Nearest)),
            aspect: options.aspect.or(Some(Aspect::Equal)),
            ..options
        };
        let image = self.imshow(data, options)?;
        self.style_matrix_axes(SpineSide::Top);
        Ok(image)
    }

    /// The axis styling `matshow` and `spy` share: the title nudged up,
    /// x ticks labelled on `x_side` with marks on both sides, and integer
    /// tick locators.
    pub(crate) fn style_matrix_axes(&self, x_side: SpineSide) {
        let title = self.raw().with::<AxesState, _>(|axes| axes.titles[1]);
        let title = Text::from_raw(RawHandle::new(Rc::clone(self.raw().figure_state()), title));
        let x = title.position().x;
        title.set_position(Point::new(x, 1.05));
        let xaxis = self.xaxis();
        xaxis
            .tick_to(x_side)
            .and_then(|axis| axis.set_ticks_position(TickPosition::Both))
            .expect("top, bottom and both are positions of the x axis");
        for name in [AxisName::X, AxisName::Y] {
            let locator = MaxNLocator::new(Nbins::Fixed(9))
                .with_steps(&[1.0, 2.0, 5.0, 10.0])
                .expect("1, 2, 5, 10 are valid steps")
                .with_integer(true);
            self.axis_of(name).set_major_locator(locator);
        }
    }

    /// Adds an image in data coordinates (matplotlib's `set_extent` and
    /// `add_image`): the extent extends the data limits and becomes the
    /// sticky edges, and each autoscaled axis is fitted to it.
    pub(crate) fn add_image(&self, mut artist: AxesImageArtist) -> AxesImage {
        let Extent {
            left,
            right,
            bottom,
            top,
        } = artist.extent;
        let base = artist.base_mut();
        base.transform = Some(self.trans_data());
        base.sticky_edges = (vec![left, right], vec![bottom, top]);
        let update = DataUpdate {
            points: vec![Point::new(left, bottom), Point::new(right, top)],
            update: (true, true),
            autoscale: (false, false),
        };
        let raw = self.adopt(artist, &update);
        let autoscale = self.raw().with::<AxesState, _>(|axes| {
            (axes.xaxis.autoscale.enabled, axes.yaxis.autoscale.enabled)
        });
        let mut figure = self.raw().figure_state().borrow_mut();
        if autoscale.0 {
            figure.set_axis_limits(self.id(), AxisName::X, (left, right), None, true);
        }
        if autoscale.1 {
            figure.set_axis_limits(self.id(), AxisName::Y, (bottom, top), None, true);
        }
        drop(figure);
        AxesImage::from_raw(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::numeric::Grid;

    #[test]
    fn extents_set_limits_and_sticky_edges() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let image = ax
            .imshow(
                Grid::filled(2, 2, 1.0),
                ImshowOptions {
                    extent: Some(Extent::new(8.0, 0.0, 0.0, 4.0)),
                    ..ImshowOptions::default()
                },
            )
            .unwrap();
        assert_eq!(ax.xlim(), (8.0, 0.0), "a reversed extent inverts x");
        assert_eq!(ax.ylim(), (0.0, 4.0));
        assert_eq!(image.sticky_edges(), (vec![8.0, 0.0], vec![0.0, 4.0]));
        assert_eq!(ax.aspect(), Aspect::Ratio(1.0));
    }

    #[test]
    fn later_data_autoscales_within_the_image_edges() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.imshow(Grid::filled(4, 4, 1.0), ImshowOptions::default())
            .unwrap();
        ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        assert_eq!(ax.xlim(), (-0.5, 3.5));
    }

    #[test]
    fn invalid_options_are_reported() {
        let ax = Figure::new().subplot();
        let bad_radius = ImshowOptions {
            filterrad: Some(0.0),
            ..ImshowOptions::default()
        };
        assert_eq!(
            ax.imshow(Grid::filled(1, 1, 0.0), bad_radius).err(),
            Some(ImageError::FilterRadius(0.0))
        );
        let bad_aspect = ImshowOptions {
            aspect: Some(Aspect::Ratio(-1.0)),
            ..ImshowOptions::default()
        };
        assert!(matches!(
            ax.imshow(Grid::filled(1, 1, 0.0), bad_aspect),
            Err(ImageError::Aspect(_))
        ));
    }

    #[test]
    fn matshow_puts_the_x_axis_on_top() {
        let ax = Figure::new().subplot();
        let image = ax
            .matshow(Grid::filled(2, 3, 0.0), ImshowOptions::default())
            .unwrap();
        assert_eq!(image.interpolation(), Interpolation::Nearest);
        ax.raw().with::<AxesState, _>(|axes| {
            let style = &axes.xaxis.major_style;
            assert!(style.labels.second && !style.labels.first);
            assert!(style.ticks.first && style.ticks.second);
        });
    }
}
