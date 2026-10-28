//! Plotting calls of fields: images, meshes, contours, vector fields,
//! streamlines, binned densities and triangulations.

use plt::axes::{
    Axes, HexBins, HexGridSize, HexbinOptions, Hist2dOptions, ImshowOptions, Mesh, PcolorOptions,
    Shading, SpyOptions, TriValues, TripcolorOptions,
};
use plt::colors::{NormName, NormSpec};
use plt::contour::{ClabelOptions, ContourOptions, Levels, QuadField};
use plt::image::{Extent, ImageAlpha, Interpolation, Origin};
use plt::lines::LineProps;
use plt::numeric::Grid;
use plt::quiver::{BarbsOptions, QuiverOptions, VectorField};
use plt::stats::AxisBins;
use plt::streamplot::{StreamField, StreamplotOptions};
use plt::transforms::Point;
use plt::tri::Triangulation;

use super::Mappables;
use super::data::{count, grid, length, partner, series};
use crate::harness::Gen;

/// Interpolations and norms the generators pick from.
const INTERPOLATIONS: [&str; 7] = [
    "nearest",
    "bilinear",
    "bicubic",
    "lanczos",
    "antialiased",
    "none",
    "sinc",
];
const NORMS: [NormName; 5] = [
    NormName::Linear,
    NormName::Log,
    NormName::Symlog,
    NormName::Asinh,
    NormName::Logit,
];
const SHADINGS: [Shading; 4] = [
    Shading::Auto,
    Shading::Flat,
    Shading::Nearest,
    Shading::Gouraud,
];

/// A plotting call of a field.
#[derive(Debug)]
pub(crate) enum FieldCall {
    Image {
        rows: Vec<Vec<f64>>,
        interpolation: usize,
        norm: usize,
        upper: bool,
        extent: Option<[f64; 4]>,
        alpha: f64,
        limits: Option<(f64, f64)>,
    },
    Mesh {
        rows: Vec<Vec<f64>>,
        shading: usize,
        norm: usize,
    },
    Contour {
        rows: Vec<Vec<f64>>,
        levels: Result<i64, Vec<f64>>,
        filled: bool,
        labels: bool,
        hatch: bool,
    },
    Quiver {
        x: Vec<f64>,
        y: Vec<f64>,
        u: Vec<f64>,
        v: Vec<f64>,
        scale: Option<f64>,
        width: Option<f64>,
        barbs: bool,
    },
    Stream {
        columns: usize,
        rows: usize,
        u: Vec<f64>,
        v: Vec<f64>,
        density: f64,
        max_length: f64,
        arrows: usize,
        start: Option<(f64, f64)>,
    },
    Hexbin {
        x: Vec<f64>,
        y: Vec<f64>,
        gridsize: usize,
        log: bool,
        marginals: bool,
    },
    Hist2d {
        x: Vec<f64>,
        y: Vec<f64>,
        bins: (usize, usize),
    },
    Tri {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
        /// Explicit triangles, points repeated included; `None` takes the
        /// Delaunay triangulation.
        triangles: Option<Vec<[usize; 3]>>,
        kind: usize,
        levels: i64,
    },
    Spy {
        rows: Vec<Vec<f64>>,
        precision: f64,
        markersize: Option<f64>,
    },
}

impl FieldCall {
    /// Runs a [`FieldCall::Image`].
    fn image(&self, ax: &Axes, mappables: &mut Mappables) {
        let Self::Image {
            rows,
            interpolation,
            norm,
            upper,
            extent,
            alpha,
            limits,
        } = self
        else {
            return;
        };
        let Some(values) = to_grid(rows) else { return };
        let mut options = ImshowOptions {
            interpolation: INTERPOLATIONS[*interpolation].parse::<Interpolation>().ok(),
            origin: Some(if *upper { Origin::Upper } else { Origin::Lower }),
            extent: extent.map(|[left, right, bottom, top]| Extent {
                left,
                right,
                bottom,
                top,
            }),
            alpha: Some(ImageAlpha::Uniform(*alpha)),
            ..ImshowOptions::default()
        };
        options.colorizer.norm = Some(NormSpec::Name(NORMS[*norm]));
        options.colorizer.vmin = limits.map(|(low, _)| low);
        options.colorizer.vmax = limits.map(|(_, high)| high);
        if let Ok(image) = ax.imshow(values, options) {
            mappables.images.push(image);
        }
    }

    /// Runs a [`FieldCall::Contour`], labelled now and then.
    fn contour(&self, ax: &Axes, mappables: &mut Mappables) {
        let Self::Contour {
            rows,
            levels,
            filled,
            labels,
            hatch,
        } = self
        else {
            return;
        };
        let Some(values) = to_grid(rows) else { return };
        let options = ContourOptions {
            levels: match levels {
                Ok(count) => Levels::Count(*count),
                Err(values) => Levels::Values(values.clone()),
            },
            hatches: if *hatch {
                vec![Some("/".to_owned()), None]
            } else {
                Vec::new()
            },
            ..ContourOptions::default()
        };
        let field = QuadField::new(values);
        let set = if *filled {
            ax.contourf(&field, options)
        } else {
            ax.contour(&field, options)
        };
        if let Ok(set) = set {
            if *labels {
                let _ = set.clabel(ClabelOptions::default());
            }
            mappables.contours.push(set);
        }
    }

    /// Runs a [`FieldCall::Quiver`]: arrows or wind barbs.
    fn vectors(&self, ax: &Axes) {
        let Self::Quiver {
            x,
            y,
            u,
            v,
            scale,
            width,
            barbs,
        } = self
        else {
            return;
        };
        let field = VectorField {
            x: x.clone(),
            y: y.clone(),
            u: u.clone(),
            v: v.clone(),
            colors: None,
        };
        if *barbs {
            drop(ax.barbs(field, BarbsOptions::default()));
        } else {
            let options = QuiverOptions {
                scale: *scale,
                width: *width,
                ..QuiverOptions::default()
            };
            drop(ax.quiver(field, options));
        }
    }

    /// Runs a [`FieldCall::Tri`] on its triangles, or the Delaunay
    /// triangulation of its points.
    fn triangulated(&self, ax: &Axes) {
        let Self::Tri {
            x,
            y,
            z,
            triangles,
            kind,
            levels,
        } = self
        else {
            return;
        };
        let tri = match triangles {
            Some(triangles) => Triangulation::new(x, y, triangles.clone()),
            None => Triangulation::delaunay(x, y),
        };
        let Ok(tri) = tri else {
            return;
        };
        let options = || ContourOptions {
            levels: Levels::Count(*levels),
            ..ContourOptions::default()
        };
        match kind {
            0 => drop(ax.tripcolor(
                &tri,
                TriValues::Auto(z.clone()),
                TripcolorOptions::default(),
            )),
            1 => drop(ax.tricontour(&tri, z, options())),
            2 => drop(ax.tricontourf(&tri, z, options())),
            _ => drop(ax.triplot(&tri, "o-", LineProps::default())),
        }
    }

    /// Runs a [`FieldCall::Stream`].
    fn stream(&self, ax: &Axes) {
        let Self::Stream {
            columns,
            rows,
            u,
            v,
            density,
            max_length,
            arrows,
            start,
        } = self
        else {
            return;
        };
        let (Ok(u), Ok(v)) = (
            Grid::from_vec(*rows, *columns, u.clone()),
            Grid::from_vec(*rows, *columns, v.clone()),
        ) else {
            return;
        };
        let field = StreamField {
            x: (0..*columns).map(|i| i as f64).collect(),
            y: (0..*rows).map(|i| i as f64).collect(),
            u,
            v,
        };
        let options = StreamplotOptions {
            density: (*density, *density),
            maxlength: *max_length,
            num_arrows: *arrows,
            start_points: start
                .map(|(x, y)| vec![Point::new(x * *columns as f64, y * *rows as f64)]),
            ..StreamplotOptions::default()
        };
        drop(ax.streamplot(&field, options));
    }
}

/// A grid of the values, or `None` when the rows are ragged.
fn to_grid(rows: &[Vec<f64>]) -> Option<Grid<f64>> {
    Grid::from_rows(rows.to_vec()).ok()
}

impl FieldCall {
    /// A random field call.
    pub(crate) fn generate(g: &mut Gen) -> Self {
        let n = length(g);
        match g.below(9) {
            0 => Self::Image {
                rows: grid(g),
                interpolation: g.below(INTERPOLATIONS.len()),
                norm: g.below(NORMS.len()),
                upper: g.chance(0.5),
                extent: g
                    .chance(0.3)
                    .then(|| [g.number(), g.number(), g.number(), g.number()]),
                alpha: g.number_in(0.0, 1.0),
                limits: g.chance(0.3).then(|| (g.number(), g.number())),
            },
            1 => Self::Mesh {
                rows: grid(g),
                shading: g.below(SHADINGS.len()),
                norm: g.below(NORMS.len()),
            },
            2 => Self::Contour {
                rows: grid(g),
                levels: if g.chance(0.6) {
                    Ok(count(g, 12) as i64)
                } else {
                    let len = g.len(8);
                    Err(series(g, len))
                },
                filled: g.chance(0.5),
                labels: g.chance(0.3),
                hatch: g.chance(0.2),
            },
            3 => Self::Quiver {
                x: series(g, n),
                y: partner(g, n),
                u: partner(g, n),
                v: partner(g, n),
                scale: g.chance(0.3).then(|| g.number()),
                width: g.chance(0.3).then(|| g.number_in(0.0, 0.1)),
                barbs: g.chance(0.3),
            },
            4 => {
                let (columns, rows) = (g.below(12), g.below(12));
                Self::Stream {
                    columns,
                    rows,
                    u: series(g, columns * rows),
                    v: series(g, columns * rows),
                    // Past a density of a few, streamlines dithering in
                    // place are millions of vertices, which take seconds
                    // to draw on a map: rare, to keep the default run short.
                    density: if g.chance(0.97) {
                        g.between(0.1, 3.0)
                    } else {
                        g.number()
                    },
                    max_length: g.number_in(0.1, 10.0),
                    arrows: count(g, 3),
                    start: g
                        .chance(0.3)
                        .then(|| (g.number_in(0.0, 1.0), g.number_in(0.0, 1.0))),
                }
            }
            5 => Self::Hexbin {
                x: series(g, n),
                y: partner(g, n),
                gridsize: count(g, 30),
                log: g.chance(0.3),
                marginals: g.chance(0.3),
            },
            6 => Self::Hist2d {
                x: series(g, n),
                y: partner(g, n),
                bins: (count(g, 20), count(g, 20)),
            },
            7 => Self::Tri {
                x: series(g, n),
                y: partner(g, n),
                z: partner(g, n),
                triangles: g.chance(0.3).then(|| {
                    (0..g.below(12))
                        .map(|_| [g.below(n), g.below(n), g.below(n)])
                        .collect()
                }),
                kind: g.below(4),
                levels: count(g, 10) as i64,
            },
            _ => Self::Spy {
                rows: grid(g),
                precision: g.number_in(0.0, 1.0),
                markersize: g.chance(0.5).then(|| g.number_in(0.0, 10.0)),
            },
        }
    }

    /// Runs the call on `ax`; errors are fine.
    pub(crate) fn run(&self, ax: &Axes, mappables: &mut Mappables) {
        match self {
            Self::Image { .. } => self.image(ax, mappables),
            Self::Mesh {
                rows,
                shading,
                norm,
            } => {
                let Some(values) = to_grid(rows) else { return };
                let mut options = PcolorOptions {
                    shading: Some(SHADINGS[*shading]),
                    ..PcolorOptions::default()
                };
                options.colorizer.norm = Some(NormSpec::Name(NORMS[*norm]));
                if let Ok(mesh) = ax.pcolormesh(Mesh::new(values), options) {
                    mappables.collections.push(mesh);
                }
            }
            Self::Contour { .. } => self.contour(ax, mappables),
            Self::Quiver { .. } => self.vectors(ax),
            Self::Stream { .. } => self.stream(ax),
            Self::Hexbin {
                x,
                y,
                gridsize,
                log,
                marginals,
            } => {
                let options = HexbinOptions {
                    gridsize: HexGridSize::Count(*gridsize),
                    bins: log.then_some(HexBins::Log),
                    marginals: *marginals,
                    ..HexbinOptions::default()
                };
                if let Ok(bins) = ax.hexbin(x, y, options) {
                    mappables.collections.push(bins);
                }
            }
            Self::Hist2d { x, y, bins } => {
                let mut options = Hist2dOptions::default();
                options.histogram.bins = [AxisBins::Count(bins.0), AxisBins::Count(bins.1)];
                drop(ax.hist2d(x, y, options));
            }
            Self::Tri { .. } => self.triangulated(ax),
            Self::Spy {
                rows,
                precision,
                markersize,
            } => {
                let Some(values) = to_grid(rows) else { return };
                let options = SpyOptions {
                    precision: *precision,
                    markersize: *markersize,
                    ..SpyOptions::default()
                };
                drop(ax.spy(&values, options));
            }
        }
    }
}
