//! The second half of `annotations`: anchored texts, drawing areas and
//! packed boxes, annotation boxes placed relative to other artists, and
//! annotations whose points are given in the coordinates of other axes,
//! artists or offsets from them, and a connection between two axes
//! (`annotations_13` to `annotations_26`).

use plt::artist::ArtistHandle;
use plt::axes_grid1::anchored_artists::{AnchoredAuxTransformBox, AnchoredDrawingArea};
use plt::colors::{ColorizerOptions, colormaps};
use plt::coords::{CoordUnit, Coords};
use plt::gridspec::SubplotOverrides;
use plt::image::BboxImageOptions;
use plt::numeric::Grid;
use plt::offsetbox::{
    AnchorLoc, AnchoredBox, AnchoredOptions, AnnotationBboxOptions, BoxItem, DrawingArea,
    OffsetBox, OffsetImage, PackAlign, Packer, TextArea,
};
use plt::patches::{Annulus, BoxStyle, Circle, ConnectionEnd, ConnectionPatch};
use plt::prelude::*;
use plt::text::{
    AnnotateOptions, Annotation, ArrowProps, FancyArrowProps, HAlign, TextProps, VAlign,
};
use plt::transforms::Point;

use crate::figures::cases::common::{ellipse, sized, styled_box};
use crate::figures::{CaseResult, FigureCase};

/// The `->` arrow of most of the guide's annotations.
fn plain_arrow() -> Result<FancyArrowProps, Box<dyn std::error::Error>> {
    Ok("->".parse::<FancyArrowProps>()?)
}

fn annotations_13(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let at = ax.add_anchored_box(AnchoredBox::text(
        "Figure 1a",
        TextProps::default().font(sized(15.0)),
        AnchoredOptions::new(AnchorLoc::UpperLeft),
    )?);
    let rounded: BoxStyle = "round,pad=0.,rounding_size=0.2".parse()?;
    at.patch().update_shape(|frame| frame.boxstyle = rounded);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn annotations_14(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let ada = AnchoredDrawingArea {
        pad: 0.0,
        frameon: false,
        ..AnchoredDrawingArea::new(40.0, 20.0, 0.0, 0.0, AnchorLoc::UpperRight)
    }
    .with_artist(BoxItem::patch(
        Circle::new((10.0, 10.0), 10.0),
        PatchProps::default(),
    ))
    .with_artist(BoxItem::patch(
        Circle::new((30.0, 10.0), 5.0),
        PatchProps::default().facecolor("r"),
    ));
    ax.add_anchored_box(ada);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn annotations_15(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let aux = AnchoredAuxTransformBox::new(ax.trans_data(), AnchorLoc::UpperLeft).with_artist(
        BoxItem::patch(ellipse((0.0, 0.0), 0.1, 0.4, 30.0), PatchProps::default()),
    );
    ax.add_anchored_box(aux);
    Ok(())
}

fn annotations_16(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let mut ellipses = DrawingArea::new(60.0, 20.0, 0.0, 0.0);
    for (x, angle, color) in [(10.0, 30.0, "r"), (30.0, 170.0, "g"), (50.0, 230.0, "b")] {
        ellipses = ellipses.with_artist(BoxItem::patch(
            ellipse((x, 10.0), 16.0, 5.0, angle),
            PatchProps::default().facecolor(color),
        ));
    }
    let test = TextArea::from_text(" Test: ", TextProps::default().color("k"));
    let packed = OffsetBox::HPacker(Packer::new(
        0.0,
        5.0,
        PackAlign::Center,
        vec![test.into(), ellipses.into()],
    ));
    ax.add_anchored_box(AnchoredBox::new(
        packed,
        AnchoredOptions {
            pad: 0.0,
            borderpad: (0.0, 0.0),
            bbox_to_anchor: Some((0.0, 1.02).into()),
            bbox_transform: Some(ax.trans_axes()),
            ..AnchoredOptions::new(AnchorLoc::LowerLeft)
        },
    ));
    fig.subplots_adjust(&SubplotOverrides {
        top: Some(0.8),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

fn annotations_17(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let text = ax.text(0.2, 0.8, "Green!");
    text.set_color("green");
    let da = DrawingArea::new(20.0, 20.0, 0.0, 0.0).with_artist(BoxItem::patch(
        Annulus::new((10.0, 10.0), 10.0, 5.0)?,
        PatchProps::default().color("tab:green"),
    ));
    let translucent = PatchProps::default().alpha(0.5);
    let ab1 = ax.add_annotation_bbox(
        da,
        (0.5, 0.0),
        AnnotationBboxOptions {
            xybox: Some((0.5, 0.25).into()),
            xycoords: Coords::Artist(text.id()),
            boxcoords: Some(Coords::blended(Coords::Artist(text.id()), Coords::Data)),
            arrowprops: Some(plain_arrow()?),
            bboxprops: translucent.clone(),
            ..AnnotationBboxOptions::default()
        },
    )?;
    let annulus = *ab1.artist_ids().first().ok_or("the annulus")?;
    let n = 25;
    let column = linspace(0.0, 1.0, n);
    let arr = Grid::from_fn(n, n, |r, _| column[r]);
    let im = OffsetImage::new(
        arr,
        1.0,
        BboxImageOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("Greens")?),
            ..BboxImageOptions::default()
        },
    )?;
    ax.add_annotation_bbox(
        im,
        (0.5, 0.0),
        AnnotationBboxOptions {
            xybox: Some((0.75, 0.0).into()),
            xycoords: Coords::Artist(text.id()),
            boxcoords: Some(Coords::blended(Coords::Data, Coords::Artist(annulus))),
            arrowprops: Some(plain_arrow()?),
            bboxprops: translucent,
            ..AnnotationBboxOptions::default()
        },
    )?;
    Ok(())
}

/// An annotation without arrow at `xy` in `xycoords`.
fn note(
    ax: &Axes,
    text: &str,
    xy: (f64, f64),
    xycoords: Coords,
) -> Result<Annotation, Box<dyn std::error::Error>> {
    Ok(ax.annotate(
        text,
        xy,
        AnnotateOptions {
            xycoords,
            ..AnnotateOptions::default()
        },
    )?)
}

fn annotations_18(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    note(ax1, "Test", (0.2, 0.2), Coords::Transform(ax1.trans_axes()))?;
    note(ax2, "Test", (0.2, 0.2), "axes fraction".parse()?)?;
    Ok(())
}

fn annotations_19(fig: &Figure) -> CaseResult {
    let x = linspace(-1.0, 1.0, 50);
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax1.plot(&x, x.iter().map(|x| -x.powi(3)).collect::<Vec<_>>())?;
    ax2.plot(&x, x.iter().map(|x| -3.0 * x.powi(2)).collect::<Vec<_>>())?;
    ax2.annotate(
        "",
        (0.0, 0.0),
        AnnotateOptions {
            xycoords: Coords::Transform(ax1.trans_data()),
            xytext: Some(Point::new(0.0, 0.0)),
            textcoords: Some(Coords::Transform(ax2.trans_data())),
            arrowprops: Some(ArrowProps::Fancy("<->".parse()?)),
            ..AnnotateOptions::default()
        },
    )?;
    Ok(())
}

/// The guide's "Test 1" annotation, centred in a white round box.
fn test_1(ax: &Axes) -> Result<Annotation, Box<dyn std::error::Error>> {
    let an1 = ax.annotate("Test 1", (0.5, 0.5), AnnotateOptions::default())?;
    an1.set_valign(VAlign::Center)
        .set_halign(HAlign::Center)
        .set_bbox(styled_box("round", PatchProps::default().facecolor("w"))?);
    Ok(an1)
}

/// "Test 2" in a white round box, 30 points right of the point `(1, 0.5)`
/// of the extent of "Test 1" and pointing at it.
fn test_2_beside(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let an1 = test_1(&ax)?;
    ax.annotate(
        "Test 2",
        (1.0, 0.5),
        AnnotateOptions {
            xycoords: Coords::Artist(an1.id()),
            xytext: Some(Point::new(30.0, 0.0)),
            textcoords: Some("offset points".parse()?),
            arrowprops: Some(ArrowProps::Fancy(plain_arrow()?)),
            ..AnnotateOptions::default()
        },
    )?
    .set_valign(VAlign::Center)
    .set_halign(HAlign::Left)
    .set_bbox(styled_box("round", PatchProps::default().facecolor("w"))?);
    Ok(())
}

fn annotations_22(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    note(ax1, "Test1", (0.5, 0.5), "axes fraction".parse()?)?;
    note(ax2, "Test 2", (0.5, 0.5), Coords::Artist(ax2.id()))?;
    Ok(())
}

fn annotations_23(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    note(
        &ax,
        "Test",
        (0.5, 1.0),
        Coords::blended(Coords::Data, "axes fraction".parse()?),
    )?;
    ax.axvline(0.5, LineProps::default().color("lightgray"));
    ax.set_xlim(0.0, 2.0)?.set_ylim(1.0, 2.0)?;
    Ok(())
}

fn annotations_24(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t1 = ax.text(0.05, 0.05, "Text 1");
    t1.set_valign(VAlign::Bottom).set_halign(HAlign::Left);
    let t2 = ax.text(0.90, 0.90, "Text 2");
    t2.set_halign(HAlign::Right);
    note(
        &ax,
        "Anchored to 1 & 2",
        (0.0, 0.0),
        Coords::blended(Coords::Artist(t1.id()), Coords::Artist(t2.id())),
    )?
    .set_valign(VAlign::Bottom)
    .set_color("tab:orange");
    Ok(())
}

fn annotations_25(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let an1 = test_1(&ax)?;
    let offset_from = Coords::OffsetFrom {
        artist: an1.id(),
        reference: Point::new(0.5, 0.0),
        unit: CoordUnit::Points,
    };
    ax.annotate(
        "Test 2",
        (0.1, 0.1),
        AnnotateOptions {
            xytext: Some(Point::new(0.0, -10.0)),
            textcoords: Some(offset_from),
            arrowprops: Some(ArrowProps::Fancy(plain_arrow()?)),
            ..AnnotateOptions::default()
        },
    )?
    .set_valign(VAlign::Top)
    .set_halign(HAlign::Center)
    .set_bbox(styled_box("round", PatchProps::default().facecolor("w"))?);
    Ok(())
}

fn annotations_26(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let xy = (0.3, 0.2);
    fig.add_artist(ConnectionPatch::new(
        ConnectionEnd::new(xy, Coords::Transform(ax1.trans_data()), None),
        ConnectionEnd::new(xy, Coords::Transform(ax2.trans_data()), None),
    ));
    Ok(())
}

/// The cases of this module; `annotations_21` annotates relative to the
/// window extent of "Test 1" (matplotlib's `an1.get_window_extent`), which
/// is what coordinates relative to an artist mean.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_colors_text/annotations_13", annotations_13)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_14", annotations_14)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_15", annotations_15)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_16", annotations_16)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_17", annotations_17),
    FigureCase::new("gallery_guides_colors_text/annotations_18", annotations_18)
        .sized(6.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_19", annotations_19)
        .sized(6.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_20", test_2_beside)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_21", test_2_beside)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_22", annotations_22)
        .sized(6.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_23", annotations_23)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_24", annotations_24)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_25", annotations_25)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_26", annotations_26)
        .sized(6.0, 3.0, 100.0),
];
