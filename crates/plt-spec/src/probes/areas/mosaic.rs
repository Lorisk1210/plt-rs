//! Probe operations for the `mosaic` area: `Figure::subplot_mosaic` from
//! strings and nested lists.

use plt::figure::{Figure, Mosaic, MosaicCell, MosaicOptions};

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("figure.subplot_mosaic", subplot_mosaic)];

/// A mosaic from nested JSON lists: strings label cells, `null` and the
/// string `empty` (matplotlib's `empty_sentinel`, if any) leave them empty
/// and lists nest mosaics.
pub(crate) fn nested(rows: &[Json], empty: Option<&str>) -> Result<Mosaic, ProbeError> {
    let bad = || ProbeError::BadArgs("mosaic is malformed".into());
    let rows = rows
        .iter()
        .map(|row| {
            row.as_array()
                .ok_or_else(bad)?
                .iter()
                .map(|cell| match cell {
                    Json::String(label) if Some(label.as_str()) == empty => Ok(MosaicCell::Empty),
                    Json::String(label) => Ok(MosaicCell::Label(label.clone())),
                    Json::Null => Ok(MosaicCell::Empty),
                    Json::Array(inner) => nested(inner, empty).map(MosaicCell::Nested),
                    _ => Err(bad()),
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    Mosaic::new(rows).map_err(ProbeError::rejected)
}

/// Every axes of the mosaic in creation order, with its position.
fn subplot_mosaic(args: Args<'_>) -> Outcome {
    let mosaic = match args.json("mosaic")? {
        Json::String(layout) => layout.parse::<Mosaic>().map_err(ProbeError::rejected)?,
        Json::Array(rows) => nested(rows, None)?,
        _ => return Err(ProbeError::BadArgs("mosaic is malformed".into())),
    };
    let ratios = |key: &str| args.get(key).map(|_| args.f64_vec(key)).transpose();
    let options = MosaicOptions {
        width_ratios: ratios("width_ratios")?,
        height_ratios: ratios("height_ratios")?,
        ..MosaicOptions::default()
    };
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let axes = fig
        .subplot_mosaic(&mosaic, options)
        .map_err(ProbeError::rejected)?;
    Ok(Json::Array(
        axes.iter()
            .map(|(label, ax)| {
                let bbox = ax.position();
                let bounds = [bbox.x0(), bbox.y0(), bbox.width(), bbox.height()];
                Json::Array(vec![
                    Json::String(label.to_owned()),
                    Json::Array(bounds.into_iter().map(Json::Number).collect()),
                ])
            })
            .collect(),
    ))
}
