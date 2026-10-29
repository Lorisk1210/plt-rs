//! Voxels: `voxels`, `voxels_numpy_logo`, `voxels_rgb` and
//! `voxels_torus`.

use std::f64::consts::PI;

use pltrs::axes::Adjustable;
use pltrs::collections::CollectionProps;
use pltrs::colors::{Color, hsv_to_rgb};
use pltrs::figure::Figure;
use pltrs::mplot3d::{Array3, Aspect3D, Vec3, VoxelColors, VoxelOptions};

use super::label_xyz;
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_3d/voxels", voxels),
    FigureCase::new("gallery_3d/voxels_numpy_logo", voxels_numpy_logo),
    FigureCase::new("gallery_3d/voxels_rgb", voxels_rgb),
    FigureCase::new("gallery_3d/voxels_torus", voxels_torus),
];

fn voxels(fig: &Figure) -> CaseResult {
    let cube1 = |[x, y, z]: [usize; 3]| x < 3 && y < 3 && z < 3;
    let cube2 = |[x, y, z]: [usize; 3]| x >= 5 && y >= 5 && z >= 5;
    let link = |[x, y, z]: [usize; 3]| x.abs_diff(y) + y.abs_diff(z) + z.abs_diff(x) <= 2;
    let shape = [8, 8, 8];
    let filled = Array3::from_fn(shape, |c| cube1(c) || cube2(c) || link(c));
    // Later assignments win: the cubes over the link.
    let colors = Array3::from_fn(shape, |c| {
        Color::from(if cube2(c) {
            "green"
        } else if cube1(c) {
            "blue"
        } else {
            "red"
        })
    });
    let options = VoxelOptions {
        facecolors: Some(VoxelColors::PerCell(colors)),
        props: CollectionProps::default().edgecolor("k"),
        ..VoxelOptions::default()
    };
    fig.subplot_3d().voxels(&filled, options)?;
    Ok(())
}

/// Whether voxel `cell` of the 4 × 3 × 4 block of `voxels_numpy_logo`
/// belongs to the letter N.
fn in_letter_n([i, j, k]: [usize; 3]) -> bool {
    j == 0 && (i == 0 || i == 3 || (i, k) == (1, 2) || (i, k) == (2, 1))
}

fn voxels_numpy_logo(fig: &Figure) -> CaseResult {
    // The block spread out with an empty cell between neighbours
    // (`explode`): cell [i, j, k] holds voxel [i, j, k] / 2 when even.
    let shape = [7, 5, 7];
    let filled = Array3::from_fn(shape, |c| c.iter().all(|v| v.is_multiple_of(2)));
    let logo = |letter: &str, rest: &str| {
        Array3::from_fn(shape, |c| {
            Color::from(if in_letter_n(c.map(|v| v / 2)) {
                letter
            } else {
                rest
            })
        })
    };
    // Each voxel shrunk to [0.05, 0.95] within its unit cell.
    let corner = |v: usize| (v / 2) as f64 + if v.is_multiple_of(2) { 0.05 } else { 0.95 };
    let corners = Array3::from_fn([8, 6, 8], |c| c.map(corner));
    let options = VoxelOptions {
        facecolors: Some(VoxelColors::PerCell(logo("#FFD65DC0", "#7A88CCC0"))),
        edgecolors: Some(VoxelColors::PerCell(logo("#BFAB6E", "#7D84A6"))),
        corners: Some(corners),
        ..VoxelOptions::default()
    };
    let ax = fig.subplot_3d();
    ax.voxels(&filled, options)?;
    ax.set_aspect(Aspect3D::Equal, Adjustable::Box)?;
    Ok(())
}

/// The voxel examples' `midpoints` along one axis: the mean of the
/// coordinates of a cell's two sides.
fn midpoint(coordinate: impl Fn(usize) -> f64, i: usize) -> f64 {
    f64::midpoint(coordinate(i), coordinate(i + 1))
}

/// The face colours given and their brighter edges,
/// `np.clip(2 * colors - 0.5, 0, 1)`, with edges half a point wide.
fn bright_edges(colors: &Array3<[f64; 3]>) -> VoxelOptions {
    let cells = colors.shape();
    let at = |cell| *colors.get(cell).expect("the cell is in the array");
    VoxelOptions {
        facecolors: Some(VoxelColors::PerCell(Array3::from_fn(cells, |c| {
            Color::from(at(c))
        }))),
        edgecolors: Some(VoxelColors::PerCell(Array3::from_fn(cells, |c| {
            Color::from(at(c).map(|v| (2.0 * v - 0.5).clamp(0.0, 1.0)))
        }))),
        props: CollectionProps::default().linewidth(0.5),
        ..VoxelOptions::default()
    }
}

fn voxels_rgb(fig: &Figure) -> CaseResult {
    // np.indices((17, 17, 17)) / 16.0
    let coordinate = |v: usize| v as f64 / 16.0;
    let corners: Array3<Vec3> = Array3::from_fn([17; 3], |c| c.map(coordinate));
    let centres: Array3<[f64; 3]> =
        Array3::from_fn([16; 3], |c| c.map(|v| midpoint(coordinate, v)));
    let sphere = Array3::from_fn([16; 3], |c| {
        let [r, g, b] = *centres.get(c).expect("the cell is in the array");
        (r - 0.5).powi(2) + (g - 0.5).powi(2) + (b - 0.5).powi(2) < 0.25
    });
    let options = VoxelOptions {
        corners: Some(corners),
        ..bright_edges(&centres)
    };
    let ax = fig.subplot_3d();
    ax.voxels(&sphere, options)?;
    label_xyz(&ax, ["r", "g", "b"]);
    ax.set_aspect(Aspect3D::Equal, Adjustable::Box)?;
    Ok(())
}

fn voxels_torus(fig: &Figure) -> CaseResult {
    // np.mgrid[0:1:11j, 0:np.pi * 2:25j, -0.5:0.5:11j]
    let r = |i: usize| i as f64 * 0.1;
    let theta = |j: usize| j as f64 * (2.0 * PI / 24.0);
    let z = |k: usize| k as f64 * 0.1 - 0.5;
    let corners = Array3::from_fn([11, 25, 11], |[i, j, k]| {
        [r(i) * theta(j).cos(), r(i) * theta(j).sin(), z(k)]
    });
    let shape = [10, 24, 10];
    let centre = |[i, j, k]: [usize; 3]| [midpoint(r, i), midpoint(theta, j), midpoint(z, k)];
    // A wobbly torus about r = 0.7, z = 0.
    let torus = Array3::from_fn(shape, |c| {
        let [rc, thetac, zc] = centre(c);
        (rc - 0.7).powi(2) + (zc + 0.2 * (thetac * 2.0).cos()).powi(2) < 0.2_f64.powi(2)
    });
    let colors = Array3::from_fn(shape, |c| {
        let [rc, thetac, zc] = centre(c);
        hsv_to_rgb([thetac / (PI * 2.0), rc, zc + 0.5])
    });
    let options = VoxelOptions {
        corners: Some(corners),
        ..bright_edges(&colors)
    };
    fig.subplot_3d().voxels(&torus, options)?;
    Ok(())
}
