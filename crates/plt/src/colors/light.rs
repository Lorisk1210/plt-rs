//! Shaded relief: illumination of a surface by a distant light and its
//! blending with colours (matplotlib's `colors.LightSource`).

use std::fmt;
use std::rc::Rc;

use plt_core::Rgba;

use super::{
    ColorError, ColorizerOptions, ColormapError, NormError, Normalize, ScalarMappable, hsv_to_rgb,
    impl_colorize, rgb_to_hsv,
};
use crate::numeric::Grid;

/// How elevations become illumination (matplotlib's `vert_exag`, `dx`,
/// `dy` and `fraction` arguments).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Relief {
    /// Factor applied to the elevations before taking slopes.
    pub vert_exag: f64,
    /// Spacing of the columns.
    pub dx: f64,
    /// Spacing of the rows.
    pub dy: f64,
    /// Contrast of the illumination: above 1, intermediate values move
    /// towards full light or shadow.
    pub fraction: f64,
}

impl Default for Relief {
    fn default() -> Self {
        Self {
            vert_exag: 1.0,
            dx: 1.0,
            dy: 1.0,
            fraction: 1.0,
        }
    }
}

/// How [`LightSource::shade_rgb`] combines colours with the illumination
/// (matplotlib's `blend_mode`).
#[derive(Clone)]
pub enum BlendMode {
    /// Saturation and value move towards those of full shadow or light
    /// (`"hsv"`).
    Hsv,
    /// Soft light with the "pegtop" formula (`"soft"`).
    SoftLight,
    /// Overlay (`"overlay"`).
    Overlay,
    /// A function of a pixel's RGB colour and its illumination.
    Custom(Rc<dyn Fn([f64; 3], f64) -> [f64; 3]>),
}

impl fmt::Debug for BlendMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hsv => f.write_str("Hsv"),
            Self::SoftLight => f.write_str("SoftLight"),
            Self::Overlay => f.write_str("Overlay"),
            Self::Custom(_) => f.write_str("Custom(..)"),
        }
    }
}

/// Options of [`LightSource::shade`].
#[derive(Clone, Debug)]
pub struct ShadeOptions {
    /// The colormap (`image.cmap` by default), and the norm mapping data to
    /// it, or the limits of a linear one (the data's minimum and maximum by
    /// default); a norm makes the limits unused, as in matplotlib.
    pub colorizer: ColorizerOptions,
    /// How colours and illumination combine (`blend_mode`, overlay by
    /// default).
    pub blend: BlendMode,
    /// How elevations become illumination.
    pub relief: Relief,
}

impl Default for ShadeOptions {
    fn default() -> Self {
        Self {
            colorizer: ColorizerOptions::default(),
            blend: BlendMode::Overlay,
            relief: Relief::default(),
        }
    }
}

impl_colorize!(ShadeOptions);

/// Why a surface cannot be shaded.
#[derive(Clone, Debug, PartialEq)]
pub enum ShadeError {
    /// Slopes need at least two rows and two columns.
    TooSmall {
        /// Rows given.
        rows: usize,
        /// Columns given.
        cols: usize,
    },
    /// The colours and the elevations differ in shape.
    ShapeMismatch,
    /// A colour component is outside `[0, 1]`.
    Color(ColorError),
    /// The norm cannot map the data.
    Norm(NormError),
    /// The default colormap `image.cmap` is unknown.
    Colormap(ColormapError),
}

impl fmt::Display for ShadeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooSmall { rows, cols } => write!(
                f,
                "shape of array too small to calculate a numerical gradient ({rows} × {cols})"
            ),
            Self::ShapeMismatch => f.write_str("the colours and elevations differ in shape"),
            Self::Color(err) => write!(f, "invalid colour: {err}"),
            Self::Norm(err) => write!(f, "cannot normalize the data: {err}"),
            Self::Colormap(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ShadeError {}

/// A light infinitely far away, and how it shades surfaces (matplotlib's
/// `colors.LightSource`).
///
/// ```
/// use plt::colors::{LightSource, Relief, ColorizerOptions};
/// use plt::numeric::Grid;
/// let hill = Grid::from_fn(5, 5, |r, c| -((r as f64 - 2.0).powi(2) + (c as f64 - 2.0).powi(2)));
/// let light = LightSource::default();
/// let shade = light.hillshade(&hill, &Relief::default())?;
/// // The north-west slope faces the light.
/// assert!(shade.get(1, 1) > shade.get(3, 3));
/// # Ok::<(), plt::colors::ShadeError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightSource {
    /// Azimuth of the light in degrees clockwise from north.
    pub azdeg: f64,
    /// Altitude of the light in degrees above the horizon.
    pub altdeg: f64,
    /// The value fully shaded pixels move to in HSV blending.
    pub hsv_min_val: f64,
    /// The value fully lit pixels move to in HSV blending.
    pub hsv_max_val: f64,
    /// The saturation fully shaded pixels move to in HSV blending.
    pub hsv_min_sat: f64,
    /// The saturation fully lit pixels move to in HSV blending.
    pub hsv_max_sat: f64,
}

impl Default for LightSource {
    /// matplotlib's default light: from the north-west (`azdeg=315`) at
    /// `altdeg=45`.
    fn default() -> Self {
        Self::new(315.0, 45.0)
    }
}

impl LightSource {
    /// A light at azimuth `azdeg` and altitude `altdeg`, with matplotlib's
    /// HSV blending limits (shadows go black, light goes white).
    pub const fn new(azdeg: f64, altdeg: f64) -> Self {
        Self {
            azdeg,
            altdeg,
            hsv_min_val: 0.0,
            hsv_max_val: 1.0,
            hsv_min_sat: 1.0,
            hsv_max_sat: 0.0,
        }
    }

    /// The light of 3-D shading unless another is given (`azdeg=225,
    /// altdeg=19.4712`, kept by matplotlib for backwards compatibility).
    pub const fn mplot3d() -> Self {
        Self::new(225.0, 19.4712)
    }

    /// The unit vector pointing towards the light (matplotlib's
    /// `direction`).
    pub fn direction(&self) -> [f64; 3] {
        let az = (90.0 - self.azdeg).to_radians();
        let alt = self.altdeg.to_radians();
        [az.cos() * alt.cos(), az.sin() * alt.cos(), alt.sin()]
    }

    /// How much light each point of the surface `elevation` receives,
    /// from 0 (in shadow) to 1 (fully lit) (matplotlib's `hillshade`).
    ///
    /// # Errors
    ///
    /// When `elevation` has fewer than two rows or columns.
    pub fn hillshade(
        &self,
        elevation: &Grid<f64>,
        relief: &Relief,
    ) -> Result<Grid<f64>, ShadeError> {
        let scaled = elevation.map(|&z| relief.vert_exag * z);
        // The first row is the top of the image (as `imshow` assumes), so
        // y spacing is negative.
        let (e_dy, e_dx) = gradient(&scaled, -relief.dy, relief.dx)?;
        let normals = Grid::from_fn(elevation.rows(), elevation.cols(), |r, c| {
            let n = [-e_dx.row(r)[c], -e_dy.row(r)[c], 1.0];
            let magnitude = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            n.map(|v| v / magnitude)
        });
        Ok(self.shade_normals(&normals, relief.fraction))
    }

    /// The illumination of surface normals, rescaled to `[0, 1]` and
    /// stretched by `fraction` (matplotlib's `shade_normals`).
    pub fn shade_normals(&self, normals: &Grid<[f64; 3]>, fraction: f64) -> Grid<f64> {
        let d = self.direction();
        let mut intensity = normals.map(|n| n[0] * d[0] + n[1] * d[1] + n[2] * d[2]);
        // numpy's min and max are NaN when any value is.
        let (imin, imax) =
            intensity
                .as_slice()
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                    if v.is_nan() || lo.is_nan() {
                        (f64::NAN, f64::NAN)
                    } else {
                        (lo.min(v), hi.max(v))
                    }
                });
        let stretch = imax - imin > 1e-6;
        for v in intensity.as_mut_slice() {
            *v *= fraction;
            if stretch {
                *v = (*v - imin) / (imax - imin);
            }
            *v = v.clamp(0.0, 1.0);
        }
        intensity
    }

    /// `data` coloured through `options.colorizer` and shaded by its own
    /// relief (matplotlib's `shade`): RGBA colours whose alpha comes from
    /// the colormap.
    ///
    /// # Errors
    ///
    /// When the data is too small for slopes, the norm cannot map it or
    /// `image.cmap` is unknown.
    pub fn shade(
        &self,
        data: &Grid<f64>,
        options: &ShadeOptions,
    ) -> Result<Grid<Rgba>, ShadeError> {
        let colorizer = &options.colorizer;
        let cmap = colorizer
            .colormap(&crate::rc::params())
            .map_err(ShadeError::Colormap)?;
        let values = data.as_slice();
        let min = || values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = || values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let norm = colorizer.norm.clone().unwrap_or_else(|| {
            let vmin = colorizer.vmin.unwrap_or_else(min);
            let vmax = colorizer.vmax.unwrap_or_else(max);
            Box::new(Normalize::new(vmin, vmax))
        });
        let mut mappable = ScalarMappable::with_norm(cmap, norm);
        let masked: Vec<Option<f64>> = values.iter().map(|&v| Some(v)).collect();
        let colors = mappable.to_rgba(&masked, None).map_err(ShadeError::Norm)?;
        let rgba = Grid::from_vec(data.rows(), data.cols(), colors)
            .map_err(|_| ShadeError::ShapeMismatch)?;
        let rgb = rgba.map(|c| [c.r, c.g, c.b]);
        let shaded = self.shade_rgb(&rgb, data, &options.blend, &options.relief)?;
        Ok(Grid::from_fn(data.rows(), data.cols(), |r, c| {
            let [red, green, blue] = shaded.row(r)[c];
            Rgba::new(red, green, blue, rgba.row(r)[c].a)
        }))
    }

    /// RGB colours combined with the relief of `elevation` (matplotlib's
    /// `shade_rgb`).
    ///
    /// # Errors
    ///
    /// When the shapes differ, the elevation is too small for slopes, or
    /// a colour is outside `[0, 1]` in HSV blending.
    pub fn shade_rgb(
        &self,
        rgb: &Grid<[f64; 3]>,
        elevation: &Grid<f64>,
        blend: &BlendMode,
        relief: &Relief,
    ) -> Result<Grid<[f64; 3]>, ShadeError> {
        if (rgb.rows(), rgb.cols()) != (elevation.rows(), elevation.cols()) {
            return Err(ShadeError::ShapeMismatch);
        }
        let intensity = self.hillshade(elevation, relief)?;
        let blended = rgb
            .as_slice()
            .iter()
            .zip(intensity.as_slice())
            .map(|(&color, &light)| match blend {
                BlendMode::Hsv => self.blend_hsv(color, light),
                BlendMode::SoftLight => Ok(blend_soft_light(color, light)),
                BlendMode::Overlay => Ok(blend_overlay(color, light)),
                BlendMode::Custom(f) => Ok(f(color, light)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Grid::from_vec(rgb.rows(), rgb.cols(), blended).map_err(|_| ShadeError::ShapeMismatch)
    }

    /// A colour moved in HSV towards black when shaded and white when lit,
    /// within the light's HSV limits (matplotlib's `blend_hsv`).
    ///
    /// # Errors
    ///
    /// When a component of `rgb` is outside `[0, 1]`.
    pub fn blend_hsv(&self, rgb: [f64; 3], intensity: f64) -> Result<[f64; 3], ShadeError> {
        let intensity = 2.0 * intensity - 1.0;
        let [hue, mut sat, mut val] = rgb_to_hsv(rgb).map_err(ShadeError::Color)?;
        if sat.abs() > 1e-10 && intensity > 0.0 {
            sat = (1.0 - intensity) * sat + intensity * self.hsv_max_sat;
        }
        if sat.abs() > 1e-10 && intensity < 0.0 {
            sat = (1.0 + intensity) * sat - intensity * self.hsv_min_sat;
        }
        if intensity > 0.0 {
            val = (1.0 - intensity) * val + intensity * self.hsv_max_val;
        }
        if intensity < 0.0 {
            val = (1.0 + intensity) * val - intensity * self.hsv_min_val;
        }
        Ok(hsv_to_rgb([hue, sat.clamp(0.0, 1.0), val.clamp(0.0, 1.0)]))
    }
}

/// A colour under soft light of `intensity` with the "pegtop" formula
/// (matplotlib's `blend_soft_light`).
pub fn blend_soft_light(rgb: [f64; 3], intensity: f64) -> [f64; 3] {
    rgb.map(|c| 2.0 * intensity * c + (1.0 - 2.0 * intensity) * (c * c))
}

/// A colour overlaid with `intensity` (matplotlib's `blend_overlay`).
pub fn blend_overlay(rgb: [f64; 3], intensity: f64) -> [f64; 3] {
    rgb.map(|c| {
        if c <= 0.5 {
            2.0 * intensity * c
        } else {
            1.0 - 2.0 * (1.0 - intensity) * (1.0 - c)
        }
    })
}

/// numpy's `gradient(f, dy, dx)` with first-order edges: the slopes along
/// the rows and along the columns.
fn gradient(f: &Grid<f64>, dy: f64, dx: f64) -> Result<(Grid<f64>, Grid<f64>), ShadeError> {
    let (rows, cols) = (f.rows(), f.cols());
    if rows < 2 || cols < 2 {
        return Err(ShadeError::TooSmall { rows, cols });
    }
    let at = |r: usize, c: usize| f.row(r)[c];
    let slope = |before: f64, after: f64, span: f64| (after - before) / span;
    let along_rows = Grid::from_fn(rows, cols, |r, c| match r {
        0 => slope(at(0, c), at(1, c), dy),
        r if r == rows - 1 => slope(at(r - 1, c), at(r, c), dy),
        r => slope(at(r - 1, c), at(r + 1, c), 2.0 * dy),
    });
    let along_cols = Grid::from_fn(rows, cols, |r, c| match c {
        0 => slope(at(r, 0), at(r, 1), dx),
        c if c == cols - 1 => slope(at(r, c - 1), at(r, c), dx),
        c => slope(at(r, c - 1), at(r, c + 1), 2.0 * dx),
    });
    Ok((along_rows, along_cols))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradients_follow_numpy() {
        let f = Grid::from_rows(vec![vec![1.0, 2.0, 4.0], vec![2.0, 4.0, 8.0]]).unwrap();
        let (dy, dx) = gradient(&f, 1.0, 2.0).unwrap();
        assert_eq!(dx.row(0), [0.5, 0.75, 1.0]);
        assert_eq!(dy.row(1), [1.0, 2.0, 4.0]);
        assert_eq!(
            gradient(&Grid::filled(1, 4, 0.0), 1.0, 1.0),
            Err(ShadeError::TooSmall { rows: 1, cols: 4 })
        );
    }

    #[test]
    fn flat_ground_is_evenly_lit_and_blends_follow_matplotlib() {
        let light = LightSource::default();
        let flat = light
            .hillshade(&Grid::filled(3, 3, 2.0), &Relief::default())
            .unwrap();
        let expected = 45_f64.to_radians().sin();
        assert!(
            flat.as_slice()
                .iter()
                .all(|&v| (v - expected).abs() < 1e-12)
        );
        assert_eq!(blend_overlay([0.25, 0.75, 0.5], 0.5), [0.25, 0.75, 0.5]);
        assert_eq!(blend_soft_light([0.5, 0.5, 0.5], 1.0), [0.75, 0.75, 0.75]);
        let lit = light.blend_hsv([0.8, 0.2, 0.2], 1.0).unwrap();
        assert_eq!(lit, [1.0, 1.0, 1.0], "fully lit goes white");
        assert!(light.blend_hsv([1.5, 0.0, 0.0], 0.5).is_err());
    }
}
