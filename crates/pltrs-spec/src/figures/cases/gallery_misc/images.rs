//! Images and contours: contour sets given as polygons, hillshaded
//! surfaces and terrain, and an MRI slice beside its histogram and EEG
//! traces (`contour_manual`, `advanced_hillshading`,
//! `topographic_hillshading` and `mri_with_eeg`).

use pltrs::axes::{HistPlotOptions, ImshowOptions};
use pltrs::colorbar::ColorbarOptions;
use pltrs::colors::{
    BlendMode, ColorizerOptions, Colormap, LightSource, Normalize, Relief, ScalarMappable,
    ShadeOptions, colormaps,
};
use pltrs::contour::ContourOptions;
use pltrs::coords::Coords;
use pltrs::figure::MosaicOptions;
use pltrs::fonts::{FontSize, RelativeSize};
use pltrs::gridspec::SubplotOverrides;
use pltrs::image::{Interpolation, ScalarField};
use pltrs::layout::LayoutEngine;
use pltrs::numeric::Grid;
use pltrs::prelude::*;
use pltrs::stats::Bins;
use pltrs::text::{AnnotateOptions, HAlign, VAlign};
use pltrs::transforms::Point;
use pltrs_core::{Path, PathCode};

use super::{named_mosaic, panel};
use crate::figures::cases::common::{GALLERY_SEED, mgrid_count, scan, segment, terrain, tickless};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

fn contour_manual_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let filled = [
        vec![segment(&[
            (0.0, 0.0),
            (0.0, 4.0),
            (1.0, 3.0),
            (1.0, 2.0),
            (2.0, 0.0),
        ])],
        vec![
            segment(&[(2.0, 0.0), (3.0, 0.0), (3.0, 2.0), (1.0, 3.0), (1.0, 2.0)]),
            segment(&[(1.0, 4.0), (3.0, 4.0), (3.0, 3.0)]),
        ],
    ];
    let cs = ax.contour_segments(
        &[0.0, 1.0, 2.0],
        &filled,
        true,
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("bone")?),
                ..ColorizerOptions::default()
            },
            ..ContourOptions::default()
        },
    )?;
    let cbar = fig.colorbar(&cs, ColorbarOptions::default())?;
    let lines = [
        vec![segment(&[(0.0, 0.0), (0.0, 4.0)])],
        vec![segment(&[(2.0, 0.0), (1.0, 2.0), (1.0, 3.0)])],
        vec![
            segment(&[(3.0, 0.0), (3.0, 2.0)]),
            segment(&[(3.0, 3.0), (3.0, 4.0)]),
        ],
    ];
    let lines = ax.contour_segments(
        &[0.0, 1.0, 2.0],
        &lines,
        false,
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("cool")?),
                ..ColorizerOptions::default()
            },
            linewidths: Some(vec![3.0]),
            ..ContourOptions::default()
        },
    )?;
    cbar.add_lines(&lines, true)?;
    ax.set_xlim(-0.5, 3.5)?.set_ylim(-0.5, 4.5)?;
    ax.set_title("User-specified contours");
    Ok(())
}

fn contour_manual_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let (m, l) = (PathCode::MoveTo, PathCode::LineTo);
    let outline = [
        (0.0, 0.0),
        (3.0, 0.0),
        (3.0, 3.0),
        (0.0, 3.0),
        (1.0, 1.0),
        (1.0, 2.0),
        (2.0, 2.0),
        (2.0, 1.0),
    ];
    let with_hole = Path::new(
        outline.iter().map(|&(x, y)| Point::new(x, y)).collect(),
        vec![m, l, l, l, m, l, l, l],
    )?;
    let cs = ax.contour_segments(
        &[0.0, 1.0],
        &[vec![with_hole]],
        true,
        ContourOptions::default(),
    )?;
    fig.colorbar(&cs, ColorbarOptions::default())?;
    ax.set_xlim(-0.5, 3.5)?.set_ylim(-0.5, 3.5)?;
    ax.set_title("User specified filled contours with holes");
    Ok(())
}

/// `np.mgrid[-4:2:200j, -4:2:200j]` as `(y, x)` grids.
fn square_grid() -> (Grid<f64>, Grid<f64>) {
    let v = mgrid_count(-4.0, 2.0, 200);
    (
        Grid::from_fn(200, 200, |r, _| v[r]),
        Grid::from_fn(200, 200, |_, c| v[c]),
    )
}

/// `10 cos(x² + y²)` over [`square_grid`].
fn cos_rings() -> Grid<f64> {
    let (y, x) = square_grid();
    Grid::from_fn(200, 200, |r, c| {
        let (x, y) = (x[(r, c)], y[(r, c)]);
        10.0 * (x * x + y * y).cos()
    })
}

/// RGBA colours as an image.
fn rgba_image(colors: &Grid<pltrs_core::Rgba>) -> Grid<[f64; 4]> {
    colors.map(|c| [c.r, c.g, c.b, c.a])
}

/// Bilinear interpolation (`interpolation='bilinear'`).
fn bilinear() -> ImshowOptions {
    ImshowOptions {
        interpolation: Some(Interpolation::Bilinear),
        ..ImshowOptions::default()
    }
}

/// The smallest and largest values of `z`.
fn extent_of(z: &Grid<f64>) -> (f64, f64) {
    z.as_slice()
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        })
}

fn advanced_hillshading_1(fig: &Figure) -> CaseResult {
    let z = cos_rings();
    let cmap = colormaps::get("copper")?;
    let rgb =
        LightSource::new(315.0, 45.0).shade(&z, ShadeOptions::default().cmap(cmap.clone()))?;
    let ax = fig.subplot();
    ax.imshow(rgba_image(&rgb), bilinear())?;
    // The colorbar of the data the colours stand for.
    let (vmin, vmax) = extent_of(&z);
    let proxy = ScalarMappable::with_norm(cmap, Box::new(Normalize::new(vmin, vmax)));
    fig.colorbar(&proxy, ColorbarOptions::beside(&ax))?;
    ax.set_title("Using a colorbar with a shaded plot")
        .set_fontsize(RelativeSize::XLarge);
    Ok(())
}

fn advanced_hillshading_2(fig: &Figure) -> CaseResult {
    let mut z = cos_rings();
    z[(100, 105)] = 2000.0;
    z[(120, 110)] = -9000.0;
    let ls = LightSource::new(315.0, 45.0);
    let copper = colormaps::get("copper")?;
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax1.imshow(
        rgba_image(&ls.shade(&z, ShadeOptions::default().cmap(copper.clone()))?),
        bilinear(),
    )?;
    ax1.set_title("Full range of data");
    let limited = ShadeOptions {
        colorizer: ColorizerOptions {
            cmap: Some(copper),
            vmin: Some(-10.0),
            vmax: Some(10.0),
            ..ColorizerOptions::default()
        },
        ..ShadeOptions::default()
    };
    ax2.imshow(rgba_image(&ls.shade(&z, limited)?), bilinear())?;
    ax2.set_title("Manually set range");
    fig.suptitle("Avoiding Outliers in Shaded Plots")
        .set_fontsize(RelativeSize::XLarge);
    Ok(())
}

fn advanced_hillshading_3(fig: &Figure) -> CaseResult {
    let (y, x) = square_grid();
    let z1 = Grid::from_fn(200, 200, |r, c| (x[(r, c)] * x[(r, c)]).sin());
    let z2 = Grid::from_fn(200, 200, |r, c| {
        let (x, y) = (x[(r, c)], y[(r, c)]);
        (x * x + y * y).cos()
    });
    let (low, high) = extent_of(&z2);
    let mut colored =
        ScalarMappable::with_norm(colormaps::get("RdBu")?, Box::new(Normalize::new(low, high)));
    let values: Vec<Option<f64>> = z2.as_slice().iter().map(|&v| Some(v)).collect();
    let colors = colored.to_rgba(&values, None)?;
    let rgb = Grid::from_vec(200, 200, colors.iter().map(|c| [c.r, c.g, c.b]).collect())?;
    let shaded =
        LightSource::new(315.0, 45.0).shade_rgb(&rgb, &z1, BlendMode::Hsv, Relief::default())?;
    let ax = fig.subplot();
    ax.imshow(shaded, bilinear())?;
    ax.set_title("Shade by one variable, color by another")
        .set_fontsize(RelativeSize::XLarge);
    Ok(())
}

/// A text of `points` points.
fn sized(points: f64) -> FontSize {
    FontSize::Points(points)
}

fn topographic_hillshading(fig: &Figure) -> CaseResult {
    // The cell size and latitude of `jacksboro_fault_dem.npz`.
    let z = terrain();
    let cell = 0.000_833_333_333_333;
    let dy = 111_200.0 * cell;
    let dx = 111_200.0 * cell * 36.732_916_666_666_7_f64.to_radians().cos();
    let ls = LightSource::new(315.0, 45.0);
    let cmap: Colormap = colormaps::get("gist_earth")?;
    let axs = fig.subplots(4, 3)?;
    for ax in axs.as_slice() {
        tickless(ax);
    }
    let gray = colormaps::get("gray")?;
    for (col, vert_exag) in [0.1, 1.0, 10.0].into_iter().enumerate() {
        let relief = Relief {
            vert_exag,
            dx,
            dy,
            ..Relief::default()
        };
        axs[(0, col)].imshow(
            ls.hillshade(&z, relief)?,
            ImshowOptions::default().cmap(gray.clone()),
        )?;
        for (row, blend) in [BlendMode::Hsv, BlendMode::Overlay, BlendMode::SoftLight]
            .into_iter()
            .enumerate()
        {
            let options = ShadeOptions {
                blend,
                relief,
                ..ShadeOptions::default()
            }
            .cmap(cmap.clone());
            let rgb = ls.shade(&z, options)?;
            axs[(row + 1, col)].imshow(rgba_image(&rgb), ImshowOptions::default())?;
        }
        axs[(0, col)]
            .set_title(format!("{vert_exag}"))
            .set_fontsize(sized(18.0));
    }
    for (row, mode) in ["Hillshade", "hsv", "overlay", "soft"]
        .into_iter()
        .enumerate()
    {
        let ax = &axs[(row, 0)];
        ax.set_ylabel(mode);
        ax.yaxis().set_label_fontsize(sized(18.0));
    }
    let axes_fraction =
        || -> Result<Coords, Box<dyn std::error::Error>> { Ok("axes fraction".parse()?) };
    axs[(0, 1)]
        .annotate(
            "Vertical Exaggeration",
            (0.5, 1.0),
            AnnotateOptions {
                xytext: Some(Point::new(0.0, 30.0).into()),
                xycoords: axes_fraction()?,
                textcoords: Some("offset points".parse()?),
                ..AnnotateOptions::default()
            },
        )?
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Bottom)
        .set_fontsize(sized(20.0));
    axs[(2, 0)]
        .annotate(
            "Blend Mode",
            (0.0, 0.5),
            AnnotateOptions {
                xytext: Some(Point::new(-30.0, 0.0).into()),
                xycoords: axes_fraction()?,
                textcoords: Some("offset points".parse()?),
                ..AnnotateOptions::default()
            },
        )?
        .set_halign(HAlign::Right)
        .set_valign(VAlign::Center)
        .set_fontsize(sized(20.0))
        .set_rotation(90.0);
    fig.subplots_adjust(SubplotOverrides {
        bottom: Some(0.05),
        right: Some(0.95),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

/// 800 samples of 4 EEG-like channels in place of `eeg.dat` (`_eeg`):
/// noise on top of a slow rhythm per channel, as columns.
fn eeg(t: &[f64]) -> Vec<Vec<f64>> {
    let noise = SpecRng::new(GALLERY_SEED).normal(3200, 0.0, 0.8);
    (0..4)
        .map(|channel| {
            let f = channel as f64;
            t.iter()
                .enumerate()
                .map(|(i, &t)| {
                    let rhythm = 0.6 * (2.0 * std::f64::consts::PI * (1.5 + 0.5 * f) * t + f).sin();
                    noise[i * 4 + channel] + rhythm
                })
                .collect()
        })
        .collect()
}

fn mri_with_eeg(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axd = fig.subplot_mosaic(
        &named_mosaic(&[&["image", "density"], &["EEG", "EEG"]])?,
        MosaicOptions {
            width_ratios: Some(vec![1.05, 2.0]),
            ..MosaicOptions::default()
        },
    )?;
    // A 16-bit slice like `s1045.ima.gz`.
    let slice = scan().map(|&v| (v * 55.0) as u16);
    let image = panel(&axd, "image")?;
    image.imshow(
        ScalarField::integers(&slice.map(|&v| i64::from(v))),
        ImshowOptions::default().cmap(colormaps::get("gray")?),
    )?;
    image.set_axis_off();
    let intensities: Vec<u16> = slice
        .as_slice()
        .iter()
        .copied()
        .filter(|&v| v != 0)
        .collect();
    let density = panel(&axd, "density")?;
    let edges: Vec<f64> = (0..=128).map(|i| f64::from(i) * 512.0).collect();
    density.hist(
        &[intensities],
        HistPlotOptions {
            bins: Some(Bins::Edges(edges)),
            ..HistPlotOptions::default()
        },
    )?;
    density
        .set_xlabel("Intensity (a.u.)")
        .set_xlim(0.0, 65536.0)?
        .set_ylabel("MRI density")
        .set_yticks(Vec::<f64>::new());
    density.minorticks_on();
    let t: Vec<f64> = (0..800).map(|i| 10.0 * f64::from(i) / 800.0).collect();
    let data = eeg(&t);
    let eeg_axes = panel(&axd, "EEG")?;
    eeg_axes.set_xlabel("Time (s)");
    eeg_axes.set_xlim(0.0, 10.0)?;
    let (low, high) = data
        .iter()
        .flatten()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    let dy = (low - high) * 0.7;
    eeg_axes.set_ylim(-dy, 4.0 * dy)?;
    eeg_axes.set_yticks([0.0, dy, 2.0 * dy, 3.0 * dy]);
    eeg_axes.set_yticklabels(["PG3", "PG5", "PG7", "PG9"])?;
    for (i, channel) in data.iter().enumerate() {
        let shifted: Vec<f64> = channel.iter().map(|v| v + i as f64 * dy).collect();
        eeg_axes.plot_with(&t, &shifted, LineProps::default().color("C0"))?;
    }
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_misc/contour_manual_1", contour_manual_1),
    FigureCase::new("gallery_misc/contour_manual_2", contour_manual_2),
    FigureCase::new(
        "gallery_misc/advanced_hillshading_1",
        advanced_hillshading_1,
    ),
    FigureCase::new(
        "gallery_misc/advanced_hillshading_2",
        advanced_hillshading_2,
    )
    .sized(8.0, 4.5, 100.0),
    FigureCase::new(
        "gallery_misc/advanced_hillshading_3",
        advanced_hillshading_3,
    ),
    FigureCase::new(
        "gallery_misc/topographic_hillshading",
        topographic_hillshading,
    )
    .sized(8.0, 9.0, 100.0),
    FigureCase::new("gallery_misc/mri_with_eeg", mri_with_eeg),
];
