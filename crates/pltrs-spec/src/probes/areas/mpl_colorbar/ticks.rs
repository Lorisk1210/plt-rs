//! The long axis of colorbars: minor ticks under the rc parameters, the
//! label, the ticks of boundary norms and the tick helpers behind the
//! colorbar's properties.

use std::any::Any;
use std::rc::Rc;

use pltrs::axes::{AxisName, Mesh, PcolorOptions};
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions};
use pltrs::colors::{BoundaryNorm, ColorizerOptions, Extend, ScalarMappable, colormaps};
use pltrs::figure::Figure;
use pltrs::numeric::{Grid, arange};
use pltrs::orientation::Orientation;
use pltrs::ticker::{FixedLocator, Formatter, Locator, LogFormatter, LogLabelStyle};

use super::super::mpl_tests::in_test_style;
use super::super::mpl_ticker::{formatter_name, locator_name};
use super::image;
use crate::json::Json;
use crate::probes::encode::{numbers, object};
use crate::probes::{Args, Outcome, ProbeError};
use crate::rng::SpecRng;

/// `test_cbar_minorticks_for_rc_xyminortickvisible`: the minor ticks of a
/// vertical and of a horizontal colorbar of a mesh with limits 0.4 and
/// 2.6, both extended.
pub(super) fn minor_ticks(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let options = PcolorOptions {
            colorizer: ColorizerOptions {
                vmin: Some(0.4),
                vmax: Some(2.6),
                ..ColorizerOptions::default()
            },
            ..PcolorOptions::default()
        };
        let mesh = ax
            .pcolormesh(
                Mesh::new(Grid::from_fn(1, 2, |_, c| (c + 1) as f64)),
                options,
            )
            .map_err(ProbeError::rejected)?;
        [Orientation::Vertical, Orientation::Horizontal]
            .into_iter()
            .map(|orientation| {
                let options = ColorbarOptions {
                    orientation: Some(orientation),
                    extend: Some(Extend::Both),
                    ..ColorbarOptions::default()
                };
                let colorbar = fig.colorbar(&mesh, options).map_err(ProbeError::rejected)?;
                colorbar
                    .long_axis()
                    .ticklocs(true)
                    .map(numbers)
                    .map_err(ProbeError::rejected)
            })
            .collect::<Result<_, _>>()
            .map(Json::Array)
    })
}

/// `test_colorbar_label`: the y label of a colorbar made with a label,
/// after `set_label(None)` and `set_label("cbar 2")`, of a colorbar made
/// without one, and the x label of a horizontal one.
pub(super) fn labels(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let image = image(&ax, vec![vec![1.0, 2.0], vec![3.0, 4.0]])?;
        let make = |label: &str, orientation| {
            let options = ColorbarOptions {
                label: label.to_owned(),
                orientation: Some(orientation),
                ..ColorbarOptions::default()
            };
            fig.colorbar(&image, options).map_err(ProbeError::rejected)
        };
        let colorbar = make("cbar", Orientation::Vertical)?;
        let ylabel = || colorbar.ax().label(AxisName::Y);
        let mut labels = vec![ylabel()];
        colorbar.set_label("", None);
        labels.push(ylabel());
        colorbar.set_label("cbar 2", None);
        labels.push(ylabel());
        labels.push(make("", Orientation::Vertical)?.ax().label(AxisName::Y));
        labels.push(
            make("horizontal cbar", Orientation::Horizontal)?
                .ax()
                .label(AxisName::X),
        );
        Ok(Json::from(labels))
    })
}

/// `test_negative_boundarynorm`: the y limits and ticks of four colorbars
/// of `BoundaryNorm`s drawn in turn into one axes.
pub(super) fn boundary_norms(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::with_size(1.0, 3.0, pltrs::rc::params().figure_dpi);
        let ax = fig.subplot();
        let cmap = colormaps::get("viridis").map_err(ProbeError::rejected)?;
        [(-94, -85), (85, 94), (-3, 3), (-8, 1)]
            .into_iter()
            .map(|(start, stop)| {
                let levels: Vec<f64> = (start..stop).map(f64::from).collect();
                let norm = BoundaryNorm::new(levels, cmap.n(), Extend::Neither)
                    .map_err(ProbeError::rejected)?;
                let mappable = ScalarMappable::with_norm(cmap.clone(), Box::new(norm));
                let options = ColorbarOptions {
                    axes: ColorbarAxes::Cax(ax.clone()),
                    ..ColorbarOptions::default()
                };
                let colorbar = fig
                    .colorbar(&mappable, options)
                    .map_err(ProbeError::rejected)?;
                let (bottom, top) = colorbar.ax().ylim();
                let ticks = colorbar
                    .long_axis()
                    .ticklocs(false)
                    .map_err(ProbeError::rejected)?;
                Ok(object([
                    ("ylim", numbers([bottom, top])),
                    ("yticks", numbers(ticks)),
                ]))
            })
            .collect::<Result<_, _>>()
            .map(Json::Array)
    })
}

/// `test_colorbar_set_formatter_locator`: the colorbar's locators and
/// formatters set on the long axis, compared with the axis' ones, and
/// the axis' ones once set through the colorbar.
pub(super) fn tickers(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let values = SpecRng::new(0).normal(100, 0.0, 1.0);
        let mesh = ax
            .pcolormesh(
                Mesh::new(Grid::from_fn(10, 10, |r, c| values[r * 10 + c])),
                PcolorOptions::default(),
            )
            .map_err(ProbeError::rejected)?;
        let colorbar = fig
            .colorbar(&mesh, ColorbarOptions::default())
            .map_err(ProbeError::rejected)?;
        let axis = colorbar.long_axis();
        let log = || LogFormatter::new(LogLabelStyle::Plain, 10.0);
        axis.set_major_locator(FixedLocator::new(arange(0.0, 10.0, 1.0)))
            .set_minor_locator(FixedLocator::new(arange(0.0, 10.0, 0.2)))
            .set_major_formatter(log())
            .set_minor_formatter(log());
        let same = object([
            (
                "locator",
                Rc::ptr_eq(&colorbar.locator(), &axis.major_locator()).into(),
            ),
            (
                "minorlocator",
                Rc::ptr_eq(&colorbar.minor_locator(), &axis.minor_locator()).into(),
            ),
            (
                "formatter",
                Rc::ptr_eq(&colorbar.formatter(), &axis.major_formatter()).into(),
            ),
            (
                "minorformatter",
                Rc::ptr_eq(&colorbar.minor_formatter(), &axis.minor_formatter()).into(),
            ),
        ]);
        let through_axis = object([
            ("locator", locator(&colorbar.locator())?),
            ("minorlocator", locator(&colorbar.minor_locator())?),
            ("formatter", formatter(&colorbar.formatter())?),
            ("minorformatter", formatter(&colorbar.minor_formatter())?),
        ]);
        colorbar
            .set_locator(FixedLocator::new(arange(0.0, 7.0, 1.0)))
            .set_minor_locator(FixedLocator::new(arange(0.0, 7.0, 0.1)))
            .set_formatter(log())
            .set_minor_formatter(log());
        let through_colorbar = object([
            ("locator", locator(&axis.major_locator())?),
            ("minorlocator", locator(&axis.minor_locator())?),
            ("formatter", formatter(&axis.major_formatter())?),
            ("minorformatter", formatter(&axis.minor_formatter())?),
        ]);
        let long = match axis.name() {
            AxisName::X => "x",
            AxisName::Y => "y",
        };
        Ok(object([
            ("same", same),
            ("axis", through_axis),
            ("colorbar", through_colorbar),
            ("long_axis", Json::from(long)),
        ]))
    })
}

/// `[type name, ticks]` of a fixed locator.
fn locator(locator: &Rc<dyn Locator>) -> Result<Json, ProbeError> {
    let name = locator_name(locator)?;
    let fixed = (locator.as_ref() as &dyn Any)
        .downcast_ref::<FixedLocator>()
        .ok_or_else(|| ProbeError::rejected(format!("{name} has no fixed ticks")))?;
    Ok(Json::Array(vec![
        Json::from(name),
        numbers(fixed.locs().iter().copied()),
    ]))
}

/// `[type name]` of a formatter.
fn formatter(formatter: &Rc<dyn Formatter>) -> Result<Json, ProbeError> {
    Ok(Json::Array(vec![Json::from(formatter_name(formatter)?)]))
}
