//! Images: `barcode_demo`, `colormap_interactive_adjustment`,
//! `demo_bboximage`, `figimage_demo`, `image_clip_path`, `image_demo`,
//! `image_zcoord`, `interpolation_methods`, `layer_images`, `matshow`,
//! `multi_image` and `watermark_image`.

use std::f64::consts::PI;

use plt::axes::{BarOptions, ImshowOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{Normalize, colormaps};
use plt::figure::{Figure, Mosaic, MosaicCell, MosaicOptions, Share};
use plt::image::{
    BboxImageOptions, ColorizerOptions, Extent, FigimageOptions, ImageAlpha, Interpolation, Origin,
    ScalarField,
};
use plt::layout::TightLayout;
use plt::numeric::{Grid, arange, linspace};
use plt::orientation::Orientation;
use plt::patches::{Circle, PatchProps, PathShape};
use plt::prelude::ArtistHandle;
use plt::rc::Aspect;
use plt::text::HAlign;
use plt::transforms::{Bbox, Point};
use plt_core::Path;

use super::data::{SEED, logo, mesh_field, photo, uniform_grid};
use crate::figures::cases::common::{func3, image_field, scan, tickless};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `barcode_demo`'s 95 bars.
const BARCODE: [i64; 95] = [
    1, 0, 1, 0, 1, 1, 1, 0, 1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0,
    0, 0, 0, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1, 0, 0,
    1, 1, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1,
];

/// The width in inches of `barcode_demo`'s figure: four pixels per bar at
/// 100 dpi.
const BARCODE_WIDTH: f64 = BARCODE.len() as f64 * 4.0 / 100.0;

/// An integer grid as image data.
fn integers(rows: usize, cols: usize, f: impl Fn(usize, usize) -> i64) -> ScalarField {
    ScalarField::integers(&Grid::from_fn(rows, cols, f))
}

fn barcode_demo(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    ax.set_axis_off();
    let options = ImshowOptions {
        aspect: Some(Aspect::Auto),
        ..ImshowOptions::default()
    }
    .cmap(colormaps::get("binary")?)
    .interpolation(Interpolation::Nearest);
    ax.imshow(integers(1, BARCODE.len(), |_, c| BARCODE[c]), options)?;
    Ok(())
}

fn colormap_interactive_adjustment(fig: &Figure) -> CaseResult {
    let t = linspace(0.0, 2.0 * PI, 1024);
    let data2d = Grid::from_fn(1024, 1024, |r, c| t[r].sin() * t[c].cos());
    let ax = fig.subplot();
    let im = ax.imshow(data2d, ImshowOptions::default())?;
    ax.set_title(
        "Pan on the colorbar to shift the color mapping\n\
         Zoom on the colorbar to scale the color mapping",
    );
    let colorbar = ColorbarOptions {
        label: "Interactive colorbar".into(),
        ..ColorbarOptions::beside(&ax)
    };
    fig.colorbar(&im, colorbar)?;
    Ok(())
}

fn demo_bboximage(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let ramp = || Grid::from_fn(1, 256, |_, c| c as f64);
    let txt = ax1.text(0.5, 0.5, "test");
    txt.set_fontsize(30.0)
        .set_halign(HAlign::Center)
        .set_color("w");
    ax1.add_bbox_image_over(ramp(), &txt, BboxImageOptions::default())?;

    let mut names: Vec<&String> = colormaps::names()
        .iter()
        .filter(|name| !name.ends_with("_r"))
        .collect();
    names.sort();
    let (ncol, pad) = (2, 0.3);
    let nrow = names.len() / ncol + 1;
    let dx = 1.0 / (ncol as f64 + pad * (ncol - 1) as f64);
    let dy = 1.0 / (nrow as f64 + pad * (nrow - 1) as f64);
    for (i, name) in names.into_iter().enumerate() {
        let (ix, iy) = (i / nrow, i % nrow);
        let x0 = ix as f64 * dx * (1.0 + pad);
        let y0 = 1.0 - iy as f64 * dy * (1.0 + pad) - dy;
        let options = BboxImageOptions {
            colors: ColorizerOptions::default().cmap(colormaps::get(name)?),
            ..BboxImageOptions::default()
        };
        ax2.add_bbox_image(
            ramp(),
            Bbox::from_bounds(x0, y0, dx, dy),
            ax2.trans_axes(),
            options,
        )?;
    }
    Ok(())
}

fn figimage_demo(fig: &Figure) -> CaseResult {
    let z = || {
        integers(
            100,
            100,
            |r, c| if c >= 50 { 1 } else { (r * 100 + c) as i64 },
        )
    };
    let lower = FigimageOptions {
        xo: 50.0,
        yo: 0.0,
        origin: Some(Origin::Lower),
        ..FigimageOptions::default()
    };
    fig.figimage(z(), lower)?;
    let translucent = FigimageOptions {
        xo: 100.0,
        yo: 100.0,
        alpha: Some(ImageAlpha::Uniform(0.8)),
        origin: Some(Origin::Lower),
        ..FigimageOptions::default()
    };
    fig.figimage(z(), translucent)?;
    Ok(())
}

fn image_clip_path(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(photo(), ImshowOptions::default())?;
    im.set_clip_patch(&Circle::new((260.0, 200.0), 200.0), ax.trans_data());
    ax.set_axis_off();
    Ok(())
}

/// The largest magnitude of `z`.
fn magnitude(z: &Grid<f64>) -> f64 {
    z.as_slice().iter().fold(0.0, |m, v| m.max(v.abs()))
}

/// Bilinear interpolation, row 0 at the bottom, over `[-3, 3]²`.
fn lower_square(colors: ColorizerOptions) -> ImshowOptions {
    ImshowOptions {
        colors,
        interpolation: Some(Interpolation::Bilinear),
        origin: Some(Origin::Lower),
        extent: Some(Extent::new(-3.0, 3.0, -3.0, 3.0)),
        ..ImshowOptions::default()
    }
}

fn image_demo_1(fig: &Figure) -> CaseResult {
    let z = image_field();
    let top = magnitude(&z);
    let colors = ColorizerOptions::default()
        .cmap(colormaps::get("RdYlBu")?)
        .limits(Some(-top), Some(top));
    fig.subplot().imshow(z, lower_square(colors))?;
    Ok(())
}

fn image_demo_2(fig: &Figure) -> CaseResult {
    let mosaic = Mosaic::new(vec![vec![
        MosaicCell::Label("hopper".into()),
        MosaicCell::Label("mri".into()),
    ]])?;
    let ax = fig.subplot_mosaic(&mosaic, MosaicOptions::default())?;
    let hopper = ax.get("hopper").ok_or("the mosaic has a hopper axes")?;
    let mri = ax.get("mri").ok_or("the mosaic has an MRI axes")?;
    hopper.imshow(photo(), ImshowOptions::default())?;
    hopper.set_axis_off();
    let options = ImshowOptions {
        origin: Some(Origin::Upper),
        extent: Some(Extent::new(0.0, 25.0, 0.0, 25.0)),
        ..ImshowOptions::default()
    }
    .cmap(colormaps::get("hot")?);
    mri.imshow(scan(), options)?;
    mri.plot_fmt([15.9, 16.8], [14.5, 15.0], "o")?;
    mri.set_title("MRI");
    Ok(())
}

fn image_demo_3(fig: &Figure) -> CaseResult {
    let a = uniform_grid(SEED, 5, 5);
    let axs = fig.subplots(1, 3)?;
    for (ax, (interp, title)) in axs.as_slice().iter().zip([
        (Interpolation::Nearest, "Nearest"),
        (Interpolation::Bilinear, "Bilinear"),
        (Interpolation::Bicubic, "Bicubic"),
    ]) {
        ax.imshow(a.clone(), ImshowOptions::default().interpolation(interp))?;
        ax.set_title(title);
        ax.grid(true);
    }
    Ok(())
}

fn image_demo_4(fig: &Figure) -> CaseResult {
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    for (ax, (title, origin)) in axs.as_slice().iter().zip([
        ("blue should be up", Origin::Upper),
        ("blue should be down", Origin::Lower),
    ]) {
        ax.set_title(title);
        let options = ImshowOptions {
            origin: Some(origin),
            ..ImshowOptions::default()
        }
        .interpolation(Interpolation::Bilinear);
        ax.imshow(integers(10, 12, |r, c| (r * 12 + c) as i64), options)?;
    }
    Ok(())
}

fn image_demo_5(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let diamond = Path::polyline(
        [(0.0, 1.0), (1.0, 0.0), (0.0, -1.0), (-1.0, 0.0), (0.0, 1.0)]
            .into_iter()
            .map(|(x, y)| Point::new(x, y))
            .collect(),
    );
    ax.add_patch_with(
        PathShape(diamond.clone()),
        PatchProps::default().facecolor("none"),
    );
    let options = ImshowOptions {
        clip_path: Some((diamond, ax.trans_data())),
        ..lower_square(ColorizerOptions::default().cmap(colormaps::get("gray")?))
    };
    ax.imshow(image_field(), options)?;
    Ok(())
}

fn image_zcoord(fig: &Figure) -> CaseResult {
    let x = uniform_grid(SEED, 5, 3).map(|v| 10.0 * v);
    fig.subplot().imshow(x, ImshowOptions::default())?;
    Ok(())
}

/// `interpolation_methods`' methods; `None` is the default.
const METHODS: [Option<Interpolation>; 18] = [
    None,
    Some(Interpolation::None),
    Some(Interpolation::Nearest),
    Some(Interpolation::Bilinear),
    Some(Interpolation::Bicubic),
    Some(Interpolation::Spline16),
    Some(Interpolation::Spline36),
    Some(Interpolation::Hanning),
    Some(Interpolation::Hamming),
    Some(Interpolation::Hermite),
    Some(Interpolation::Kaiser),
    Some(Interpolation::Quadric),
    Some(Interpolation::Catrom),
    Some(Interpolation::Gaussian),
    Some(Interpolation::Bessel),
    Some(Interpolation::Mitchell),
    Some(Interpolation::Sinc),
    Some(Interpolation::Lanczos),
];

fn interpolation_methods(fig: &Figure) -> CaseResult {
    let grid = uniform_grid(SEED, 4, 4);
    let axs = fig.subplots(3, 6)?;
    for (ax, method) in axs.as_slice().iter().zip(METHODS) {
        tickless(ax);
        let options = ImshowOptions {
            interpolation: method,
            ..ImshowOptions::default()
        }
        .cmap(colormaps::get("viridis")?);
        ax.imshow(grid.clone(), options)?;
        ax.set_title(method.map_or("None", Interpolation::name));
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn layer_images(fig: &Figure) -> CaseResult {
    let x = arange(-3.0, 3.0, 0.05);
    let (first, last) = (x[0], x[x.len() - 1]);
    let extent = Extent::new(first, last, first, last);
    fig.set_frameon(false);
    let ax = fig.add_subplot(1, 1, 1)?;
    let chessboard = ImshowOptions {
        extent: Some(extent),
        ..ImshowOptions::default()
    }
    .cmap(colormaps::get("gray")?)
    .interpolation(Interpolation::Nearest);
    ax.imshow(integers(8, 8, |r, c| ((r + c) % 2) as i64), chessboard)?;
    let overlay = ImshowOptions {
        alpha: Some(ImageAlpha::Uniform(0.9)),
        extent: Some(extent),
        ..ImshowOptions::default()
    }
    .cmap(colormaps::get("viridis")?)
    .interpolation(Interpolation::Bilinear);
    ax.imshow(mesh_field(&x, &x, func3).2, overlay)?;
    Ok(())
}

fn matshow(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.15, 0.09, 0.775, 0.775]);
    let diagonal = integers(15, 15, |r, c| if r == c { r as i64 } else { 0 });
    ax.matshow(diagonal, ImshowOptions::default())?;
    Ok(())
}

fn multi_image(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let datasets: Vec<Grid<f64>> = (0..4)
        .map(|i| {
            let scale = f64::from(i + 1) / 10.0;
            let values = rng.uniform(200, 0.0, 1.0);
            Grid::from_fn(10, 20, |r, c| scale * values[r * 20 + c])
        })
        .collect();
    let all = datasets.iter().flat_map(|d| d.as_slice().iter().copied());
    let (lo, hi) = all.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    let axs = fig.subplots(2, 2)?;
    fig.suptitle("Multiple images");
    // The example shares one colorizer, whose fixed norm every image
    // copies here.
    let mut images = Vec::new();
    for (ax, data) in axs.as_slice().iter().zip(datasets) {
        let options = ImshowOptions::default()
            .colors(ColorizerOptions::default().norm(Normalize::new(lo, hi)));
        images.push(ax.imshow(data, options)?);
    }
    fig.colorbar(
        &images[0],
        ColorbarOptions {
            axes: ColorbarAxes::Parents(axs.as_slice().to_vec()),
            orientation: Some(Orientation::Horizontal),
            fraction: 0.1,
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

fn watermark_image(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let noise = SpecRng::new(SEED).normal(30, 0.0, 1.0);
    let x = arange(0.0, 30.0, 1.0);
    let y: Vec<f64> = x.iter().zip(&noise).map(|(x, n)| x + n).collect();
    let bars = BarOptions {
        color: vec!["#6bbc6b".into()],
        ..BarOptions::default()
    };
    ax.bar(x, &y, bars)?;
    ax.grid(true);
    let watermark = FigimageOptions {
        xo: 25.0,
        yo: 25.0,
        alpha: Some(ImageAlpha::Uniform(0.7)),
        ..FigimageOptions::default()
    };
    fig.figimage(logo(), watermark)?.set_zorder(3.0);
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_images/barcode_demo", barcode_demo).sized(BARCODE_WIDTH, 2.0, 100.0),
    FigureCase::new(
        "gallery_images/colormap_interactive_adjustment",
        colormap_interactive_adjustment,
    ),
    FigureCase::new("gallery_images/demo_bboximage", demo_bboximage),
    FigureCase::new("gallery_images/figimage_demo", figimage_demo),
    FigureCase::new("gallery_images/image_clip_path", image_clip_path),
    FigureCase::new("gallery_images/image_demo_1", image_demo_1),
    FigureCase::new("gallery_images/image_demo_2", image_demo_2).sized(7.0, 3.5, 100.0),
    FigureCase::new("gallery_images/image_demo_3", image_demo_3).sized(10.0, 3.0, 100.0),
    FigureCase::new("gallery_images/image_demo_4", image_demo_4).sized(3.0, 5.0, 100.0),
    FigureCase::new("gallery_images/image_demo_5", image_demo_5),
    FigureCase::new("gallery_images/image_zcoord", image_zcoord),
    FigureCase::new(
        "gallery_images/interpolation_methods",
        interpolation_methods,
    )
    .sized(9.0, 6.0, 100.0),
    FigureCase::new("gallery_images/layer_images", layer_images),
    FigureCase::new("gallery_images/matshow", matshow).sized(4.8, 4.8, 100.0),
    FigureCase::new("gallery_images/multi_image", multi_image),
    FigureCase::new("gallery_images/watermark_image", watermark_image),
];
