//! `ContourSet.clabel` probes (`clabel_op` in the spec): label placement
//! on line contours in a figure of given size and axes rectangle.

use pltrs::contour::{
    ClabelFormat, ClabelOptions, ContourOptions, ContourSet, Levels, QuadCoords, QuadField,
};
use pltrs::figure::Figure;
use pltrs::fonts::{FontSize, RelativeSize};
use pltrs::numeric::Grid;

use super::{canon, input};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// Builds the figure, contours and labels, and reports the labels, their
/// widths and the (cut) paths.
pub(super) fn clabel(args: Args<'_>) -> Outcome {
    input::with_rc(args, || {
        let cs = contour_in(args, false)?;
        cs.clabel(clabel_options(Args::new(args.json("clabel")?))?)
            .map_err(ProbeError::rejected)?;
        let labels = cs
            .labels()
            .iter()
            .map(|label| {
                Json::Object(vec![
                    ("level".into(), Json::from(label.level)),
                    ("text".into(), Json::from(label.text.clone())),
                    (
                        "display".into(),
                        Json::from(vec![label.position.x, label.position.y]),
                    ),
                    ("rotation".into(), Json::from(label.rotation)),
                ])
            })
            .collect();
        Ok(Json::Object(vec![
            ("labels".into(), Json::Array(labels)),
            ("label_widths".into(), Json::from(cs.label_widths())),
            (
                "paths".into(),
                Json::Array(cs.paths().iter().map(canon::canonical).collect()),
            ),
        ]))
    })
}

/// The line (or, with `filled`, filled) contours of `x`, `y`, `z` at
/// `levels` in a figure of size `figsize` and `dpi` with the axes at
/// `rect`, limited to `xlim` and `ylim` afterwards.
pub(super) fn contour_in(args: Args<'_>, filled: bool) -> Result<ContourSet, ProbeError> {
    let size = args.f64_vec("figsize")?;
    let rect = args.f64_vec("rect")?;
    let (xlim, ylim) = (args.f64_vec("xlim")?, args.f64_vec("ylim")?);
    let (&[w, h], &[l, b, rw, rh], &[x0, x1], &[y0, y1]) = (
        size.as_slice(),
        rect.as_slice(),
        xlim.as_slice(),
        ylim.as_slice(),
    ) else {
        return Err(ProbeError::BadArgs("bad figure geometry".into()));
    };
    let fig = Figure::with_size(w, h, args.f64("dpi")?);
    let ax = fig.add_axes([l, b, rw, rh]).map_err(ProbeError::rejected)?;
    let z = args
        .json("z")?
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .map(Json::as_f64_vec)
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| ProbeError::BadArgs("bad z".into()))?;
    let field = QuadField::new(Grid::from_rows(z).map_err(ProbeError::rejected)?).with_coords(
        QuadCoords::Axes {
            x: args.f64_vec("x")?,
            y: args.f64_vec("y")?,
        },
    );
    let options = ContourOptions {
        levels: Levels::Values(args.f64_vec("levels")?),
        ..ContourOptions::default()
    };
    let cs = if filled {
        ax.contourf(&field, options)
    } else {
        ax.contour(&field, options)
    }
    .map_err(ProbeError::rejected)?;
    ax.set_xlim(x0, x1).map_err(ProbeError::rejected)?;
    ax.set_ylim(y0, y1).map_err(ProbeError::rejected)?;
    Ok(cs)
}

/// The `clabel` keyword arguments.
pub(super) fn clabel_options(args: Args<'_>) -> Result<ClabelOptions, ProbeError> {
    let mut options = ClabelOptions::default();
    if let Some(levels) = args.get("levels") {
        options.levels = Some(
            levels
                .as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs("bad levels".into()))?,
        );
    }
    options.fontsize = match args.get("fontsize") {
        None => None,
        Some(Json::String(name)) => Some(FontSize::Relative(relative_size(name)?)),
        Some(value) => {
            Some(FontSize::Points(value.as_f64().ok_or_else(|| {
                ProbeError::BadArgs(format!("bad fontsize {value}"))
            })?))
        }
    };
    if let Some(inline) = args.opt_bool("inline")? {
        options.inline = inline;
    }
    if let Some(spacing) = args.opt_f64("inline_spacing")? {
        options.inline_spacing = spacing;
    }
    if let Some(upright) = args.opt_bool("rightside_up")? {
        options.rightside_up = upright;
    }
    if let Some(data) = args.opt_bool("use_clabeltext")? {
        options.use_clabeltext = data;
    }
    options.fmt = match args.get("fmt") {
        None => ClabelFormat::Scalar,
        Some(Json::String(fmt)) => ClabelFormat::Label(fmt.as_str().into()),
        Some(value) => ClabelFormat::Map(
            value
                .as_array()
                .and_then(|pairs| {
                    pairs
                        .iter()
                        .map(|pair| match pair.as_array() {
                            Some([level, text]) => {
                                Some((level.as_f64()?, text.as_str()?.to_owned()))
                            }
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .ok_or_else(|| ProbeError::BadArgs(format!("bad fmt {value}")))?,
        ),
    };
    if args.get("manual").is_some() {
        options.manual = Some(args.pairs("manual")?);
    }
    Ok(options)
}

/// A relative font size keyword.
fn relative_size(name: &str) -> Result<RelativeSize, ProbeError> {
    use RelativeSize::{Large, Larger, Medium, Small, Smaller, XLarge, XSmall, XxLarge, XxSmall};
    [
        XxSmall, XSmall, Small, Medium, Large, XLarge, XxLarge, Larger, Smaller,
    ]
    .into_iter()
    .find(|size| size.name() == name)
    .ok_or_else(|| ProbeError::rejected(format!("unknown font size {name:?}")))
}
