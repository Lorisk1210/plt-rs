//! Showing images in an axes: `imshow`, `matshow`, images on
//! non-uniform grids and images stretched over boxes.

use std::rc::Rc;

use plt_core::{Bbox, Path, Point};

use super::children::DataUpdate;
use super::{Axes, AxesState, AxisName, TickPosition};
use crate::artist::{Artist, ArtistHandle, ClipPathSpec, RawHandle};
use crate::image::{
    AxesImage, AxesImageArtist, BboxImage, BboxImageArtist, BboxImageOptions, ColorizerOptions,
    CoreSettings, Extent, ImageAlpha, ImageCore, ImageData, ImageError, Interpolation,
    InterpolationStage, NonUniformImage, NonUniformImageArtist, NonUniformInterpolation, Origin,
};
use crate::rc::Aspect;
use crate::spines::SpineSide;
use crate::text::Text;
use crate::ticker::{MaxNLocator, Nbins};
use crate::transforms::Transform;

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
    /// The coordinates the extent is in (`None` = the axes' data
    /// coordinates); outside data coordinates, the axes aspect is only set
    /// when `aspect` is given (matplotlib's `transform`).
    pub transform: Option<Transform>,
    /// Clip the image to this path, in the coordinates of the transform,
    /// instead of to the axes (matplotlib's `clip_path=`).
    pub clip_path: Option<(Path, Transform)>,
}

/// Options of [`Axes::nonuniform_image`] (matplotlib's `NonUniformImage`
/// keyword arguments).
#[derive(Debug, Default)]
pub struct NonUniformOptions {
    /// Colormap, norm and limits for scalar data.
    pub colors: ColorizerOptions,
    /// How pixels between the centres are coloured.
    pub interpolation: NonUniformInterpolation,
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
    /// When the colormap, norm, aspect or filter radius is invalid, or the
    /// colour options are for data of several variates, which images do not
    /// take (matplotlib 3.11 maps it in `pcolormesh` only).
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
        let in_data = options
            .transform
            .as_ref()
            .is_none_or(|transform| transform.data_directions(self.id()) == (true, true));
        if let Some(aspect) = options.aspect.or(in_data.then_some(rc.image_aspect)) {
            self.set_aspect(aspect)?;
        }
        let (rows, cols) = core.data.shape();
        let extent = options
            .extent
            .unwrap_or_else(|| Extent::default_for(rows, cols, core.resampling.origin));
        let mut artist = AxesImageArtist::new(core, extent);
        artist.base_mut().alpha = uniform;
        let image = self.add_image(artist);
        if let Some(transform) = options.transform {
            image.set_transform(transform);
        }
        if let Some((path, transform)) = options.clip_path {
            // matplotlib's imshow clips to the axes patch only when no
            // clip path is given.
            image.raw().with_base_mut(|base| {
                base.clip_path = Some(ClipPathSpec::Path { path, transform });
                base.clip_box = None;
            });
        }
        Ok(image)
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

    /// Shows `data` with its pixel centres at the increasing coordinates
    /// `x` (one per column) and `y` (one per row), which need not be evenly
    /// spaced (matplotlib's `NonUniformImage`, `set_data` and `add_image`).
    /// The image fills the whole axes box: pixels beyond the grid repeat
    /// its edge.
    ///
    /// Like `add_image`, this leaves the data and view limits alone; set
    /// them, or call [`Axes::relim`], to frame the grid.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::NonUniformOptions;
    /// use plt::image::Extent;
    /// use plt::numeric::Grid;
    /// let ax = Figure::new().subplot();
    /// let data = Grid::from_fn(2, 3, |r, c| (r * 3 + c) as f64);
    /// let image = ax.nonuniform_image(&[0.0, 1.0, 4.0], &[0.0, 2.0], data, NonUniformOptions::default())?;
    /// assert_eq!(image.extent(), Some(Extent::new(0.0, 4.0, 0.0, 2.0)));
    /// assert_eq!(ax.xlim(), (0.0, 1.0));
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the grid does not fit the data or the colormap options are
    /// invalid.
    pub fn nonuniform_image(
        &self,
        x: &[f64],
        y: &[f64],
        data: impl Into<ImageData>,
        options: NonUniformOptions,
    ) -> Result<NonUniformImage, ImageError> {
        let data = data.into().normalized();
        let mut mappable = options.colors.into_mappable(&crate::rc::params())?;
        if let Some(field) = data.scalars() {
            mappable.norm_mut().autoscale_none(&field.masked_values());
        }
        let mut artist = NonUniformImageArtist::new(data, mappable, options.interpolation, x, y)?;
        artist.base_mut().transform = Some(self.trans_data());
        let raw = self.adopt(artist, &DataUpdate::default());
        // `add_image` leaves the image unclipped: it covers the axes box.
        raw.with_base_mut(|base| base.clip_box = None);
        Ok(NonUniformImage::from_raw(raw))
    }

    /// Adds an image stretched over `bbox`, a box in `coords` (matplotlib's
    /// `add_artist(BboxImage(TransformedBbox(bbox, coords)))`): the image
    /// is flipped along the directions in which the box is inverted. Like
    /// `add_artist`, this leaves the data limits alone and clips the image
    /// to the axes unless clipping is turned off.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::image::BboxImageOptions;
    /// use plt::numeric::Grid;
    /// use plt::transforms::{Bbox, Transform};
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let corner = Bbox::from_extents(0.1, 0.1, 0.3, 0.2);
    /// let image = ax.add_bbox_image(
    ///     Grid::filled(2, 3, 1.0),
    ///     corner,
    ///     Transform::Figure(fig.id()),
    ///     BboxImageOptions::default(),
    /// )?;
    /// assert_eq!(image.shape(), (2, 3));
    /// assert_eq!(ax.xlim(), (0.0, 1.0));
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the colormap, norm or filter radius is invalid.
    pub fn add_bbox_image(
        &self,
        data: impl Into<ImageData>,
        bbox: Bbox,
        coords: Transform,
        options: BboxImageOptions,
    ) -> Result<BboxImage, ImageError> {
        let mut artist = BboxImageArtist::new(data, options, &crate::rc::params())?;
        artist.region = Some((bbox, coords));
        Ok(BboxImage::from_raw(
            self.adopt(artist, &DataUpdate::default()),
        ))
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
    fn images_outside_data_coordinates_leave_the_aspect_alone() {
        let ax = Figure::new().subplot();
        let at = |transform: Transform, aspect: Option<Aspect>| ImshowOptions {
            transform: Some(transform),
            aspect,
            ..ImshowOptions::default()
        };
        let image = ax
            .imshow(Grid::filled(2, 2, 1.0), at(ax.trans_axes(), None))
            .unwrap();
        assert_eq!(ax.aspect(), Aspect::Auto);
        assert_eq!(
            image.raw().with_base(|b| b.transform.clone()),
            Some(ax.trans_axes())
        );
        ax.imshow(
            Grid::filled(2, 2, 1.0),
            at(ax.trans_axes(), Some(Aspect::Ratio(2.0))),
        )
        .unwrap();
        assert_eq!(ax.aspect(), Aspect::Ratio(2.0));
        let scaled = Transform::Affine(plt_core::Affine2D::scaling(2.0, 2.0)).then(ax.trans_data());
        ax.imshow(Grid::filled(2, 2, 1.0), at(scaled, None))
            .unwrap();
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
        let cmap = crate::colors::multivar::MultiColormap::get("BiPeak").unwrap();
        let bivariate = ImshowOptions {
            colors: ColorizerOptions::default()
                .multivariate(crate::image::MultivarColorizerOptions::new(cmap)),
            ..ImshowOptions::default()
        };
        assert_eq!(
            ax.imshow(Grid::filled(2, 2, 0.0), bivariate).err(),
            Some(ImageError::MultivariateColormap)
        );
    }

    #[test]
    fn a_clip_path_replaces_the_clip_to_the_axes() {
        let ax = Figure::new().subplot();
        let plain = ax
            .imshow(Grid::filled(2, 2, 0.0), ImshowOptions::default())
            .unwrap();
        assert_eq!(plain.raw().with_base(|b| b.clip_box), Some(ax.id()));
        let triangle = Path::polygon(vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
        ]);
        let clipped = ImshowOptions {
            clip_path: Some((triangle, ax.trans_data())),
            ..ImshowOptions::default()
        };
        let image = ax.imshow(Grid::filled(2, 2, 0.0), clipped).unwrap();
        image.raw().with_base(|base| {
            assert_eq!(base.clip_box, None);
            assert!(matches!(base.clip_path, Some(ClipPathSpec::Path { .. })));
        });
    }

    #[test]
    fn bbox_images_fill_their_box_in_display_pixels() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot();
        ax.set_xlim(0.0, 100.0).set_ylim(0.0, 100.0);
        let inverted = ax
            .add_bbox_image(
                Grid::filled(2, 2, 0.0),
                Bbox::from_extents(100.0, 100.0, 0.0, 0.0),
                ax.trans_data(),
                BboxImageOptions::default(),
            )
            .unwrap();
        let axes = fig.resolve_transform(&ax.trans_axes()).unwrap();
        let extent = inverted.window_extent().unwrap();
        assert_eq!(
            (extent.p0, extent.p1),
            (
                axes.transform_point(Point::new(1.0, 1.0)),
                axes.transform_point(Point::ORIGIN)
            ),
            "the box keeps its orientation"
        );
        assert_eq!(inverted.zorder(), 0.0);
        let in_figure = ax
            .add_bbox_image(
                Grid::filled(2, 2, 0.0),
                Bbox::from_extents(0.25, 0.5, 0.5, 1.0),
                Transform::Figure(fig.id()),
                BboxImageOptions::default(),
            )
            .unwrap();
        assert_eq!(
            in_figure.window_extent(),
            Some(Bbox::from_extents(100.0, 150.0, 200.0, 300.0))
        );
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
