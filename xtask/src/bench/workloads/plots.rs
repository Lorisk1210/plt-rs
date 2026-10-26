//! Lines, markers, meshes and images, and statistics.

use plt::axes::{
    BoxplotOptions, FillBetweenOptions, HexbinOptions, Hist2dOptions, HistPlotOptions,
    ImshowOptions, Mesh, PcolorOptions, StackplotOptions, ViolinplotOptions,
};
use plt::colorbar::ColorbarOptions;
use plt::figure::SaveFormat;
use plt::image::Interpolation;
use plt::prelude::*;
use plt::stats::{AxisBins, BinEstimator, Bins, Histogram2dOptions};
use plt_spec::rng::SpecRng;

use super::{Build, Workload, random_walk, smooth_field};

/// The workloads of this module.
pub(super) const WORKLOADS: &[Workload] = &[
    Workload::new("lines/plot_1e6_png", walk_1e6),
    Workload::new("lines/plot_1e6_svg", walk_1e6).saved_as(SaveFormat::Svg),
    Workload::new("lines/plot_1e6_pdf", walk_1e6).saved_as(SaveFormat::Pdf),
    Workload::new("lines/1000_lines_of_1000", many_lines),
    Workload::new("lines/dashed_1e5", dashed),
    Workload::new("lines/markevery_1e5", markevery),
    Workload::new("lines/fill_between_1e5", fill_between),
    Workload::new("lines/stackplot_5x1e5", stackplot),
    Workload::new("markers/scatter_1e5", scatter),
    Workload::new("markers/plot_o_1e5", plot_markers),
    Workload::new("meshes/pcolormesh_1000", pcolormesh),
    Workload::new("meshes/imshow_4000_antialiased", || {
        imshow(Interpolation::Antialiased)
    }),
    Workload::new("meshes/imshow_4000_nearest", || {
        imshow(Interpolation::Nearest)
    }),
    Workload::new("meshes/imshow_2000_bicubic_colorbar", imshow_colorbar),
    Workload::new("meshes/hexbin_1e6", hexbin),
    Workload::new("meshes/hist2d_1e6", hist2d),
    Workload::new("stats/hist_1e7", hist),
    Workload::new("stats/hist_1e6_auto", hist_auto),
    Workload::new("stats/boxplot_100", boxplot),
    Workload::new("stats/violinplot_50", violinplot),
];

/// `ax.plot` of a 10⁶-point random walk.
fn walk_1e6() -> Build {
    let (x, y) = random_walk(1_000_000, 1);
    Box::new(move |fig| {
        fig.subplot().plot(&x, &y)?;
        Ok(())
    })
}

/// 1000 sine waves of 1000 points each, one `plot` call apiece.
fn many_lines() -> Build {
    let x = linspace(0.0, 10.0, 1000);
    let waves: Vec<Vec<f64>> = (0..1000)
        .map(|k| {
            let k = f64::from(k);
            x.iter()
                .map(|x| (x * (1.0 + k / 1000.0)).sin() + k / 100.0)
                .collect()
        })
        .collect();
    Box::new(move |fig| {
        let ax = fig.subplot();
        for wave in &waves {
            ax.plot(&x, wave)?;
        }
        Ok(())
    })
}

/// A dashed 10⁵-point random walk (`'--'`).
fn dashed() -> Build {
    let (x, y) = random_walk(100_000, 2);
    Box::new(move |fig| {
        fig.subplot().plot_fmt(&x, &y, "--")?;
        Ok(())
    })
}

/// A 10⁵-point random walk with a circle on every 100th point.
fn markevery() -> Build {
    let (x, y) = random_walk(100_000, 3);
    Box::new(move |fig| {
        let props = LineProps::fmt("-o")?.markevery(Some(MarkEvery::Every {
            start: 0,
            step: 100,
        }));
        fig.subplot().plot_with(&x, &y, props)?;
        Ok(())
    })
}

/// A 10⁵-point random walk filled down to zero.
fn fill_between() -> Build {
    let (x, y) = random_walk(100_000, 14);
    Box::new(move |fig| {
        fig.subplot()
            .fill_between(&x, &y, [0.0], FillBetweenOptions::default())?;
        Ok(())
    })
}

/// Five stacked sine layers of 10⁵ points each.
fn stackplot() -> Build {
    let x = linspace(0.0, 100.0, 100_000);
    let layers: Vec<Vec<f64>> = (0..5)
        .map(|k| {
            let k = f64::from(k);
            x.iter()
                .map(|x| 1.5 + (x * (k + 1.0) / 7.0 + k).sin())
                .collect()
        })
        .collect();
    Box::new(move |fig| {
        fig.subplot()
            .stackplot(&x, &layers, StackplotOptions::default())?;
        Ok(())
    })
}

/// 10⁵ uniform points with per-point sizes and colour-mapped values.
fn scatter() -> Build {
    let mut rng = SpecRng::new(4);
    let x = rng.uniform(100_000, 0.0, 1.0);
    let y = rng.uniform(100_000, 0.0, 1.0);
    let sizes = rng.uniform(100_000, 5.0, 50.0);
    let values = rng.uniform(100_000, 0.0, 1.0);
    Box::new(move |fig| {
        let options = ScatterOptions {
            sizes: Some(sizes.clone()),
            colors: MappedColors::new(values.clone()).into(),
            ..ScatterOptions::default()
        };
        fig.subplot().scatter_with(&x, &y, options)?;
        Ok(())
    })
}

/// 10⁵ uniform points as `plot(x, y, 'o')`.
fn plot_markers() -> Build {
    let mut rng = SpecRng::new(5);
    let x = rng.uniform(100_000, 0.0, 1.0);
    let y = rng.uniform(100_000, 0.0, 1.0);
    Box::new(move |fig| {
        fig.subplot().plot_fmt(&x, &y, "o")?;
        Ok(())
    })
}

/// `pcolormesh` of a smooth 1000 × 1000 field.
fn pcolormesh() -> Build {
    let z = smooth_field(1000);
    Box::new(move |fig| {
        fig.subplot()
            .pcolormesh(Mesh::new(z.clone()), PcolorOptions::default())?;
        Ok(())
    })
}

/// `imshow` of a smooth 4000 × 4000 field.
fn imshow(interpolation: Interpolation) -> Build {
    let z = smooth_field(4000);
    Box::new(move |fig| {
        let options = ImshowOptions {
            interpolation: Some(interpolation),
            ..ImshowOptions::default()
        };
        fig.subplot().imshow(z.clone(), options)?;
        Ok(())
    })
}

/// `imshow` of a smooth 2000 × 2000 field, bicubic, with a colorbar.
fn imshow_colorbar() -> Build {
    let z = smooth_field(2000);
    Box::new(move |fig| {
        let options = ImshowOptions {
            interpolation: Some(Interpolation::Bicubic),
            ..ImshowOptions::default()
        };
        let shown = fig.subplot().imshow(z.clone(), options)?;
        fig.colorbar(&shown, ColorbarOptions::default())?;
        Ok(())
    })
}

/// 10⁶ correlated normal points, independent and correlated halves.
fn correlated_normals(seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut rng = SpecRng::new(seed);
    let x = rng.normal(1_000_000, 0.0, 1.0);
    let noise = rng.normal(1_000_000, 0.0, 1.0);
    let y = x.iter().zip(noise).map(|(x, n)| 0.5 * x + n).collect();
    (x, y)
}

/// `hexbin` of 10⁶ correlated normal points.
fn hexbin() -> Build {
    let (x, y) = correlated_normals(6);
    Box::new(move |fig| {
        fig.subplot().hexbin(&x, &y, HexbinOptions::default())?;
        Ok(())
    })
}

/// `hist2d` of 10⁶ correlated normal points in 100 × 100 bins.
fn hist2d() -> Build {
    let (x, y) = correlated_normals(7);
    Box::new(move |fig| {
        let options = Hist2dOptions {
            histogram: Histogram2dOptions {
                bins: [AxisBins::Count(100), AxisBins::Count(100)],
                ..Histogram2dOptions::default()
            },
            ..Hist2dOptions::default()
        };
        fig.subplot().hist2d(&x, &y, options)?;
        Ok(())
    })
}

/// `hist` of 10⁷ normal values in 1000 bins.
fn hist() -> Build {
    let values = SpecRng::new(8).normal(10_000_000, 0.0, 1.0);
    Box::new(move |fig| {
        let options = HistPlotOptions {
            bins: Some(Bins::Count(1000)),
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[values.as_slice()], options)?;
        Ok(())
    })
}

/// `hist` of 10⁶ normal values, binned by numpy's `"auto"` rule.
fn hist_auto() -> Build {
    let values = SpecRng::new(15).normal(1_000_000, 0.0, 1.0);
    Box::new(move |fig| {
        let options = HistPlotOptions {
            bins: Some(Bins::Estimator(BinEstimator::Auto)),
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[values.as_slice()], options)?;
        Ok(())
    })
}

/// `groups` normal samples of 1000 values with growing means and spreads.
fn normal_groups(groups: u32, seed: u64) -> Vec<Vec<f64>> {
    let mut rng = SpecRng::new(seed);
    (0..groups)
        .map(|g| {
            let g = f64::from(g);
            rng.normal(1000, g / 10.0, 1.0 + g / 100.0)
        })
        .collect()
}

/// `boxplot` of 100 groups of 1000 values.
fn boxplot() -> Build {
    let groups = normal_groups(100, 9);
    Box::new(move |fig| {
        fig.subplot().boxplot(&groups, BoxplotOptions::default())?;
        Ok(())
    })
}

/// `violinplot` of 50 groups of 1000 values.
fn violinplot() -> Build {
    let groups = normal_groups(50, 10);
    Box::new(move |fig| {
        fig.subplot()
            .violinplot(&groups, ViolinplotOptions::default())?;
        Ok(())
    })
}
