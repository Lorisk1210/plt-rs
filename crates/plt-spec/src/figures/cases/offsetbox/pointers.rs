//! Annotation boxes holding packers, aux-transform and padded boxes, and
//! placed in mixed coordinate systems, of `cases_offsetbox.py`.

use std::error::Error;

use plt::coords::Coords;
use plt::fonts::{FontProperties, FontWeight};
use plt::offsetbox::{
    AnnotationBboxOptions, AuxTransformBox, BoxItem, DrawingArea, OffsetBox, PackAlign, Packer,
    PaddedBox,
};
use plt::patches::{ArrowStyle, BoxStyle, Circle, ConnectionStyle, Rectangle};
use plt::prelude::*;
use plt::text::{FancyArrowProps, TextProps};

use super::{of_t, t, text_area, unit_axes};
use crate::figures::CaseResult;

/// `dict(arrowstyle=style)`.
fn arrow(style: &str) -> Result<FancyArrowProps, Box<dyn Error>> {
    Ok(FancyArrowProps::new(style.parse::<ArrowStyle>()?))
}

/// A coordinate system by its matplotlib name.
fn coords(name: &str) -> Result<Coords, Box<dyn Error>> {
    Ok(name.parse()?)
}

/// A patch of `shape` with face colour `face`.
fn filled<S: plt::patches::PatchShape>(shape: S, face: &str) -> BoxItem {
    BoxItem::patch(shape, PatchProps::default().facecolor(face))
}

pub(super) fn packed_children(fig: &Figure) -> CaseResult {
    let ax = unit_axes(fig);
    let xy = (0.3, 0.4);
    ax.plot_fmt([xy.0], [xy.1], "o")?.set_color("tab:red");
    let mut bars = DrawingArea::new(36.0, 24.0, 0.0, 0.0);
    for (i, height) in [10.0, 24.0, 16.0].into_iter().enumerate() {
        let x = 2.0 + 12.0 * i as f64;
        bars = bars.with_artist(filled(
            Rectangle::new((x, 0.0), 8.0, height),
            &format!("C{i}"),
        ));
    }
    let bold = FontProperties::from_rc(&plt::rc::params()).with_weight(FontWeight::BOLD);
    let card = Packer::new(
        0.0,
        3.0,
        PackAlign::Center,
        vec![
            text_area("sales", TextProps::default().font(bold)),
            bars.into(),
        ],
    );
    let curved = FancyArrowProps {
        connectionstyle: "arc3,rad=-0.2".parse::<ConnectionStyle>()?,
        ..arrow("->")?
    };
    ax.add_annotation_bbox(
        OffsetBox::VPacker(card),
        xy,
        AnnotationBboxOptions {
            xybox: Some((60.0, 50.0).into()),
            boxcoords: Some(coords("offset points")?),
            arrowprops: Some(curved),
            ..AnnotationBboxOptions::default()
        },
    )?;
    let swatch = DrawingArea::new(12.0, 12.0, 0.0, 0.0)
        .with_artist(filled(Circle::new((6.0, 6.0), 6.0), "tab:green"));
    let row = Packer::new(
        0.0,
        4.0,
        PackAlign::Center,
        vec![
            swatch.into(),
            text_area("green marker", TextProps::default()),
        ],
    );
    let green = FancyArrowProps {
        patch: PatchProps::default().color("tab:green"),
        ..arrow("-|>")?
    };
    ax.add_annotation_bbox(
        OffsetBox::HPacker(row),
        (0.75, 0.25),
        AnnotationBboxOptions {
            xybox: Some((0.95, 0.6).into()),
            box_alignment: (1.0, 0.0),
            boxstyle: Some("round".parse::<BoxStyle>()?),
            bboxprops: PatchProps::default().facecolor("0.95"),
            arrowprops: Some(green),
            ..AnnotationBboxOptions::default()
        },
    )?;
    ax.plot_fmt([0.75], [0.25], "s")?.set_color("tab:green");
    Ok(())
}

pub(super) fn aux_and_padded(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 5.0);
    let tilted = Rectangle {
        angle: 20.0,
        ..Rectangle::new((0.0, 0.0), 1.5, 0.6)
    };
    let aux = AuxTransformBox::new(ax.trans_data())
        .with_artist(filled(tilted, "tab:blue"))
        .with_artist(filled(Circle::new((2.2, 0.3), 0.3), "tab:orange"));
    ax.add_annotation_bbox(
        aux,
        (2.0, 1.0),
        AnnotationBboxOptions {
            xybox: Some((6.0, 3.5).into()),
            arrowprops: Some(arrow("->")?),
            ..AnnotationBboxOptions::default()
        },
    )?;
    let padded = PaddedBox::new(
        text_area("padded text", TextProps::default()),
        5.0,
        true,
        PatchProps::default()
            .facecolor("honeydew")
            .edgecolor("tab:green"),
    );
    ax.add_annotation_bbox(
        padded,
        (8.0, 1.0),
        AnnotationBboxOptions {
            xybox: Some((8.0, 2.5).into()),
            frameon: false,
            arrowprops: Some(arrow("-[")?),
            ..AnnotationBboxOptions::default()
        },
    )?;
    ax.plot_fmt([2.0, 8.0], [1.0, 1.0], "k.")?;
    Ok(())
}

pub(super) fn mixed_coords(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot(t(), of_t(f64::sin))?;
    let data_x_axes_y = Coords::blended(Coords::Data, coords("axes fraction")?);
    ax.add_annotation_bbox(
        text_area("blended: x data, y axes", TextProps::default()),
        (5.0, 0.9),
        AnnotationBboxOptions {
            xycoords: data_x_axes_y.clone(),
            xybox: Some((2.0, 0.6).into()),
            boxcoords: Some(data_x_axes_y),
            arrowprops: Some(arrow("->")?),
            ..AnnotationBboxOptions::default()
        },
    )?;
    let wedge = FancyArrowProps {
        patch: PatchProps::default().facecolor("0.8").edgecolor("0.4"),
        ..arrow("wedge,tail_width=0.5")?
    };
    ax.add_annotation_bbox(
        text_area("figure fraction", TextProps::default()),
        (0.5, 0.05),
        AnnotationBboxOptions {
            xycoords: coords("figure fraction")?,
            xybox: Some((0.8, 0.2).into()),
            boxcoords: Some(coords("axes fraction")?),
            annotation_clip: Some(false),
            arrowprops: Some(wedge),
            ..AnnotationBboxOptions::default()
        },
    )?;
    let shrunk = FancyArrowProps {
        shrink_b: 8.0,
        ..arrow("->")?
    };
    ax.add_annotation_bbox(
        text_area("axes pixels", TextProps::default()),
        (8.0, -0.9),
        AnnotationBboxOptions {
            xybox: Some((300.0, 40.0).into()),
            boxcoords: Some(coords("axes pixels")?),
            pad: 0.8,
            boxstyle: Some("larrow".parse::<BoxStyle>()?),
            bboxprops: PatchProps::default().facecolor("lavender"),
            arrowprops: Some(shrunk),
            ..AnnotationBboxOptions::default()
        },
    )?;
    Ok(())
}
