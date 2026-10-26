//! The agg filter example: contour labels with white glows, lines with
//! blurred shadows, bars with one shadow for the group and pie wedges lit
//! by their own relief (`demo_agg_filter`).

use plt::artist::{AggFilter, FilterImage, Filtered, RawHandle};
use plt::axes::{ImshowOptions, PieOptions};
use plt::colors::{BlendMode, LightSource, Relief, colormaps};
use plt::contour::{
    ClabelFormat, ClabelOptions, ContourOptions, ContourOrigin, Levels, QuadCoords, QuadField,
};
use plt::fonts::FontSize;
use plt::image::{Extent, Interpolation, Origin};
use plt::numeric::Grid;
use plt::patheffects::PathEffect;
use plt::prelude::*;
use plt::rc::Aspect;

use crate::figures::cases::common::{contour_field, smooth2d};
use crate::figures::{CaseResult, FigureCase};

/// An RGBA image being filtered: `width` pixels wide, top row first.
struct Layer {
    width: usize,
    pixels: Vec<[f64; 4]>,
}

impl Layer {
    /// `image` with `pad` transparent pixels on every side (`np.pad`).
    fn padded(image: &FilterImage, pad: usize) -> Self {
        let width = image.width() + 2 * pad;
        let height = image.height() + 2 * pad;
        let pixels = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    x.checked_sub(pad)
                        .zip(y.checked_sub(pad))
                        .and_then(|(x, y)| image.pixel(x, y))
                        .unwrap_or([0.0; 4])
                })
            })
            .collect();
        Self { width, pixels }
    }

    fn height(&self) -> usize {
        self.pixels.len() / self.width
    }

    /// The opacities scaled by `factor`.
    fn alphas(&self, factor: f64) -> Vec<f64> {
        self.pixels.iter().map(|p| p[3] * factor).collect()
    }

    /// The example's `GaussianFilter.process_image`: `color` under the
    /// opacity scaled by `alpha` and blurred over `sigma` points.
    fn blurred(&self, sigma: f64, alpha: f64, color: [f64; 3], dpi: f64) -> Self {
        let blurred = smooth2d(&self.alphas(alpha), self.width, sigma / 72.0 * dpi);
        Self {
            width: self.width,
            pixels: blurred
                .into_iter()
                .map(|a| [color[0], color[1], color[2], a])
                .collect(),
        }
    }

    /// The example's `OffsetFilter.process_image`: the pixels moved by
    /// `offsets` points, wrapping around (`np.roll`).
    fn rolled(&self, offsets: (f64, f64), dpi: f64) -> Self {
        let (width, height) = (self.width, self.height());
        let shift = |points: f64| (points / 72.0 * dpi) as isize;
        let (dx, dy) = (shift(offsets.0), -shift(offsets.1));
        let wrap =
            |i: usize, by: isize, n: usize| (i as isize - by).rem_euclid(n as isize) as usize;
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| self.pixels[wrap(y, dy, height) * width + wrap(x, dx, width)])
            .collect();
        Self { width, pixels }
    }

    /// The layer as a filter's result, `pad` pixels below and left of the
    /// image it came from.
    fn filtered(self, pad: usize) -> Filtered {
        let height = self.height();
        let offset = -(pad as f64);
        Filtered {
            image: FilterImage::new(self.width, height, self.pixels)
                .expect("one pixel per position"),
            offset: (offset, offset),
        }
    }
}

/// The padding of a blur over `sigma` points (`GaussianFilter.get_pad`).
fn blur_pad(sigma: f64, dpi: f64) -> usize {
    (sigma * 3.0 / 72.0 * dpi) as usize
}

/// The example's `DropShadowFilter(sigma, alpha, offsets=offsets)`: a
/// black shadow of the drawing, blurred and moved.
fn drop_shadow(sigma: f64, alpha: f64, offsets: (f64, f64)) -> AggFilter {
    AggFilter::new(move |image, dpi| {
        let offset_pad = (offsets.0.max(offsets.1) / 72.0 * dpi) as usize;
        let pad = blur_pad(sigma, dpi).max(offset_pad);
        Layer::padded(image, pad)
            .blurred(sigma, alpha, [0.0; 3], dpi)
            .rolled(offsets, dpi)
            .filtered(pad)
    })
}

/// The example's `LightFilter(sigma)`: the drawing lit by the relief of
/// its blurred opacity, overlaid.
fn light(sigma: f64) -> AggFilter {
    AggFilter::new(move |image, dpi| {
        let pad = blur_pad(sigma, dpi);
        let source = Layer::padded(image, pad);
        let height = source.height();
        let elevation = Grid::from_vec(
            height,
            source.width,
            source.blurred(sigma, 1.0, [0.0; 3], dpi).alphas(1.0),
        )
        .expect("one elevation per pixel");
        let rgb = Grid::from_vec(
            height,
            source.width,
            source.pixels.iter().map(|p| [p[0], p[1], p[2]]).collect(),
        )
        .expect("one colour per pixel");
        let lit = LightSource::default()
            .shade_rgb(&rgb, &elevation, &BlendMode::Overlay, &Relief::default())
            .expect("colours and elevations of one shape");
        Layer {
            width: source.width,
            pixels: lit
                .as_slice()
                .iter()
                .zip(&source.pixels)
                .map(|(&[r, g, b], p)| [r, g, b, p[3]])
                .collect(),
        }
        .filtered(pad)
    })
}

/// The example's `GrowFilter(pixels)`: the drawing's opacity widened by
/// `pixels`, in white.
fn grow(pixels: usize) -> AggFilter {
    AggFilter::new(move |image, dpi| {
        let source = Layer::padded(image, pixels);
        let grown = smooth2d(
            &source.alphas(1.0),
            source.width,
            pixels as f64 / 72.0 * dpi,
        );
        Layer {
            width: source.width,
            pixels: grown
                .into_iter()
                .map(|a| [1.0, 1.0, 1.0, (a * 5.0).clamp(0.0, 1.0)])
                .collect(),
        }
        .filtered(pixels)
    })
}

/// `filtered_text`: an image with contours whose labels glow white.
fn filtered_text(ax: &Axes) -> CaseResult {
    let (_, _, z) = contour_field();
    let extent = [-3.0, 3.0, -2.0, 2.0];
    ax.imshow(
        z.clone(),
        ImshowOptions {
            origin: Some(Origin::Lower),
            extent: Some(Extent::new(extent[0], extent[1], extent[2], extent[3])),
            aspect: Some(Aspect::Auto),
            ..ImshowOptions::default()
        }
        .cmap(colormaps::get("gray")?)
        .interpolation(Interpolation::Bilinear),
    )?;
    let levels = arange(-1.2, 1.6, 0.2);
    let field = QuadField {
        coords: QuadCoords::Implicit {
            origin: Some(ContourOrigin::Lower),
            extent: Some(extent),
        },
        ..QuadField::new(z)
    };
    let contours = ax.contour(
        &field,
        ContourOptions {
            levels: Levels::Values(levels.clone()),
            linewidths: Some(vec![2.0]),
            ..ContourOptions::default()
        },
    )?;
    let labels = contours.clabel(ClabelOptions {
        levels: Some(levels.iter().skip(1).step_by(2).copied().collect()),
        fmt: ClabelFormat::Label("%1.1f".into()),
        fontsize: Some(FontSize::Points(11.0)),
        ..ClabelOptions::default()
    })?;
    for label in &labels {
        label.set_color("k");
        // Drawn as paths, the same in every backend.
        label.set_path_effects(vec![PathEffect::normal()]);
    }
    let members: Vec<RawHandle> = labels.iter().map(|t| t.raw().clone()).collect();
    let zorder = labels.first().map_or(3.0, ArtistHandle::zorder) - 0.1;
    ax.add_filtered_group(&members, grow(3)).set_zorder(zorder);
    ax.xaxis().set_visible(false);
    ax.yaxis().set_visible(false);
    Ok(())
}

/// `drop_shadow_line`: two lines with blurred shadows below them.
fn drop_shadow_line(ax: &Axes) -> CaseResult {
    let x = [0.1, 0.5, 0.9];
    let shadow = drop_shadow(4.0, 0.3, (0.0, 0.0));
    for (y, fmt) in [([0.1, 0.9, 0.5], "bo-"), ([0.5, 0.2, 0.7], "ro-")] {
        let line = ax.plot_fmt(x, y, fmt)?;
        ax.plot_fmt(x, y, fmt)?
            .set_transform(ax.trans_data().offset_points(4.0, -6.0))
            .set_zorder(line.zorder() - 0.5)
            .set_agg_filter(Some(shadow.clone()))
            .set_rasterized(true);
    }
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    ax.xaxis().set_visible(false);
    ax.yaxis().set_visible(false);
    Ok(())
}

/// `drop_shadow_patches`: two groups of bars sharing one shadow.
fn drop_shadow_patches(ax: &Axes) -> CaseResult {
    let width = 0.35;
    let mut bars = Vec::new();
    for (offset, means, color) in [
        (0.0, [20.0, 35.0, 30.0, 35.0, 27.0], "r"),
        (width + 0.1, [25.0, 32.0, 34.0, 20.0, 25.0], "y"),
    ] {
        let x: Vec<f64> = (0..5).map(|i| f64::from(i) + offset).collect();
        let container = ax.bar(
            x.as_slice(),
            means,
            BarOptions {
                thickness: width.into(),
                color: vec![Color::from(color)],
                edgecolor: vec![Color::from("w")],
                linewidth: vec![2.0],
                ..BarOptions::default()
            },
        )?;
        bars.extend(container.patches);
    }
    let members: Vec<RawHandle> = bars.iter().map(|bar| bar.raw().clone()).collect();
    let zorder = bars.first().map_or(1.0, ArtistHandle::zorder) - 0.1;
    ax.add_filtered_group(&members, drop_shadow(5.0, 0.3, (1.0, 1.0)))
        .set_zorder(zorder);
    ax.set_ylim(0.0, 40.0)?;
    ax.xaxis().set_visible(false);
    ax.yaxis().set_visible(false);
    Ok(())
}

/// `light_filter_pie`: lit pie wedges over their common shadow.
fn light_filter_pie(ax: &Axes) -> CaseResult {
    let pie = ax.pie(
        [15.0, 30.0, 45.0, 10.0],
        PieOptions {
            explode: vec![0.1, 0.2, 0.1, 0.1],
            ..PieOptions::default()
        },
    )?;
    let lit = light(9.0);
    for wedge in &pie.wedges {
        wedge.set_agg_filter(Some(lit.clone())).set_rasterized(true);
        wedge.set_edgecolor("none").set_linewidth(2.0);
    }
    let members: Vec<RawHandle> = pie.wedges.iter().map(|w| w.raw().clone()).collect();
    let zorder = pie.wedges.first().map_or(1.0, ArtistHandle::zorder) - 0.1;
    ax.add_filtered_group(&members, drop_shadow(9.0, 0.7, (3.0, -4.0)))
        .set_zorder(zorder);
    Ok(())
}

fn demo_agg_filter(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    filtered_text(&axs[(0, 0)])?;
    drop_shadow_line(&axs[(0, 1)])?;
    drop_shadow_patches(&axs[(1, 0)])?;
    light_filter_pie(&axs[(1, 1)])?;
    axs[(1, 1)].set_frame_on(true);
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[FigureCase::new(
    "gallery_misc/demo_agg_filter",
    demo_agg_filter,
)];
