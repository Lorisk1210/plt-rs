//! Shaded reliefs, spectrograms and sparsity patterns: `shading_example`,
//! `specgram_demo` and `spy_demos`.

use std::f64::consts::PI;

use plt::axes::{ImshowOptions, SpecgramOptions, SpyOptions};
use plt::colors::{BlendMode, Colorize, Colormap, LightSource, Relief, ShadeOptions, colormaps};
use plt::figure::{Figure, Share};
use plt::mlab::SpectralOptions;
use plt::numeric::{Grid, arange};

use super::data::{SEED, mgrid, mgrid_step};
use crate::figures::cases::common::{terrain, tickless};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `compare(z, cmap, ve)`: the data, its illumination and the two blend
/// modes, without ticks.
fn compare_shading(fig: &Figure, z: &Grid<f64>, cmap: &Colormap, ve: f64) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    for ax in axs.as_slice() {
        tickless(ax);
    }
    let ls = LightSource::new(315.0, 45.0);
    let relief = Relief {
        vert_exag: ve,
        ..Relief::default()
    };
    axs[(0, 0)].imshow(z.clone(), ImshowOptions::default().cmap(cmap.clone()))?;
    axs[(0, 0)].set_xlabel("Colormapped Data");
    let gray = ImshowOptions::default().cmap(colormaps::get("gray")?);
    axs[(0, 1)].imshow(ls.hillshade(z, relief)?, gray)?;
    axs[(0, 1)].set_xlabel("Illumination Intensity");
    for (ax, blend, label) in [
        (
            &axs[(1, 0)],
            BlendMode::Hsv,
            "Blend Mode: \"hsv\" (default)",
        ),
        (&axs[(1, 1)], BlendMode::Overlay, "Blend Mode: \"overlay\""),
    ] {
        let options = ShadeOptions {
            blend,
            relief,
            ..ShadeOptions::default()
        }
        .cmap(cmap.clone());
        let rgba = ls.shade(z, options)?;
        ax.imshow(rgba.map(|c| [c.r, c.g, c.b, c.a]), ImshowOptions::default())?;
        ax.set_xlabel(label);
    }
    Ok(())
}

fn shading_example_1(fig: &Figure) -> CaseResult {
    let steps = mgrid_step(-5.0, 5.0, 0.05);
    let (x, y) = mgrid(&steps, &steps);
    let z = Grid::from_fn(x.rows(), x.cols(), |r, c| {
        let r2 = x[(r, c)] * x[(r, c)] + y[(r, c)] * y[(r, c)];
        5.0 * (r2.sqrt() + r2.sin())
    });
    compare_shading(fig, &z, &colormaps::get("copper")?, 1.0)?;
    fig.suptitle("HSV Blending Looks Best with Smooth Surfaces")
        .set_y(0.95);
    Ok(())
}

fn shading_example_2(fig: &Figure) -> CaseResult {
    compare_shading(fig, &terrain(), &colormaps::get("gist_earth")?, 0.05)?;
    fig.suptitle("Overlay Blending Looks Best with Rough Surfaces")
        .set_y(0.95);
    Ok(())
}

fn specgram_demo(fig: &Figure) -> CaseResult {
    let dt = 0.0005;
    let t = arange(0.0, 20.5, dt);
    let noise = SpecRng::new(SEED).uniform(t.len(), 0.0, 1.0);
    let x: Vec<f64> = t
        .iter()
        .zip(noise)
        .map(|(&t, noise)| {
            let s1 = (2.0 * PI * 100.0 * t).sin();
            let s2 = if t <= 10.0 || 12.0 <= t {
                0.0
            } else {
                2.0 * (2.0 * PI * 400.0 * t).sin()
            };
            s1 + s2 + 0.01 * noise
        })
        .collect();
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    ax1.plot(&t, &x)?;
    ax1.set_ylabel("Signal");
    let options = SpecgramOptions {
        spectral: SpectralOptions {
            nfft: 1024,
            fs: 1.0 / dt,
            ..SpectralOptions::default()
        },
        ..SpecgramOptions::default()
    };
    ax2.specgram(&x, options)?;
    ax2.set_xlabel("Time (s)");
    ax2.set_ylabel("Frequency (Hz)");
    ax2.set_xlim(0.0, 20.0)?;
    Ok(())
}

fn spy_demos(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let values = SpecRng::new(SEED).normal(400, 0.0, 1.0);
    let x = Grid::from_fn(20, 20, |r, c| {
        if r == 5 || c == 12 {
            0.0
        } else {
            values[r * 20 + c]
        }
    });
    let spy = |markersize, precision| SpyOptions {
        markersize,
        precision,
        ..SpyOptions::default()
    };
    axs[(0, 0)].spy(&x, spy(Some(5.0), 0.0))?;
    axs[(0, 1)].spy(&x, spy(Some(5.0), 0.1))?;
    axs[(1, 0)].spy(&x, spy(None, 0.0))?;
    axs[(1, 1)].spy(&x, spy(None, 0.1))?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_images/shading_example_1", shading_example_1),
    FigureCase::new("gallery_images/shading_example_2", shading_example_2),
    FigureCase::new("gallery_images/specgram_demo", specgram_demo),
    FigureCase::new("gallery_images/spy_demos", spy_demos),
];
