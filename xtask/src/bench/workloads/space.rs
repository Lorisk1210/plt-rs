//! 3-D axes beyond surfaces and scatters: a wireframe, bars and voxels.

use pltrs::collections::CollectionProps;
use pltrs::mplot3d::{
    Array3, Bar3DOptions, Boxes, Sampling, VoxelColors, VoxelOptions, WireframeOptions,
};
use pltrs::numeric::{Grid, linspace, meshgrid};
use pltrs_spec::rng::SpecRng;

use super::{Build, Tier, Workload};

/// The workloads of this module.
pub(super) const WORKLOADS: &[Workload] = &[
    Workload::new("3d/wireframe_60", "wireframe_60", wireframe).in_tier(Tier::Medium),
    Workload::new("3d/bar3d_10x10", "bar3d_10x10", bar3d).in_tier(Tier::Medium),
    Workload::new("3d/voxels_12", "voxels_12", voxels).in_tier(Tier::Medium),
];

/// `plot_wireframe` of `sin(r)` on a 60 × 60 grid, every second line.
fn wireframe() -> Build {
    let t = linspace(-5.0, 5.0, 60);
    let (x, y) = meshgrid(&t, &t);
    let z = Grid::from_fn(60, 60, |i, j| t[j].hypot(t[i]).sin());
    Box::new(move |fig| {
        let options = WireframeOptions {
            sampling: Sampling::Strides { rows: 2, cols: 2 },
            props: CollectionProps::default().linewidth(0.5),
            ..WireframeOptions::default()
        };
        fig.subplot_3d().plot_wireframe(&x, &y, &z, options)?;
        Ok(())
    })
}

/// `bar3d` of 100 shaded columns on a 10 × 10 floor.
fn bar3d() -> Build {
    let (x, y): (Vec<f64>, Vec<f64>) = (0..100)
        .map(|k| (f64::from(k % 10), f64::from(k / 10)))
        .unzip();
    let heights = SpecRng::new(52).uniform(100, 1.0, 10.0);
    Box::new(move |fig| {
        let boxes = Boxes::new(x.clone(), y.clone(), [0.0], [0.8], [0.8], heights.clone());
        fig.subplot_3d().bar3d(boxes, Bar3DOptions::default())?;
        Ok(())
    })
}

/// `voxels` of a ball in a 12 × 12 × 12 block, black-edged and shaded.
fn voxels() -> Build {
    let ball = Array3::from_fn([12, 12, 12], |[i, j, k]| {
        let squared = |n: usize| (n as f64 - 5.5).powi(2);
        squared(i) + squared(j) + squared(k) < 30.0
    });
    Box::new(move |fig| {
        let options = VoxelOptions {
            edgecolors: Some(VoxelColors::Single("k".into())),
            props: CollectionProps::default().linewidth(0.3),
            ..VoxelOptions::default()
        };
        fig.subplot_3d().voxels(&ball, options)?;
        Ok(())
    })
}
