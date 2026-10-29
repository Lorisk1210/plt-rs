//! Plotting calls on 3-D axes: points, lines, surfaces, bars, contours,
//! arrows, triangulated surfaces and voxels, and camera changes.

use pltrs::mplot3d::{
    Array3, Axes3D, Bar3DOptions, Boxes, Contour3DOptions, Quiver3DOptions, Sampling,
    SurfaceOptions, TrisurfOptions, VoxelOptions, WireframeOptions,
};
use pltrs::numeric::Grid;
use pltrs::tri::Triangulation;

use super::data::{count, length, partner, series};
use crate::harness::Gen;

/// A 3-D plotting call.
#[derive(Debug)]
pub(crate) enum Call3D {
    Points {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
        line: bool,
    },
    Surface {
        rows: usize,
        columns: usize,
        z: Vec<f64>,
        kind: usize,
        sampling: (usize, usize),
        strides: bool,
    },
    Bars {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
        size: Vec<f64>,
    },
    Arrows {
        points: Vec<[f64; 6]>,
        length: f64,
        normalize: bool,
    },
    Trisurf {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
    },
    Voxels {
        shape: [usize; 3],
        seed: u64,
    },
    View {
        elev: f64,
        azim: f64,
        roll: f64,
        zlim: (f64, f64),
    },
}

impl Call3D {
    /// A random 3-D call.
    pub(crate) fn generate(g: &mut Gen) -> Self {
        let n = length(g);
        match g.below(7) {
            0 => Self::Points {
                x: series(g, n),
                y: partner(g, n),
                z: partner(g, n),
                line: g.chance(0.5),
            },
            1 => {
                let (rows, columns) = (g.below(10), g.below(10));
                Self::Surface {
                    rows,
                    columns,
                    z: series(g, rows * columns),
                    kind: g.below(3),
                    sampling: (count(g, 20), count(g, 20)),
                    strides: g.chance(0.5),
                }
            }
            2 => Self::Bars {
                x: series(g, n),
                y: partner(g, n),
                z: partner(g, n),
                size: partner(g, n),
            },
            3 => {
                let points = (0..n.min(30))
                    .map(|_| {
                        [
                            g.number(),
                            g.number(),
                            g.number(),
                            g.number(),
                            g.number(),
                            g.number(),
                        ]
                    })
                    .collect();
                Self::Arrows {
                    points,
                    length: g.number_in(0.0, 3.0),
                    normalize: g.chance(0.5),
                }
            }
            4 => Self::Trisurf {
                x: series(g, n),
                y: partner(g, n),
                z: partner(g, n),
            },
            5 => Self::Voxels {
                shape: [g.below(6), g.below(6), g.below(6)],
                seed: g.below(1 << 30) as u64,
            },
            _ => Self::View {
                elev: g.number_in(-180.0, 180.0),
                azim: g.number_in(-360.0, 360.0),
                roll: g.number_in(-180.0, 180.0),
                zlim: (g.number(), g.number()),
            },
        }
    }

    /// Runs a [`Call3D::Surface`]: a surface, a wireframe or contours over
    /// a grid of the cell indices.
    fn surface(&self, ax: &Axes3D) {
        let Self::Surface {
            rows,
            columns,
            z,
            kind,
            sampling,
            strides,
        } = self
        else {
            return;
        };
        let (rows, columns) = (*rows, *columns);
        let indices = |f: fn(usize, usize) -> usize| {
            Grid::from_vec(
                rows,
                columns,
                (0..rows * columns)
                    .map(|i| f(i, columns.max(1)) as f64)
                    .collect(),
            )
        };
        let (Ok(x), Ok(y), Ok(z)) = (
            indices(|i, cols| i % cols),
            indices(|i, cols| i / cols),
            Grid::from_vec(rows, columns, z.clone()),
        ) else {
            return;
        };
        let (r, c) = *sampling;
        let sampling = if *strides {
            Sampling::Strides { rows: r, cols: c }
        } else {
            Sampling::Counts { rows: r, cols: c }
        };
        match kind {
            0 => drop(ax.plot_surface(
                &x,
                &y,
                &z,
                SurfaceOptions {
                    sampling,
                    ..SurfaceOptions::default()
                },
            )),
            1 => drop(ax.plot_wireframe(
                &x,
                &y,
                &z,
                WireframeOptions {
                    sampling,
                    ..WireframeOptions::default()
                },
            )),
            _ => drop(ax.contour(&x, &y, &z, Contour3DOptions::default())),
        }
    }

    /// Runs the call on `ax`; errors are fine.
    pub(crate) fn run(&self, ax: &Axes3D) {
        match self {
            Self::Points { x, y, z, line } => {
                if *line {
                    drop(ax.plot(x, y, z));
                } else {
                    drop(ax.scatter(x, y, z));
                }
            }
            Self::Surface { .. } => self.surface(ax),
            Self::Bars { x, y, z, size } => {
                let boxes = Boxes::new(x, y, z, size, size, size);
                drop(ax.bar3d(boxes, Bar3DOptions::default()));
            }
            Self::Arrows {
                points,
                length,
                normalize,
            } => {
                let positions: Vec<[f64; 3]> = points.iter().map(|p| [p[0], p[1], p[2]]).collect();
                let vectors: Vec<[f64; 3]> = points.iter().map(|p| [p[3], p[4], p[5]]).collect();
                let options = Quiver3DOptions {
                    length: *length,
                    normalize: *normalize,
                    ..Quiver3DOptions::default()
                };
                drop(ax.quiver(&positions, &vectors, options));
            }
            Self::Trisurf { x, y, z } => {
                if let Ok(tri) = Triangulation::delaunay(x, y) {
                    drop(ax.plot_trisurf(&tri, z, TrisurfOptions::default()));
                }
            }
            Self::Voxels { shape, seed } => {
                let filled = Array3::from_fn(*shape, |[i, j, k]| {
                    (seed >> ((i * 7 + j * 3 + k) % 30)) & 1 == 1
                });
                drop(ax.voxels(&filled, VoxelOptions::default()));
            }
            Self::View {
                elev,
                azim,
                roll,
                zlim,
            } => {
                ax.view_init(*elev, *azim, *roll);
                let _ = ax.set_zlim(zlim.0, zlim.1);
            }
        }
    }
}
