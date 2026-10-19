//! The image tests of `test_backend_pdf.py`: the 14 standard PDF fonts,
//! hatched legend entries after empty labels, a masked grayscale image,
//! markers without colour, kerning, text needing fallback fonts, and the
//! scatter plots and hexagons the PDF backend leaves out when they lie
//! outside the canvas.

use std::f64::consts::TAU;

use plt::axes::{HexGridSize, HexbinOptions, ImshowOptions, MappedColors, ScatterOptions};
use plt::collections::CollectionProps;
use plt::colors::colormaps;
use plt::image::{ColorizerOptions, Interpolation};
use plt::legend::{LegendHandle, LegendOptions};
use plt::numeric::Grid;
use plt::prelude::*;
use plt::text::{HorizontalAlignment, VerticalAlignment};

use super::{SEED, classic};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `test_use14corefonts`: Helvetica, which the PDF viewer supplies; the
/// raster backend falls back to `DejaVu Sans`.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn use14corefonts(fig: &Figure) -> CaseResult {
    let text = "A three-line text positioned just above a blue line\n\
                and containing some French characters and the euro symbol:\n\
                \"Merci pépé pour les 10 €\"";
    let ax = fig.subplot();
    ax.set_title("Test PDF backend with option use14corefonts=True");
    ax.text(0.5, 0.5, text)
        .set_halign(HorizontalAlignment::Center)
        .set_valign(VerticalAlignment::Bottom)
        .set_fontsize(14.0);
    ax.axhline(0.5, LineProps::default().linewidth(0.5));
    Ok(())
}

/// `test_hatching_legend`: hatched entries after empty labels.
fn hatching_legend(fig: &Figure) -> CaseResult {
    let hatched =
        |color: &str| LegendHandle::patch(PatchProps::default().facecolor(color).hatch("XXXX"));
    let (a, b) = (hatched("green"), hatched("blue"));
    fig.legend(
        LegendOptions::default()
            .handles([a.clone(), b.clone(), a, b])
            .labels(["", "", "", ""]),
    )?;
    Ok(())
}

/// `test_grayscale_alpha`: a Gaussian with its low values masked (NaN).
fn grayscale_alpha(fig: &Figure) -> CaseResult {
    // `np.ogrid[-2:2:.1]`: `i * 0.1 - 2` for 40 values of `i`.
    let grid = |i: usize| i as f64 * 0.1 - 2.0;
    let dd = Grid::from_fn(40, 40, |r, c| {
        let value = (-(grid(r).powi(2) + grid(c).powi(2))).exp();
        if value < 0.1 { f64::NAN } else { value }
    });
    let ax = fig.subplot();
    ax.imshow(
        dd,
        ImshowOptions {
            interpolation: Some(Interpolation::None),
            colors: ColorizerOptions::default().cmap(colormaps::get("gray_r")?),
            ..ImshowOptions::default()
        },
    )?;
    ax.set_xticks(Vec::<f64>::new());
    ax.set_yticks(Vec::<f64>::new());
    Ok(())
}

/// `test_pdf_eps_savefig_when_color_is_none`: markers of no colour.
fn savefig_when_color_is_none(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_axis_off();
    let y: Vec<f64> = linspace(-5.0, 5.0, 100).iter().map(|x| x.sin()).collect();
    let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
    let props = LineProps::default()
        .marker("v".parse::<Marker>()?)
        .linestyle(LineStyle::None)
        .color("none");
    ax.plot_with(x, y, props)?;
    Ok(())
}

/// `test_kerning`: a string kerned at 5 and 20 points.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn kerning(fig: &Figure) -> CaseResult {
    let s = "AVAVAVAVAVAVAVAV€AAVV";
    fig.text(0.0, 0.25, s).set_fontsize(5.0);
    fig.text(0.0, 0.75, s).set_fontsize(20.0);
    Ok(())
}

/// The scatter culling tests: points coloured by `c` with markers of
/// `s` points² in a 0..10 square.
fn scatter_on_square(fig: &Figure, x: Vec<f64>, y: Vec<f64>, c: Vec<f64>, s: f64) -> CaseResult {
    let ax = fig.subplot();
    ax.scatter_with(x, y, mapped(c, s))?;
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    Ok(())
}

/// `scatter(c=c, s=s)`.
fn mapped(c: Vec<f64>, s: f64) -> ScatterOptions {
    ScatterOptions {
        sizes: Some(vec![s]),
        colors: MappedColors::new(c).into(),
        ..ScatterOptions::default()
    }
}

/// `rng.random(n) * scale + shift`.
fn random(rng: &mut SpecRng, n: usize, scale: f64, shift: f64) -> Vec<f64> {
    rng.uniform(n, 0.0, 1.0)
        .into_iter()
        .map(|u| u * scale + shift)
        .collect()
}

/// `test_scatter_offaxis_colored_visual`: every point in view.
fn scatter_offaxis_colored_visual(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let x = random(&mut rng, 100, 5.0, 0.0);
    let y = random(&mut rng, 100, 5.0, 0.0);
    let c = rng.uniform(100, 0.0, 1.0);
    scatter_on_square(fig, x, y, c, 50.0)
}

/// `test_scatter_mixed_onoff_axis`: half of the points out of view.
fn scatter_mixed_onoff_axis(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let mut x = random(&mut rng, 50, 5.0, 0.0);
    let mut y = random(&mut rng, 50, 5.0, 0.0);
    x.extend(random(&mut rng, 50, 5.0, 15.0));
    y.extend(random(&mut rng, 50, 5.0, 15.0));
    let c = rng.uniform(100, 0.0, 1.0);
    scatter_on_square(fig, x, y, c, 50.0)
}

/// `test_scatter_large_markers_partial_clip`: markers centred outside
/// the view reaching into it.
fn scatter_large_markers_partial_clip(fig: &Figure) -> CaseResult {
    scatter_on_square(
        fig,
        vec![-0.5, 10.5, 5.0],
        vec![5.0, 5.0, -0.5],
        vec![0.2, 0.5, 0.8],
        500.0,
    )
}

/// `test_scatter_logscale`: four decades of points, one in view.
fn scatter_logscale(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let mut decades = |n| -> Vec<f64> {
        random(&mut rng, n, 4.0, 0.0)
            .into_iter()
            .map(|e| 10f64.powf(e))
            .collect()
    };
    let (x, y) = (decades(50), decades(50));
    let c = rng.uniform(50, 0.0, 1.0);
    let ax = fig.subplot();
    ax.scatter_with(x, y, mapped(c, 50.0))?;
    ax.set_xscale("log")?.set_yscale("log")?;
    ax.set_xlim(100.0, 1000.0).set_ylim(100.0, 1000.0);
    Ok(())
}

/// `test_scatter_polar`: points out to radius 3 with the view cut at 2.
fn scatter_polar(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let theta = random(&mut rng, 50, TAU, 0.0);
    let r = random(&mut rng, 50, 3.0, 0.0);
    let c = rng.uniform(50, 0.0, 1.0);
    let ax = fig.polar_subplot();
    ax.scatter_with(theta, r, mapped(c, 50.0))?;
    ax.set_ylim(0.0, 2.0);
    Ok(())
}

/// `test_hexbin_negative_offsets`: hexagons around the origin.
fn hexbin_negative_offsets(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let x = rng.normal(10_000, 0.0, 1.0);
    let y = rng.normal(10_000, 0.0, 1.0);
    let ax = fig.subplot();
    let options = HexbinOptions {
        gridsize: HexGridSize::Count(25),
        props: CollectionProps::default().edgecolor("none"),
        ..HexbinOptions::default()
    };
    ax.hexbin(&x, &y, options)?;
    ax.set_xlim(-2.0, 2.0).set_ylim(-2.0, 2.0);
    ax.set_axis_off();
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_backends/pdf_use14corefonts", use14corefonts).with_rc_setup(|rc| {
        rc.set("pdf.use14corefonts", "True")?;
        rc.set("font.family", "sans-serif")?;
        rc.set("font.size", "8")?;
        rc.set("font.sans-serif", "Helvetica")?;
        rc.set("pdf.compression", "0")?;
        Ok(())
    }),
    FigureCase::new("mpl_backends/pdf_hatching_legend", hatching_legend).sized(1.0, 2.0, 100.0),
    classic("mpl_backends/pdf_grayscale_alpha", grayscale_alpha),
    FigureCase::new(
        "mpl_backends/pdf_eps_savefig_when_color_is_none",
        savefig_when_color_is_none,
    ),
    FigureCase::new("mpl_backends/pdf_kerning", kerning),
    classic(
        "mpl_backends/pdf_scatter_offaxis_colored_visual",
        scatter_offaxis_colored_visual,
    ),
    classic(
        "mpl_backends/pdf_scatter_mixed_onoff_axis",
        scatter_mixed_onoff_axis,
    ),
    classic(
        "mpl_backends/pdf_scatter_large_markers_partial_clip",
        scatter_large_markers_partial_clip,
    ),
    classic("mpl_backends/pdf_scatter_logscale", scatter_logscale),
    classic("mpl_backends/pdf_scatter_polar", scatter_polar),
    classic(
        "mpl_backends/pdf_hexbin_negative_offsets",
        hexbin_negative_offsets,
    ),
];
