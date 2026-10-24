//! The annotation boxes of `cases_offsetbox.py`: boxes of texts, images
//! and drawing areas placed away from the points they annotate.

use std::error::Error;

use plt::colors::{ColorizerOptions, colormaps};
use plt::coords::Coords;
use plt::fonts::FontSize;
use plt::image::{BboxImageOptions, Interpolation, ScalarField};
use plt::numeric::Grid;
use plt::offsetbox::{AnnotationBboxOptions, BoxItem, DrawingArea, OffsetImage};
use plt::patches::{BoxStyle, Circle, ConnectionStyle, Rectangle};
use plt::prelude::*;
use plt::text::{FancyArrowProps, TextProps};

use super::{gradient, text_area, unit_axes};
use crate::figures::CaseResult;

/// A coordinate system by its matplotlib name.
fn coords(name: &str) -> Result<Coords, Box<dyn Error>> {
    Ok(name.parse()?)
}

/// `("axes fraction", "data")`.
fn axes_x_data_y() -> Result<Coords, Box<dyn Error>> {
    Ok(Coords::blended(coords("axes fraction")?, Coords::Data))
}

pub(super) fn text(fig: &Figure) -> CaseResult {
    let ax = unit_axes(fig)?;
    let xy = (0.5, 0.7);
    ax.plot_fmt([xy.0], [xy.1], ".r")?;
    ax.add_annotation_bbox(
        text_area("Test 1", TextProps::default()),
        xy,
        AnnotationBboxOptions {
            xybox: Some((-20.0, 40.0).into()),
            boxcoords: Some(coords("offset points")?),
            arrowprops: Some("->".parse::<FancyArrowProps>()?),
            boxstyle: Some("sawtooth".parse::<BoxStyle>()?),
            ..AnnotationBboxOptions::default()
        },
    )?;
    ax.add_annotation_bbox(
        text_area("Test 2", TextProps::default()),
        xy,
        AnnotationBboxOptions {
            xybox: Some((1.02, xy.1).into()),
            boxcoords: Some(axes_x_data_y()?),
            box_alignment: (0.0, 0.5),
            arrowprops: Some("->".parse::<FancyArrowProps>()?),
            ..AnnotationBboxOptions::default()
        },
    )?;
    let curved = FancyArrowProps {
        connectionstyle: "arc3,rad=0.3".parse::<ConnectionStyle>()?,
        patch: PatchProps::default().color("tab:red"),
        .."-|>".parse::<FancyArrowProps>()?
    };
    ax.add_annotation_bbox(
        text_area("fontsize 14,\nround box", TextProps::default()),
        (0.2, 0.2),
        AnnotationBboxOptions {
            xybox: Some((0.45, 0.35).into()),
            fontsize: Some(FontSize::Points(14.0)),
            pad: 0.6,
            boxstyle: Some("round".parse::<BoxStyle>()?),
            bboxprops: PatchProps::default().facecolor("lavender"),
            arrowprops: Some(curved),
            ..AnnotationBboxOptions::default()
        },
    )?;
    ax.add_annotation_bbox(
        text_area("hidden: point outside", TextProps::default()),
        (1.5, 0.5),
        AnnotationBboxOptions {
            xybox: Some((0.5, 0.1).into()),
            ..AnnotationBboxOptions::default()
        },
    )?;
    ax.add_annotation_bbox(
        text_area("shown: annotation_clip=False", TextProps::default()),
        (1.5, 0.1),
        AnnotationBboxOptions {
            xybox: Some((0.7, 0.05).into()),
            annotation_clip: Some(false),
            frameon: false,
            arrowprops: Some(FancyArrowProps {
                relpos: (1.0, 0.5),
                .."-[".parse::<FancyArrowProps>()?
            }),
            ..AnnotationBboxOptions::default()
        },
    )?;
    Ok(())
}

pub(super) fn image(fig: &Figure) -> CaseResult {
    let ax = unit_axes(fig)?;
    let xy = (0.3, 0.55);
    ax.plot_fmt([xy.0], [xy.1], "o")?.set_color("tab:red");
    let counts = Grid::from_fn(10, 10, |r, c| i64::try_from(r * 10 + c).unwrap_or(0));
    let counted = OffsetImage::new(
        ScalarField::integers(&counts),
        2.0,
        BboxImageOptions::default(),
    )?;
    ax.add_annotation_bbox(
        counted,
        xy,
        AnnotationBboxOptions {
            xybox: Some((-50.0, 50.0).into()),
            boxcoords: Some(coords("offset points")?),
            pad: 0.3,
            arrowprops: Some("->".parse::<FancyArrowProps>()?),
            ..AnnotationBboxOptions::default()
        },
    )?;
    let small = OffsetImage::new(
        gradient(24, 32),
        1.5,
        BboxImageOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("plasma")?),
            interpolation: Some(Interpolation::Bilinear),
            ..BboxImageOptions::default()
        },
    )?;
    ax.add_annotation_bbox(
        small,
        (0.75, 0.3),
        AnnotationBboxOptions {
            frameon: false,
            ..AnnotationBboxOptions::default()
        },
    )?;
    let (reds, greens) = (linspace(0.0, 1.0, 16), linspace(1.0, 0.0, 12));
    let rgb = Grid::from_fn(12, 16, |r, c| [reds[c], greens[r], 0.5]);
    ax.add_annotation_bbox(
        OffsetImage::new(rgb, 3.0, BboxImageOptions::default())?,
        (0.8, 0.8),
        AnnotationBboxOptions {
            xybox: Some((0.55, 0.85).into()),
            box_alignment: (1.0, 1.0),
            bboxprops: PatchProps::default().edgecolor("tab:blue").linewidth(2.0),
            arrowprops: Some(FancyArrowProps {
                patch: PatchProps::default().color("0.5"),
                .."fancy".parse::<FancyArrowProps>()?
            }),
            ..AnnotationBboxOptions::default()
        },
    )?;
    Ok(())
}

pub(super) fn drawing_area(fig: &Figure) -> CaseResult {
    let ax = unit_axes(fig)?;
    let xy = (0.6, 0.4);
    ax.plot_fmt([xy.0], [xy.1], "s")?.set_color("k");
    let disc = DrawingArea::new(20.0, 20.0, 0.0, 0.0).with_artist(BoxItem::patch(
        Circle::new((10.0, 10.0), 10.0),
        PatchProps::default(),
    ));
    ax.add_annotation_bbox(
        disc,
        xy,
        AnnotationBboxOptions {
            xybox: Some((1.0, xy.1).into()),
            boxcoords: Some(axes_x_data_y()?),
            box_alignment: (0.2, 0.5),
            arrowprops: Some("->".parse::<FancyArrowProps>()?),
            bboxprops: PatchProps::default().alpha(0.5),
            ..AnnotationBboxOptions::default()
        },
    )?;
    let mut bars = DrawingArea::new(36.0, 24.0, 0.0, 0.0);
    for (i, height) in [8.0, 20.0, 14.0].into_iter().enumerate() {
        let left = 2.0 + 12.0 * i as f64;
        bars = bars.with_artist(BoxItem::patch(
            Rectangle::new((left, 0.0), 8.0, height),
            PatchProps::default().facecolor(format!("C{i}")),
        ));
    }
    let elbow = FancyArrowProps {
        connectionstyle: "angle,angleA=0,angleB=90,rad=10".parse::<ConnectionStyle>()?,
        shrink_a: 0.0,
        shrink_b: 5.0,
        mutation_scale: Some(20.0),
        .."-|>".parse::<FancyArrowProps>()?
    };
    ax.add_annotation_bbox(
        bars,
        xy,
        AnnotationBboxOptions {
            xybox: Some((0.2, 0.8).into()),
            boxcoords: Some(coords("axes fraction")?),
            arrowprops: Some(elbow),
            ..AnnotationBboxOptions::default()
        },
    )?;
    ax.add_annotation_bbox(
        text_area("figure pixels", TextProps::default()),
        (0.6, 0.4),
        AnnotationBboxOptions {
            xybox: Some((80.0, 60.0).into()),
            boxcoords: Some(coords("figure pixels")?),
            box_alignment: (0.0, 0.0),
            arrowprops: Some("wedge".parse::<FancyArrowProps>()?),
            ..AnnotationBboxOptions::default()
        },
    )?;
    Ok(())
}
