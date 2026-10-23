//! Contour lines and filled contours (matplotlib.contour, contourpy).
//!
//! * [`QuadContourGenerator`] — the `mpl2014` algorithm for fields on
//!   quadrilateral grids (matplotlib's default), with corner masking and
//!   chunking. Triangular grids use [`crate::tri::TriContourGenerator`].
//! * [`Levels`] and [`ContourOptions`] — level selection (explicit values,
//!   or a `MaxNLocator`/`LogLocator` over the data range), `extend`,
//!   colours, colormap and norm, line widths and styles.
//! * [`QuadField`] — z on a grid, with optional mask and coordinates
//!   (explicit, or placed like an image with `origin`/`extent`).
//! * [`ContourSet`] — what `Axes::contour`, `contourf`, `tricontour`,
//!   `tricontourf` and `contour_segments` (contours given as paths)
//!   return: the chosen levels, the bands of filled contours,
//!   one compound path per level or band, their colours and styles, and
//!   [`ContourSet::clabel`] to label line contours,
//!   [`ContourSet::find_nearest_contour`] to find the line nearest to a
//!   point and
//!   [`ContourSet::legend_elements`] to describe them in a legend.
//!   Changing its colormap,
//!   norm or limits recolours the contours, their labels and colorbars.
//!
//! ```
//! use plt::contour::{ContourOptions, Levels, QuadField};
//! use plt::numeric::Grid;
//! use plt::prelude::*;
//!
//! let z = Grid::from_fn(5, 5, |i, j| ((i * i + j * j) as f64).sqrt());
//! let ax = Figure::new().subplot();
//! let options = ContourOptions { levels: Levels::Values(vec![1.0, 2.0, 3.0]), ..ContourOptions::default() };
//! let cs = ax.contour(&QuadField::new(z), options)?;
//! assert_eq!(cs.paths().len(), 3);
//! # Ok::<(), plt::contour::ContourError>(())
//! ```

mod assemble;
mod chunks;
mod error;
mod given;
mod grid;
mod handle;
mod label;
mod legend;
mod levels;
mod mpl2005;
mod mpl2014;
mod nearest;
mod options;
mod quad;
mod serial;
mod set;
mod style;

pub use error::ContourError;
pub use grid::{Origin, QuadCoords, QuadField};
pub use handle::ContourSet;
pub use label::{ClabelOptions, ContourLabel, LabelFormat};
pub use levels::Levels;
pub use nearest::{NearestContour, NearestIn};
pub use options::{ContourNorm, ContourOptions};
pub use quad::{ContourAlgorithm, QuadContourGenerator};
pub use set::ContourWarning;

pub(crate) use assemble::LevelPath;
pub(crate) use given::given_source;
pub(crate) use handle::ContourSetArtist;
pub(crate) use set::{ContourPlan, ContourSource, Generator};
