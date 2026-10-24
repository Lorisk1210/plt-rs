//! Probe operations for the `artist_behaviour` area: default z-orders,
//! transforms, removal, alpha validation, cursor data of boundary norms
//! and marker culling, ported from matplotlib's `test_artist.py` (see
//! `spec/reference/probes/test_artist_behaviour.py`). Every op runs under
//! the reference configuration.

use plt::artist::{ArtistHandle, Sketch};
use plt::axes::{BarOptions, FillBetweenOptions, ImshowOptions, ScatterOptions};
use plt::collections::CollectionSpec;
use plt::colors::{Alpha, BoundaryNorm, ColorizerOptions, Extend, colormaps};
use plt::contour::{ContourOptions, QuadField};
use plt::figure::Figure;
use plt::image::ImageData;
use plt::lines::LineProps;
use plt::navigation::Navigation;
use plt::numeric::Grid;
use plt::patches::{Ellipse, Rectangle};
use plt::transforms::Transform;
use plt_core::{Bbox, Point};

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::{bbox, object};
use crate::probes::{Args, Op, Outcome, ProbeError};
use crate::rng::SpecRng;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("artist.default_zorder", |args| {
        with_rc(args, default_zorder)
    }),
    Op::new("artist.transform_of_none", |args| {
        with_rc(args, transform_of_none)
    }),
    Op::new("artist.remove", |args| with_rc(args, remove)),
    Op::new("artist.base_properties", |args| {
        with_rc(args, base_properties)
    }),
    Op::new("artist.alpha", |args| with_rc(args, || alpha(args))),
    Op::new("artist.cursor_boundary", |args| {
        with_rc(args, || cursor_boundary(args))
    }),
    Op::new("artist.cull_markers", |args| with_rc(args, cull_markers)),
];

fn made<T>(result: Result<T, impl std::fmt::Display>) -> Result<T, ProbeError> {
    result.map_err(ProbeError::rejected)
}

/// `arange(16.0).reshape(4, 4)`.
fn field() -> Grid<f64> {
    Grid::from_fn(4, 4, |r, c| (4 * r + c) as f64)
}

/// The z-order of a new artist of each kind.
fn default_zorder() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let line_none = made(ax.plot([0.0, 1.0], [0.0, 1.0]))?;
    let line_set = made(ax.plot([0.0, 1.0], [0.0, 1.0]))?;
    line_set.set_zorder(123_456.0);
    let contour = made(ax.contour(&QuadField::new(field()), ContourOptions::default()))?;
    let contourf = made(ax.contourf(&QuadField::new(field()), ContourOptions::default()))?;
    let set_zorder = |set: &plt::contour::ContourSet| {
        set.collections()
            .first()
            .map_or(Json::Null, |c| c.zorder().into())
    };
    let segments = vec![vec![Point::ORIGIN, Point::new(1.0, 1.0)]];
    let bar = made(ax.bar(vec![0.0], [1.0], BarOptions::default()))?;
    let zorders = [
        ("plot", made(ax.plot([0.0, 1.0], [0.0, 1.0]))?.zorder()),
        ("scatter", made(ax.scatter(vec![0.0], vec![0.0]))?.zorder()),
        ("text", ax.text(0.0, 0.0, "t").zorder()),
        (
            "annotate",
            made(ax.annotate("a", (0.0, 0.0), plt::text::AnnotateOptions::default()))?.zorder(),
        ),
        (
            "imshow",
            made(ax.imshow(ImageData::from(field()), ImshowOptions::default()))?.zorder(),
        ),
        (
            "patch",
            ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0)).zorder(),
        ),
        ("axhline", ax.axhline(0.5, LineProps::default()).zorder()),
        ("axvspan", ax.axvspan(0.2, 0.4).zorder()),
        (
            "bar",
            bar.patches.first().map_or(f64::NAN, ArtistHandle::zorder),
        ),
        (
            "fill_between",
            made(ax.fill_between([0.0, 1.0], [0.0, 1.0], [0.0], FillBetweenOptions::default()))?
                .zorder(),
        ),
        (
            "line_collection",
            ax.add_collection(CollectionSpec::lines(segments)).zorder(),
        ),
    ];
    let mut entries: Vec<(String, Json)> = zorders
        .into_iter()
        .map(|(kind, z)| (kind.to_owned(), z.into()))
        .collect();
    entries.push(("contour".to_owned(), set_zorder(&contour)));
    entries.push(("contourf".to_owned(), set_zorder(&contourf)));
    entries.push(("line_none".to_owned(), line_none.zorder().into()));
    entries.push(("line_set".to_owned(), line_set.zorder().into()));
    Ok(Json::Object(entries))
}

/// Ellipses in data and in display coordinates.
fn transform_of_none() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.set_xlim(1.0, 3.0).set_ylim(1.0, 3.0);
    let pixel =
        made(fig.resolve_transform(&ax.trans_data()))?.transform_point(Point::new(2.0, 2.0));
    let data = ax.add_patch(Ellipse::new((2.0, 2.0), 1.0, 1.0));
    let display = |size: f64| {
        let ellipse = ax.add_patch(Ellipse::new(pixel, size, size));
        ellipse.set_transform(Transform::Display);
        ellipse
    };
    let (none, identity) = (display(120.0), display(100.0));
    let extents = [
        data.window_extent(),
        none.window_extent(),
        identity.window_extent(),
    ]
    .iter()
    .map(|extent| bbox(&extent.unwrap_or(Bbox::NULL)))
    .collect();
    Ok(Json::Array(extents))
}

/// The properties every artist has, as a line is made and after setting
/// them, with the figure's frame width.
fn base_properties() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let line = made(ax.plot([0.0, 1.0], [0.0, 1.0]))?;
    fig.layout();
    let read = |line: &plt::lines::Line2D| {
        let sketch = line.sketch_params().map_or(Json::Null, |sketch| {
            Json::from(vec![sketch.scale, sketch.length, sketch.randomness])
        });
        object([
            ("rasterized", line.rasterized().into()),
            ("animated", line.animated().into()),
            ("in_layout", line.in_layout().into()),
            ("snap", line.snap().map_or(Json::Null, Json::Bool)),
            ("sketch", sketch),
            ("clip_on", line.clip_on().into()),
            ("url", line.url().map_or(Json::Null, Json::String)),
            ("transform_set", line.transform().is_some().into()),
            (
                "clip_box_is_axes",
                line.clip_box()
                    .map_or(Json::Null, |owner| (owner == ax.id()).into()),
            ),
            ("clip_path", line.clip_path().is_some().into()),
            ("path_effects", Json::from(line.path_effects().len() as f64)),
        ])
    };
    let before = read(&line);
    line.set_rasterized(true)
        .set_animated(true)
        .set_in_layout(false)
        .set_snap(Some(false))
        .set_sketch_params(Some(Sketch {
            scale: 1.0,
            length: 100.0,
            randomness: 2.0,
        }))
        .set_clip_on(false)
        .set_clip_box(None)
        .set_url("u");
    Ok(object([
        ("before", before),
        ("after", read(&line)),
        ("figure_linewidth", fig.linewidth().into()),
    ]))
}

/// Cursor data of an image and a line, and what removing them leaves.
fn remove() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let data = Grid::from_fn(6, 6, |r, c| (6 * r + c) as f64);
    let image = made(ax.imshow(ImageData::from(data), ImshowOptions::default()))?;
    let line = ax.plot_y([0.0, 1.0, 2.0, 3.0, 4.0]);
    fig.layout();
    let observed = [
        ("image_mouseover", image.mouseover().into()),
        ("line_mouseover", line.mouseover().into()),
    ];
    image.remove();
    line.remove();
    let count = |n: usize| Json::Integer(i64::try_from(n).expect("few artists"));
    Ok(object([
        observed[0].clone(),
        observed[1].clone(),
        ("images", count(ax.images().len())),
        ("lines", count(ax.lines().len())),
    ]))
}

/// `set_alpha` on an artist of `kind`, then a draw.
fn alpha(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let kind = args.str("kind")?;
    if kind == "scatter_array" {
        let alphas = args.f64_vec("alpha")?;
        let scatter = made(ax.scatter(vec![0.0, 1.0], vec![0.0, 1.0]))?;
        scatter.set_alphas(alphas.clone());
        made(fig.render_raster())?;
        return Ok(Json::from(alphas));
    }
    let value = args.opt_f64("alpha")?;
    let artist: Box<dyn Fn(Option<f64>)> = match kind {
        "line" => {
            let line = made(ax.plot([0.0, 1.0], [0.0, 1.0]))?;
            Box::new(move |a| {
                line.set_alpha(a);
            })
        }
        "patch" => {
            let patch = ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
            Box::new(move |a| {
                patch.set_alpha(a);
            })
        }
        "text" => {
            let text = ax.text(0.5, 0.5, "t");
            Box::new(move |a| {
                text.set_alpha(a);
            })
        }
        "scatter" => {
            let options = ScatterOptions {
                alpha: Alpha::Keep,
                ..ScatterOptions::default()
            };
            let scatter = made(ax.scatter_with(vec![0.0, 1.0], vec![0.0, 1.0], options))?;
            Box::new(move |a| {
                scatter.set_alpha(a);
            })
        }
        other => return Err(ProbeError::BadArgs(format!("unknown kind {other:?}"))),
    };
    artist(value);
    made(fig.render_raster())?;
    Ok(value.into())
}

/// The status bar message over each cell of an image under a boundary
/// norm.
fn cursor_boundary(args: Args<'_>) -> Outcome {
    let rows: Vec<Vec<f64>> = args
        .json("data")?
        .as_array()
        .and_then(|rows| rows.iter().map(Json::as_f64_vec).collect())
        .ok_or_else(|| ProbeError::BadArgs("data is not a 2-D list".into()))?;
    let data = made(Grid::from_rows(rows))?;
    let ncolors = usize::try_from(args.i64("ncolors")?).map_err(ProbeError::rejected)?;
    let extend: Extend = made(args.str("extend")?.parse())?;
    let norm = made(
        made(BoundaryNorm::new(
            args.f64_vec("boundaries")?,
            ncolors,
            extend,
        ))?
        .with_clip(args.bool("clip")?),
    )?;
    let fig = Figure::new();
    let ax = fig.subplot();
    let options = ImshowOptions {
        colorizer: ColorizerOptions {
            cmap: Some(made(colormaps::get("RdBu_r"))?),
            norm: Some(Box::new(norm)),
            ..ColorizerOptions::default()
        },
        ..ImshowOptions::default()
    };
    let (height, width) = data.shape();
    made(ax.imshow(ImageData::from(data), options))?;
    fig.layout();
    let to_display = made(fig.resolve_transform(&ax.trans_data()))?;
    let navigation = Navigation::new();
    let messages = (0..height)
        .flat_map(|row| (0..width).map(move |column| (row, column)))
        .map(|(row, column)| {
            let at = Point::new(column as f64 + 0.25, row as f64 + 0.25);
            let pixel = to_display.transform_point(at);
            let pixel = Point::new(pixel.x.round_ties_even(), pixel.y.round_ties_even());
            Json::String(navigation.readout(&fig, pixel))
        })
        .collect();
    Ok(Json::Array(messages))
}

/// Whether markers outside the view are left out of vector output.
fn cull_markers() -> Outcome {
    let mut generator = SpecRng::new(3);
    let (x, y) = (
        generator.uniform(20_000, 0.0, 1.0),
        generator.uniform(20_000, 0.0, 1.0),
    );
    let fig = Figure::new();
    let ax = fig.subplot();
    made(ax.plot_fmt(x, y, "k."))?;
    ax.set_xlim(2.0, 3.0);
    let pdf = made(fig.to_pdf())?;
    let svg = made(fig.to_svg())?;
    Ok(object([
        ("pdf_small", (pdf.len() < 8000).into()),
        ("svg_small", (svg.len() < 20_000).into()),
    ]))
}
