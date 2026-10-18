//! Anchored texts at every location and with styled frames, packer
//! alignments and modes, drawing-area descents and multi-line text areas
//! of `cases_offsetbox.py`.

use plt::figure::SubfigureOptions;
use plt::offsetbox::{
    AnchorLoc, AnchoredBox, AnchoredOptions, BoxItem, DrawingArea, OffsetBox, PackAlign, PackMode,
    Packer, TextArea,
};
use plt::patches::{BoxStyle, Circle, Ellipse, Rectangle};
use plt::prelude::*;
use plt::text::{HorizontalAlignment, TextProps};
use plt::transforms::Transform;

use super::{of_t, sized, t, text_area, unit_axes};
use crate::figures::CaseResult;

/// `AnchoredText(text, loc="center", bbox_to_anchor=at, bbox_transform=ax.transAxes, prop=props)`.
fn centred_text_at(
    ax: &Axes,
    text: &str,
    at: (f64, f64),
    props: TextProps,
) -> Result<plt::offsetbox::AnchoredOffsetbox, Box<dyn std::error::Error>> {
    let options = AnchoredOptions {
        bbox_to_anchor: Some(at.into()),
        bbox_transform: Some(ax.trans_axes()),
        ..AnchoredOptions::new(AnchorLoc::Center)
    };
    Ok(ax.add_anchored_box(AnchoredBox::text(text, props, options)?))
}

pub(super) fn location_codes(fig: &Figure) -> CaseResult {
    let ax = unit_axes(fig);
    for code in 1..=10 {
        let loc = AnchorLoc::from_code(code).ok_or("an anchor code")?;
        let options = AnchoredOptions {
            pad: 0.2,
            borderpad: (0.3, 0.3),
            ..AnchoredOptions::new(loc)
        };
        let props = TextProps::default().font(sized(8.0));
        ax.add_anchored_box(AnchoredBox::text(format!("loc={code}"), props, options)?);
    }
    let props = TextProps::default().font(sized(11.0)).color("tab:brown");
    let styled = centred_text_at(&ax, "styled frame", (0.5, 0.3), props)?;
    let frame = styled.patch();
    frame
        .set_facecolor("lightyellow")
        .set_edgecolor("tab:orange")
        .set_linewidth(1.5);
    let round4: BoxStyle = "round4,pad=0.2".parse()?;
    frame.update_shape(|shape| shape.boxstyle = round4);
    Ok(())
}

/// Text areas reading their own font sizes.
fn sized_texts(sizes: &[f64]) -> Vec<OffsetBox<BoxItem>> {
    sizes
        .iter()
        .map(|&size| text_area(&format!("{size}pt"), TextProps::default().font(sized(size))))
        .collect()
}

/// `child` in a framed anchored box at `loc`.
fn framed(ax: &Axes, child: OffsetBox<BoxItem>, loc: AnchorLoc) {
    ax.add_anchored_box(AnchoredBox::new(child, AnchoredOptions::new(loc)));
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn packer_alignments(fig: &Figure) -> CaseResult {
    let ax = unit_axes(fig);
    for (loc, align) in [
        (AnchorLoc::UpperLeft, PackAlign::Top),
        (AnchorLoc::UpperRight, PackAlign::Bottom),
        (AnchorLoc::CenterLeft, PackAlign::Center),
        (AnchorLoc::CenterRight, PackAlign::Baseline),
    ] {
        let row = Packer::new(2.0, 6.0, align, sized_texts(&[8.0, 14.0, 20.0]));
        framed(&ax, OffsetBox::HPacker(row), loc);
    }
    for (loc, align) in [
        (AnchorLoc::LowerLeft, PackAlign::Left),
        (AnchorLoc::LowerRight, PackAlign::Right),
    ] {
        let column = Packer::new(2.0, 2.0, align, sized_texts(&[8.0, 12.0, 16.0]));
        framed(&ax, OffsetBox::VPacker(column), loc);
    }
    let mut equal = Packer::new(
        0.0,
        4.0,
        PackAlign::Baseline,
        sized_texts(&[8.0, 10.0, 12.0]),
    );
    equal.mode = PackMode::Equal;
    equal.width = Some(150.0);
    framed(&ax, OffsetBox::HPacker(equal), AnchorLoc::LowerCenter);
    let fixed = Packer::new(0.0, 10.0, PackAlign::Center, sized_texts(&[9.0, 9.0]));
    framed(&ax, OffsetBox::VPacker(fixed), AnchorLoc::UpperCenter);
    Ok(())
}

/// A patch of `shape` styled by `props`.
fn item<S: plt::patches::PatchShape>(shape: S, props: PatchProps) -> BoxItem {
    BoxItem::patch(shape, props)
}

pub(super) fn drawing_area_descents(fig: &Figure) -> CaseResult {
    let ax = unit_axes(fig);
    let shifted = DrawingArea::new(30.0, 20.0, 5.0, 8.0)
        .with_artist(item(
            Rectangle::new((-5.0, -8.0), 30.0, 20.0),
            PatchProps::default().facecolor("tab:cyan"),
        ))
        .with_artist(item(
            Circle::new((0.0, 0.0), 3.0),
            PatchProps::default().facecolor("k"),
        ));
    let row = Packer::new(
        0.0,
        4.0,
        PackAlign::Baseline,
        vec![
            text_area("before", TextProps::default()),
            shifted.into(),
            text_area("after", TextProps::default()),
        ],
    );
    let anchored = ax.add_anchored_box(AnchoredBox::new(
        OffsetBox::HPacker(row),
        AnchoredOptions::new(AnchorLoc::UpperLeft),
    ));
    let round: BoxStyle = "round,pad=0.1".parse()?;
    anchored
        .patch()
        .update_shape(|shape| shape.boxstyle = round);
    let mut clipped = DrawingArea::new(50.0, 30.0, 10.0, 10.0).with_artist(item(
        Ellipse::new((15.0, 5.0), 60.0, 40.0),
        PatchProps::default().facecolor("tab:pink").edgecolor("k"),
    ));
    clipped.clip = true;
    ax.add_anchored_box(AnchoredBox::new(
        clipped,
        AnchoredOptions {
            pad: 0.0,
            ..AnchoredOptions::new(AnchorLoc::Center)
        },
    ));
    let top = DrawingArea::new(40.0, 25.0, 0.0, 0.0).with_artist(item(
        Rectangle::new((0.0, 0.0), 40.0, 25.0),
        PatchProps::default().facecolor("tab:olive").alpha(0.5),
    ));
    let column = Packer::new(
        0.0,
        2.0,
        PackAlign::Center,
        vec![
            top.into(),
            text_area("under the area", TextProps::default()),
        ],
    );
    ax.add_anchored_box(AnchoredBox::new(
        OffsetBox::VPacker(column),
        AnchoredOptions {
            frameon: false,
            ..AnchoredOptions::new(AnchorLoc::LowerRight)
        },
    ));
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn multiline_baseline(fig: &Figure) -> CaseResult {
    let ax = unit_axes(fig);
    let mut first = TextArea::from_text("one\ntwo\nthree", TextProps::default());
    first.multilinebaseline = true;
    let row = Packer::new(
        0.0,
        8.0,
        PackAlign::Baseline,
        vec![
            first.into(),
            text_area("base", TextProps::default()),
            text_area("left\nlines", TextProps::default()),
            text_area(
                "right\naligned lines",
                TextProps::default().halign(HorizontalAlignment::Right),
            ),
        ],
    );
    framed(&ax, OffsetBox::HPacker(row), AnchorLoc::Center);
    ax.axhline(0.5, LineProps::default().color("0.8"));
    Ok(())
}

pub(super) fn figure_anchored(fig: &Figure) -> CaseResult {
    fig.subplot().plot(t(), of_t(f64::sin))?;
    let corner = AnchoredOptions {
        bbox_to_anchor: Some((0.0, 1.0).into()),
        bbox_transform: Some(Transform::Figure(fig.id())),
        ..AnchoredOptions::new(AnchorLoc::UpperLeft)
    };
    let props = TextProps::default().font(sized(9.0));
    fig.add_anchored_box(AnchoredBox::text("figure corner", props, corner)?);
    let pixels = AnchoredOptions {
        bbox_to_anchor: Some((400.0, 0.0).into()),
        frameon: false,
        ..AnchoredOptions::new(AnchorLoc::LowerRight)
    };
    fig.add_anchored_box(AnchoredBox::text("pixels", TextProps::default(), pixels)?);
    let centre = AnchoredOptions {
        bbox_to_anchor: Some((0.5, 0.95).into()),
        bbox_transform: Some(Transform::Figure(fig.id())),
        ..AnchoredOptions::new(AnchorLoc::Center)
    };
    fig.add_anchored_box(AnchoredBox::new(
        text_area("child", TextProps::default()),
        centre,
    ));
    Ok(())
}

pub(super) fn subfigure_anchored(fig: &Figure) -> CaseResult {
    let subfigures = fig.subfigures(1, 2, &SubfigureOptions::default())?;
    for (i, sub) in subfigures.as_slice().iter().enumerate() {
        let shift = i as f64;
        sub.subplot().plot(t(), of_t(|t| (t + shift).cos()))?;
        let options = AnchoredOptions {
            bbox_to_anchor: Some((1.0, 1.0).into()),
            bbox_transform: Some(Transform::Figure(sub.id())),
            ..AnchoredOptions::new(AnchorLoc::UpperRight)
        };
        let props = TextProps::default().font(sized(8.0));
        sub.add_anchored_box(AnchoredBox::text(format!("panel {i}"), props, options)?);
    }
    Ok(())
}

pub(super) fn text_boxstyles(fig: &Figure) -> CaseResult {
    let ax = unit_axes(fig);
    let styles = [
        "round4,pad=0.3",
        "sawtooth",
        "larrow",
        "roundtooth",
        "circle",
    ];
    for (i, style) in styles.into_iter().enumerate() {
        let at = (0.2 + 0.15 * i as f64, 0.15 + 0.17 * i as f64);
        let anchored = centred_text_at(&ax, style, at, TextProps::default().font(sized(9.0)))?;
        let frame = anchored.patch();
        let boxstyle: BoxStyle = style.parse()?;
        frame.update_shape(|shape| shape.boxstyle = boxstyle);
        frame
            .set_facecolor(format!("C{i}").as_str())
            .set_alpha(Some(0.4));
        if i % 2 == 1 {
            frame.set_hatch("//");
        }
    }
    Ok(())
}
