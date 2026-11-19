//! Showing images in an axes: `imshow`, `matshow`, images on
//! non-uniform grids and images stretched over boxes.

use std::any::Any;
use std::rc::Rc;

use pltrs_core::{Bbox, Path, Point};

use super::children::DataUpdate;
use super::{Axes, AxesState, AxisName, TickPosition};
use crate::artist::{Artist, ArtistHandle, ClipPathSpec, RawHandle};
use crate::colors::{ColorizerOptions, impl_colorize};
use crate::image::{
    AxesImage, AxesImageArtist, BboxImage, BboxImageArtist, BboxImageOptions, CoreSettings, Extent,
    ImageAlpha, ImageCore, ImageData, ImageError, Interpolation, InterpolationStage,
    NonUniformImage, NonUniformImageArtist, NonUniformInterpolation, Origin, PcolorImageArtist,
    Region,
};
use crate::rc::Aspect;
use crate::spines::SpineSide;
use crate::text::Text;
use crate::ticker::{MaxNLocator, Nbins};
use crate::transforms::Transform;

/// Options of [`Axes::imshow`] (matplotlib's keyword arguments); unset
/// ones follow the `image.*` rc parameters. Its fields are set with
/// struct update syntax, the colormap, norm and limits of its colorizer
/// with the [`Colorize`](crate::colors::Colorize) builders:
///
/// ```
/// use pltrs::axes::ImshowOptions;
/// use pltrs::colors::{Colorize, colormaps};
/// use pltrs::image::Interpolation;
/// let options = ImshowOptions {
///     interpolation: Some(Interpolation::Nearest),
///     extent: Some([0.0, 10.0, 0.0, 3.0].into()),
///     ..ImshowOptions::default().cmap(colormaps::get("magma")?)
/// };
/// assert!(options.colorizer.cmap.is_some());
/// # Ok::<(), pltrs::colors::ColormapError>(())
/// ```
#[derive(Debug, Default)]
pub struct ImshowOptions {
    /// Colormap, norm and limits for scalar data.
    pub colorizer: ColorizerOptions,
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

impl_colorize!(ImshowOptions, NonUniformOptions);

/// Options of [`Axes::nonuniform_image`] (matplotlib's `NonUniformImage`
/// keyword arguments).
#[derive(Debug, Default)]
pub struct NonUniformOptions {
    /// Colormap, norm and limits for scalar data.
    pub colorizer: ColorizerOptions,
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

    /// The images of the axes placed in data coordinates, of every kind
    /// ([`AxesImage`], [`PcolorImage`](crate::image::PcolorImage),
    /// [`NonUniformImage`]), in the order they were added: matplotlib's
    /// `ax.images`, which lists `AxesImage` and its subclasses.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::{MeshCoords, PcolorFast, PcolorfastOptions};
    /// use pltrs::numeric::Grid;
    /// let ax = Figure::new().subplot();
    /// let c = Grid::from_fn(2, 2, |r, c| (r + c) as f64);
    /// let edges = (MeshCoords::from([0.0, 1.0, 4.0]), MeshCoords::from([0.0, 1.0, 4.0]));
    /// let drawn = ax.pcolorfast(Some(edges), c, PcolorfastOptions::default())?;
    /// assert!(matches!(drawn, PcolorFast::PcolorImage(_)));
    /// assert_eq!((ax.images().len(), ax.data_images().len()), (0, 1));
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    pub fn data_images(&self) -> Vec<RawHandle> {
        self.children_where(|artist| {
            let artist = artist as &dyn Any;
            artist.is::<AxesImageArtist>()
                || artist.is::<PcolorImageArtist>()
                || artist.is::<NonUniformImageArtist>()
        })
    }

    /// Shows `data` as an image on a regular grid (matplotlib's `imshow`):
    /// scalars are colour-mapped, RGB(A) colours drawn as they are. The
    /// axes aspect becomes `image.aspect` (equal) unless given, and the view
    /// limits fit the image when autoscaling.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::ImshowOptions;
    /// use pltrs::numeric::Grid;
    /// let ax = Figure::new().subplot();
    /// let data = Grid::from_fn(3, 4, |r, c| (r + c) as f64);
    /// let image = ax.imshow(data, ImshowOptions::default())?;
    /// assert_eq!(image.shape(), (3, 4));
    /// assert_eq!(ax.xlim(), (-0.5, 3.5));
    /// assert_eq!(ax.ylim(), (2.5, -0.5), "row 0 on top");
    /// # Ok::<(), pltrs::image::ImageError>(())
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
            colorizer: options.colorizer,
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
    /// use pltrs::prelude::*;
    /// use pltrs::axes::NonUniformOptions;
    /// use pltrs::image::Extent;
    /// use pltrs::numeric::Grid;
    /// let ax = Figure::new().subplot();
    /// let data = Grid::from_fn(2, 3, |r, c| (r * 3 + c) as f64);
    /// let image = ax.nonuniform_image(&[0.0, 1.0, 4.0], &[0.0, 2.0], data, NonUniformOptions::default())?;
    /// assert_eq!(image.extent(), Some(Extent::new(0.0, 4.0, 0.0, 2.0)));
    /// assert_eq!(ax.xlim(), (0.0, 1.0));
    /// # Ok::<(), pltrs::image::ImageError>(())
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
        let mut mappable = options.colorizer.into_mappable(&crate::rc::params())?;
        if let Some(field) = data.scalars() {
            field.autoscale(mappable.norm_mut());
        }
        let mut artist = NonUniformImageArtist::new(data, mappable, options.interpolation, x, y)?;
        artist.base_mut().transform = Some(self.trans_data());
        let raw = self.adopt_in_autoscale(artist, &DataUpdate::default());
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
    /// use pltrs::prelude::*;
    /// use pltrs::image::BboxImageOptions;
    /// use pltrs::numeric::Grid;
    /// use pltrs::transforms::{Bbox, Transform};
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
    /// # Ok::<(), pltrs::image::ImageError>(())
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
        self.add_region_image(data, Region::Transformed(bbox, coords), options)
    }

    /// Adds an image stretched over the box `artist` covers on the canvas
    /// whenever the image is drawn (matplotlib's
    /// `add_artist(BboxImage(artist.get_window_extent))`), such as a
    /// gradient behind a text; otherwise like [`Axes::add_bbox_image`].
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::image::BboxImageOptions;
    /// use pltrs::numeric::Grid;
    /// let ax = Figure::new().subplot();
    /// let text = ax.text(0.5, 0.5, "backed");
    /// let ramp = Grid::from_fn(1, 256, |_, c| c as f64);
    /// let image = ax.add_bbox_image_over(ramp, &text, BboxImageOptions::default())?;
    /// assert_eq!(image.window_extent(), text.window_extent());
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the colormap, norm or filter radius is invalid.
    pub fn add_bbox_image_over(
        &self,
        data: impl Into<ImageData>,
        artist: &impl ArtistHandle,
        options: BboxImageOptions,
    ) -> Result<BboxImage, ImageError> {
        self.add_region_image(data, Region::ExtentOf(artist.id()), options)
    }

    /// Adds an image filling `region` as `add_artist` adds artists.
    fn add_region_image(
        &self,
        data: impl Into<ImageData>,
        region: Region,
        options: BboxImageOptions,
    ) -> Result<BboxImage, ImageError> {
        let mut artist = BboxImageArtist::new(data, options, &crate::rc::params())?;
        artist.region = Some(region);
        Ok(BboxImage::from_raw(
            self.adopt(artist, &DataUpdate::default()),
        ))
    }

    /// The axis styling `matshow` and `spy` share: the title nudged up,
    /// x ticks labelled on `x_side` with marks on both sides, and integer
    /// tick locators (what matplotlib's `matshow` does after its
    /// `imshow`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// ax.style_matrix_axes(SpineSide::Top);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn style_matrix_axes(&self, x_side: SpineSide) {
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
        let extent = artist.extent();
        artist.base_mut().transform = Some(self.trans_data());
        let image = AxesImage::from_raw(self.adopt_in_autoscale(artist, &DataUpdate::default()));
        self.fit_image(image.raw(), extent);
        self.hide_grid_under_image();
        image
    }

    /// Turns the grid off when `pltrs.image.hide_grid` asks images to hide
    /// it: images lie below even the gridlines `axes.axisbelow` lowers.
    pub(crate) fn hide_grid_under_image(&self) {
        if crate::rc::params().pltrs_image_hide_grid {
            self.grid(false);
        }
    }

    /// Fits the axes to `image` covering `extent`: its sticky edges are
    /// the extent's sides, the data limits grow to include it and the view
    /// limits of the autoscaled axes become it (matplotlib's
    /// `AxesImage.set_extent`, which the image kinds of an axes share).
    pub(crate) fn fit_image(&self, image: &RawHandle, extent: Extent) {
        let Extent {
            left,
            right,
            bottom,
            top,
        } = extent;
        self.update_datalim(
            &[Point::new(left, bottom), Point::new(right, top)],
            true,
            true,
        );
        let autoscale = self.raw().with::<AxesState, _>(|axes| {
            (axes.xaxis.autoscale.enabled, axes.yaxis.autoscale.enabled)
        });
        if autoscale.0 || autoscale.1 {
            // matplotlib's `set_xlim` sets `viewLim`, which autoscales the
            // data so far first, the image's corners included but not its
            // sticky edges (`imshow` adds the image after): a log axis
            // keeps that view's limit for a side that is not positive.
            self.unstale_view();
        }
        image.with_base_mut(|base| base.sticky_edges = (vec![left, right], vec![bottom, top]));
        {
            let mut figure = self.raw().figure_state().borrow_mut();
            if autoscale.0 {
                figure.set_axis_limits(self.id(), AxisName::X, (left, right), None, true);
            }
            if autoscale.1 {
                figure.set_axis_limits(self.id(), AxisName::Y, (bottom, top), None, true);
            }
        }
        super::run_limit_callbacks(self.raw().figure_state());
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
    fn a_log_axis_keeps_the_autoscaled_limit_for_a_side_not_positive() {
        // matplotlib's `set_extent` sets the limits through `set_ylim`,
        // which autoscales the data waiting first (the image's corners
        // included) and keeps that view's upper limit for the image's -0.5.
        let ax = Figure::new().subplot();
        ax.plot([1.0, 2.0], [1.0, 2.0]).expect("finite data");
        ax.set_yscale("log").unwrap();
        ax.plot([1.0, 2.0], [1.0, 3.0]).expect("finite data");
        let upper = ImshowOptions {
            extent: Some(Extent::new(-0.5, 10.5, 8.5, -0.5)),
            ..ImshowOptions::default()
        };
        ax.imshow(Grid::filled(9, 11, 1.0), upper).unwrap();
        let (bottom, top) = ax.ylim();
        assert_eq!(bottom, 8.5);
        assert!((top - 9.459_972_458_265_13).abs() < 1e-12, "{top}");
    }

    #[test]
    fn images_hide_the_grid_only_when_asked() {
        let gridded_after_image = |hide: &str| {
            crate::rc::context(|| {
                crate::rc::set("axes.grid", "True").unwrap();
                crate::rc::set("pltrs.image.hide_grid", hide).unwrap();
                let ax = Figure::new().subplot();
                ax.imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())
                    .unwrap();
                ax.has_grid(AxisName::X, true) || ax.has_grid(AxisName::Y, true)
            })
        };
        assert!(gridded_after_image("False"), "matplotlib keeps the grid");
        assert!(!gridded_after_image("True"));
    }

    #[test]
    fn moving_an_image_extends_the_limits_and_moves_the_view() {
        let ax = Figure::new().subplot();
        let image = ax
            .imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())
            .unwrap();
        image.set_extent(Extent::new(20.0, 10.0, 5.0, 7.0));
        assert_eq!(image.extent(), Extent::new(20.0, 10.0, 5.0, 7.0));
        assert_eq!((ax.xlim(), ax.ylim()), ((20.0, 10.0), (5.0, 7.0)));
        assert_eq!(image.sticky_edges(), (vec![20.0, 10.0], vec![5.0, 7.0]));
        let limits = ax.data_lim().bbox;
        assert_eq!((limits.x0(), limits.x1()), (-0.5, 20.0), "both extents");
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
        let scaled =
            Transform::Affine(pltrs_core::Affine2D::scaling(2.0, 2.0)).then(ax.trans_data());
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
        ax.set_xlim(0.0, 100.0)
            .and_then(|ax| ax.set_ylim(0.0, 100.0))
            .expect("finite limits");
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
