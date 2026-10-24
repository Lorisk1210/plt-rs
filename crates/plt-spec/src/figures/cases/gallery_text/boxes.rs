//! Annotation boxes: a text box annotating a connection between two axes,
//! images and a drawing area placed away from the points they annotate
//! (`demo_annotation_box`).

use plt::colors::{ColorizerOptions, colormaps};
use plt::coords::Coords;
use plt::figure::{Mosaic, MosaicCell, MosaicOptions};
use plt::image::{BboxImageOptions, ScalarField};
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::offsetbox::{AnnotationBboxOptions, BoxItem, DrawingArea, OffsetImage, TextArea};
use plt::patches::{Annulus, BoxStyle, Circle, ConnectionEnd, ConnectionPatch};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{FancyArrowProps, TextProps};

use crate::figures::cases::common::arrow;
use crate::figures::{CaseResult, FigureCase};

/// A big `X` marker of `size` points² at `xy`.
fn cross(ax: &Axes, (x, y): (f64, f64), size: f64) -> CaseResult {
    ax.scatter_with(
        vec![x],
        vec![y],
        ScatterOptions {
            sizes: Some(vec![size]),
            marker: Some("X".parse()?),
            ..ScatterOptions::default()
        },
    )?;
    Ok(())
}

/// A smooth RGB image the size of the gallery's Grace Hopper photograph
/// (512 × 600 pixels), standing in for it.
fn photo() -> Grid<[u8; 3]> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "numpy's astype(np.uint8) of values in [0, 255]"
    )]
    let byte = |v: f64| (v * 255.0) as u8;
    Grid::from_fn(600, 512, |row, col| {
        #[expect(clippy::cast_precision_loss, reason = "small pixel indices")]
        let (x, y) = (col as f64, row as f64);
        let red = 0.5 + 0.5 * (x / 40.0).sin() * (y / 55.0).cos();
        let green = (x + y) / (512.0 + 600.0);
        let blue = 0.5 + 0.5 * ((x - 256.0).hypot(y - 250.0) / 30.0).cos();
        [byte(red), byte(green), byte(blue)]
    })
}

fn demo_annotation_box_1(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::compressed())?;
    let label = |name: &str| MosaicCell::Label(name.into());
    let mosaic = Mosaic::new(vec![vec![label("t1"), MosaicCell::Empty, label("t2")]])?;
    let axd = fig.subplot_mosaic(&mosaic, MosaicOptions::default())?;
    let (t1, t2) = (
        axd.get("t1").ok_or("no axes t1")?,
        axd.get("t2").ok_or("no axes t2")?,
    );
    let (xy1, xy2) = ((0.25, 0.75), (0.75, 0.25));
    t1.plot_fmt([xy1.0], [xy1.1], ".r")?;
    t2.plot_fmt([xy2.0], [xy2.1], ".r")?;
    for ax in [t1, t2] {
        ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
        ax.set_aspect(Aspect::Equal)?;
    }
    let mut connection = ConnectionPatch::new(
        ConnectionEnd::new(xy1, Coords::Transform(t1.trans_data()), None),
        ConnectionEnd::new(xy2, Coords::Transform(t2.trans_data()), None),
    );
    connection.spec.arrowstyle = "->".parse()?;
    let connection = fig.add_artist(connection);
    fig.add_annotation_bbox(
        TextArea::from_text("Test 1", TextProps::default()),
        (0.5, 0.5),
        AnnotationBboxOptions {
            xycoords: Coords::Artist(connection.id()),
            xybox: Some((0.0, 30.0).into()),
            boxcoords: Some("offset points".parse()?),
            arrowprops: Some("->".parse::<FancyArrowProps>()?),
            boxstyle: Some("sawtooth".parse::<BoxStyle>()?),
            ..AnnotationBboxOptions::default()
        },
    )?;
    Ok(())
}

fn demo_annotation_box_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let xy = (0.3, 0.55);
    cross(&ax, xy, 200.0)?;
    let counts = Grid::from_fn(10, 10, |row, col| {
        i64::try_from(row * 10 + col).unwrap_or_default()
    });
    let counted = OffsetImage::new(
        ScalarField::integers(&counts),
        2.0,
        BboxImageOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("viridis")?),
            ..BboxImageOptions::default()
        },
    )?;
    let offset_points: Coords = "offset points".parse()?;
    ax.add_annotation_bbox(
        counted,
        xy,
        AnnotationBboxOptions {
            xybox: Some((-50.0, 50.0).into()),
            boxcoords: Some(offset_points.clone()),
            pad: 0.3,
            arrowprops: Some("->".parse::<FancyArrowProps>()?),
            ..AnnotationBboxOptions::default()
        },
    )?;
    ax.add_annotation_bbox(
        OffsetImage::new(photo(), 0.2, BboxImageOptions::default())?,
        xy,
        AnnotationBboxOptions {
            xybox: Some((120.0, -80.0).into()),
            boxcoords: Some(offset_points),
            pad: 0.5,
            arrowprops: Some(arrow("->", "angle,angleA=0,angleB=90,rad=3")?),
            ..AnnotationBboxOptions::default()
        },
    )?;
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    Ok(())
}

fn demo_annotation_box_3(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let xy = (0.05, 0.5);
    cross(&ax, xy, 500.0)?;
    let area = DrawingArea::new(120.0, 120.0, 0.0, 0.0)
        .with_artist(BoxItem::patch(
            Circle::new((30.0, 30.0), 25.0),
            PatchProps::default().color("C0"),
        ))
        .with_artist(BoxItem::patch(
            Annulus::new((65.0, 65.0), 50.0, 5.0)?,
            PatchProps::default().color("C1"),
        ));
    ax.add_annotation_bbox(
        area,
        xy,
        AnnotationBboxOptions {
            xybox: Some((0.55, xy.1).into()),
            boxcoords: Some(Coords::blended("axes fraction".parse()?, Coords::Data)),
            box_alignment: (0.0, 0.5),
            arrowprops: Some("->".parse::<FancyArrowProps>()?),
            bboxprops: PatchProps::default().alpha(0.5),
            ..AnnotationBboxOptions::default()
        },
    )?;
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_text/demo_annotation_box_1", demo_annotation_box_1),
    FigureCase::new("gallery_text/demo_annotation_box_2", demo_annotation_box_2),
    FigureCase::new("gallery_text/demo_annotation_box_3", demo_annotation_box_3),
];
