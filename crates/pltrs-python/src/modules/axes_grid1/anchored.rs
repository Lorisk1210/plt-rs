//! Ready-made anchored boxes (matplotlib's `axes_grid1.anchored_artists`):
//! the Python classes keep their arguments and pltrs (Rust) builds the box
//! when an axes adds it (`ax.add_artist(box)`).

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes::Axes;
use pltrs::axes_grid1::anchored_artists::{
    AnchoredAuxTransformBox, AnchoredDirectionArrows, AnchoredDrawingArea, AnchoredSizeBar,
};
use pltrs::colors::Color;
use pltrs::offsetbox::{AnchoredBox, BoxItem};
use pltrs::transforms::Transform;

use super::super::font_manager::FontProps;
use super::super::patches::PatchKwargs;
use super::inset_locator::PyAnchorLoc;
use crate::binding::prelude::*;

/// The paddings and frame of an anchored box: `(pad, borderpad, frameon)`.
type Frame = (f64, f64, bool);

/// The artists of a drawing area or an aux-transform box: loose artists
/// taken out of their holders.
fn box_items(children: &[RawHandle]) -> PyResult<Vec<BoxItem>> {
    children
        .iter()
        .map(|child| {
            BoxItem::from_loose(child).map_err(|_| {
                PyErr::value_error("an artist of an anchored box must not be in a figure")
            })
        })
        .collect()
}

/// Adds a scale bar to `axes` (`AnchoredSizeBar`): `size` long and
/// `size_vertical` tall in the coordinates of `transform`, labelled.
fn add_size_bar(
    axes: Axes,
    (transform, size, label, loc): (Transform, f64, String, PyAnchorLoc),
    ((pad, borderpad, frameon), sep, size_vertical): (Frame, f64, f64),
    (color, label_top, fontproperties, fill_bar): (Color, bool, Option<FontProps>, Option<bool>),
) -> PyResult<RawHandle> {
    let bar = AnchoredSizeBar {
        pad,
        borderpad,
        sep,
        frameon,
        size_vertical,
        color,
        label_top,
        fontproperties: fontproperties.map(|props| props.0),
        fill_bar,
        ..AnchoredSizeBar::new(transform, size, label, loc.0)
    };
    Ok(axes.add_anchored_box(bar).raw().clone())
}

/// The geometry of direction arrows: `(length, fontsize, angle,
/// aspect_ratio, sep_x, sep_y, back_length)` in the coordinates of their
/// transform, and the `(head_width, head_length, tail_width)` of the
/// arrow style.
type ArrowGeometry = ((f64, f64, f64, f64, f64, f64, f64), (f64, f64, f64));

/// Adds direction arrows labelled `labels` to `axes`
/// (`AnchoredDirectionArrows`).
fn add_direction_arrows(
    axes: Axes,
    (transform, (label_x, label_y), loc, (pad, borderpad, frameon)): (
        Transform,
        (String, String),
        PyAnchorLoc,
        Frame,
    ),
    ((length, fontsize, angle, aspect_ratio, sep_x, sep_y, back_length), heads): ArrowGeometry,
    (color, alpha, fontproperties): (Color, f64, Option<FontProps>),
    (text_props, arrow_props): (PatchKwargs, PatchKwargs),
) -> PyResult<RawHandle> {
    let (head_width, head_length, tail_width) = heads;
    let arrows = AnchoredDirectionArrows {
        length,
        fontsize,
        loc: loc.0,
        angle,
        aspect_ratio,
        pad,
        borderpad,
        frameon,
        color,
        alpha,
        sep_x,
        sep_y,
        fontproperties: fontproperties.map(|props| props.0),
        back_length,
        head_width,
        head_length,
        tail_width,
        text_props: text_props.0,
        arrow_props: arrow_props.0,
        ..AnchoredDirectionArrows::new(transform, label_x, label_y)
    };
    let anchored =
        AnchoredBox::try_from(arrows).map_err(|err| PyErr::value_error(err.to_string()))?;
    Ok(axes.add_anchored_box(anchored).raw().clone())
}

/// Adds a drawing area of `size` (width, height, xdescent, ydescent, in
/// points) holding `children` to `axes` (`AnchoredDrawingArea`).
fn add_drawing_area(
    axes: Axes,
    ((width, height, xdescent, ydescent), loc): ((f64, f64, f64, f64), PyAnchorLoc),
    (pad, borderpad, frameon): Frame,
    children: Vec<RawHandle>,
) -> PyResult<RawHandle> {
    let mut area = AnchoredDrawingArea {
        pad,
        borderpad,
        frameon,
        ..AnchoredDrawingArea::new(width, height, xdescent, ydescent, loc.0)
    };
    area.drawing_area.artists = box_items(&children)?;
    Ok(axes.add_anchored_box(area).raw().clone())
}

/// Adds a box of `children` in the coordinates of `transform` to `axes`
/// (`AnchoredAuxTransformBox`).
fn add_aux_transform_box(
    axes: Axes,
    (transform, loc, prop): (Transform, PyAnchorLoc, Option<FontProps>),
    (pad, borderpad, frameon): Frame,
    children: Vec<RawHandle>,
) -> PyResult<RawHandle> {
    let mut aux = AnchoredAuxTransformBox {
        pad,
        borderpad,
        frameon,
        prop: prop.map(|props| props.0),
        ..AnchoredAuxTransformBox::new(transform, loc.0)
    };
    aux.drawing_area.artists = box_items(&children)?;
    Ok(axes.add_anchored_box(aux).raw().clone())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("add_size_bar", add_size_bar)?;
    module.add("add_direction_arrows", add_direction_arrows)?;
    module.add("add_drawing_area", add_drawing_area)?;
    module.add("add_aux_transform_box", add_aux_transform_box)
}
