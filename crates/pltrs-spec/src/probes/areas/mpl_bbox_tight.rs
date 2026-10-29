//! Probe operations for the `mpl_bbox_tight` area: matplotlib's own
//! non-image tests of saving with `bbox_inches="tight"` (see
//! `spec/reference/probes/test_mpl_bbox_tight.py`).

use pltrs::artist::ArtistHandle;
use pltrs::axes::{ImshowOptions, PcolorfastOptions};
use pltrs::figure::{BboxInches, Figure, PadInches, SaveOptions};
use pltrs::layout::{ConstrainedLayout, LayoutEngine};
use pltrs::numeric::Grid;
use pltrs::rc::Aspect;
use pltrs::text::AnnotateOptions;
use pltrs::ticker::NullFormatter;
use pltrs_render::RgbaImage;

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Figure.savefig", savefig),
    Op::new("Figure.savefig.noop", savefig_noop),
];

/// The figure saved with `bbox_inches="tight"` and `pad_inches`, as an
/// image.
fn save_tight(fig: &Figure, pad_inches: Option<PadInches>) -> Result<RgbaImage, ProbeError> {
    let options = SaveOptions {
        bbox_inches: Some(BboxInches::Tight),
        pad_inches,
        ..SaveOptions::default()
    };
    Ok(fig
        .render_raster_with(options)
        .map_err(ProbeError::rejected)?
        .to_image())
}

/// `[height, width]` of an image.
fn shape(image: &RgbaImage) -> Json {
    numbers([image.height() as f64, image.width() as f64])
}

/// `[height, width]` of a scene's figure saved tight.
fn savefig(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let pad = match args.str("scene")? {
            "layout_constrained" => {
                let layout = ConstrainedLayout {
                    h_pad: 0.5,
                    ..ConstrainedLayout::default()
                };
                fig.set_layout_engine(Some(LayoutEngine::Constrained(layout)))
                    .map_err(ProbeError::rejected)?;
                let ax = fig.subplot();
                ax.set_aspect(Aspect::Equal).map_err(ProbeError::rejected)?;
                for axis in [ax.xaxis(), ax.yaxis()] {
                    axis.set_major_formatter(NullFormatter)
                        .set_minor_formatter(NullFormatter);
                }
                Some(PadInches::Layout)
            }
            "layout_notconstrained" => {
                fig.subplot();
                Some(PadInches::Layout)
            }
            "non_finite_annotation" => {
                let ax = fig.subplot();
                ax.annotate("", [0.0, f64::NAN], AnnotateOptions::default())
                    .map_err(ProbeError::rejected)?;
                ax.set_axis_off();
                None
            }
            "pcolorfast" => {
                let ax = fig.subplot();
                let values = Grid::from_fn(2, 2, |r, c| (2 * r + c) as f64);
                ax.pcolorfast(None, values, PcolorfastOptions::default())
                    .map_err(ProbeError::rejected)?;
                ax.set_ylim(0.0, 0.1).map_err(ProbeError::rejected)?;
                None
            }
            other => return Err(ProbeError::BadArgs(format!("unknown scene {other}"))),
        };
        Ok(shape(&save_tight(&fig, pad)?))
    })
}

/// `test_noop_tight_bbox`: a figure exactly as large as its rasterized
/// image, saved tight as a PDF then as a PNG.
fn savefig_noop(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::with_size(10.0 / 100.0, 7.0 / 100.0, 100.0);
        fig.set_frameon(false);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .map_err(ProbeError::rejected)?;
        ax.set_axis_off();
        let data = Grid::from_fn(7, 10, |r, c| (10 * r + c) as f64);
        ax.imshow(data, ImshowOptions::default())
            .map_err(ProbeError::rejected)?
            .set_rasterized(true);
        let no_pad = Some(PadInches::Inches(0.0));
        let pdf = SaveOptions {
            bbox_inches: Some(BboxInches::Tight),
            pad_inches: no_pad,
            ..SaveOptions::default()
        };
        fig.to_pdf_with(pdf).map_err(ProbeError::rejected)?;
        let image = save_tight(&fig, no_pad)?;
        let (pixels, _) = image.data().as_chunks::<4>();
        Ok(Json::Object(vec![
            ("shape".into(), shape(&image)),
            (
                "opaque".into(),
                Json::Bool(pixels.iter().all(|&[.., alpha]| alpha == 255)),
            ),
            (
                "white".into(),
                Json::Bool(pixels.iter().all(|&[r, g, b, _]| [r, g, b] == [255; 3])),
            ),
        ]))
    })
}
