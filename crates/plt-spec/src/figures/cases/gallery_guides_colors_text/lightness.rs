//! The lightness of the colormaps in `colormaps`: a port of
//! colorspacious' CAM02-UCS lightness (`_lightness`; colorspacious is not
//! part of the reference environment), plotted as bands of each colormap's
//! colours and as grey strips beside the colormaps.

use plt::axes::{AxisName, ImshowOptions, MappedColors, ScatterOptions, TickParams, TickPosition};
use plt::colors::{ColorizerOptions, colormaps};
use plt::fonts::FontSize;
use plt::gridspec::SubplotOverrides;
use plt::layout::TightLayout;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::text::{HAlign, VAlign};
use plt::ticker::{FixedFormatter, FixedLocator};

use super::colormaps::{Category, strip};
use crate::figures::CaseResult;

/// A 3 × 3 matrix, row by row.
type Matrix = [[f64; 3]; 3];

/// The sRGB to XYZ matrix colorspacious uses (XYZ in `[0, 1]`).
const SRGB_TO_XYZ: Matrix = [
    [0.4124, 0.3576, 0.1805],
    [0.2126, 0.7152, 0.0722],
    [0.0193, 0.1192, 0.9505],
];

/// CIECAM02's chromatic adaptation matrix.
const CAT02: Matrix = [
    [0.7328, 0.4296, -0.1624],
    [-0.7036, 1.6975, 0.0061],
    [0.0030, 0.0136, 0.9834],
];

/// The Hunt-Pointer-Estevez cone response matrix.
const HPE: Matrix = [
    [0.38971, 0.68898, -0.07868],
    [-0.22981, 1.18340, 0.04641],
    [0.00000, 0.00000, 1.00000],
];

/// The D65 white point, in percent.
const D65: [f64; 3] = [95.047, 100.0, 108.883];

/// `m @ v`.
fn apply(m: &Matrix, v: [f64; 3]) -> [f64; 3] {
    m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

/// `a @ b`.
fn product(a: &Matrix, b: &Matrix) -> Matrix {
    a.map(|row| std::array::from_fn(|c| row[0] * b[0][c] + row[1] * b[1][c] + row[2] * b[2][c]))
}

/// The inverse of `m`, through its adjugate.
fn inverse(m: &Matrix) -> Matrix {
    let cofactor = |r: usize, c: usize| {
        let (r1, r2) = ((r + 1) % 3, (r + 2) % 3);
        let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
        m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
    };
    let det = (0..3).map(|c| m[0][c] * cofactor(0, c)).sum::<f64>();
    std::array::from_fn(|r| std::array::from_fn(|c| cofactor(c, r) / det))
}

/// The CIECAM02 viewing conditions of sRGB: a 64 lux surround (adapting
/// luminance `64 / π / 5`), background luminance 20, an average surround.
struct Viewing {
    /// Luminance-level adaptation factor `F_L`.
    f_l: f64,
    /// Per-channel degree of adaptation.
    d_rgb: [f64; 3],
    /// Background induction factor `N_bb`.
    n_bb: f64,
    /// The exponent of the lightness, `c z`.
    exponent: f64,
    /// Adapted CAT02 responses to cone responses.
    to_cones: Matrix,
}

impl Viewing {
    fn srgb() -> Self {
        let (l_a, y_b, f, c) = (64.0 / std::f64::consts::PI / 5.0, 20.0, 1.0, 0.69);
        let k = 1.0 / (5.0 * l_a + 1.0);
        let k4 = k.powf(4.0);
        let f_l = 0.2 * k4 * (5.0 * l_a) + 0.1 * (1.0 - k4).powf(2.0) * (5.0 * l_a).powf(1.0 / 3.0);
        let d = f * (1.0 - (1.0 / 3.6) * ((-l_a - 42.0) / 92.0).exp());
        let d_rgb = apply(&CAT02, D65).map(|white| d * D65[1] / white + 1.0 - d);
        let n = y_b / D65[1];
        let z = 1.48 + n.sqrt();
        Self {
            f_l,
            d_rgb,
            n_bb: 0.725 * (1.0 / n).powf(0.2),
            exponent: c * z,
            to_cones: product(&HPE, &inverse(&CAT02)),
        }
    }

    /// The achromatic response `A` to `xyz` (in percent); negative
    /// responses (pure black, through rounding) count as zero.
    fn achromatic(&self, xyz: [f64; 3]) -> f64 {
        let cat = apply(&CAT02, xyz);
        let adapted: [f64; 3] = std::array::from_fn(|i| self.d_rgb[i] * cat[i]);
        let cones = apply(&self.to_cones, adapted).map(|cone| {
            let t = (self.f_l * cone.abs() / 100.0).powf(0.42);
            cone.signum() * 400.0 * (t / (t + 27.13)) + 0.1
        });
        ((cones[0] * 2.0 + cones[1] + cones[2] * (1.0 / 20.0) - 0.305) * self.n_bb).max(0.0)
    }
}

/// The CAM02-UCS lightness J' of the sRGB colour `rgb` (colorspacious'
/// `cspace_converter("sRGB1", "CAM02-UCS")`, first channel).
fn cam02ucs_lightness(viewing: &Viewing, rgb: [f64; 3]) -> f64 {
    let linear = rgb.map(|c| {
        if c < 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    let xyz = apply(&SRGB_TO_XYZ.map(|row| row.map(|v| 100.0 * v)), linear);
    let j = 100.0 * (viewing.achromatic(xyz) / viewing.achromatic(D65)).powf(viewing.exponent);
    1.7 * j / (1.0 + 0.007 * j)
}

/// The lightness of the colormap `name` at `x`.
fn colormap_lightness(name: &str, x: &[f64]) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let viewing = Viewing::srgb();
    let cmap = colormaps::get(name)?;
    Ok(x.iter()
        .map(|&v| {
            let color = cmap.sample(v);
            cam02ucs_lightness(&viewing, [color.r, color.g, color.b])
        })
        .collect())
}

/// The lightness figure of a category: the lightness of each colormap as
/// a band of its colours, a few colormaps per subplot.
pub(super) fn lightness(fig: &Figure, category: &Category) -> CaseResult {
    let x = linspace(0.0, 1.0, 100);
    let axs = fig.subplots(category.subplots(), 1)?;
    let axs = axs.as_slice();
    let sequential = category.name == "Sequential";
    let ends_labelled = sequential || category.name == "Perceptually Uniform Sequential";
    for (ax, names) in axs.iter().zip(category.cmaps.chunks(category.per_subplot)) {
        let mut locs = Vec::new();
        for (j, &name) in names.iter().enumerate() {
            let offset = j as f64 * category.spacing;
            let mut y = colormap_lightness(name, &x)?;
            let mut c = x.clone();
            if sequential {
                y.reverse();
                c.reverse();
            }
            let shifted: Vec<f64> = x.iter().map(|v| v + offset).collect();
            ax.scatter_with(
                shifted,
                y,
                ScatterOptions {
                    sizes: Some(vec![300.0]),
                    colors: MappedColors::new(c).cmap(colormaps::get(name)?).into(),
                    linewidths: Some(vec![0.0]),
                    ..ScatterOptions::default()
                },
            )?;
            locs.push(if ends_labelled { x[99] } else { x[50] } + offset);
        }
        let (left, right) = axs[0].xlim();
        ax.set_xlim(left, right)?.set_ylim(0.0, 100.0)?;
        let xaxis = ax.xaxis();
        xaxis.set_ticks_position(TickPosition::Side(SpineSide::Top))?;
        xaxis.set_major_locator(FixedLocator::new(locs));
        xaxis.set_major_formatter(FixedFormatter::new(
            names.iter().map(|&name| name.to_owned()).collect(),
        ));
        ax.tick_params(&TickParams {
            axis: Some(AxisName::X),
            labelrotation: Some(50.0),
            ..TickParams::default()
        });
        ax.set_ylabel("Lightness $L^*$");
        ax.yaxis().set_label_fontsize(FontSize::Points(12.0));
    }
    let last = axs.last().ok_or("no subplots")?;
    last.set_xlabel(format!("{} colormaps", category.name));
    last.xaxis().set_label_fontsize(FontSize::Points(14.0));
    fig.tight_layout(&TightLayout {
        pad: 1.5,
        h_pad: Some(0.0),
        ..TightLayout::default()
    })?;
    Ok(())
}

/// The second `plot_color_gradients`: each colormap of the category
/// beside the grey of its lightness (rounded to single precision, as the
/// guide stores it).
pub(super) fn grayscale(fig: &Figure, category: &Category) -> CaseResult {
    let x = linspace(0.0, 1.0, 100);
    let axs = fig.subplots(category.cmaps.len(), 2)?;
    fig.subplots_adjust(&SubplotOverrides {
        top: Some(0.95),
        bottom: Some(0.01),
        left: Some(0.2),
        right: Some(0.99),
        wspace: Some(0.05),
        ..SubplotOverrides::default()
    })?;
    fig.suptitle(format!("{} colormaps", category.name))
        .set_fontsize(14.0)
        .set_y(1.0)
        .set_x(0.6);
    for (row, &name) in category.cmaps.iter().enumerate() {
        let single: Vec<f64> = colormap_lightness(name, &x)?
            .into_iter()
            .map(|l| f64::from(l as f32))
            .collect();
        let grey = Grid::from_fn(3, single.len(), |_, c| single[c]);
        let (colors, lightness) = (&axs[(row, 0)], &axs[(row, 1)]);
        strip(colors, name)?;
        let binary = ColorizerOptions::default()
            .cmap(colormaps::get("binary_r")?)
            .limits(Some(0.0), Some(100.0));
        lightness.imshow(
            grey,
            ImshowOptions {
                aspect: Some(Aspect::Auto),
                ..ImshowOptions::default().colorizer(binary)
            },
        )?;
        let (x0, y0, _, height) = colors.position().bounds();
        fig.text(x0 - 0.01, y0 + height / 2.0, name)
            .set_valign(VAlign::Center)
            .set_halign(HAlign::Right)
            .set_fontsize(10.0);
    }
    for ax in axs.as_slice() {
        ax.set_axis_off();
    }
    Ok(())
}
