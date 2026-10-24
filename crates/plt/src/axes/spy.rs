//! The sparsity pattern of a matrix (matplotlib's `spy`).

use super::image::ImshowOptions;
use super::{Axes, AxisName};
use crate::artist::Artist;
use crate::colors::{ColorizerOptions, Colormap, NoNorm, Norm, impl_colorize};
use crate::image::{AxesImage, ImageData, ImageError, Interpolation, Origin, ScalarField};
use crate::lines::{Line2D, LineArtist, LineProps, LineStyle};
use crate::markers::{Marker, MarkerSymbol};
use crate::numeric::Grid;
use crate::rc::Aspect;
use crate::spines::SpineSide;

/// Options of [`Axes::spy`].
#[derive(Clone, Debug)]
pub struct SpyOptions {
    /// Entries with an absolute value above this count as non-zero.
    pub precision: f64,
    /// Draw non-zero entries as this marker instead of as an image.
    pub marker: Option<Marker>,
    /// Draw non-zero entries as markers of this size (points) instead of
    /// as an image.
    pub markersize: Option<f64>,
    /// The axes aspect.
    pub aspect: Aspect,
    /// Where row 0 is drawn.
    pub origin: Origin,
    /// Colormap, norm and limits of the image (default: white for zero
    /// and black otherwise, through the identity norm), as matplotlib's
    /// `spy` hands them to `imshow`.
    pub colorizer: ColorizerOptions,
    /// Style of the markers.
    pub line: LineProps,
}

impl Default for SpyOptions {
    fn default() -> Self {
        Self {
            precision: 0.0,
            marker: None,
            markersize: None,
            aspect: Aspect::Equal,
            origin: Origin::Upper,
            colorizer: ColorizerOptions::default(),
            line: LineProps::default(),
        }
    }
}

impl_colorize!(SpyOptions);

/// The colours of a `spy` image: `colorizer` with matplotlib's binary
/// colormap when it names none, and the identity norm when it gives
/// neither a norm nor limits.
fn spy_colorizer(colorizer: ColorizerOptions) -> Result<ColorizerOptions, ImageError> {
    let cmap = match colorizer.cmap {
        Some(cmap) => cmap,
        None => Colormap::listed("binary", vec!["w", "k"])?,
    };
    let scaled = colorizer.norm.is_some() || colorizer.vmin.is_some() || colorizer.vmax.is_some();
    let norm: Option<Box<dyn Norm>> = if scaled {
        colorizer.norm
    } else {
        Some(Box::new(NoNorm::default()))
    };
    Ok(ColorizerOptions {
        cmap: Some(cmap),
        norm,
        ..colorizer
    })
}

/// What [`Axes::spy`] drew.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Spy {
    /// The pattern as a two-colour image.
    Image(AxesImage),
    /// The non-zero entries as markers.
    Markers(Line2D),
}

impl Axes {
    /// Plots which entries of `z` are non-zero (matplotlib's `spy`): as a
    /// black-on-white image, or as markers when a marker or marker size is
    /// given. Rows run downwards with the x axis on top for
    /// [`Origin::Upper`].
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::{Spy, SpyOptions};
    /// use plt::numeric::Grid;
    /// let ax = Figure::new().subplot();
    /// let z = Grid::from_vec(2, 2, vec![1.0, 0.0, 0.0, 2.0]).unwrap();
    /// let markers = SpyOptions { markersize: Some(4.0), ..SpyOptions::default() };
    /// assert!(matches!(ax.spy(&z, markers)?, Spy::Markers(_)));
    /// assert_eq!(ax.ylim(), (1.5, -0.5));
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the aspect is invalid.
    pub fn spy(&self, z: &Grid<f64>, options: SpyOptions) -> Result<Spy, ImageError> {
        let nonzero = z.map(|v| v.abs() > options.precision);
        let spy = if options.marker.is_none() && options.markersize.is_none() {
            let imshow = ImshowOptions {
                colorizer: spy_colorizer(options.colorizer)?,
                interpolation: Some(Interpolation::Nearest),
                aspect: Some(options.aspect),
                origin: Some(options.origin),
                ..ImshowOptions::default()
            };
            let data = ImageData::from(ScalarField::booleans(&nonzero));
            Spy::Image(self.imshow(data, imshow)?)
        } else {
            Spy::Markers(self.spy_markers(&nonzero, &options)?)
        };
        let x_side = match options.origin {
            Origin::Upper => SpineSide::Top,
            Origin::Lower => SpineSide::Bottom,
        };
        self.style_matrix_axes(x_side);
        Ok(spy)
    }

    /// The marker variant of `spy`: one marker per non-zero entry, the
    /// view fitted to the matrix.
    fn spy_markers(
        &self,
        nonzero: &Grid<bool>,
        options: &SpyOptions,
    ) -> Result<Line2D, ImageError> {
        let (rows, cols) = nonzero.shape();
        let (x, y): (Vec<f64>, Vec<f64>) = (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (r, c)))
            .filter(|&(r, c)| nonzero[(r, c)])
            .map(|(r, c)| (c as f64, r as f64))
            .unzip();
        let mut line = LineArtist::new(x, y, crate::rc::params());
        line.linestyle = LineStyle::None;
        line.marker = options
            .marker
            .clone()
            .unwrap_or(Marker::Symbol(MarkerSymbol::Square));
        line.markersize = options.markersize.unwrap_or(10.0);
        options.line.clone().apply(&mut line);
        line.base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data());
        let line = self.add_line_artist(line);
        let (rows, cols) = (rows as f64, cols as f64);
        let (bottom, top) = match options.origin {
            Origin::Upper => (rows - 0.5, -0.5),
            Origin::Lower => (-0.5, rows - 0.5),
        };
        self.set_limits(AxisName::X, -0.5, cols - 0.5)
            .and_then(|ax| ax.set_limits(AxisName::Y, bottom, top))
            .expect("a matrix shape gives finite limits");
        self.set_aspect(options.aspect)?;
        Ok(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn diagonal() -> Grid<f64> {
        Grid::from_fn(3, 4, |r, c| if r == c { 0.5 } else { 0.0 })
    }

    #[test]
    fn image_mode_shows_the_pattern() {
        let ax = Figure::new().subplot();
        let Spy::Image(image) = ax.spy(&diagonal(), SpyOptions::default()).unwrap() else {
            unreachable!("no marker requested")
        };
        assert_eq!(image.shape(), (3, 4));
        assert_eq!(
            image.clim(),
            (Some(0.0), Some(1.0)),
            "autoscaled, but unused"
        );
        assert_eq!(ax.xlim(), (-0.5, 3.5));
    }

    #[test]
    fn marker_mode_respects_precision_and_origin() {
        let ax = Figure::new().subplot();
        let options = SpyOptions {
            precision: 0.6,
            marker: Some(Marker::Symbol(MarkerSymbol::Circle)),
            origin: Origin::Lower,
            ..SpyOptions::default()
        };
        let Spy::Markers(line) = ax.spy(&diagonal(), options).unwrap() else {
            unreachable!("a marker was requested")
        };
        assert!(line.xdata().is_empty(), "nothing exceeds the precision");
        assert_eq!(ax.ylim(), (-0.5, 2.5));
        let ax = Figure::new().subplot();
        let options = SpyOptions {
            markersize: Some(3.0),
            ..SpyOptions::default()
        };
        let Spy::Markers(line) = ax.spy(&diagonal(), options).unwrap() else {
            unreachable!("a marker size was requested")
        };
        assert_eq!(line.xdata(), vec![0.0, 1.0, 2.0]);
        assert_eq!(line.markersize(), 3.0);
        assert_eq!(line.linestyle(), LineStyle::None);
    }
}
