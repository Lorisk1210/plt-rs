//! Probe operations for the `api_audit` area: the public API the API audit
//! found missing and added (see `spec/reference/probes/test_api_audit.py`).

use std::cell::RefCell;
use std::rc::Rc;

use plt::artist::ArtistHandle;
use plt::axes::{
    Axes, AxisName, ColorizingArtist, ImshowOptions, NonUniformOptions, TitleLocation,
};
use plt::axes_grid1::{Divider, Size, make_axes_locatable};
use plt::collections::CollectionSpec;
use plt::colorbar::ColorbarOptions;
use plt::contour::{ContourOptions, Levels, QuadField};
use plt::figure::{Figure, Share};
use plt::image::FigimageOptions;
use plt::legend::LegendOptions;
use plt::lines::LineProps;
use plt::mplot3d::Axis3;
use plt::numeric::Grid;
use plt::patches::Circle;
use plt::path::Path;
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::table::TableOptions;
use plt_core::Point;

use crate::json::Json;
use crate::probes::encode::{count, numbers, pair, parse_points, points};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Axes.has_data", has_data),
    Op::new("Figure.artists", figure_artists),
    Op::new("Axes.callbacks", limit_callbacks),
    Op::new("Axes.set_subplotspec", set_subplot_spec),
    Op::new("YAxis.set_offset_position", offset_position),
    Op::new("PolyCollection.set_verts", set_verts),
    Op::new("Collection.set_paths", set_paths),
    Op::new("FigureImage.get_extent", figure_image),
    Op::new("NonUniformImage.set_data", nonuniform_set_data),
    Op::new("ScalarMappable.colorbar", mappable_colorbar),
    Op::new("Colorbar.solids", colorbar_solids),
    Op::new("Divider", divider),
    Op::new("Line3D.set_data_3d", line_data_3d),
    Op::new("Text3D.get_position_3d", text_position_3d),
    Op::new("Axis3D.get_rotate_label", rotate_label),
    Op::new("Axes3D.set_title", title_3d),
];

fn has_data(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    match args.str("artist")? {
        "none" => {}
        "text" => {
            ax.text(0.5, 0.5, "note");
        }
        "line" => {
            ax.plot([0.0, 1.0], [0.0, 1.0])
                .map_err(ProbeError::rejected)?;
        }
        "patch" => {
            ax.add_patch(Circle::new((0.5, 0.5), 0.1));
        }
        "scatter" => {
            ax.scatter(vec![0.0, 1.0], vec![0.0, 1.0])
                .map_err(ProbeError::rejected)?;
        }
        "image" => {
            let data = Grid::from_rows(vec![vec![0.0, 1.0], vec![1.0, 0.0]])
                .map_err(ProbeError::rejected)?;
            ax.imshow(data, ImshowOptions::default())
                .map_err(ProbeError::rejected)?;
        }
        "table" => {
            let options = TableOptions {
                cell_text: vec![vec!["1".into(), "2".into()]],
                ..TableOptions::default()
            };
            ax.table(options).map_err(ProbeError::rejected)?;
        }
        "removed" => {
            ax.plot([0.0, 1.0], [0.0, 1.0])
                .map_err(ProbeError::rejected)?
                .remove();
        }
        other => return Err(ProbeError::BadArgs(format!("unknown artist {other}"))),
    }
    Ok(Json::Bool(ax.has_data()))
}

fn figure_artists(_args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let line = ax
        .plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("data"))
        .map_err(ProbeError::rejected)?;
    fig.text(0.5, 0.5, "note");
    fig.add_artist(Circle::new((0.5, 0.5), 0.1));
    fig.add_line(&[0.0, 1.0], &[0.0, 1.0], LineProps::default())
        .map_err(ProbeError::rejected)?;
    fig.figimage(Grid::filled(2, 2, 1.0), FigimageOptions::default())
        .map_err(ProbeError::rejected)?;
    fig.legend(LegendOptions::default().handles([&line]).labels(["data"]))
        .map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        count(fig.artists().len()),
        count(fig.images().len()),
        count(fig.texts().len()),
        count(fig.legends().len()),
    ]))
}

/// The limits the x and y limit callbacks of an axes saw, call by call.
#[derive(Default)]
struct Calls {
    x: Vec<(f64, f64)>,
    y: Vec<(f64, f64)>,
}

/// Connects x and y limit callbacks to `ax` recording what they saw;
/// returns the record and the id of the x callback.
fn record(ax: &Axes) -> (Rc<RefCell<Calls>>, plt::axes::LimitsCallbackId) {
    let calls = Rc::new(RefCell::new(Calls::default()));
    let seen = Rc::clone(&calls);
    // Reading the limits may run the other callback: read them first.
    let x = ax.on_limits_changed(AxisName::X, move |ax| {
        let limits = ax.xlim();
        seen.borrow_mut().x.push(limits);
    });
    let seen = Rc::clone(&calls);
    ax.on_limits_changed(AxisName::Y, move |ax| {
        let limits = ax.ylim();
        seen.borrow_mut().y.push(limits);
    });
    (calls, x)
}

/// `[x calls, y calls]`.
fn counts(calls: &Calls) -> Json {
    Json::Array(vec![count(calls.x.len()), count(calls.y.len())])
}

/// The limits one callback saw, call by call.
fn limits(seen: &[(f64, f64)]) -> Json {
    Json::Array(seen.iter().copied().map(pair).collect())
}

/// `[counts, x limits seen, y limits seen]`.
fn summary(calls: &Calls) -> Json {
    Json::Array(vec![counts(calls), limits(&calls.x), limits(&calls.y)])
}

fn limit_callbacks(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let scenario = args.str("scenario")?;
    match scenario {
        "shared" => {
            let grid = fig
                .subplots_shared(1, 2, Share::All, Share::None)
                .map_err(ProbeError::rejected)?;
            let [first, second] = [0, 1].map(|i| grid.as_slice()[i].clone());
            let (first_calls, _) = record(&first);
            let (second_calls, _) = record(&second);
            first.set_xlim(0.0, 3.0).map_err(ProbeError::rejected)?;
            let second_calls = second_calls.borrow();
            return Ok(Json::Array(vec![
                counts(&first_calls.borrow()),
                counts(&second_calls),
                limits(&second_calls.x),
            ]));
        }
        "chain" => {
            let first = fig.add_subplot(1, 2, 1).map_err(ProbeError::rejected)?;
            let second = fig.add_subplot(1, 2, 2).map_err(ProbeError::rejected)?;
            let (calls, _) = record(&second);
            let target = second.clone();
            first.on_limits_changed(AxisName::X, move |ax| {
                let (low, high) = ax.xlim();
                // Finite limits read back from an axes are always valid.
                let _ = target.set_ylim(low, high);
            });
            first.set_xlim(1.0, 2.0).map_err(ProbeError::rejected)?;
            let calls = calls.borrow();
            return Ok(Json::Array(vec![counts(&calls), limits(&calls.y)]));
        }
        _ => {}
    }
    let ax = fig.subplot();
    let (calls, x_id) = record(&ax);
    match scenario {
        "set_xlim" => {
            ax.set_xlim(0.0, 2.0).map_err(ProbeError::rejected)?;
        }
        "set_xbound" => {
            ax.set_bound(AxisName::X, Some(1.0), Some(3.0))
                .map_err(ProbeError::rejected)?;
        }
        "invert" => {
            ax.yaxis().set_inverted(true);
        }
        "read" => {
            ax.plot([0.0, 1.0], [0.0, 4.0])
                .map_err(ProbeError::rejected)?;
            let before = counts(&calls.borrow());
            ax.ylim();
            ax.ylim();
            let calls = calls.borrow();
            return Ok(Json::Array(vec![before, counts(&calls), limits(&calls.y)]));
        }
        "draw" => {
            ax.plot([0.0, 1.0], [0.0, 4.0])
                .map_err(ProbeError::rejected)?;
            fig.layout();
        }
        "clear" => {
            ax.clear();
            ax.set_xlim(0.0, 2.0).map_err(ProbeError::rejected)?;
        }
        "disconnect" => {
            ax.disconnect_limits_changed(x_id);
            ax.set_xlim(0.0, 2.0).map_err(ProbeError::rejected)?;
            ax.set_ylim(0.0, 2.0).map_err(ProbeError::rejected)?;
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scenario {other}"))),
    }
    Ok(summary(&calls.borrow()))
}

fn set_subplot_spec(args: Args<'_>) -> Outcome {
    let to = args.f64_vec("to")?;
    let [row, col] = [to[0], to[1]].map(|v| v as usize);
    let fig = Figure::new();
    let grid = fig.add_gridspec(2, 2).map_err(ProbeError::rejected)?;
    let cell = |row, col| grid.cell(row, col).map_err(ProbeError::rejected);
    let ax = fig
        .add_subplot_spec(&cell(0, 0)?)
        .map_err(ProbeError::rejected)?;
    let twin = ax.twinx().map_err(ProbeError::rejected)?;
    ax.set_subplot_spec(cell(row, col)?)
        .map_err(ProbeError::rejected)?;
    let bounds = |ax: &Axes| {
        let (x, y, w, h) = ax.position().bounds();
        numbers([x, y, w, h])
    };
    Ok(Json::Array(vec![bounds(&ax), bounds(&twin)]))
}

fn offset_position(args: Args<'_>) -> Outcome {
    let side = match args.str("side")? {
        "left" => SpineSide::Left,
        "right" => SpineSide::Right,
        other => return Err(ProbeError::BadArgs(format!("unknown side {other}"))),
    };
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let ax = fig.subplot();
    ax.plot([0.0, 1.0], [1e6, 1e6 + 1.0])
        .map_err(ProbeError::rejected)?;
    ax.yaxis()
        .set_offset_position(side)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    let text = ax
        .yaxis()
        .offset_text_extent()
        .ok_or_else(|| ProbeError::Rejected("no offset text".into()))?;
    let left = ax.bbox().x0();
    Ok(pair((text.x0() - left, text.x1() - left)))
}

/// Argument `key` as lists of points.
fn polylines(args: Args<'_>, key: &str) -> Result<Vec<Vec<Point>>, ProbeError> {
    args.json(key)?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a list")))?
        .iter()
        .map(parse_points)
        .collect()
}

fn set_verts(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let triangle = vec![
        Point::new(0.0, 0.0),
        Point::new(1.0, 0.0),
        Point::new(0.0, 1.0),
    ];
    let polys = ax.add_collection(CollectionSpec::polygons(vec![triangle]));
    polys.set_verts(polylines(args, "verts")?);
    let encoded = polys
        .paths()
        .iter()
        .map(|path| {
            let codes = path.codes().map_or(Json::Null, |codes| {
                numbers(codes.iter().map(|&code| f64::from(code as u8)))
            });
            Json::Array(vec![points(path.vertices()), codes])
        })
        .collect();
    Ok(Json::Array(encoded))
}

fn set_paths(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let start = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
    let paths = ax.add_collection(CollectionSpec::paths(vec![start]));
    paths.set_paths(
        polylines(args, "verts")?
            .into_iter()
            .map(Path::polyline)
            .collect(),
    );
    Ok(Json::Array(
        paths
            .paths()
            .iter()
            .map(|path| points(path.vertices()))
            .collect(),
    ))
}

fn figure_image(args: Args<'_>) -> Outcome {
    let (rows, cols) = (args.usize("rows")?, args.usize("cols")?);
    let fig = Figure::new();
    let options = FigimageOptions {
        xo: args.f64("xo")?,
        yo: args.f64("yo")?,
        ..FigimageOptions::default()
    };
    let image = fig
        .figimage(Grid::filled(rows, cols, 1.0), options)
        .map_err(ProbeError::rejected)?;
    let extent = image.extent();
    image.set_data(Grid::filled(rows + 1, cols + 1, 1.0));
    let (rows, cols) = image.shape();
    Ok(Json::Array(vec![
        numbers([extent.left, extent.right, extent.bottom, extent.top]),
        Json::Array(vec![count(rows), count(cols)]),
    ]))
}

fn nonuniform_set_data(args: Args<'_>) -> Outcome {
    let (x, y) = (args.f64_vec("x")?, args.f64_vec("y")?);
    let ax = Figure::new().subplot();
    let image = ax
        .nonuniform_image(
            &[0.0, 1.0],
            &[0.0, 1.0],
            Grid::filled(2, 2, 1.0),
            NonUniformOptions::default(),
        )
        .map_err(ProbeError::rejected)?;
    image
        .set_data(&x, &y, Grid::filled(y.len(), x.len(), 1.0))
        .map_err(ProbeError::rejected)?;
    let extent = image
        .extent()
        .ok_or_else(|| ProbeError::Rejected("an empty image".into()))?;
    Ok(numbers([
        extent.left,
        extent.right,
        extent.bottom,
        extent.top,
    ]))
}

fn mappable_colorbar(_args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let data =
        Grid::from_rows(vec![vec![0.0, 1.0], vec![1.0, 0.0]]).map_err(ProbeError::rejected)?;
    let image = ColorizingArtist::from(
        ax.imshow(data, ImshowOptions::default())
            .map_err(ProbeError::rejected)?,
    );
    let before = image.colorbar().is_none();
    let shows = |colorbar: &plt::colorbar::Colorbar| {
        image
            .colorbar()
            .is_some_and(|current| current.ax() == colorbar.ax())
    };
    let first = fig
        .colorbar(&image, ColorbarOptions::default())
        .map_err(ProbeError::rejected)?;
    let after_first = shows(&first);
    let second = fig
        .colorbar(&image, ColorbarOptions::default())
        .map_err(ProbeError::rejected)?;
    Ok(Json::Array(
        [before, after_first, shows(&second)]
            .map(Json::Bool)
            .to_vec(),
    ))
}

fn colorbar_solids(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let colorbar = if args.bool("hatched")? {
        let z = Grid::from_rows(vec![
            vec![0.0, 1.0, 2.0],
            vec![1.0, 2.0, 3.0],
            vec![2.0, 3.0, 4.0],
        ])
        .map_err(ProbeError::rejected)?;
        let options = ContourOptions {
            levels: Levels::Values(vec![0.0, 2.0, 4.0]),
            hatches: vec![Some("/".into()), Some("x".into())],
            ..ContourOptions::default()
        };
        let contours = ax
            .contourf(&QuadField::new(z), options)
            .map_err(ProbeError::rejected)?;
        fig.colorbar(&contours, ColorbarOptions::default())
    } else {
        let data =
            Grid::from_rows(vec![vec![0.0, 1.0], vec![1.0, 0.0]]).map_err(ProbeError::rejected)?;
        let image = ax
            .imshow(data, ImshowOptions::default())
            .map_err(ProbeError::rejected)?;
        fig.colorbar(&image, ColorbarOptions::default())
    }
    .map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        Json::Bool(colorbar.solids().is_some()),
        count(colorbar.solids_patches().len()),
    ]))
}

fn divider(_args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let divider = Divider::new(
        &fig,
        [0.1, 0.1, 0.8, 0.8],
        vec![Size::Fixed(1.0)],
        vec![Size::Fixed(1.0)],
        false,
    );
    divider.append_size(SpineSide::Left, Size::Fixed(0.5));
    divider.append_size(SpineSide::Top, Size::Fixed(0.25));
    let plain = make_axes_locatable(&fig.add_subplot(1, 2, 1).map_err(ProbeError::rejected)?);
    let equal_ax = fig.add_subplot(1, 2, 2).map_err(ProbeError::rejected)?;
    equal_ax
        .set_aspect(Aspect::Equal)
        .map_err(ProbeError::rejected)?;
    let equal = make_axes_locatable(&equal_ax);
    Ok(Json::Array(vec![
        count(divider.horizontal().len()),
        count(divider.vertical().len()),
        Json::Bool(divider.aspect()),
        Json::Bool(plain.divider().aspect()),
        Json::Bool(equal.divider().aspect()),
    ]))
}

fn line_data_3d(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    let line = ax
        .plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
        .map_err(ProbeError::rejected)?;
    fig.layout();
    let before = ax.xlim();
    ax.set_line_data(
        &line,
        &args.f64_vec("x")?,
        &args.f64_vec("y")?,
        &args.f64_vec("z")?,
    )
    .map_err(ProbeError::rejected)?;
    let data = ax.line_data(&line).map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        Json::Array(data.iter().map(numbers).collect()),
        pair(before),
        pair(ax.xlim()),
    ]))
}

fn text_position_3d(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot_3d();
    let text = ax.text(args.f64("x")?, args.f64("y")?, args.f64("z")?, "here", None);
    let position = ax.text_position(&text).map_err(ProbeError::rejected)?;
    Ok(numbers(position))
}

fn rotate_label(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot_3d();
    ax.set_label(Axis3::Z, args.str("label")?);
    if let Some(rotate) = args.opt_bool("rotate")? {
        ax.set_rotate_label(Axis3::Z, Some(rotate));
    }
    Ok(Json::Bool(ax.rotate_label(Axis3::Z)))
}

fn title_3d(args: Args<'_>) -> Outcome {
    let location = match args.str("loc")? {
        "left" => TitleLocation::Left,
        "center" => TitleLocation::Center,
        "right" => TitleLocation::Right,
        other => {
            return Err(ProbeError::BadArgs(format!(
                "unknown title location {other}"
            )));
        }
    };
    let ax = Figure::new().subplot_3d();
    ax.set_title_at(location, "t");
    Ok(Json::Array(
        [
            TitleLocation::Left,
            TitleLocation::Center,
            TitleLocation::Right,
        ]
        .map(|location| Json::from(ax.title_at(location)))
        .to_vec(),
    ))
}
