//! The anchored boxes of `cases_offsetbox.py`: anchored texts, packers,
//! padded and aux-transform boxes and the `axes_grid1` anchored artists.

use plt::axes::ImshowOptions;
use plt::axes_grid1::anchored_artists::{
    AnchoredAuxTransformBox, AnchoredDirectionArrows, AnchoredDrawingArea, AnchoredEllipse,
    AnchoredSizeBar,
};
use plt::colors::colormaps;
use plt::fonts::{FontProperties, FontWeight};
use plt::image::ColorizerOptions;
use plt::offsetbox::{
    AnchorLoc, AnchoredBox, AnchoredOptions, AuxTransformBox, BoxItem, DrawingArea, OffsetBox,
    PackAlign, PackMode, Packer, PaddedBox,
};
use plt::patches::{BoxStyle, Circle, Ellipse, Rectangle};
use plt::prelude::*;
use plt::text::{HorizontalAlignment, TextProps};

use super::{gradient, of_t, sized, t, text_area};
use crate::figures::CaseResult;

/// `AnchoredText(text, loc=loc, prop=props, **options)`.
fn anchored_text_box(
    ax: &Axes,
    text: &str,
    props: TextProps,
    options: AnchoredOptions,
) -> Result<plt::offsetbox::AnchoredOffsetbox, Box<dyn std::error::Error>> {
    Ok(ax.add_anchored_box(AnchoredBox::text(text, props, options)?))
}

/// A patch of `shape` with face colour `face`.
fn filled<S: plt::patches::PatchShape>(shape: S, face: &str) -> BoxItem {
    BoxItem::patch(shape, PatchProps::default().facecolor(face))
}

/// `Ellipse(center, width, height, angle=angle)`.
fn ellipse(center: (f64, f64), width: f64, height: f64, angle: f64) -> Ellipse {
    Ellipse {
        angle,
        ..Ellipse::new(center, width, height)
    }
}

pub(super) fn anchored_text(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot(t(), of_t(f64::sin))?;
    let figure_label = anchored_text_box(
        &ax,
        "Figure 1a",
        TextProps::default().font(sized(15.0)),
        AnchoredOptions::new(AnchorLoc::UpperLeft),
    )?;
    let rounded: BoxStyle = "round,pad=0.,rounding_size=0.2".parse()?;
    figure_label
        .patch()
        .update_shape(|frame| frame.boxstyle = rounded);
    anchored_text_box(
        &ax,
        "lower right\ntwo lines",
        TextProps::default().color("tab:red"),
        AnchoredOptions::new(AnchorLoc::LowerRight),
    )?;
    anchored_text_box(
        &ax,
        "no frame",
        TextProps::default(),
        AnchoredOptions {
            frameon: false,
            pad: 0.1,
            borderpad: (0.2, 0.2),
            ..AnchoredOptions::new(AnchorLoc::Center)
        },
    )?;
    anchored_text_box(
        &ax,
        "centred, wide pads",
        TextProps::default()
            .halign(HorizontalAlignment::Center)
            .font(sized(8.0)),
        AnchoredOptions {
            borderpad: (2.0, 0.3),
            ..AnchoredOptions::new(AnchorLoc::UpperCenter)
        },
    )?;
    anchored_text_box(
        &ax,
        "anchored at (0.6, 0.3)",
        TextProps::default(),
        AnchoredOptions {
            bbox_to_anchor: Some((0.6, 0.3).into()),
            bbox_transform: Some(ax.trans_axes()),
            borderpad: (0.0, 0.0),
            ..AnchoredOptions::new(AnchorLoc::LowerLeft)
        },
    )?;
    Ok(())
}

/// One row of the key: a colour swatch beside its label.
fn key_row(label: &str, color: &str) -> OffsetBox<BoxItem> {
    let swatch = DrawingArea::new(20.0, 10.0, 0.0, 0.0)
        .with_artist(filled(Rectangle::new((0.0, 0.0), 20.0, 10.0), color));
    OffsetBox::HPacker(Packer::new(
        0.0,
        4.0,
        PackAlign::Baseline,
        vec![swatch.into(), text_area(label, TextProps::default())],
    ))
}

pub(super) fn anchored_packers(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_with(t(), of_t(f64::cos), LineProps::default().color("0.6"))?;
    let mut ellipses = DrawingArea::new(60.0, 20.0, 0.0, 0.0);
    for (x, angle, color) in [(10.0, 30.0, "r"), (30.0, 170.0, "g"), (50.0, 230.0, "b")] {
        ellipses = ellipses.with_artist(filled(ellipse((x, 10.0), 16.0, 5.0, angle), color));
    }
    let test = OffsetBox::HPacker(Packer::new(
        0.0,
        5.0,
        PackAlign::Center,
        vec![
            text_area(" Test: ", TextProps::default().color("k")),
            ellipses.into(),
        ],
    ));
    ax.add_anchored_box(AnchoredBox::new(
        test,
        AnchoredOptions {
            pad: 0.0,
            borderpad: (0.0, 0.0),
            bbox_to_anchor: Some((0.0, 1.02).into()),
            bbox_transform: Some(ax.trans_axes()),
            ..AnchoredOptions::new(AnchorLoc::LowerLeft)
        },
    ));
    let title = text_area(
        "Key",
        TextProps::default()
            .font(FontProperties::from_rc(&plt::rc::params()).with_weight(FontWeight::BOLD)),
    );
    let column = OffsetBox::VPacker(Packer::new(
        2.0,
        3.0,
        PackAlign::Right,
        vec![
            title,
            key_row("alpha", "tab:blue"),
            key_row("beta, longer", "tab:orange"),
        ],
    ));
    ax.add_anchored_box(AnchoredBox::new(
        column,
        AnchoredOptions {
            pad: 0.3,
            ..AnchoredOptions::new(AnchorLoc::UpperRight)
        },
    ));
    let letters = ["a", "b", "c"].map(|letter| text_area(letter, TextProps::default()));
    let mut spread = Packer::new(0.0, 0.0, PackAlign::Baseline, letters.into());
    spread.mode = PackMode::Expand;
    let mut wide = Packer::new(
        0.0,
        2.0,
        PackAlign::Center,
        vec![
            text_area("expand mode", TextProps::default()),
            OffsetBox::HPacker(spread),
        ],
    );
    wide.width = Some(120.0);
    ax.add_anchored_box(AnchoredBox::new(
        OffsetBox::VPacker(wide),
        AnchoredOptions::new(AnchorLoc::LowerRight),
    ));
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn padded_and_aux_boxes(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 5.0);
    let padded = PaddedBox::new(
        text_area("padded box", TextProps::default()),
        6.0,
        true,
        PatchProps::default()
            .facecolor("lightyellow")
            .edgecolor("tab:brown"),
    );
    ax.add_anchored_box(AnchoredBox::new(
        padded,
        AnchoredOptions {
            frameon: false,
            ..AnchoredOptions::new(AnchorLoc::UpperLeft)
        },
    ));
    let aux = AuxTransformBox::new(ax.trans_data())
        .with_artist(filled(Rectangle::new((0.0, 0.0), 2.0, 1.0), "tab:blue"))
        .with_artist(BoxItem::patch(
            Ellipse::new((3.0, 0.5), 1.5, 1.0),
            PatchProps::default().facecolor("tab:green").alpha(0.7),
        ));
    ax.add_anchored_box(AnchoredBox::new(
        aux,
        AnchoredOptions {
            pad: 0.2,
            ..AnchoredOptions::new(AnchorLoc::LowerRight)
        },
    ));
    let circle = || filled(Circle::new((40.0, 30.0), 25.0), "tab:purple");
    let mut clipped = DrawingArea::new(40.0, 30.0, 0.0, 0.0).with_artist(circle());
    clipped.clip = true;
    ax.add_anchored_box(AnchoredBox::new(
        clipped,
        AnchoredOptions::new(AnchorLoc::Center),
    ));
    ax.add_anchored_box(AnchoredBox::new(
        DrawingArea::new(40.0, 30.0, 5.0, 5.0).with_artist(circle()),
        AnchoredOptions::new(AnchorLoc::CenterLeft),
    ));
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn anchored_size_bar(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    ax.add_anchored_box(AnchoredSizeBar::new(
        ax.trans_data(),
        1.0,
        "1 unit",
        AnchorLoc::LowerLeft,
    ));
    ax.add_anchored_box(AnchoredSizeBar {
        pad: 0.5,
        sep: 5.0,
        borderpad: 0.5,
        frameon: false,
        size_vertical: 0.5,
        color: "tab:red".into(),
        label_top: true,
        fontproperties: Some(sized(14.0)),
        ..AnchoredSizeBar::new(ax.trans_data(), 3.0, "3 units", AnchorLoc::UpperRight)
    });
    ax.add_anchored_box(AnchoredSizeBar {
        size_vertical: 0.4,
        fill_bar: Some(false),
        color: "tab:blue".into(),
        ..AnchoredSizeBar::new(
            ax.trans_data(),
            2.0,
            "2 units, unfilled",
            AnchorLoc::LowerRight,
        )
    });
    ax.add_anchored_box(AnchoredSizeBar {
        size_vertical: 0.01,
        ..AnchoredSizeBar::new(ax.trans_axes(), 0.25, "quarter axes", AnchorLoc::CenterLeft)
    });
    Ok(())
}

pub(super) fn anchored_direction_arrows(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.imshow(
        gradient(30, 40),
        ImshowOptions {
            colors: ColorizerOptions::default().cmap(colormaps::get("gray")?),
            ..ImshowOptions::default()
        },
    )?;
    ax.add_anchored_box(AnchoredBox::try_from(AnchoredDirectionArrows::new(
        ax.trans_axes(),
        "X",
        "Y",
    ))?);
    ax.add_anchored_box(AnchoredBox::try_from(AnchoredDirectionArrows {
        loc: AnchorLoc::LowerLeft,
        color: "k".into(),
        aspect_ratio: -1.0,
        sep_x: 0.02,
        sep_y: -0.01,
        text_props: PatchProps::default().edgecolor("w").facecolor("k"),
        ..AnchoredDirectionArrows::new(ax.trans_axes(), "East", "South")
    })?);
    ax.add_anchored_box(AnchoredBox::try_from(AnchoredDirectionArrows {
        loc: AnchorLoc::UpperRight,
        angle: 30.0,
        color: "tab:orange".into(),
        frameon: true,
        length: 0.1,
        fontsize: 0.05,
        back_length: 0.3,
        head_width: 6.0,
        head_length: 8.0,
        tail_width: 1.0,
        alpha: 0.8,
        ..AnchoredDirectionArrows::new(ax.trans_axes(), "a", "b")
    })?);
    Ok(())
}

pub(super) fn anchored_drawing_area_and_aux(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_with(t(), of_t(|x| x * 0.1), LineProps::default().color("0.7"))?;
    let area = AnchoredDrawingArea {
        pad: 0.0,
        frameon: false,
        ..AnchoredDrawingArea::new(40.0, 20.0, 0.0, 0.0, AnchorLoc::UpperRight)
    }
    .with_artist(filled(Circle::new((10.0, 10.0), 10.0), "tab:blue"))
    .with_artist(filled(Circle::new((30.0, 10.0), 5.0), "tab:red"));
    ax.add_anchored_box(area);
    let aux = AnchoredAuxTransformBox::new(ax.trans_data(), AnchorLoc::UpperLeft).with_artist(
        BoxItem::patch(ellipse((0.0, 0.0), 1.5, 0.4, 30.0), PatchProps::default()),
    );
    ax.add_anchored_box(aux);
    ax.add_anchored_box(AnchoredEllipse::new(
        ax.trans_data(),
        1.0,
        0.3,
        0.0,
        AnchorLoc::LowerRight,
    ));
    Ok(())
}
