//! `imshow_extent`: where `imshow` puts an image for each origin and
//! extent, with the edges and corner pixels labelled.

use plt::axes::ImshowOptions;
use plt::gridspec::GridSpec;
use plt::image::{Extent, Origin};
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::text::{AnnotateOptions, HAlign, TextProps, VAlign};

use crate::figures::cases::common::styled_box;
use crate::figures::{CaseResult, FigureCase};

/// The corner pixels the page labels, with the data index of each.
const CORNERS: [(&str, (usize, usize)); 4] = [
    ("[0, 0]", (0, 0)),
    ("[0, N']", (0, 6)),
    ("[M', 0]", (5, 0)),
    ("[M', N']", (5, 6)),
];

/// The extents of the second and third figures, as
/// `(left, right, bottom, top)`.
const EXTENTS: [[f64; 4]; 4] = [
    [-0.5, 6.5, -0.5, 5.5],
    [-0.5, 6.5, 5.5, -0.5],
    [6.5, -0.5, -0.5, 5.5],
    [6.5, -0.5, 5.5, -0.5],
];

/// The page's `data`: `x + y` over a 6 × 7 grid.
fn data() -> Grid<f64> {
    Grid::from_fn(6, 7, |row, col| (row + col) as f64)
}

/// `index_to_coordinate`: the centre of the corner pixel `index`.
fn index_to_coordinate(
    index: &str,
    [left, right, bottom, top]: [f64; 4],
    origin: Origin,
) -> (f64, f64) {
    let hshift = 0.5 * (right - left).signum();
    let (left, right) = (left + hshift, right - hshift);
    let vshift = 0.5 * (top - bottom).signum();
    let (mut bottom, mut top) = (bottom + vshift, top - vshift);
    if origin == Origin::Upper {
        (bottom, top) = (top, bottom);
    }
    match index {
        "[0, 0]" => (left, bottom),
        "[M', 0]" => (left, top),
        "[0, N']" => (right, bottom),
        _ => (right, top),
    }
}

/// `get_index_label_pos`: where the label of corner `index` goes and how
/// it is aligned.
fn index_label_pos(
    index: &str,
    extent: Option<[f64; 4]>,
    origin: Origin,
    inverted_xindex: bool,
) -> (f64, f64, HAlign) {
    let extent = extent.unwrap_or(match origin {
        Origin::Lower => [-0.5, 6.5, -0.5, 5.5],
        Origin::Upper => [-0.5, 6.5, 5.5, -0.5],
    });
    let [left, right, ..] = extent;
    let (x, y) = index_to_coordinate(index, extent, origin);
    let is_x0 = index.ends_with("0]");
    let halign = if is_x0 ^ inverted_xindex {
        HAlign::Left
    } else {
        HAlign::Right
    };
    let hshift = 0.5 * (left - right).signum();
    (x + hshift * if is_x0 { 1.0 } else { -1.0 }, y, halign)
}

/// The names of the image's edges at the axes edges (`top` and `bottom`,
/// `left` and `right` as the image shows them when `zoomed`); returns
/// whether the image's columns run right to left.
fn edge_labels(
    ax: &Axes,
    image: &Extent,
    zoomed: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let (upper, lower) = if !zoomed || image.top > image.bottom {
        ("top", "bottom")
    } else {
        ("bottom", "top")
    };
    let inverted_xindex = zoomed && image.left >= image.right;
    let (port, starboard) = if inverted_xindex {
        ("right", "left")
    } else {
        ("left", "right")
    };
    let (center, middle) = (HAlign::Center, VAlign::Center);
    for (label, xy, xytext, halign, valign, rotation) in [
        (upper, (0.5, 1.0), (0.0, -1.0), center, VAlign::Top, 0.0),
        (lower, (0.5, 0.0), (0.0, 1.0), center, VAlign::Bottom, 0.0),
        (port, (0.0, 0.5), (1.0, 0.0), HAlign::Left, middle, 90.0),
        (
            starboard,
            (1.0, 0.5),
            (-1.0, 0.0),
            HAlign::Right,
            middle,
            -90.0,
        ),
    ] {
        ax.annotate(
            label,
            xy,
            AnnotateOptions {
                xytext: Some(xytext.into()),
                xycoords: "axes fraction".parse()?,
                textcoords: Some("offset points".parse()?),
                ..AnnotateOptions::default()
            },
        )?
        .update_props(
            TextProps::default()
                .halign(halign)
                .valign(valign)
                .rotation(rotation),
        )
        .set_bbox(styled_box(
            "round4",
            PatchProps::default().facecolor("w").alpha(0.75),
        )?);
    }
    Ok(inverted_xindex)
}

/// `plot_imshow_with_labels`: the image from `origin` over `extent`,
/// its edges named and its corner pixels labelled in white on their own
/// colour, in the view `xlim` × `ylim`.
fn imshow_with_labels(
    ax: &Axes,
    extent: Option<[f64; 4]>,
    origin: Origin,
    view: Option<((f64, f64), (f64, f64))>,
) -> CaseResult {
    let image = ax.imshow(
        data(),
        ImshowOptions {
            origin: Some(origin),
            extent: extent.map(|[left, right, bottom, top]| Extent {
                left,
                right,
                bottom,
                top,
            }),
            ..ImshowOptions::default()
        },
    )?;
    let inverted_xindex = edge_labels(ax, &image.extent(), view.is_some())?;
    let name = if origin == Origin::Upper {
        "upper"
    } else {
        "lower"
    };
    ax.set_title(format!("origin: {name}"));
    let values = data();
    let max = values
        .as_slice()
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let cmap = image.cmap();
    for (index, cell) in CORNERS {
        let (tx, ty, halign) = index_label_pos(index, extent, origin, inverted_xindex);
        let facecolor = cmap.sample(values[cell] / max);
        ax.text(tx, ty, index)
            .update_props(
                TextProps::default()
                    .color("white")
                    .halign(halign)
                    .valign(VAlign::Center),
            )
            .set_bbox(styled_box(
                "square",
                PatchProps::default().facecolor(facecolor),
            )?);
    }
    if let Some(((x0, x1), (y0, y1))) = view {
        ax.set_xlim(x0, x1).set_ylim(y0, y1);
    }
    Ok(())
}

/// `generate_imshow_demo_grid`: a row per extent with its values, and the
/// image from the upper and from the lower origin; returns the image axes
/// of the two origins.
fn imshow_demo_grid(
    fig: &Figure,
    extents: &[Option<[f64; 4]>],
    view: Option<((f64, f64), (f64, f64))>,
) -> Result<[Vec<Axes>; 2], Box<dyn std::error::Error>> {
    let n = extents.len();
    fig.set_layout_engine(LayoutEngine::tight());
    let gs = GridSpec::new(n, 5)?;
    let column = |cols: std::ops::Range<usize>| {
        (0..n)
            .map(|row| fig.add_subplot_spec(&gs.span(row..row + 1, cols.clone())?))
            .collect::<Result<Vec<_>, _>>()
    };
    let (labels, upper, lower) = (column(0..1)?, column(1..3)?, column(3..5)?);
    for (origin, axes) in [(Origin::Upper, &upper), (Origin::Lower, &lower)] {
        for (ax, &extent) in axes.iter().zip(extents) {
            imshow_with_labels(ax, extent, origin, view)?;
        }
    }
    labels[0].set_title("extent=");
    for (ax, extent) in labels.iter().zip(extents) {
        let text = match extent {
            None => "None".to_owned(),
            Some([left, right, bottom, top]) => {
                format!("left: {left:.1}\nright: {right:.1}\nbottom: {bottom:.1}\ntop: {top:.1}\n")
            }
        };
        ax.text(1.0, 0.5, text)
            .set_transform(ax.trans_axes())
            .update_props(
                TextProps::default()
                    .halign(HAlign::Right)
                    .valign(VAlign::Center),
            );
        ax.set_axis_off();
    }
    Ok([upper, lower])
}

/// `set_extent_None_text`: notes that `ax` shows the default extent.
fn extent_none_text(ax: &Axes) {
    ax.text(3.0, 2.5, "equals\nextent=None")
        .update_props(
            TextProps::default()
                .halign(HAlign::Center)
                .valign(VAlign::Center)
                .color("w"),
        )
        .set_fontsize(plt::fonts::RelativeSize::Large);
}

fn imshow_extent_1(fig: &Figure) -> CaseResult {
    imshow_demo_grid(fig, &[None], None)?;
    Ok(())
}

fn imshow_extent_2(fig: &Figure) -> CaseResult {
    let [upper, lower] = imshow_demo_grid(fig, &EXTENTS.map(Some), None)?;
    extent_none_text(&upper[1]);
    extent_none_text(&lower[0]);
    Ok(())
}

fn imshow_extent_3(fig: &Figure) -> CaseResult {
    let extents: Vec<Option<[f64; 4]>> = std::iter::once(None).chain(EXTENTS.map(Some)).collect();
    imshow_demo_grid(fig, &extents, Some(((-2.0, 8.0), (-1.0, 6.0))))?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_axes/imshow_extent_1", imshow_extent_1).sized(
        6.0,
        11.25 / 5.0,
        100.0,
    ),
    FigureCase::new("gallery_guides_axes/imshow_extent_2", imshow_extent_2).sized(
        6.0,
        4.0 * 11.25 / 5.0,
        100.0,
    ),
    FigureCase::new("gallery_guides_axes/imshow_extent_3", imshow_extent_3).sized(
        6.0,
        5.0 * 11.25 / 5.0,
        100.0,
    ),
];
