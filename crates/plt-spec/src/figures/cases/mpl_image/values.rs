//! Masks, norms and extreme values: masked and NaN pixels with extreme
//! colours and colorbars, every interpolation over masked and infinite
//! data, flat and huge-range data, norms replaced after `imshow`, norms
//! given by scale name, and `spy`.

use plt::axes::{Axes, ImshowOptions, SpyOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions, Spacing};
use plt::colors::{BoundaryNorm, Colormap, Extend, LogNorm, Norm, NormName, Normalize, colormaps};
use plt::figure::Figure;
use plt::image::{ColorizerOptions, Extent, Interpolation, Origin, ScalarField};
use plt::numeric::Grid;
use plt::orientation::Orientation;

use super::{classic, gaussians, interpolated, remove_ticks_and_titles, uniform};
use crate::figures::{CaseResult, FigureCase};

/// `rcParams['image.interpolation'] = 'nearest'`.
const NEAREST_RC: &[(&str, &str)] = &[("image.interpolation", "nearest")];

pub(crate) const CASES: &[FigureCase] = &[
    classic("mpl_image/mask_image_over_under", mask_image_over_under),
    classic("mpl_image/mask_image", |fig| {
        let axs = fig.subplots(1, 2)?;
        let mut with_nan = Grid::filled(5, 5, 1.0);
        with_nan[(1, 1)] = f64::NAN;
        axs[(0, 0)].imshow(with_nan, interpolated(Interpolation::Nearest))?;
        let mask = Grid::from_fn(5, 5, |r, c| (r, c) == (1, 1));
        let masked = ScalarField::integers(&Grid::filled(5, 5, 1)).with_mask(&mask)?;
        axs[(0, 1)].imshow(masked, interpolated(Interpolation::Nearest))?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_image/imshow_endianess", |fig| {
        let z = Grid::from_fn(10, 10, |r, c| (c as f64 - 5.0).hypot(r as f64 - 5.0));
        let axs = fig.subplots(1, 2)?;
        for ax in axs.as_slice() {
            ax.imshow(
                z.clone(),
                ImshowOptions {
                    colors: ColorizerOptions::default().cmap(colormaps::get("viridis")?),
                    origin: Some(Origin::Lower),
                    ..interpolated(Interpolation::Nearest)
                },
            )?;
        }
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new(
        "mpl_image/imshow_masked_interpolation",
        imshow_masked_interpolation,
    ),
    FigureCase::new("mpl_image/imshow_flatfield", |fig| {
        let image = fig.subplot().imshow(
            Grid::filled(5, 5, 1.0),
            interpolated(Interpolation::Nearest),
        )?;
        image.set_clim(Some(0.5), Some(1.5));
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_image/imshow_bignumbers", |fig| {
        let values = Grid::from_vec(2, 3, vec![1, 2, 1_000_000_000_000, 3, 1, 4])?;
        let image = fig
            .subplot()
            .imshow(ScalarField::integers(&values), ImshowOptions::default())?;
        image.set_clim(Some(0.0), Some(5.0));
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .with_rc(NEAREST_RC),
    FigureCase::new("mpl_image/imshow_bignumbers_real", |fig| {
        let values = Grid::from_vec(2, 3, vec![2.0, 1.0, 1e22, 4.0, 1.0, 3.0])?;
        let image = fig.subplot().imshow(values, ImshowOptions::default())?;
        image.set_clim(Some(0.0), Some(5.0));
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .with_rc(NEAREST_RC),
    classic("mpl_image/norm_change", norm_change),
    classic("mpl_image/huge_range_log_negative", |fig| {
        huge_range_log(fig, -1.0)
    }),
    classic("mpl_image/huge_range_log_positive", |fig| {
        huge_range_log(fig, 1.0)
    }),
    classic("mpl_image/str_norms", str_norms),
    classic("mpl_image/spy_box", |fig| {
        let axs = fig.subplots(1, 3)?;
        let plots = [
            ("ones", [1.0, 1.0, 1.0, 1.0]),
            ("zeros", [0.0, 0.0, 0.0, 0.0]),
            ("mixed", [0.0, 1.0, 1.0, 0.0]),
        ];
        for (ax, (title, values)) in axs.as_slice().iter().zip(plots) {
            ax.set_title(title);
            ax.spy(
                &Grid::from_vec(2, 2, values.to_vec())?,
                &SpyOptions::default(),
            )?;
        }
        Ok(())
    }),
];

/// A copy of `cmap` with the extreme colours given (`with_extremes`).
fn with_extremes(
    cmap: &str,
    over: &str,
    under: &str,
    bad: &str,
) -> Result<Colormap, Box<dyn std::error::Error>> {
    Ok(colormaps::get(cmap)?
        .with_over(over)?
        .with_under(under)?
        .with_bad(bad)?)
}

/// `test_mask_image_over_under`: masked Gaussians in grey with green,
/// red and blue extremes, linearly and with a `BoundaryNorm`, each with
/// an extended horizontal colorbar.
fn mask_image_over_under(fig: &Figure) -> CaseResult {
    let z = gaussians(0.025, 10.0);
    let mask = z.map(|&v| v > 1.2);
    let palette = with_extremes("gray", "r", "g", "b")?;
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let placed = |interpolation: Interpolation| ImshowOptions {
        origin: Some(Origin::Lower),
        extent: Some(Extent::new(-3.0, 3.0, -3.0, 3.0)),
        ..interpolated(interpolation)
    };
    let image = ax1.imshow(
        ScalarField::masked(z.clone(), &mask)?,
        ImshowOptions {
            colors: ColorizerOptions::default()
                .cmap(palette.clone())
                .norm(Normalize::new(-1.0, 1.0)),
            ..placed(Interpolation::Bilinear)
        },
    )?;
    ax1.set_title("Green=low, Red=high, Blue=bad");
    let horizontal = |ax: &Axes, spacing: Spacing| ColorbarOptions {
        axes: ColorbarAxes::Parent(ax.clone()),
        orientation: Some(Orientation::Horizontal),
        extend: Some(Extend::Both),
        aspect: 10.0,
        spacing,
        ..ColorbarOptions::default()
    };
    fig.colorbar(&image, horizontal(ax1, Spacing::Uniform))?;
    let boundaries = BoundaryNorm::new(
        vec![-1.0, -0.5, -0.2, 0.0, 0.2, 0.5, 1.0],
        256,
        Extend::Neither,
    )?;
    let image = ax2.imshow(
        ScalarField::masked(z, &mask)?,
        ImshowOptions {
            colors: ColorizerOptions::default().cmap(palette).norm(boundaries),
            ..placed(Interpolation::Nearest)
        },
    )?;
    ax2.set_title("With BoundaryNorm");
    fig.colorbar(&image, horizontal(ax2, Spacing::Proportional))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_imshow_masked_interpolation`: every interpolation over data
/// with a masked, an infinite, an under and a ringing over pixel.
fn imshow_masked_interpolation(fig: &Figure) -> CaseResult {
    /// `sorted(mimage._interpd_)` without `auto` and `antialiased`.
    const INTERPOLATIONS: [&str; 18] = [
        "bessel", "bicubic", "bilinear", "blackman", "catrom", "gaussian", "hamming", "hanning",
        "hermite", "kaiser", "lanczos", "mitchell", "nearest", "none", "quadric", "sinc",
        "spline16", "spline36",
    ];
    let cmap = with_extremes("viridis", "r", "b", "k")?;
    let mut data = Grid::from_fn(20, 20, |r, c| (r * 20 + c) as f64);
    data[(5, 5)] = -1.0;
    data[(15, 5)] = 1e5;
    data[(15, 15)] = f64::INFINITY;
    let mask = Grid::from_fn(20, 20, |r, c| (r, c) == (5, 15));
    let field = ScalarField::masked(data, &mask)?;
    let axs = fig.subplots(3, 6)?;
    for (ax, name) in axs.as_slice().iter().zip(INTERPOLATIONS) {
        ax.set_title(name);
        ax.imshow(
            field.clone(),
            ImshowOptions {
                colors: ColorizerOptions::default()
                    .cmap(cmap.clone())
                    .norm(Normalize::new(0.0, 399.0)),
                ..interpolated(name.parse()?)
            },
        )?;
        ax.set_axis_off();
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_norm_change`: log norms replaced by linear ones after `imshow`
/// mask nothing permanently.
fn norm_change(fig: &Figure) -> CaseResult {
    let data = Grid::from_fn(5, 5, |r, _| if r < 2 { -1.0 } else { 1.0 });
    let mask = Grid::from_fn(5, 5, |r, c| r < 2 && c < 2);
    let cmap = colormaps::get("viridis")?.with_under("w")?;
    let ax = fig.subplot();
    let images = [
        (
            ScalarField::new(data.clone()),
            Extent::new(0.0, 5.0, 0.0, 5.0),
        ),
        (
            ScalarField::masked(data, &mask)?,
            Extent::new(5.0, 10.0, 5.0, 10.0),
        ),
    ];
    for (field, extent) in images {
        let image = ax.imshow(
            field,
            ImshowOptions {
                colors: ColorizerOptions::default()
                    .cmap(cmap.clone())
                    .norm(LogNorm::new(0.5, 1.0)),
                extent: Some(extent),
                ..interpolated(Interpolation::Nearest)
            },
        )?;
        image.set_norm(Box::new(Normalize::new(-2.0, 2.0)));
    }
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    Ok(())
}

/// `test_huge_range_log`: a log norm over twenty decades, with `x` (bad
/// for -1) in the lower rows.
fn huge_range_log(fig: &Figure, x: f64) -> CaseResult {
    let data = Grid::from_fn(5, 5, |r, _| if r < 2 { 1e20 } else { x });
    fig.subplot().imshow(
        data,
        ImshowOptions {
            colors: ColorizerOptions::default()
                .cmap(colormaps::get("viridis")?)
                .norm(LogNorm::new(1.0, 1e20)),
            ..interpolated(Interpolation::Nearest)
        },
    )?;
    Ok(())
}

/// `test_str_norms`: norms given by the name of a scale, with and without
/// limits.
fn str_norms(fig: &Figure) -> CaseResult {
    let t = uniform(19_680_801, 10, 10).map(|&v| v * 0.8 + 0.1);
    let norms = [
        (NormName::Log, None, None),
        (NormName::Log, Some(0.2), None),
        (NormName::Symlog, None, None),
        (NormName::Symlog, Some(0.3), Some(0.7)),
        (NormName::Logit, Some(0.3), Some(0.7)),
    ];
    let axs = fig.subplots(1, 5)?;
    for (ax, (name, vmin, vmax)) in axs.as_slice().iter().zip(norms) {
        let mut norm: Box<dyn Norm> = name.norm();
        norm.set_vmin(vmin);
        norm.set_vmax(vmax);
        ax.imshow(
            t.clone(),
            ImshowOptions {
                colors: ColorizerOptions {
                    norm: Some(norm),
                    ..ColorizerOptions::default()
                },
                ..ImshowOptions::default()
            },
        )?;
    }
    Ok(())
}
