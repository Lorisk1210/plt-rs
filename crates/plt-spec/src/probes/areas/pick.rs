//! Probe operations for the `pick` area (pick events fired by mouse
//! presses and scrolls over artists with a picker).

use std::cell::RefCell;
use std::rc::Rc;

use plt::artist::{ArtistHandle, Picker};
use plt::axes::{Axes, ImshowOptions, ScatterOptions};
use plt::collections::CollectionSpec;
use plt::events::{Event, EventKind, EventSource};
use plt::figure::Figure;
use plt::image::{Extent, ImageData};
use plt::lines::DrawStyle;
use plt::navigation::PointerButton;
use plt::numeric::Grid;
use plt::patches::{Annulus, ArrowStyle, BoxStyle, Circle, PatchProps, Polygon, Rectangle};
use plt::rc::Aspect;
use plt::text::{AnnotateOptions, TextBoxProps};
use plt_core::Point;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("pick.events", events)];

/// Every probe failure here is plt-rs refusing part of a scene.
type Built = Result<(), ProbeError>;

const XS: [f64; 4] = [0.5, 1.5, 2.5, 3.5];

fn points(xy: &[(f64, f64)]) -> Vec<Point> {
    xy.iter().map(|&(x, y)| Point::new(x, y)).collect()
}

/// The axes most scenes use: `xlim=(0, 4), ylim=(0, 3)`.
fn axes(fig: &Figure) -> Result<Axes, ProbeError> {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 4.0)
        .and_then(|ax| ax.set_ylim(0.0, 3.0))
        .map_err(ProbeError::rejected)?;
    Ok(ax)
}

fn lines(fig: &Figure) -> Built {
    let ax = axes(fig)?;
    let plot = |y: [f64; 4], format: &str| ax.plot_fmt(XS, y, format).map_err(ProbeError::rejected);
    plot([0.5, 2.5, 0.5, 2.5], "")?
        .set_gid("zigzag")
        .set_picker(Some(Picker::Radius(5.0)));
    plot([1.5; 4], "o")?
        .set_gid("markers")
        .set_picker(Some(Picker::Radius(3.0)));
    plot([0.2, 1.0, 0.4, 2.8], "")?
        .set_drawstyle(DrawStyle::StepsMid)
        .set_gid("steps")
        .set_picker(Some(Picker::Radius(4.0)));
    // matplotlib's `set_picker(True)` on a line sets a pick radius of 1.
    ax.plot_fmt([0.0, 4.0], [2.8, 0.2], "--")
        .map_err(ProbeError::rejected)?
        .set_gid("dashed")
        .set_picker(Some(Picker::Radius(1.0)));
    Ok(())
}

fn collections(fig: &Figure) -> Built {
    let ax = axes(fig)?;
    let options = ScatterOptions {
        sizes: Some(vec![20.0, 80.0, 200.0, 20.0, 400.0]),
        ..ScatterOptions::default()
    };
    ax.scatter_with(
        vec![0.5, 1.0, 1.5, 2.0, 3.5],
        vec![2.5, 2.4, 2.6, 2.5, 0.5],
        options,
    )
    .map_err(ProbeError::rejected)?
    .set_gid("scatter")
    .set_picker(Some(Picker::Default));
    let segments = vec![
        points(&[(0.2, 0.2), (1.8, 1.8)]),
        points(&[(0.2, 1.8), (1.8, 0.2)]),
        points(&[(2.2, 1.0), (3.8, 1.2), (3.0, 2.0)]),
    ];
    ax.add_collection(CollectionSpec::lines(segments))
        .set_gid("segments")
        .set_picker(Some(Picker::Radius(2.0)));
    let polygons = vec![
        points(&[(2.2, 2.0), (3.0, 2.0), (2.6, 2.9)]),
        points(&[(0.2, 0.2), (0.8, 0.2), (0.8, 0.8), (0.2, 0.8)]),
    ];
    ax.add_collection(CollectionSpec::polygons(polygons))
        .set_pickradius(0.0)
        .set_gid("polygons")
        .set_picker(Some(Picker::Default));
    Ok(())
}

fn patches(fig: &Figure) -> Built {
    let ax = axes(fig)?;
    ax.add_patch(Rectangle::new((0.3, 0.3), 1.0, 0.8))
        .set_gid("rectangle")
        .set_picker(Some(Picker::Default));
    ax.add_patch(Circle::new((2.5, 0.8), 0.5))
        .set_gid("circle")
        .set_picker(Some(Picker::Radius(3.0)));
    let annulus = Annulus::new((1.0, 2.2), 0.6, 0.2).map_err(ProbeError::rejected)?;
    ax.add_patch(annulus)
        .set_gid("annulus")
        .set_picker(Some(Picker::Default));
    let open = Polygon::new(points(&[(2.2, 1.8), (3.8, 2.0), (3.2, 2.8)]), false);
    ax.add_patch(open)
        .update_props(PatchProps {
            fill: Some(false),
            ..PatchProps::default()
        })
        .set_gid("polygon")
        .set_picker(Some(Picker::Radius(2.0)));
    Ok(())
}

fn texts(fig: &Figure) -> Built {
    let ax = axes(fig)?;
    let round: BoxStyle = "round".parse().map_err(ProbeError::rejected)?;
    ax.text(0.5, 2.0, "boxed")
        .set_bbox(Some(TextBoxProps::new(round)))
        .set_gid("boxed")
        .set_picker(Some(Picker::Default));
    ax.set_title("title")
        .set_gid("title")
        .set_picker(Some(Picker::Default));
    let arrow: ArrowStyle = "->".parse().map_err(ProbeError::rejected)?;
    let options = AnnotateOptions {
        xytext: Some(Point::new(2.0, 1.5).into()),
        arrowprops: Some(arrow.into()),
        ..AnnotateOptions::default()
    };
    ax.annotate("note", (3.0, 0.5), options)
        .map_err(ProbeError::rejected)?
        .set_gid("note")
        .set_picker(Some(Picker::Default));
    Ok(())
}

fn layers(fig: &Figure) -> Built {
    let ax = axes(fig)?;
    ax.set_gid("axes").set_picker(Some(Picker::Default));
    let data =
        Grid::from_rows(vec![vec![0.0, 1.0], vec![1.0, 0.0]]).map_err(ProbeError::rejected)?;
    let options = ImshowOptions {
        extent: Some(Extent {
            left: 1.0,
            right: 3.0,
            bottom: 0.5,
            top: 2.5,
        }),
        aspect: Some(Aspect::Auto),
        ..ImshowOptions::default()
    };
    ax.imshow(ImageData::from(data), options)
        .map_err(ProbeError::rejected)?
        .set_gid("image")
        .set_picker(Some(Picker::Default));
    ax.plot([0.0, 4.0], [1.5, 1.5])
        .map_err(ProbeError::rejected)?
        .set_gid("line")
        .set_picker(Some(Picker::Radius(5.0)));
    fig.text(0.02, 0.02, "figure text")
        .set_gid("figtext")
        .set_picker(Some(Picker::Default));
    Ok(())
}

fn twoaxes(fig: &Figure) -> Built {
    let grid = fig.subplots(1, 2).map_err(ProbeError::rejected)?;
    for ((x, y), gid, ax) in [
        (([0.0, 3.0], 0.5), "left", &grid[(0, 0)]),
        (([-2.0, 1.0], 0.3), "right", &grid[(0, 1)]),
    ] {
        ax.set_xlim(0.0, 1.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .map_err(ProbeError::rejected)?;
        ax.plot(x, [y, y])
            .map_err(ProbeError::rejected)?
            .set_clip_on(false)
            .set_gid(gid)
            .set_picker(Some(Picker::Radius(5.0)));
    }
    Ok(())
}

/// The reference figure: `scene` in a 4 × 3 inch figure at 100 dpi.
fn scene(name: &str) -> Result<Figure, ProbeError> {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let build = match name {
        "lines" => lines,
        "collections" => collections,
        "patches" => patches,
        "texts" => texts,
        "layers" => layers,
        "twoaxes" => twoaxes,
        _ => return Err(ProbeError::BadArgs(format!("unknown scene {name:?}"))),
    };
    build(&fig)?;
    fig.layout();
    Ok(fig)
}

fn events(args: Args<'_>) -> Outcome {
    let fig = scene(args.str("scene")?)?;
    let scroll = args.opt_bool("scroll")?.unwrap_or(false);
    let pixels = args
        .json("pixels")?
        .as_array()
        .and_then(|pixels| {
            pixels
                .iter()
                .map(|pixel| match pixel.as_array()? {
                    [x, y] => Some((x.as_f64()?, y.as_f64()?)),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| ProbeError::BadArgs("pixels is malformed".into()))?;
    let picks: Rc<RefCell<Vec<Json>>> = Rc::default();
    let log = Rc::clone(&picks);
    fig.connect(EventKind::Pick, move |event| {
        if let Event::Pick(pick) = event {
            let ind = pick
                .ind
                .iter()
                .map(|&i| Json::Integer(i64::try_from(i).expect("indices fit in i64")))
                .collect();
            let gid = pick.artist.gid().map_or(Json::Null, Json::String);
            log.borrow_mut()
                .push(Json::Array(vec![gid, Json::Array(ind)]));
        }
    });
    let mut source = EventSource::new();
    let mut per_pixel = Vec::with_capacity(pixels.len());
    for (x, y) in pixels {
        if scroll {
            source.scroll(&fig, x, y, 1.0);
        } else {
            source.button_press(&fig, x, y, PointerButton::Left, false);
        }
        per_pixel.push(Json::Array(picks.take()));
    }
    Ok(Json::Array(per_pixel))
}
