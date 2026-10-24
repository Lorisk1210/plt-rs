//! Sizes, resolution and saving: `set_size_inches` and its siblings, the
//! renderer's size after a change of resolution, files saved to paths,
//! metadata, hidden figures and colorbars placed while saving a region.

use plt::artist::ArtistHandle;
use plt::axes::{Mesh, PcolorOptions};
use plt::colorbar::ColorbarOptions;
use plt::figure::{BboxInches, Figure, Metadata, SaveFormat, SaveOptions};
use plt::numeric::Grid;
use plt_core::Bbox;

use super::super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Args, Outcome, ProbeError};
use crate::rng::SpecRng;
use crate::scratch::ScratchDir;

/// The size after each call `[method, *args]` on a new figure.
pub(super) fn set_size_inches(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let mut out = Vec::new();
        for call in args.json("calls")?.as_array().unwrap_or(&[]) {
            let parts = call.as_array().unwrap_or(&[]);
            let numbers_of = |items: &[Json]| -> Vec<f64> {
                items
                    .iter()
                    .flat_map(|item| {
                        item.as_f64_vec()
                            .unwrap_or_else(|| item.as_f64().into_iter().collect())
                    })
                    .collect()
            };
            let (name, values) = match parts.split_first() {
                Some((Json::String(name), rest)) => (name.as_str(), numbers_of(rest)),
                _ => return Err(ProbeError::BadArgs(format!("bad call {call}"))),
            };
            match (name, values.as_slice()) {
                ("set_figwidth", [width]) => fig.set_figwidth(*width),
                ("set_figheight", [height]) => fig.set_figheight(*height),
                ("set_size_inches", [width, height]) => fig.set_size_inches(*width, *height),
                _ => return Err(ProbeError::BadArgs(format!("bad call {call}"))),
            }
            .map_err(ProbeError::rejected)?;
            let (width, height) = fig.size_inches();
            out.push(numbers([width, height]));
        }
        Ok(Json::Array(out))
    })
}

/// The renderer's size of a 4 x 4 in figure at 100 dpi, then at 50.
pub(super) fn set_dpi(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::with_size(4.0, 4.0, plt::rc::params().figure_dpi);
        let size = |fig: &Figure| {
            fig.layout();
            let (width, height) = fig.width_height();
            numbers([f64::from(width), f64::from(height)])
        };
        let before = size(&fig);
        fig.set_dpi(50.0);
        Ok(Json::Array(vec![before, size(&fig)]))
    })
}

/// Saves to a path named after the format and checks the file's first 100
/// bytes name it, or saves with empty metadata.
pub(super) fn savefig(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let name = args.str("format")?;
        let fig = plt::pyplot::figure();
        if args.contains("metadata") {
            let format: SaveFormat = name.parse().map_err(ProbeError::rejected)?;
            let options = SaveOptions {
                metadata: Some(Metadata::default()),
                ..SaveOptions::default()
            };
            fig.to_format(format, options)
                .map_err(ProbeError::rejected)?;
            return Ok(Json::Bool(true));
        }
        let dir = ScratchDir::new("mpl-figure").map_err(ProbeError::rejected)?;
        let path = dir.path(&format!("test.{name}"));
        plt::pyplot::savefig(&path).map_err(ProbeError::rejected)?;
        let saved = std::fs::read(&path).map_err(ProbeError::rejected)?;
        let head: Vec<u8> = saved.into_iter().take(100).collect();
        let head = String::from_utf8_lossy(&head).to_lowercase();
        Ok(Json::Bool(head.contains(name)))
    })
}

/// Whether the SVG of an empty figure draws a path while the figure is
/// visible, then hidden.
pub(super) fn set_visible(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let mut out = Vec::new();
        for visible in [true, false] {
            fig.set_visible(visible);
            let svg = fig.to_svg().map_err(ProbeError::rejected)?;
            out.push(svg.contains("<path "));
        }
        Ok(Json::from(out))
    })
}

/// A colorbar axes' original and actual positions after saving the region
/// `[[0, 0], [4, 4]]` inches.
pub(super) fn locate_colorbar(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let values = SpecRng::new(2).normal(4, 0.0, 1.0);
        let grid = Grid::from_vec(2, 2, values).map_err(ProbeError::rejected)?;
        let mesh = ax
            .pcolormesh(Mesh::new(grid), PcolorOptions::default())
            .map_err(ProbeError::rejected)?;
        let options = ColorbarOptions {
            aspect: 40.0,
            ..ColorbarOptions::default()
        };
        let cbar = fig.colorbar(&mesh, options).map_err(ProbeError::rejected)?;
        let save = SaveOptions {
            bbox_inches: Some(BboxInches::Inches(Bbox::from_extents(0.0, 0.0, 4.0, 4.0))),
            ..SaveOptions::default()
        };
        fig.to_png_with(save).map_err(ProbeError::rejected)?;
        let bounds = |b: Bbox| numbers([b.x0(), b.y0(), b.width(), b.height()]);
        Ok(Json::Array(vec![
            bounds(cbar.ax().original_position()),
            bounds(cbar.ax().position()),
        ]))
    })
}
