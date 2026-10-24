//! Arranging axes: mosaics, nested grids, subfigures, shared axes with
//! one colorbar, twin and secondary axes, insets and broken axes.

use plt::axes::{BoxplotOptions, ImshowOptions, SecondaryLocation, TickParams};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{ColorizerOptions, colormaps};
use plt::figure::{Mosaic, MosaicOptions, Share, SubfigureOptions};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::legend::LegendOptions;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::rc;
use plt::scale::FuncTransform;
use plt::spines::SpineSide;
use plt::transforms::Point;

use super::data::Samples;
use super::lines::map;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure("mosaic", "subplot_mosaic with constrained layout", mosaic),
    Entry::figure(
        "nested_gridspecs",
        "Nested gridspecs under constrained layout",
        nested_gridspecs,
    ),
    Entry::figure(
        "subfigures",
        "Subfigures with their own titles and layouts",
        subfigures,
    ),
    Entry::figure(
        "shared_colorbar",
        "Shared axes with one common colorbar",
        shared_colorbar,
    ),
    Entry::figure(
        "twin_axes",
        "Two y axes sharing an x axis (twinx)",
        twin_axes,
    ),
    Entry::figure(
        "secondary_axes",
        "Secondary axes converting the units of an axis",
        secondary_axes,
    ),
    Entry::figure(
        "inset",
        "An inset zooming into a detail, with its indicator",
        inset,
    ),
    Entry::figure(
        "broken_axes",
        "A broken y axis skipping an empty range",
        broken_axes,
    ),
];

/// Four panels of different sizes, laid out by name.
fn mosaic() -> Built {
    let fig = Figure::with_size(7.0, 5.0, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axes = fig.subplot_mosaic(&"AAB;CDB".parse::<Mosaic>()?, MosaicOptions::default())?;
    let mut samples = Samples::new(3);
    for (name, ax) in axes.iter() {
        let n = 20 + name.len() * 10;
        let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let mut level = 0.0;
        let walk: Vec<f64> = (0..n)
            .map(|_| {
                level += samples.normal(0.0, 1.0);
                level
            })
            .collect();
        ax.plot(x, walk)?;
        ax.set_title(format!("panel {name}"));
    }
    Ok(fig)
}

/// Temperature and rainfall over a year on their own y axes.
fn twin_axes() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let months: Vec<f64> = (1..=12).map(f64::from).collect();
    let temperature: Vec<f64> = months
        .iter()
        .map(|m| 11.0 - 10.0 * (2.0 * std::f64::consts::PI * (m - 1.0) / 12.0).cos())
        .collect();
    let rain: Vec<f64> = months
        .iter()
        .map(|m| 60.0 + 25.0 * (2.0 * std::f64::consts::PI * m / 12.0).sin())
        .collect();
    let warm = ax.plot_with(
        &months,
        temperature,
        LineProps::fmt("C0o-")?.label("temperature"),
    )?;
    ax.set_xlabel("month").set_ylabel("temperature (°C)");
    let right = ax.twinx()?;
    let wet = right.plot_with(&months, rain, LineProps::fmt("C1s--")?.label("rain"))?;
    right.set_ylabel("rain (mm)");
    ax.set_title("A year of weather");
    // One legend for the lines of both axes.
    ax.legend(LegendOptions::default().handles([&warm, &wet]))?;
    Ok(fig)
}

/// A signal with a fine structure, magnified in an inset.
fn inset() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 10.0, 2000);
    let y: Vec<f64> = x
        .iter()
        .map(|x| x.sin() + 0.05 * (40.0 * x).sin())
        .collect();
    ax.plot(x.clone(), y.clone())?;
    ax.set_ylim(-1.5, 2.5)?;
    let zoom = ax.inset_axes([0.55, 0.6, 0.4, 0.35]);
    zoom.plot(x, y)?;
    zoom.set_xlim(1.2, 1.9)?.set_ylim(0.85, 1.08)?;
    // A rectangle around the zoomed region, joined to the inset.
    ax.indicate_inset_zoom(&zoom, PatchProps::default());
    ax.set_title("Inset axes");
    Ok(fig)
}

/// A grid within a grid: the right column of the outer grid is split
/// again into three rows, and constrained layout keeps every label clear.
fn nested_gridspecs() -> Built {
    let fig = Figure::with_size(7.5, 4.5, 100.0);
    let outer = GridSpec::with_ratios(
        1,
        2,
        Some(vec![1.6, 1.0]),
        None,
        SubplotOverrides::default(),
    )?;
    let big = fig.add_subplot_spec(&outer.cell(0, 0)?)?;
    let x = linspace(0.0, 10.0, 300);
    for k in 1..=3 {
        let k = f64::from(k);
        big.plot_with(
            &x,
            map(&x, |x| (k * x / 3.0).sin() / k),
            LineProps::default().label(format!("harmonic {k}")),
        )?;
    }
    big.set_title("outer grid, left cell");
    big.legend(LegendOptions::default())?;
    let inner = GridSpec::nested(&outer.cell(0, 1)?, 3, 1, None, None, (None, None))?;
    for row in 0..3 {
        let ax = fig.add_subplot_spec(&inner.cell(row, 0)?)?;
        let k = (row + 1) as f64;
        ax.plot_with(
            &x,
            map(&x, |x| (k * x / 3.0).sin() / k),
            LineProps::default().color(format!("C{row}")),
        )?;
        ax.set_ylabel(format!("k = {k}"));
        if row < 2 {
            ax.tick_params(&TickParams {
                labelbottom: Some(false),
                ..TickParams::default()
            });
        }
    }
    fig.suptitle("A 3 × 1 grid nested in a 1 × 2 grid");
    Ok(fig)
}

/// Two subfigures side by side, each a figure of its own: its own
/// background, title and grid of axes.
fn subfigures() -> Built {
    let fig = Figure::with_size(8.0, 4.5, 100.0);
    let options = SubfigureOptions {
        width_ratios: Some(vec![1.4, 1.0]),
        ..SubfigureOptions::default()
    };
    let panels = fig.subfigures(1, 2, options)?;
    let (left, right) = (&panels[(0, 0)], &panels[(0, 1)]);
    let mut samples = Samples::new(37);
    let axs = left.subplots_shared(2, 2, Share::All, Share::All)?;
    for (i, ax) in axs.as_slice().iter().enumerate() {
        let (x, y) = (
            samples.normals(80, 0.0, 1.0),
            samples.normals(80, 0.0, 1.0 + i as f64 * 0.3),
        );
        ax.scatter_with(
            x,
            y,
            ScatterOptions {
                sizes: Some(vec![8.0]),
                ..ScatterOptions::default()
            },
        )?;
    }
    left.suptitle("Left: four samples");
    // A translucent grey sets the panel off on light and dark figures.
    right.set_facecolor("#80808020");
    let ax = right.subplot();
    let data: Vec<Vec<f64>> = (0..4)
        .map(|i| samples.normals(100, 0.0, 1.0 + f64::from(i) * 0.3))
        .collect();
    ax.boxplot(&data, BoxplotOptions::default())?;
    right.suptitle("Right: their spread");
    Ok(fig)
}

/// Four images on one colour scale: the axes share x and y, the images
/// share the colour limits, and a single colorbar serves them all.
fn shared_colorbar() -> Built {
    let fig = Figure::with_size(6.4, 5.2, 100.0);
    let axs = fig.subplots_shared(2, 2, Share::All, Share::All)?;
    let n = 60;
    let mut images = Vec::new();
    for (k, ax) in axs.as_slice().iter().enumerate() {
        let phase = k as f64 * 0.8;
        let z = Grid::from_fn(n, n, |i, j| {
            ((i as f64 / 9.0 + phase).sin() + (j as f64 / 7.0).cos()) * (1.0 + 0.3 * k as f64)
        });
        let options = ImshowOptions {
            colorizer: ColorizerOptions::default()
                .cmap(colormaps::get("magma")?)
                .limits(Some(-2.6), Some(2.6)),
            ..ImshowOptions::default()
        };
        images.push(ax.imshow(z, options)?);
        ax.set_title(format!("t = {k}"));
    }
    let colorbar = ColorbarOptions {
        axes: ColorbarAxes::Parents(axs.as_slice().to_vec()),
        shrink: 0.8,
        label: "amplitude".into(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&images[0], colorbar)?;
    Ok(fig)
}

/// A spectrum against wavelength, with a secondary x axis showing the
/// same positions as frequencies and a secondary y axis in decibels.
fn secondary_axes() -> Built {
    let fig = Figure::with_size(6.4, 4.2, 100.0);
    let ax = fig.subplot();
    let wavelength = linspace(400.0, 700.0, 300);
    let power = map(&wavelength, |w| {
        1.0 + 0.8 * (-((w - 530.0) / 40.0).powi(2)).exp()
    });
    ax.plot(&wavelength, &power)?;
    ax.set_xlabel("wavelength (nm)")
        .set_ylabel("power (relative)");
    // Frequency in THz is 299 792 / wavelength in nm, and back.
    let to_thz = FuncTransform::new(|nm| 299_792.0 / nm, |thz| 299_792.0 / thz);
    let top = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(to_thz))?;
    top.set_xlabel("frequency (THz)");
    let to_db = FuncTransform::new(|p: f64| 10.0 * p.log10(), |db: f64| 10_f64.powf(db / 10.0));
    let right = ax.secondary_yaxis(SecondaryLocation::Side(SpineSide::Right), Some(to_db))?;
    right.set_ylabel("power (dB)");
    ax.set_title("secondary_xaxis and secondary_yaxis");
    Ok(fig)
}

/// Two outliers far above the rest: two axes sharing x show the range of
/// each, with the axis "broken" between them by diagonal marks.
fn broken_axes() -> Built {
    let fig = Figure::with_size(6.4, 4.2, 100.0);
    let tight = ConstrainedLayout {
        hspace: 0.0,
        h_pad: 0.03,
        ..ConstrainedLayout::from_rc(&rc::params())
    };
    fig.set_layout_engine(LayoutEngine::Constrained(tight))?;
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let (high, low) = (&axs[(0, 0)], &axs[(1, 0)]);
    let mut samples = Samples::new(43);
    let mut points: Vec<f64> = (0..30).map(|_| 0.2 * samples.uniform()).collect();
    points[3] += 0.8;
    points[14] += 0.8;
    for ax in [high, low] {
        ax.plot_fmt((0..30).map(f64::from).collect::<Vec<_>>(), &points, "o-")?;
    }
    high.set_ylim(0.78, 1.0)?;
    low.set_ylim(0.0, 0.22)?;
    high.spines().get(SpineSide::Bottom).set_visible(false);
    high.tick_params(&TickParams {
        labelbottom: Some(false),
        ..TickParams::default()
    });
    // Slanted marks where the axis is cut, drawn in axes coordinates.
    let cut = |ax: &Axes| {
        LineProps::default()
            .marker(Marker::Vertices(vec![
                Point::new(-1.0, -0.5),
                Point::new(1.0, 0.5),
            ]))
            .markersize(12.0)
            .linestyle(LineStyle::None)
            .color("0.5")
            .markeredgewidth(1.0)
            .clip_on(false)
            .transform(ax.trans_axes())
    };
    high.plot_with([0.0], [0.0], cut(high))?;
    low.plot_with([0.0], [1.0], cut(low))?;
    high.set_title("A broken y axis");
    Ok(fig)
}
