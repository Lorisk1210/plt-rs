//! Computed pictures: bars drawn as stretched images of a present and the
//! Mandelbrot set, shaded (`demo_ribbon_box` and `mandelbrot`).

use plt::axes::ImshowOptions;
use plt::colors::{
    BlendMode, ColorizerOptions, LightSource, PowerNorm, Relief, ShadeOptions, colormaps,
};
use plt::image::{Extent, Interpolation};
use plt::numeric::Grid;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{AnnotateOptions, HAlign, VAlign};
use plt::transforms::Point;

use crate::figures::{CaseResult, FigureCase};

/// A 128 × 128 RGBA picture of a wrapped present in place of
/// `Minduka_Present_Blue_Pack.png` (`_present`): 8-bit channels, shaded
/// like a cylinder, a pale ribbon down the middle and a tapering bow on
/// top, divided by 255.
fn present() -> Grid<[f64; 4]> {
    Grid::from_fn(128, 128, |y, x| {
        let centre = (2 * x).abs_diff(127);
        let blue = 255 - centre * centre * 160 / 16129;
        let red = if centre < 24 { blue } else { blue / 4 };
        let alpha = if y >= 24 || centre < 3 * y { 255 } else { 0 };
        [red, red, blue, alpha].map(|channel| channel as f64 / 255.0)
    })
}

/// The row of the present repeated to stretch it (`cut_location`).
const CUT_LOCATION: usize = 70;

/// `RibbonBox(color).get_stretched_image(stretch)`: the present in
/// `color`, its middle row repeated so that it is `stretch` times as
/// tall (at least as tall as the picture).
fn ribbon_box(color: [f64; 3], stretch: f64) -> Grid<[f64; 4]> {
    let picture = present();
    let tinted = picture.map(|&[red, _, blue, alpha]| {
        let shade = blue - red;
        [
            blue - shade * (1.0 - color[0]),
            blue - shade * (1.0 - color[1]),
            blue - shade * (1.0 - color[2]),
            alpha,
        ]
    });
    let rows = (128.0 * stretch.max(1.0)) as usize;
    let extra = rows - 128;
    Grid::from_fn(rows, 128, |r, c| {
        let source = if r < CUT_LOCATION {
            r
        } else if r < CUT_LOCATION + extra {
            CUT_LOCATION
        } else {
            r - extra
        };
        tinted[(source, c)]
    })
}

fn demo_ribbon_box(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let years = [2004.0, 2005.0, 2006.0, 2007.0, 2008.0];
    let heights = [7900.0, 8100.0, 7900.0, 6900.0, 2800.0];
    let box_colors = [
        [0.8, 0.2, 0.2],
        [0.2, 0.8, 0.2],
        [0.2, 0.2, 0.8],
        [0.7, 0.5, 0.8],
        [0.3, 0.8, 0.7],
    ];
    ax.set_xlim(years[0] - 0.5, years[4] + 0.5)
        .set_ylim(0.0, 10000.0);
    // The boxes are stretched to the shape they have on the canvas, as the
    // example's image class does when it is drawn.
    fig.layout();
    let to_display = fig.resolve_transform(&ax.trans_data())?;
    for ((&year, &height), color) in years.iter().zip(&heights).zip(box_colors) {
        let low = to_display.transform_point(Point::new(year - 0.4, 0.0));
        let high = to_display.transform_point(Point::new(year + 0.4, height));
        let stretch = (high.y - low.y) / (high.x - low.x);
        // The example's images are `AxesImage`s made directly, which do not
        // resample (`imshow` follows `image.resample`).
        let options = ImshowOptions {
            aspect: Some(Aspect::Auto),
            extent: Some(Extent::new(year - 0.4, year + 0.4, 0.0, height)),
            resample: Some(false),
            ..ImshowOptions::default()
        }
        .interpolation(Interpolation::Bicubic);
        ax.imshow(ribbon_box(color, stretch), options)?
            .set_zorder(1.0);
        ax.annotate(
            height.to_string(),
            (year, height),
            AnnotateOptions::default(),
        )?
        .set_valign(VAlign::Bottom)
        .set_halign(HAlign::Center);
    }
    let gradient = Grid::from_fn(2, 2, |r, c| {
        let alpha = [[0.1, 0.3], [0.3, 0.5]][r][c];
        [1.0, 1.0, 0.0, alpha]
    });
    let options = ImshowOptions {
        extent: Some(Extent::new(0.0, 1.0, 0.0, 1.0)),
        transform: Some(ax.trans_axes()),
        ..ImshowOptions::default()
    }
    .interpolation(Interpolation::Bicubic);
    ax.imshow(gradient, options)?.set_zorder(0.1);
    Ok(())
}

/// `mandelbrot_set`: the last value of the iteration `z ← z² + c` from
/// `z = 0` over a grid of single-precision `c`, and the iteration at which
/// it reached `horizon` (0 for points that never did), computed in single
/// precision as numpy does, the real part of `z²` with a fused
/// multiply-add.
fn mandelbrot_set(
    (xmin, xmax, xn): (f64, f64, usize),
    (ymin, ymax, yn): (f64, f64, usize),
    maxiter: usize,
    horizon: f32,
) -> Grid<(f32, usize)> {
    let x: Vec<f32> = linspace(xmin, xmax, xn).iter().map(|&v| v as f32).collect();
    let y: Vec<f32> = linspace(ymin, ymax, yn).iter().map(|&v| v as f32).collect();
    Grid::from_fn(yn, xn, |row, col| {
        let (cr, ci) = (x[col], y[row]);
        let (mut zr, mut zi) = (0.0_f32, 0.0_f32);
        let mut escape = 0;
        for n in 0..maxiter {
            if zr.hypot(zi) >= horizon {
                break;
            }
            escape = n;
            let real = zr.mul_add(zr, -(zi * zi));
            let imag = zr * zi + zi * zr;
            (zr, zi) = (real + cr, imag + ci);
        }
        let escape = if escape == maxiter - 1 { 0 } else { escape };
        (zr.hypot(zi), escape)
    })
}

fn mandelbrot(fig: &Figure) -> CaseResult {
    let (xmin, xmax, xn) = (-2.25, 0.75, 3000 / 2);
    let (ymin, ymax, yn) = (-1.25, 1.25, 2500 / 2);
    let maxiter = 200;
    let horizon = 2.0_f64.powi(40);
    let log_horizon = horizon.ln().log2();
    let set = mandelbrot_set((xmin, xmax, xn), (ymin, ymax, yn), maxiter, horizon as f32);
    // The normalized recount, NaN as 0 and infinities as the largest
    // finite values (`np.nan_to_num`).
    let recount = set.map(|&(magnitude, escape)| {
        let smooth = (escape + 1) as f64 - f64::from(magnitude.ln().log2()) + log_horizon;
        if smooth.is_nan() {
            0.0
        } else {
            smooth.clamp(f64::MIN, f64::MAX)
        }
    });
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    ax.set_frame_on(false);
    ax.set_aspect(Aspect::Ratio(1.0))?;
    let light = LightSource::new(315.0, 10.0);
    let options = ShadeOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("hot")?),
            norm: Some(Box::new(PowerNorm::new(0.3))),
            ..ColorizerOptions::default()
        },
        blend: BlendMode::Hsv,
        relief: Relief {
            vert_exag: 1.5,
            ..Relief::default()
        },
    };
    let shaded = light.shade(&recount, &options)?;
    ax.imshow(
        shaded.map(|c| [c.r, c.g, c.b, c.a]),
        ImshowOptions {
            extent: Some(Extent::new(xmin, xmax, ymin, ymax)),
            ..ImshowOptions::default()
        }
        .interpolation(Interpolation::Bicubic),
    )?;
    ax.set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
    let text = ax.text(
        xmin + 0.025,
        ymin + 0.025,
        "The Mandelbrot fractal set\nRendered with matplotlib - https://matplotlib.org",
    );
    text.set_color("white").set_fontsize(12.0);
    text.set_alpha(Some(0.5));
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_misc/demo_ribbon_box", demo_ribbon_box),
    FigureCase::new("gallery_misc/mandelbrot", mandelbrot).sized(
        10.0,
        10.0 * 1250.0 / 1500.0,
        72.0,
    ),
];
