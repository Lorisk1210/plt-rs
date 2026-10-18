//! `figures/cases_scatter.py`: the basic cases here, legends and
//! variants in submodules.

mod legends;
mod variants;

pub(crate) use legends::CASES as LEGENDS;
pub(crate) use variants::CASES as VARIANTS;

use plt::axes::{MappedColors, ScatterColors, ScatterOptions};
use plt::collections::EdgeColors;
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{Color, LogNorm, TwoSlopeNorm, colormaps};
use plt::numeric::{arange, linspace, logspace};
use plt::orientation::Orientation;

use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// `_points(seed, n)`: two uniform samples of `n` values.
fn points(seed: u64, n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut rng = SpecRng::new(seed);
    let x = rng.uniform(n, 0.0, 1.0);
    let y = rng.uniform(n, 0.0, 1.0);
    (x, y)
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("scatter/default", |fig| {
        let (x, y) = points(1, 60);
        fig.subplot().scatter(x, y)?;
        Ok(())
    }),
    FigureCase::new("scatter/sizes_array", |fig| {
        let (x, y) = points(3, 30);
        let options = ScatterOptions {
            sizes: Some(linspace(5.0, 600.0, 30)),
            alpha: 0.6.into(),
            ..ScatterOptions::default()
        };
        fig.subplot().scatter_with(x, y, options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("scatter/values_default_viridis", |fig| {
        let (x, y) = points(4, 80);
        let c: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x + y).collect();
        let options = ScatterOptions {
            colors: MappedColors::new(c).into(),
            ..ScatterOptions::default()
        };
        fig.subplot().scatter_with(x, y, options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("scatter/cmap_vmin_vmax_with_colorbar", |fig| {
        let ax = fig.subplot();
        let (x, y) = points(5, 80);
        let c: Vec<f64> = x.iter().map(|v| v * 10.0).collect();
        let options = ScatterOptions {
            colors: MappedColors::new(c)
                .with_cmap(colormaps::get("plasma")?)
                .with_limits(Some(2.0), Some(8.0))
                .into(),
            sizes: Some(vec![50.0]),
            ..ScatterOptions::default()
        };
        let points = ax.scatter_with(x, y, options)?;
        let options = ColorbarOptions {
            axes: ColorbarAxes::Parent(ax),
            label: "value".into(),
            ..ColorbarOptions::default()
        };
        fig.colorbar(&points, options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("scatter/lognorm_with_colorbar", |fig| {
        let ax = fig.subplot();
        let x = linspace(0.0, 1.0, 50);
        let y: Vec<f64> = x.iter().map(|v| v * v).collect();
        let c: Vec<f64> = x.iter().map(|v| 10f64.powf(4.0 * v)).collect();
        let options = ScatterOptions {
            colors: MappedColors::new(c)
                .with_cmap(colormaps::get("cividis")?)
                .with_norm(LogNorm::default())
                .into(),
            sizes: Some(vec![60.0]),
            ..ScatterOptions::default()
        };
        let points = ax.scatter_with(x, y, options)?;
        let options = ColorbarOptions {
            axes: ColorbarAxes::Parent(ax),
            ..ColorbarOptions::default()
        };
        fig.colorbar(&points, options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("scatter/twoslopenorm_diverging", |fig| {
        let ax = fig.subplot();
        let (x, y) = points(6, 60);
        let c: Vec<f64> = x.iter().map(|v| 10.0 * v - 3.0).collect();
        let options = ScatterOptions {
            colors: MappedColors::new(c)
                .with_cmap(colormaps::get("RdBu_r")?)
                .with_norm(TwoSlopeNorm::new(0.0, None, None)?)
                .into(),
            sizes: Some(vec![40.0]),
            ..ScatterOptions::default()
        };
        let points = ax.scatter_with(x, y, options)?;
        let options = ColorbarOptions {
            axes: ColorbarAxes::Parent(ax),
            orientation: Some(Orientation::Horizontal),
            ..ColorbarOptions::default()
        };
        fig.colorbar(&points, options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("scatter/markers", |fig| {
        let ax = fig.subplot();
        let markers = ["o", "s", "^", "D", "*", "P", "X", "h", "x", "+", "1", "|"];
        for (i, marker) in markers.into_iter().enumerate() {
            let options = ScatterOptions {
                marker: Some(marker.parse()?),
                sizes: Some(vec![80.0]),
                ..ScatterOptions::default()
            };
            ax.scatter_with(arange(0.0, 5.0, 1.0), vec![i as f64; 5], options)?;
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("scatter/facecolors_none_edgecolors", |fig| {
        let ax = fig.subplot();
        let (x, y) = points(7, 30);
        let hollow = ScatterOptions {
            sizes: Some(vec![120.0]),
            colors: ScatterColors::Single(Color::none()),
            edgecolors: Some(EdgeColors::Colors(vec!["tab:red".into()])),
            linewidths: Some(vec![2.0]),
            ..ScatterOptions::default()
        };
        ax.scatter_with(&x, &y, hollow)?;
        let outlined = ScatterOptions {
            sizes: Some(vec![60.0]),
            edgecolors: Some(EdgeColors::Colors(vec!["k".into()])),
            linewidths: Some(vec![0.5]),
            ..ScatterOptions::default()
        };
        ax.scatter_with(y, x, outlined)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("scatter/edgecolor_face_and_alpha", |fig| {
        let (x, y) = points(8, 60);
        let options = ScatterOptions {
            sizes: Some(vec![300.0]),
            colors: MappedColors::new(y.clone()).into(),
            edgecolors: Some(EdgeColors::Face),
            alpha: 0.4.into(),
            ..ScatterOptions::default()
        };
        fig.subplot().scatter_with(x, y, options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("scatter/explicit_color_list", |fig| {
        let ax = fig.subplot();
        let x = arange(0.0, 6.0, 1.0);
        let colors = ["red", "green", "blue", "orange", "purple", "0.3"];
        let listed = ScatterOptions {
            colors: ScatterColors::PerPoint(colors.into_iter().map(Color::from).collect()),
            sizes: Some(vec![100.0]),
            ..ScatterOptions::default()
        };
        ax.scatter_with(&x, x.iter().map(|v| v * v).collect::<Vec<_>>(), listed)?;
        let single = ScatterOptions {
            colors: ScatterColors::Single((0.1, 0.2, 0.5, 0.5).into()),
            sizes: Some(vec![100.0]),
            ..ScatterOptions::default()
        };
        ax.scatter_with(&x, x.iter().map(|v| v * 3.0).collect::<Vec<_>>(), single)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("scatter/unfilled_marker_colormapped", |fig| {
        let (x, y) = points(9, 50);
        let options = ScatterOptions {
            colors: MappedColors::new(x.clone())
                .with_cmap(colormaps::get("viridis")?)
                .into(),
            marker: Some("x".parse()?),
            sizes: Some(vec![60.0]),
            linewidths: Some(vec![2.0]),
            ..ScatterOptions::default()
        };
        fig.subplot().scatter_with(x, y, options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("scatter/plotnonfinite_bad_color", |fig| {
        let axes = fig.subplots(1, 2)?;
        let values = [1.0, 2.0, f64::NAN, 4.0, f64::INFINITY, 6.0, f64::NAN, 8.0];
        let cmap = colormaps::get("viridis")?.with_bad("red")?;
        let x = arange(0.0, 8.0, 1.0);
        for (ax, plot_nonfinite) in axes.as_slice().iter().zip([false, true]) {
            let options = ScatterOptions {
                colors: MappedColors::new(values).with_cmap(cmap.clone()).into(),
                sizes: Some(vec![80.0]),
                plot_nonfinite,
                ..ScatterOptions::default()
            };
            ax.scatter_with(&x, &x, options)?;
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("scatter/many_small_points", |fig| {
        let mut rng = SpecRng::new(10);
        let x = rng.normal(3000, 0.0, 1.0);
        let noise = rng.normal(3000, 0.0, 0.5);
        let y: Vec<f64> = x.iter().zip(noise).map(|(x, n)| 0.5 * x + n).collect();
        let options = ScatterOptions {
            sizes: Some(vec![2.0]),
            alpha: 0.5.into(),
            ..ScatterOptions::default()
        };
        fig.subplot().scatter_with(x, y, options)?;
        Ok(())
    }),
    FigureCase::new("scatter/log_axes", |fig| {
        let ax = fig.subplot();
        let x = logspace(0.0, 4.0, 30, 10.0);
        let y: Vec<f64> = x.iter().map(|v| v.powf(0.5)).collect();
        let options = ScatterOptions {
            colors: MappedColors::new(arange(0.0, 30.0, 1.0)).into(),
            ..ScatterOptions::default()
        };
        ax.scatter_with(x, y, options)?;
        ax.set_xscale("log")?;
        ax.set_yscale("log")?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
];
