//! Text probes: window extents, `Text._get_layout`, wrapping and the
//! renderer's `get_text_width_height_descent`.

use plt::artist::ArtistHandle;
use plt::figure::Figure;
use plt::rc;
use plt::text::{Text, measure_text};

use super::args::{apply_text_kw, font_dict, with_rc};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The probe canvas: a 6.4 × 4.8 inch figure.
const FIGSIZE: (f64, f64) = (6.4, 4.8);

fn dpi(args: Args<'_>) -> Result<f64, ProbeError> {
    Ok(args.opt_f64("dpi")?.unwrap_or(100.0))
}

/// `fig.text(x, y, text, **kw)` on a fresh figure (the figure is returned
/// to keep the text alive).
fn make_text(args: Args<'_>) -> Result<(Figure, Text), ProbeError> {
    let fig = Figure::with_size(FIGSIZE.0, FIGSIZE.1, dpi(args)?);
    let text = fig.text(args.f64("x")?, args.f64("y")?, args.str("text")?);
    apply_text_kw(&text, args.get("kw"))?;
    Ok((fig, text))
}

/// `Text.get_window_extent(renderer).get_points()`.
pub(super) fn window_extent(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let (_fig, text) = make_text(args)?;
        text.layout().map_err(ProbeError::rejected)?;
        let extent = text
            .window_extent()
            .ok_or_else(|| ProbeError::rejected("the text has no extent"))?;
        Ok(encode::bbox(&extent))
    })
}

/// `Text._get_layout(renderer)`.
pub(super) fn layout(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let (_fig, text) = make_text(args)?;
        let laid_out = text.layout().map_err(ProbeError::rejected)?;
        let lines = laid_out
            .lines
            .iter()
            .zip(&laid_out.block.lines)
            .map(|(line, placement)| {
                Json::Object(vec![
                    ("text".into(), line.as_str().into()),
                    ("width".into(), placement.width.into()),
                    ("ascent".into(), placement.ascent.into()),
                    ("descent".into(), placement.descent.into()),
                    ("xy".into(), encode::point(placement.origin)),
                ])
            })
            .collect();
        let (corner, (width, height)) = laid_out.block.frame;
        Ok(Json::Object(vec![
            ("bbox".into(), encode::bbox(&laid_out.block.bbox)),
            ("lines".into(), Json::Array(lines)),
            ("box_xy".into(), encode::point(corner)),
            ("box_size".into(), Json::from(vec![width, height])),
        ]))
    })
}

/// `Text._get_wrapped_text()`.
pub(super) fn wrapped_text(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let (_fig, text) = make_text(args)?;
        text.wrapped_text()
            .map(Json::from)
            .map_err(ProbeError::rejected)
    })
}

/// `RendererAgg.get_text_width_height_descent(s, prop, ismath)`.
pub(super) fn width_height_descent(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let font = font_dict(args.get("font"))?;
        let metrics = measure_text(
            args.str("s")?,
            &font,
            args.bool("ismath")?,
            dpi(args)?,
            &rc::params(),
        )
        .map_err(ProbeError::rejected)?;
        Ok(Json::from(vec![
            metrics.width,
            metrics.height,
            metrics.descent,
        ]))
    })
}
