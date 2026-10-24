//! Probe operations for the `offsetbox_behaviour` area: the packing and
//! alignment rules of offset boxes and the extents of anchored texts and
//! annotation boxes, ported from matplotlib's `test_offsetbox.py` (see
//! `spec/reference/probes/test_offsetbox_behaviour.py`). Every op runs
//! under the reference configuration.

use plt::artist::ArtistHandle;
use plt::figure::Figure;
use plt::image::BboxImageOptions;
use plt::numeric::Grid;
use plt::offsetbox::{
    AnchorLoc, AnchoredBox, AnchoredOptions, AnnotationBboxOptions, BoxItem, DrawingArea,
    OffsetImage, PackAlign, PackMode, aligned_offsets, packed_offsets,
};
use plt::patches::{ArrowStyle, Circle, PatchProps};
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps, HAlign, TextProps, VAlign};
use plt_core::{Bbox, Point};

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::bbox;
use crate::probes::{Args, Op, Outcome, ProbeError, made};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("offsetbox.packed_offsets", |args| {
        with_rc(args, || packed(args))
    }),
    Op::new("offsetbox.aligned_offsets", |args| {
        with_rc(args, || aligned(args))
    }),
    Op::new("offsetbox.extents", |args| with_rc(args, || extents(args))),
];

/// `_get_packed_offsets`; `equal` packing needs a total or a separation.
fn packed(args: Args<'_>) -> Outcome {
    let widths = args.f64_vec("widths")?;
    let total = args.opt_f64("total")?;
    let sep = args.opt_f64("sep")?;
    let mode = match args.str("mode")? {
        "fixed" => PackMode::Fixed,
        "expand" => PackMode::Expand,
        "equal" => PackMode::Equal,
        other => return Err(ProbeError::BadArgs(format!("unknown mode {other:?}"))),
    };
    let sep = match (mode, total, sep) {
        (_, _, Some(sep)) => sep,
        (PackMode::Expand, _, None) | (PackMode::Equal, Some(_), None) => 0.0,
        _ => return Err(ProbeError::rejected("a separation is needed")),
    };
    let (total, offsets) = packed_offsets(&widths, total, sep, mode);
    Ok(Json::Array(vec![total.into(), Json::from(offsets)]))
}

/// `_get_aligned_offsets`.
fn aligned(args: Args<'_>) -> Outcome {
    let spans = args.pairs("yspans")?;
    let align = match args.str("align")? {
        "baseline" => PackAlign::Baseline,
        "left" => PackAlign::Left,
        "bottom" => PackAlign::Bottom,
        "right" => PackAlign::Right,
        "top" => PackAlign::Top,
        "center" => PackAlign::Center,
        other => return Err(ProbeError::BadArgs(format!("unknown align {other:?}"))),
    };
    let ((y0, y1), offsets) = aligned_offsets(&spans, args.opt_f64("height")?, align);
    Ok(Json::Array(vec![
        Json::from(vec![y0, y1]),
        Json::from(offsets),
    ]))
}

fn arrow() -> Result<FancyArrowProps, ProbeError> {
    Ok(FancyArrowProps::new(made("->".parse::<ArrowStyle>())?))
}

/// The extents of anchored texts or annotation boxes, and the figure's
/// tight box.
fn extents(args: Args<'_>) -> Outcome {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let ax = fig.subplot();
    let extent = |extent: Option<Bbox>| bbox(&extent.unwrap_or(Bbox::NULL));
    match args.str("scene")? {
        "borderpad" => {
            let boxes = [
                ("float", (5.0, 5.0)),
                ("tuple", (5.0, 5.0)),
                ("tuple_asym", (10.0, 4.0)),
            ]
            .into_iter()
            .map(|(text, borderpad)| {
                let options = AnchoredOptions {
                    borderpad,
                    ..AnchoredOptions::new(AnchorLoc::LowerLeft)
                };
                let anchored = made(AnchoredBox::text(text, TextProps::default(), options))?;
                Ok(ax.add_anchored_box(anchored))
            })
            .collect::<Result<Vec<_>, ProbeError>>()?;
            Ok(Json::Array(
                boxes.iter().map(|b| extent(b.window_extent())).collect(),
            ))
        }
        "annotation_boxes" => {
            ax.set_xlim(0.0, 1.0)
                .and_then(|ax| ax.set_ylim(0.0, 1.0))
                .map_err(ProbeError::rejected)?;
            let options = AnnotateOptions {
                xytext: Some(Point::new(1.1, 1.1)),
                arrowprops: Some(ArrowProps::Fancy(arrow()?)),
                ..AnnotateOptions::default()
            };
            let annotation = made(ax.annotate("Annotation", (0.9, 0.9), options))?;
            annotation
                .set_clip_on(false)
                .set_valign(VAlign::Baseline)
                .set_halign(HAlign::Left);
            let mut area = DrawingArea::new(20.0, 20.0, 0.0, 0.0).with_artist(BoxItem::patch(
                Circle::new((-10.0, 30.0), 32.0),
                PatchProps::default(),
            ));
            area.clip = true;
            let drawing = made(ax.add_annotation_bbox(
                area,
                (0.5, 0.5),
                AnnotationBboxOptions {
                    xybox: Some(Point::new(-0.2, 0.5)),
                    boxcoords: Some(made("axes fraction".parse())?),
                    box_alignment: (0.0, 0.5),
                    arrowprops: Some(arrow()?),
                    ..AnnotationBboxOptions::default()
                },
            ))?;
            let data = Grid::from_fn(10, 10, |r, c| ((3 * r + 7 * c) % 10) as f64 / 10.0);
            let image = made(OffsetImage::new(data, 3.0, BboxImageOptions::default()))?;
            let pictured = made(ax.add_annotation_bbox(
                image,
                (0.5, -0.3),
                AnnotationBboxOptions {
                    xybox: Some(Point::new(0.0, 75.0)),
                    xycoords: made("axes fraction".parse())?,
                    boxcoords: Some(made("offset points".parse())?),
                    pad: 0.3,
                    arrowprops: Some(arrow()?),
                    ..AnnotationBboxOptions::default()
                },
            ))?;
            // `fig.canvas.draw()`: drawn arrows measure at the canvas dpi.
            fig.layout();
            Ok(Json::Array(vec![
                extent(annotation.window_extent()),
                extent(drawing.window_extent()),
                extent(pictured.window_extent()),
                bbox(&fig.tight_bbox()),
            ]))
        }
        other => Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }
}
