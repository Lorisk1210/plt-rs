//! The interactive colorbar op of the `mpl_backend_bases` area: the
//! toolbar's zoom and pan on a colorbar change its norm (matplotlib's
//! `test_interactive_colorbar`).

use pltrs::axes::ImshowOptions;
use pltrs::colorbar::{Colorbar, ColorbarOptions};
use pltrs::contour::{ContourOptions, QuadField};
use pltrs::figure::Figure;
use pltrs::image::AxesImage;
use pltrs::navigation::DragKey;
use pltrs::numeric::Grid;
use pltrs::orientation::Orientation;

use super::toolbar::{button, drag, pixel, tool_mode, toolbar};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};
use pltrs::colors::{Colorize, ColorizerOptions};

/// The colorbar of `plot_func(arange(12).reshape(4, 3), vmin=0,
/// vmax=10)` in `orientation`, with the image drawn (none for contours).
fn scene(fig: &Figure, args: Args<'_>) -> Result<(Colorbar, Option<AxesImage>), ProbeError> {
    let ax = fig.subplot();
    let data = Grid::from_fn(4, 3, |row, column| (row * 3 + column) as f64);
    let options = ColorbarOptions {
        orientation: Some(match args.str("orientation")? {
            "vertical" => Orientation::Vertical,
            "horizontal" => Orientation::Horizontal,
            other => return Err(ProbeError::BadArgs(format!("unknown orientation {other}"))),
        }),
        ..ColorbarOptions::default()
    };
    match args.str("plot_func")? {
        "imshow" => {
            let image = ax
                .imshow(data, ImshowOptions::default().limits(Some(0.0), Some(10.0)))
                .map_err(ProbeError::rejected)?;
            let colorbar = fig
                .colorbar(&image, options)
                .map_err(ProbeError::rejected)?;
            Ok((colorbar, Some(image)))
        }
        "contourf" => {
            let limits = ContourOptions {
                colorizer: ColorizerOptions {
                    vmin: Some(0.0),
                    vmax: Some(10.0),
                    ..ColorizerOptions::default()
                },
                ..ContourOptions::default()
            };
            let contours = ax
                .contourf(&QuadField::new(data), limits)
                .map_err(ProbeError::rejected)?;
            let colorbar = fig
                .colorbar(&contours, options)
                .map_err(ProbeError::rejected)?;
            Ok((colorbar, None))
        }
        other => Err(ProbeError::BadArgs(format!("unknown plot {other}"))),
    }
}

/// Whether the colorbar's axes navigate, and if they do the limits of the
/// image's norm after a drag on the colorbar with the toolbar.
pub(super) fn navigation(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let (colorbar, image) = scene(&fig, args)?;
    fig.layout();
    if !colorbar.ax().navigate() {
        return Ok(Json::Array(vec![Json::from(false)]));
    }
    let image = image.ok_or_else(|| ProbeError::rejected("the contours' colorbar navigates"))?;
    let mut navigation = toolbar(&fig, tool_mode(args)?)?;
    let path = (pixel(args, "start")?, pixel(args, "end")?);
    drag(
        &mut navigation,
        &fig,
        path,
        button(args.i64("button")?)?,
        DragKey::None,
        true,
    );
    let limit = |value: Option<f64>| value.map_or(Json::Null, Json::from);
    let (vmin, vmax) = image.clim();
    Ok(Json::Array(vec![
        Json::from(true),
        limit(vmin),
        limit(vmax),
    ]))
}
