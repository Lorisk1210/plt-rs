//! Probe operations for the `mpl_offsetbox` area: matplotlib's own
//! non-image tests of offset boxes that the `offsetbox_behaviour` area
//! leaves out (see `spec/reference/probes/test_mpl_offsetbox.py`).

use pltrs::artist::ArtistHandle;
use pltrs::coords::Coords;
use pltrs::events::EventSource;
use pltrs::figure::{Figure, SubfigureOptions};
use pltrs::navigation::PointerButton;
use pltrs::offsetbox::{AnnotationBboxOptions, BoxItem, DrawingArea};
use pltrs::text::{AnnotateOptions, Annotation};
use pltrs::transforms::Point;

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("AnnotationBbox.xyann", annotationbbox_xyann),
    Op::new("Annotation.draggable", annotation_draggable),
];

/// `[xyann, anncoords]` of an annotation box of a drawing area at `(0.5,
/// 0.5)`, given `xybox` and `boxcoords` when present
/// (`test_annotationbbox_properties`). The coordinates are named as the
/// golden names them when they are those asked for (or the point's,
/// `"data"`), and shown as they are otherwise.
fn annotationbbox_xyann(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let name = args.opt_str("boxcoords")?.unwrap_or("data");
        let expected: Coords = name.parse().map_err(ProbeError::rejected)?;
        let xybox = match args.get("xybox") {
            Some(_) => match args.f64_vec("xybox")?.as_slice() {
                &[x, y] => Some(Point::new(x, y)),
                _ => return Err(ProbeError::BadArgs("xybox is not a point".into())),
            },
            None => None,
        };
        let boxcoords = args
            .opt_str("boxcoords")?
            .map(str::parse)
            .transpose()
            .map_err(ProbeError::rejected)?;
        let options = AnnotationBboxOptions {
            xybox,
            boxcoords,
            ..AnnotationBboxOptions::default()
        };
        let area = DrawingArea::<BoxItem>::new(20.0, 20.0, 0.0, 0.0);
        let ab = Figure::new()
            .subplot()
            .add_annotation_bbox(area, (0.5, 0.5), options)
            .map_err(ProbeError::rejected)?;
        let actual = ab.boxcoords();
        let coords = if actual == expected {
            Json::from(name)
        } else {
            Json::from(format!("{actual:?}"))
        };
        let xyann = ab.xybox();
        Ok(Json::Array(vec![numbers([xyann.x, xyann.y]), coords]))
    })
}

/// What the draggable annotation tests observe: the texts left after
/// removing a draggable annotation and releasing the button, or the text
/// positions of an annotation in a subfigure along presses, scrolls and
/// moves.
fn annotation_draggable(args: Args<'_>) -> Outcome {
    in_test_style(args, || match args.str("scene")? {
        "remove" => {
            let fig = Figure::new();
            let ax = fig.subplot();
            let note = ax
                .annotate("foo", [0.5, 0.5], AnnotateOptions::default())
                .map_err(ProbeError::rejected)?;
            note.set_draggable(true);
            note.remove();
            EventSource::new().button_release(&fig, 1.0, 1.0, PointerButton::Left);
            Ok(Json::from(ax.texts().len() as f64))
        }
        "subfigure" => drag_in_subfigure(),
        other => Err(ProbeError::BadArgs(format!("unknown scene {other}"))),
    })
}

/// The text positions of a draggable annotation in a subfigure: at the
/// start, after a drag from it, after a scroll on it and a move, and after
/// a press beside it and a move (`test_draggable_in_subfigure`).
fn drag_in_subfigure() -> Outcome {
    let fig = Figure::new();
    let sub = fig
        .subfigures(1, 1, SubfigureOptions::default())
        .map_err(ProbeError::rejected)?
        .as_slice()[0]
        .clone();
    let note = sub
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .map_err(ProbeError::rejected)?
        .annotate("foo", [0.0, 0.0], AnnotateOptions::default())
        .map_err(ProbeError::rejected)?;
    note.set_draggable(true);
    fig.canvas().draw().map_err(ProbeError::rejected)?;
    let position = |note: &Annotation| {
        let p = note.position();
        numbers([p.x, p.y])
    };
    let mut out = vec![position(&note)];
    let mut events = EventSource::new();
    let left = PointerButton::Left;
    events.button_press(&fig, 1.0, 1.0, left, false);
    events.motion(&fig, 11.0, 6.0);
    events.button_release(&fig, 11.0, 6.0, left);
    out.push(position(&note));
    events.scroll(&fig, 12.0, 7.0, 1.0);
    events.motion(&fig, 30.0, 30.0);
    out.push(position(&note));
    let extent = note
        .window_extent()
        .ok_or_else(|| ProbeError::rejected("the annotation is not drawn"))?;
    let (x, y) = (extent.x1(), extent.y1());
    events.button_press(&fig, x + 2.0, y + 2.0, left, false);
    events.motion(&fig, x + 40.0, y + 40.0);
    events.button_release(&fig, x + 40.0, y + 40.0, left);
    out.push(position(&note));
    Ok(Json::Array(out))
}
