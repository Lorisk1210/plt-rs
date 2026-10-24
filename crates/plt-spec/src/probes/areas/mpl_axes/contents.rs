//! What axes hold and other single calls: subplot geometry, spectral
//! plots, hit testing, patch limits, artist lists, autoscaling, `ecdf`,
//! large offsets, vectors at dates, PDF output, inset indicators and box
//! aspects under a layout engine.

use plt::artist::ArtistHandle;
use plt::axes::{
    Axes, AxisName, EcdfOptions, ImshowOptions, ScatterOptions, SpecgramOptions,
    SpectralPlotOptions, SpectrumScale,
};
use plt::collections::CollectionSpec;
use plt::figure::{Figure, SaveFormat, SaveOptions};
use plt::geo::GeoProjection;
use plt::layout::LayoutEngine;
use plt::lines::LineProps;
use plt::navigation::axes_at;
use plt::numeric::Grid;
use plt::patches::{PatchProps, PathShape, Rectangle, Wedge};
use plt::pyplot;
use plt::quiver::{BarbsOptions, QuiverOptions, VectorField};
use plt_core::{Path, Point};

use super::{count, made, numbers, on_axes, pair};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::encode::parse_points;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// A non-negative integer argument.
fn index_arg(args: Args<'_>, key: &str) -> Result<usize, ProbeError> {
    usize::try_from(args.i64(key)?).map_err(ProbeError::rejected)
}

/// `plt.subplot(nrows, ncols, index)`, twinned: the subplot's
/// `get_geometry()`.
fn twinned_subplot_geometry(args: Args<'_>) -> Outcome {
    let (nrows, ncols) = (index_arg(args, "nrows")?, index_arg(args, "ncols")?);
    let ax = made(pyplot::subplot(nrows, ncols, index_arg(args, "index")?))?;
    let _twin = ax.twinx();
    let spec = ax
        .subplot_spec()
        .ok_or_else(|| ProbeError::rejected("not a subplot"))?;
    let (rows, cols) = spec.gridspec().shape();
    let (r, c) = (spec.rows(), spec.cols());
    let geometry = [
        rows,
        cols,
        r.start * cols + c.start,
        (r.end - 1) * cols + c.end - 1,
    ];
    Ok(Json::Array(geometry.map(count).to_vec()))
}

pub(super) fn subplot_geometry(args: Args<'_>) -> Outcome {
    styled(args, || {
        pyplot::close_all();
        let geometry = twinned_subplot_geometry(args);
        pyplot::close_all();
        geometry
    })
}

pub(super) fn specgram_extent(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = SpecgramOptions {
            scale: Some(match args.str("scale")? {
                "linear" => SpectrumScale::Linear,
                _ => SpectrumScale::Decibels,
            }),
            ..SpecgramOptions::default()
        };
        let plot = made(ax.specgram(&args.f64_vec("x")?, options))?;
        let extent = plot.image.extent();
        Ok(numbers(&[
            extent.left,
            extent.right,
            extent.bottom,
            extent.top,
        ]))
    })
}

pub(super) fn psd_inverted(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        ax.yaxis().set_inverted(true);
        let zeros = vec![0.0; 5];
        if args.str("kind")? == "psd" {
            made(ax.psd(&zeros, SpectralPlotOptions::default()))?;
        } else {
            made(ax.csd(&zeros, &zeros, SpectralPlotOptions::default()))?;
        }
        let points = ax.lines().first().map_or(0, |line| line.data().0.len());
        Ok(Json::Array(vec![
            Json::from(ax.yaxis().inverted()),
            count(points),
        ]))
    })
}

pub(super) fn inaxes(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let at = Point::new(200.0, 200.0);
        let before = axes_at(&fig, at).is_some();
        ax.set_visible(false);
        Ok(Json::Array(vec![
            Json::from(before),
            Json::from(axes_at(&fig, at).is_some()),
        ]))
    })
}

pub(super) fn wedge_bounds(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let mut wedge = Wedge::new((0.0, -1.0), 1.05, 60.0, 120.0);
        wedge.width = Some(0.1);
        ax.add_patch(wedge);
        let limits = ax.data_lim().bbox;
        Ok(numbers(&[
            limits.x0(),
            limits.y0(),
            limits.width(),
            limits.height(),
        ]))
    })
}

pub(super) fn artist_sublists(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let counts = || {
            Json::Array(
                [
                    ax.collections().len(),
                    ax.images().len(),
                    ax.lines().len(),
                    ax.patches().len(),
                    ax.tables().len(),
                    ax.texts().len(),
                ]
                .map(count)
                .to_vec(),
            )
        };
        let lines = (0..6)
            .map(|i| {
                let y: Vec<f64> = (i..i + 5).map(f64::from).collect();
                made(ax.plot((0..5).map(f64::from).collect::<Vec<_>>(), y))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let x: Vec<f64> = (0..5).map(f64::from).collect();
        let points = made(ax.scatter_with(x.clone(), x, ScatterOptions::default()))?;
        let image = made(ax.imshow(
            made(Grid::from_rows(vec![vec![0.0; 5]; 5]))?,
            ImshowOptions::default(),
        ))?;
        let patch = ax.add_patch(Rectangle::new((0.0, 0.0), 5.0, 5.0));
        let text = ax.text(0.0, 0.0, "foo");
        let mut recorded = vec![counts()];
        points.remove();
        recorded.push(counts());
        image.remove();
        recorded.push(counts());
        patch.remove();
        recorded.push(counts());
        text.remove();
        recorded.push(counts());
        for line in lines {
            line.remove();
            recorded.push(counts());
        }
        Ok(Json::Array(recorded))
    })
}

pub(super) fn path_autoscale(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let vertices = parse_points(args.json("vertices")?)?;
        ax.add_patch(PathShape(Path::polyline(vertices)));
        ax.autoscale(None, None);
        Ok(encode::object([
            ("xlim", pair(ax.xlim())),
            ("ylim", pair(ax.ylim())),
        ]))
    })
}

pub(super) fn ecdf(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let line = made(ax.ecdf(&args.f64_vec("x")?, EcdfOptions::default()))?;
        Ok(encode::points(&line.xydata()))
    })
}

pub(super) fn large_offset(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let y: Vec<f64> = [0.0, 1e-12].iter().map(|d| (1.0 + d) * 1e27).collect();
        made(ax.plot(vec![0.0, 1.0], y))?;
        fig.layout();
        Ok(encode::object([
            ("ylim", pair(ax.ylim())),
            ("offset", Json::from(made(ax.ticks(AxisName::Y))?.offset)),
        ]))
    })
}

pub(super) fn vectors_at_dates(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let dates = (0..60)
            .step_by(10)
            .map(|minute| super::iso_date(&format!("2017-07-15T18:{minute:02}:00")))
            .collect::<Result<Vec<_>, _>>()?;
        let x = ax.xaxis().convert_units(dates);
        let n = x.len();
        let spread =
            |high: f64| -> Vec<f64> { (0..n).map(|i| high * i as f64 / (n - 1) as f64).collect() };
        let field = VectorField::new(x, spread(5.0), spread(50.0), spread(50.0));
        if args.str("kind")? == "barbs" {
            made(ax.barbs(field, BarbsOptions::default()))?;
        } else {
            made(ax.quiver(field, QuiverOptions::default()))?;
        }
        Ok(encode::object([
            ("xlim", pair(ax.xlim())),
            ("datalim", encode::bbox(&ax.data_lim().bbox)),
        ]))
    })
}

pub(super) fn pdf_rasterized(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_rasterization_zorder(Some(5.0));
        let line = ax.plot_with(
            (0..5).map(f64::from).collect::<Vec<_>>(),
            (0..5).map(f64::from).collect::<Vec<_>>(),
            LineProps::default().zorder(1.0),
        );
        made(line)?.set_rasterized(true);
        let pdf = made(fig.to_format(SaveFormat::Pdf, SaveOptions::default()))?;
        Ok(Json::from(pdf.starts_with(b"%PDF")))
    })
}

pub(super) fn indicate_inset(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(1.0, 5.0)
            .and_then(|ax| ax.set_ylim(1.0, 5.0))
            .map_err(ProbeError::rejected)?;
        made(ax.set_aspect(plt::rc::Aspect::Ratio(1.0)))?;
        let indicator = ax.indicate_inset(
            [1.5, 2.0, 2.15 - 1.5, 2.5 - 2.0],
            None,
            PatchProps::default(),
        );
        fig.layout();
        let extent = indicator
            .window_extent()
            .ok_or_else(|| ProbeError::rejected("the indicator has no extent"))?;
        Ok(encode::object([
            ("connectors", Json::Bool(true)),
            ("extent", encode::bbox(&extent)),
        ]))
    })
}

pub(super) fn box_aspect_constrained(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let axs = made(fig.subplots(2, 3))?;
        for ax in axs.as_slice() {
            made(ax.set_box_aspect(Some(1.0)))?;
        }
        fig.set_layout_engine(LayoutEngine::constrained())
            .map_err(ProbeError::rejected)?;
        fig.layout();
        axs.as_slice()
            .iter()
            .map(|ax| {
                let extent = ax
                    .window_extent()
                    .ok_or_else(|| ProbeError::rejected("an axes has no extent"))?;
                Ok(numbers(&[extent.width(), extent.height()]))
            })
            .collect::<Result<Vec<_>, ProbeError>>()
            .map(Json::Array)
    })
}

/// The data limits and their smallest positive coordinates.
fn data_limits(ax: &Axes) -> Json {
    let limits = ax.data_lim();
    Json::Array(vec![
        encode::bbox(&limits.bbox),
        encode::point(limits.minpos),
    ])
}

pub(super) fn relim_autolim_false(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        made(ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]))?;
        ax.relim(false);
        let before = data_limits(ax);
        let offsets = vec![Point::new(100.0, 200.0), Point::new(300.0, 400.0)];
        let points = CollectionSpec::paths(Vec::new())
            .with_offsets(offsets, ax.trans_data())
            .without_autolim();
        ax.add_collection(points);
        ax.relim(false);
        Ok(encode::object([
            ("before", before),
            ("after", data_limits(ax)),
        ]))
    })
}

pub(super) fn child_axes_removal(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let marginal = ax.inset_axes([1.0, 0.0, 0.1, 1.0]);
        made(marginal.share_y(ax))?;
        let _twin = marginal.twinx();
        marginal.remove();
        ax.set_xlim(-1.0, 1.0)
            .and_then(|ax| ax.set_ylim(10.0, 20.0))
            .map_err(ProbeError::rejected)?;
        Ok(encode::object([
            ("xlim", pair(ax.xlim())),
            ("ylim", pair(ax.ylim())),
        ]))
    })
}

pub(super) fn inset_projection(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let projection = match args.str("projection")? {
            "hammer" => GeoProjection::Hammer,
            other => return Err(ProbeError::BadArgs(format!("unknown projection {other}"))),
        };
        let inset = ax.geo_inset_axes([0.2, 0.2, 0.3, 0.3], projection);
        let child = ax.child_axes().iter().any(|child| child.id() == inset.id());
        Ok(Json::Array(vec![
            Json::from(inset.projection().name()),
            Json::from(child),
        ]))
    })
}

pub(super) fn redraw_in_frame(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        made(ax.plot(vec![0.0, 1.0, 2.0], vec![1.0, 2.0, 3.0]))?;
        made(fig.render_raster())?;
        made(ax.redraw_in_frame())?;
        Ok(Json::Bool(true))
    })
}
