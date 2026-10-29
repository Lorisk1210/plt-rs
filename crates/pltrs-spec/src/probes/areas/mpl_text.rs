//! Probe operations for the `mpl_text` area: matplotlib's own non-image
//! tests of texts and mathtext that the other text areas leave out (see
//! `spec/reference/probes/test_mpl_text.py`).

use pltrs::colors::Color;
use pltrs::figure::Figure;
use pltrs::fonts::{FontProperties, FontSize, Fontset};
use pltrs::image::imread_bytes;
use pltrs::text::{AnnotateOptions, MathImageOptions};

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("FontProperties.get_size_in_points", size_in_points),
    Op::new("Figure.stale", stale),
    Op::new("Text.get_math_fontfamily", math_fontfamily),
    Op::new("mathtext.math_to_image", math_to_image),
];

/// `[depth, height, width]` of an expression rendered alone as a PNG
/// (`test_math_to_image`).
fn math_to_image(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let options = MathImageOptions {
            color: args.opt_str("color")?.map(Color::from),
            ..MathImageOptions::default()
        };
        let image =
            pltrs::text::math_to_image(args.str("s")?, options).map_err(ProbeError::rejected)?;
        let (rows, columns) = imread_bytes(&image.data)
            .map_err(ProbeError::rejected)?
            .shape();
        Ok(numbers([image.depth, rows as f64, columns as f64]))
    })
}

/// The size of a font description made with `font.size = base`, read
/// once `font.size` is `later`.
fn size_in_points(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let size: FontSize = match args.json("size")? {
            Json::String(name) => name.parse().map_err(ProbeError::rejected)?,
            _ => FontSize::Points(args.f64("size")?),
        };
        let set_base = |key| -> Result<(), ProbeError> {
            pltrs::rc::set("font.size", &args.f64(key)?.to_string()).map_err(ProbeError::rejected)
        };
        set_base("base")?;
        let params = pltrs::rc::params();
        let font = FontProperties::from_rc(&params).with_size(size, params.font_size);
        set_base("later")?;
        Ok(Json::from(font.size))
    })
}

/// The figure's staleness after a draw, a text, an annotation and another
/// draw (`test_text_stale`).
fn stale(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let axes = fig.subplots(1, 2).map_err(ProbeError::rejected)?;
        let draw = || fig.canvas().draw().map_err(ProbeError::rejected);
        draw()?;
        let mut out = vec![fig.is_stale()];
        axes[(0, 0)].text(0.5, 0.5, "aardvark");
        out.push(fig.is_stale());
        axes[(0, 1)]
            .annotate("aardvark", [0.5, 0.5], AnnotateOptions::default())
            .map_err(ProbeError::rejected)?;
        out.push(fig.is_stale());
        draw()?;
        out.push(fig.is_stale());
        Ok(Json::from(out))
    })
}

/// The math fonts of a figure text set in the font `font` describes,
/// then given `math_fontfamily` when present.
fn math_fontfamily(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let text = fig.text(0.1, 0.1, r"abc$abc\alpha$");
        let font = FontProperties::from_pattern(args.str("font")?, &pltrs::rc::params())
            .map_err(ProbeError::rejected)?;
        text.set_font(font);
        if let Some(name) = args.opt_str("math_fontfamily")? {
            text.set_math_fontfamily(name.parse::<Fontset>().map_err(ProbeError::rejected)?);
        }
        Ok(Json::from(text.font().math_fontfamily.name()))
    })
}
