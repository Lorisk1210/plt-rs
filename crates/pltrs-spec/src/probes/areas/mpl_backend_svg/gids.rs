//! Group ids in the SVG: every visible artist of `test_gid`'s figure, and
//! an annotation box (`test_annotationbbox_gid`).

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes::{BarOptions, ImshowOptions};
use pltrs::collections::Collection;
use pltrs::figure::Figure;
use pltrs::image::BboxImageOptions;
use pltrs::legend::LegendOptions;
use pltrs::lines::Line2D;
use pltrs::mplot3d::Axes3DOptions;
use pltrs::numeric::Grid;
use pltrs::offsetbox::{AnnotationBboxOptions, OffsetImage};
use pltrs::rc::Aspect;
use pltrs::text::{FancyArrowProps, Text};
use pltrs_core::Point;

use super::super::mpl_tests::in_test_style;
use super::svg;
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// The kind of artist `handle` is, as the probe reports it.
fn kind(handle: &RawHandle) -> &'static str {
    if handle.downcast::<Text>().is_some() {
        "Text"
    } else if handle.downcast::<Line2D>().is_some() {
        "Line2D"
    } else if handle.downcast::<Collection>().is_some() {
        "Collection"
    } else {
        "Artist"
    }
}

/// `test_gid`'s `include`: whether the artist draws something, so that
/// its group id must be in the SVG. Texts without text or outside axes
/// and lines of one point or outside axes are left out.
fn drawn(handle: &RawHandle) -> bool {
    if let Some(text) = handle.downcast::<Text>() {
        return !text.text().is_empty() && text.axes().is_some();
    }
    if let Some(line) = handle.downcast::<Line2D>() {
        let (x, y) = line.data();
        return !(x.len() == 1 && y.len() == 1) && line.axes().is_some();
    }
    true
}

/// The kinds of the visible artists of `test_gid`'s figure whose group
/// id is missing from the SVG.
pub(super) fn missing_gids(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax1 = fig.add_subplot(1, 3, 1).map_err(ProbeError::rejected)?;
        let image = Grid::from_vec(2, 2, vec![1.0, 2.0, 2.0, 3.0]).map_err(ProbeError::rejected)?;
        let options = ImshowOptions {
            aspect: Some(Aspect::Auto),
            ..ImshowOptions::default()
        };
        ax1.imshow(image, options).map_err(ProbeError::rejected)?;
        ax1.scatter([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])
            .map_err(ProbeError::rejected)?
            .set_label("myscatter");
        ax1.plot([0.0, 1.0, 2.0], [2.0, 3.0, 1.0])
            .map_err(ProbeError::rejected)?
            .set_label("myplot");
        ax1.legend(LegendOptions::default())
            .map_err(ProbeError::rejected)?;
        ax1.twinx()
            .map_err(ProbeError::rejected)?
            .bar([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], BarOptions::default())
            .map_err(ProbeError::rejected)?;
        fig.add_polar_subplot(1, 3, 2)
            .map_err(ProbeError::rejected)?
            .plot([0.0, 1.5, 3.0], [1.0, 2.0, 3.0])
            .map_err(ProbeError::rejected)?;
        fig.add_subplot_3d(1, 3, 3, Axes3DOptions::default())
            .map_err(ProbeError::rejected)?
            .plot([1.0, 2.0], [1.0, 2.0], [1.0, 2.0])
            .map_err(ProbeError::rejected)?;
        fig.layout();
        let artists: Vec<(String, RawHandle)> = fig
            .findobj(|_| true)
            .into_iter()
            .enumerate()
            .filter(|(_, artist)| artist.visible())
            .map(|(index, artist)| {
                let gid = format!("test123{}_{index}", kind(&artist));
                artist.set_gid(gid.clone());
                (gid, artist)
            })
            .collect();
        let document = svg(&fig)?;
        let mut missing: Vec<&str> = artists
            .iter()
            .filter(|(gid, artist)| drawn(artist) && !document.contains(gid.as_str()))
            .map(|(_, artist)| kind(artist))
            .collect();
        missing.sort_unstable();
        Ok(Json::Array(missing.into_iter().map(Json::from).collect()))
    })
}

/// Whether an annotation box of an image with an arrow writes its group
/// id.
pub(super) fn annotation_box_gid(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let ones = Grid::from_vec(32, 32, vec![1.0; 32 * 32]).map_err(ProbeError::rejected)?;
        let image = OffsetImage::new(ones, 0.1, BboxImageOptions::default())
            .map_err(ProbeError::rejected)?;
        let mut arrow = FancyArrowProps::new("->".parse().map_err(ProbeError::rejected)?);
        arrow.connectionstyle = "angle,angleA=0,angleB=90,rad=3"
            .parse()
            .map_err(ProbeError::rejected)?;
        let options = AnnotationBboxOptions {
            xybox: Some(Point::new(120.0, -80.0)),
            boxcoords: Some("offset points".parse().map_err(ProbeError::rejected)?),
            pad: 0.5,
            arrowprops: Some(arrow),
            ..AnnotationBboxOptions::default()
        };
        ax.add_annotation_bbox(image, (0.3, 0.55), options)
            .map_err(ProbeError::rejected)?
            .set_gid("a test for issue 20044");
        Ok(Json::Bool(
            svg(&fig)?.contains("<g id=\"a test for issue 20044\">"),
        ))
    })
}
