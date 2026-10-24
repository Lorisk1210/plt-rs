//! The colour mapping behind colorbars: the limits a colorbar settles on
//! and follows, norms given to its mappable, its opacity, the label
//! format, and the colorbars matplotlib refuses to make.

use plt::axes::{ImshowOptions, Mesh, PcolorOptions};
use plt::colorbar::{ColorbarAxes, ColorbarFormat, ColorbarOptions, ExtendFrac, Mappable};
use plt::colors::{CenteredNorm, Colorize, ColorizerOptions, LogNorm, ScalarMappable, colormaps};
use plt::figure::Figure;
use plt::numeric::Grid;

use super::super::mpl_tests::{in_test_style, numbers};
use super::{bump, formatter_name, image, locator_name, orientation, side};
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};

/// `test_colorbar_renorm`: the colorbar's `[vmin, vmax]` when made, after
/// a `LogNorm` of the data limits and after a `LogNorm` of a thousand
/// times the limits; its locator's type after `set_ticks([1, 2, 3])`,
/// after the first `LogNorm` and after `set_ticks` again.
pub(super) fn renorm(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let z = bump();
        let (low, high) = z
            .as_slice()
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                (lo.min(v), hi.max(v))
            });
        let fig = Figure::new();
        let ax = fig.subplot();
        let image = ax
            .imshow(z, ImshowOptions::default())
            .map_err(ProbeError::rejected)?;
        let colorbar = fig
            .colorbar(&image, ColorbarOptions::default())
            .map_err(ProbeError::rejected)?;
        let limits = || numbers([colorbar.vmin(), colorbar.vmax()]);
        let locator = || locator_name(&colorbar.locator()).map(Json::from);
        let mut seen = vec![limits()];
        colorbar.set_ticks(vec![1.0, 2.0, 3.0]);
        let mut locators = vec![locator()?];
        image.set_norm(Box::new(LogNorm::new(low, high)));
        seen.push(limits());
        locators.push(locator()?);
        colorbar.set_ticks(vec![1.0, 2.0, 3.0]);
        locators.push(locator()?);
        image.set_norm(Box::new(LogNorm::new(low * 1000.0, high * 1000.0)));
        seen.push(limits());
        Ok(object([
            ("limits", Json::Array(seen)),
            ("locators", Json::Array(locators)),
        ]))
    })
}

/// `test_colorbar_scale_reset`: the limits of an unscaled `LogNorm` given
/// to a mesh with a colorbar.
pub(super) fn assigned_norm(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let cmap = colormaps::get("RdBu_r").map_err(ProbeError::rejected)?;
        let options = PcolorOptions {
            colorizer: ColorizerOptions::default().cmap(cmap),
            ..PcolorOptions::default()
        };
        let mesh = ax
            .pcolormesh(Mesh::new(bump()), options)
            .map_err(ProbeError::rejected)?;
        fig.colorbar(&mesh, ColorbarOptions::beside(&ax))
            .map_err(ProbeError::rejected)?;
        mesh.set_norm(Box::new(LogNorm::default()));
        Ok(limits(mesh.clim()))
    })
}

/// `[vmin, vmax]`, `null` for an unset limit.
fn limits((vmin, vmax): (Option<f64>, Option<f64>)) -> Json {
    let limit = |v: Option<f64>| v.map_or(Json::Null, Json::from);
    Json::Array(vec![limit(vmin), limit(vmax)])
}

/// `test_mappable_2d_alpha`: the colorbar's alpha and the alphas of its
/// mesh, given per cell.
pub(super) fn alpha(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let x = Grid::from_fn(2, 2, |r, c| (r * 2 + c + 1) as f64 / 4.0);
        let alphas = x.as_slice().to_vec();
        let mesh = ax
            .pcolormesh(Mesh::new(x), PcolorOptions::default())
            .map_err(ProbeError::rejected)?;
        mesh.set_alphas(alphas);
        let colorbar = fig
            .colorbar(&mesh, ColorbarOptions::beside(&ax))
            .map_err(ProbeError::rejected)?;
        fig.layout();
        Ok(object([
            ("colorbar", colorbar.alpha().map_or(Json::Null, Json::from)),
            ("mappable", numbers(mesh.style().alphas.unwrap_or_default())),
        ]))
    })
}

/// `Figure.colorbar` scenes: the norm limits a colorbar settles on, its
/// labels with a `str.format` template, and the options matplotlib
/// rejects.
pub(super) fn make(args: Args<'_>) -> Outcome {
    in_test_style(args, || match args.str("scene")? {
        "clim" => clim(args),
        "centered" => centered(),
        "format" => format(args),
        "error" => rejected(args),
        other => Err(ProbeError::BadArgs(format!("unknown scene {other}"))),
    })
}

/// `test_colorbar_int`: the norm limits of `imshow([clim])` with a
/// colorbar.
fn clim(args: Args<'_>) -> Result<Json, ProbeError> {
    let fig = Figure::new();
    let ax = fig.subplot();
    let image = image(&ax, vec![args.f64_vec("clim")?])?;
    fig.colorbar(&image, ColorbarOptions::default())
        .map_err(ProbeError::rejected)?;
    Ok(limits(image.clim()))
}

/// `test_centerednorm`: the limits of a `CenteredNorm` of a mesh of
/// zeros with a colorbar.
fn centered() -> Result<Json, ProbeError> {
    let fig = Figure::with_size(1.0, 3.0, plt::rc::params().figure_dpi);
    let ax = fig.subplot();
    let options = PcolorOptions {
        colorizer: ColorizerOptions::default().norm(CenteredNorm::new(0.0, None)),
        ..PcolorOptions::default()
    };
    let mesh = ax
        .pcolormesh(Mesh::new(Grid::from_fn(3, 3, |_, _| 0.0)), options)
        .map_err(ProbeError::rejected)?;
    fig.colorbar(&mesh, ColorbarOptions::default())
        .map_err(ProbeError::rejected)?;
    Ok(limits(mesh.clim()))
}

/// `test_colorbar_format_string_and_old`: the formatter's type and the
/// tick labels of the colorbar of `imshow([[0, 1]])` made with `format`,
/// after a draw.
fn format(args: Args<'_>) -> Result<Json, ProbeError> {
    let fig = Figure::new();
    let ax = fig.subplot();
    let image = image(&ax, vec![vec![0.0, 1.0]])?;
    let options = ColorbarOptions {
        format: Some(ColorbarFormat::Label(args.str("format")?.into())),
        ..ColorbarOptions::default()
    };
    let colorbar = fig
        .colorbar(&image, options)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    let labels = colorbar
        .long_axis()
        .ticklabels(false)
        .map_err(ProbeError::rejected)?;
    Ok(object([
        (
            "formatter",
            Json::from(formatter_name(&colorbar.formatter())?),
        ),
        ("labels", Json::from(labels)),
    ]))
}

/// `test_colorbar_errors`: the colorbar of an image made with the given
/// `location`, `orientation`, `ticklocation` and `extendfrac` (a missing
/// entry is NaN, as numpy reads `None`), drawn into an inset axes with
/// `cax`.
fn rejected(args: Args<'_>) -> Result<Json, ProbeError> {
    let fig = Figure::new();
    let ax = fig.subplot();
    let image = image(&ax, vec![vec![0.0, 1.0], vec![2.0, 3.0]])?;
    let mut options = ColorbarOptions::default();
    if args.opt_bool("cax")?.unwrap_or(false) {
        options.axes = ColorbarAxes::Cax(ax.inset_axes([0.0, 1.05, 1.0, 0.05]));
    }
    options.location = args.opt_str("location")?.map(side).transpose()?;
    options.ticklocation = args.opt_str("ticklocation")?.map(side).transpose()?;
    options.orientation = args
        .opt_str("orientation")?
        .map(|_| orientation(args))
        .transpose()?;
    if let Some(fractions) = args.get("extendfrac") {
        let fraction = |index: usize| {
            fractions
                .as_array()
                .and_then(|items| items.get(index))
                .map_or(f64::NAN, |item| item.as_f64().unwrap_or(f64::NAN))
        };
        options.extendfrac = ExtendFrac::Each(fraction(0), fraction(1));
    }
    fig.colorbar(&image, options)
        .map(|_| Json::Null)
        .map_err(ProbeError::rejected)
}

/// `test_parentless_mappable`: the colorbar of a mappable drawn in no
/// axes, through pyplot.
pub(super) fn parentless(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let cmap = colormaps::get("viridis").map_err(ProbeError::rejected)?;
        let mappable = ScalarMappable::new(cmap);
        plt::pyplot::colorbar(Some(Mappable::from(&mappable)), ColorbarOptions::default())
            .map(|_| Json::Null)
            .map_err(ProbeError::rejected)
    })
}
