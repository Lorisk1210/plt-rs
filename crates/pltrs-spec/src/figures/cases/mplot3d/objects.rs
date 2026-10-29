//! Bars, arrows, voxels, stems and texts in 3-D.

use std::f64::consts::PI;

use pltrs::axes::BarOptions;
use pltrs::collections::CollectionProps;
use pltrs::mplot3d::{
    Array3, Axis3, Bar3DOptions, Boxes, Quiver3DOptions, Stem3DOptions, VoxelColors, VoxelOptions,
};
use pltrs::numeric::linspace;
use pltrs::patches::PatchProps;

use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mplot3d/bar3d", |fig| {
        let ax = fig.subplot_3d();
        // np.meshgrid(np.arange(4), np.arange(3)), raveled.
        let (x, y): (Vec<f64>, Vec<f64>) = (0..3)
            .flat_map(|row| (0..4).map(move |col| (f64::from(col), f64::from(row))))
            .unzip();
        let heights: Vec<f64> = x
            .iter()
            .zip(&y)
            .map(|(x, y)| 1.0 + (x * 3.0 + y * 5.0) % 7.0)
            .collect();
        let boxes = Boxes::new(&x, &y, [0.0], [0.6], [0.6], &heights);
        let options = Bar3DOptions {
            color: vec!["tab:cyan".into()],
            ..Bar3DOptions::default()
        };
        ax.bar3d(boxes, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/bar3d_no_shade_colors", |fig| {
        let boxes = Boxes::new(
            [0.0, 1.0, 2.0],
            [0.0, 1.0, 2.0],
            [0.0, 0.0, 0.0],
            [0.5],
            [0.5],
            [3.0, 1.0, 2.0],
        );
        let options = Bar3DOptions {
            color: vec!["red".into(), "green".into(), "blue".into()],
            shade: false,
            props: CollectionProps::default().edgecolor("k"),
            ..Bar3DOptions::default()
        };
        fig.subplot_3d().bar3d(boxes, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/bar_zdir", |fig| {
        let ax = fig.subplot_3d();
        let x: Vec<f64> = (0..8).map(f64::from).collect();
        for (i, color) in ["r", "g", "b"].into_iter().enumerate() {
            let factor = (i + 2) as f64;
            let heights: Vec<f64> = x.iter().map(|x| (x * factor) % 7.0 + 1.0).collect();
            let options = BarOptions {
                color: vec![color.into()],
                props: PatchProps::default().alpha(0.8),
                ..BarOptions::default()
            };
            ax.bar(&x, &heights, [i as f64 * 10.0], Axis3::Y, options)?;
        }
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/quiver3d", |fig| {
        let g = linspace(-1.0, 1.0, 4);
        // np.meshgrid(g, g, g): xx[i, j, k] = g[j], yy = g[i], zz = g[k].
        let mut positions = Vec::new();
        let mut vectors = Vec::new();
        for &gi in &g {
            for &gj in &g {
                for &gk in &g {
                    positions.push([gj, gi, gk]);
                    vectors.push([-gi, gj, 0.3 * gk]);
                }
            }
        }
        let options = Quiver3DOptions {
            length: 0.3,
            normalize: true,
            ..Quiver3DOptions::default()
        };
        fig.subplot_3d().quiver(&positions, &vectors, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/voxels", |fig| {
        let centered = |v: usize| (v as f64 - 2.5).powi(2);
        let filled = Array3::from_fn([6, 6, 6], |[x, y, z]| {
            centered(x) + centered(y) + centered(z) < 7.0
        });
        let colors = Array3::from_fn([6, 6, 6], |[_, _, z]| {
            if z < 3 { "tab:blue" } else { "tab:orange" }.into()
        });
        let options = VoxelOptions {
            facecolors: Some(VoxelColors::PerCell(colors)),
            props: CollectionProps::default().edgecolor("k").linewidth(0.3),
            ..VoxelOptions::default()
        };
        fig.subplot_3d().voxels(&filled, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/stem", |fig| {
        let t = linspace(0.0, 2.0 * PI, 16);
        let x: Vec<f64> = t.iter().map(|t| t.cos()).collect();
        let y: Vec<f64> = t.iter().map(|t| t.sin()).collect();
        fig.subplot_3d()
            .stem(&x, &y, &t, Stem3DOptions::default())?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/text_and_text2d", |fig| {
        let ax = fig.subplot_3d();
        ax.text(0.0, 0.0, 0.0, "origin", None).set_color("red");
        ax.text(1.0, 0.0, 0.0, "along x", Some(Axis3::X));
        ax.text(0.0, 1.0, 0.0, "along y", Some(Axis3::Y));
        ax.text(0.0, 0.0, 1.0, "along z", Some(Axis3::Z));
        ax.text2d(0.05, 0.95, "2D text");
        ax.set_xlim(0.0, 1.0)?
            .set_ylim(0.0, 1.0)?
            .set_zlim(0.0, 1.0)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
];
