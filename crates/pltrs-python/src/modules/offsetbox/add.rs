//! Offset boxes joining an axes or a figure (matplotlib's `add_artist` of
//! an `AnchoredOffsetbox`, an `AnnotationBbox` or a plain `OffsetBox`):
//! the Python box is a plain loose artist until then, whose common
//! properties the native box takes, and the artists of its tree move in
//! with it. Each function returns the box, the artists of its tree in
//! drawing order (frames before what they frame) and, but for a plain
//! box, its frame, for the Python objects to follow them. The drawing
//! areas of an added box are reached through it and their index in its
//! tree.

use pltrs::artist::{ArtistHandle, RawHandle, TypedHandle};
use pltrs::axes::Axes;
use pltrs::figure::Figure;
use pltrs::fonts::FontSize;
use pltrs::offsetbox::{
    AnchorLoc, AnchoredBox, AnchoredOffsetbox, AnchoredOptions, AnnotationBbox,
    AnnotationBboxOptions, BboxToAnchor, BoxItem, PaddedBox, PlacedOffsetBox, ProgramOffsetBox,
};
use pltrs::patches::{FancyBboxPatch, Patch};
use pltrs::text::ArrowProps;
use pltrs::transforms::Point;

use super::super::annotation::{ArrowKwargs, PyCoords};
use super::super::font_manager::FontProps;
use super::super::patch_styles::PyBoxStyle;
use super::super::patches::PatchKwargs;
use super::tree::PyOffsetBox;
use crate::binding::dict_value;
use crate::binding::prelude::*;

/// A box added: its handle, the artists of its tree and its frame.
type Added = (RawHandle, Vec<RawHandle>, Patch<FancyBboxPatch>);

/// Gives `to` the properties every artist has that were set on `from`,
/// the plain artist an offset box was before it was added.
fn carry_over(from: &RawHandle, to: &RawHandle) {
    to.set_visible(from.visible())
        .set_zorder(from.zorder())
        .set_alpha(from.alpha())
        .set_label(from.label())
        .set_picker(from.picker())
        .set_clip_on(from.clip_on())
        .set_animated(from.animated())
        .set_rasterized(from.rasterized())
        .set_in_layout(from.in_layout())
        .set_mouseover(from.mouseover())
        .set_snap(from.snap())
        .set_sketch_params(from.sketch_params())
        .set_path_effects(from.path_effects())
        .set_agg_filter(from.agg_filter());
    match from.gid() {
        Some(gid) => to.set_gid(gid),
        None => to.clear_gid(),
    };
    match from.url() {
        Some(url) => to.set_url(url),
        None => to.clear_url(),
    };
}

/// A location code of `AnchoredOffsetbox.codes` (1 to 10).
fn anchor_loc(code: usize) -> PyResult<AnchorLoc> {
    code.checked_sub(1)
        .and_then(|index| AnchorLoc::ALL.get(index).copied())
        .ok_or_else(|| PyErr::value_error(format!("{code} is not a valid location code")))
}

/// The box an offset box is anchored to: `None`, a point `(x, y)` or a
/// box `(x, y, width, height)`.
fn bbox_to_anchor(value: Option<Vec<f64>>) -> PyResult<Option<BboxToAnchor>> {
    Ok(match value.as_deref() {
        None => None,
        Some(&[x, y]) => Some(BboxToAnchor::Point(x, y)),
        Some(&[x, y, width, height]) => Some(BboxToAnchor::Bounds(x, y, width, height)),
        Some(other) => {
            return Err(PyErr::value_error(format!("Invalid bbox: {other:?}")));
        }
    })
}

/// The settings of an `AnchoredOffsetbox` as a dict: `loc` (a code),
/// `pad`, `borderpad` (one or two numbers), `prop` (a `FontProperties`),
/// `frameon`, `bbox_to_anchor` (bounds or a point) and `bbox_transform`.
pub(super) struct PyAnchoredOptions(pub(super) AnchoredOptions);

impl FromPy<'_> for PyAnchoredOptions {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let borderpad: Bound<'_> = dict_value(object, "borderpad")?;
        let borderpad = match borderpad.extract::<f64>() {
            Ok(pad) => (pad, pad),
            Err(_) => borderpad.extract()?,
        };
        Ok(Self(AnchoredOptions {
            loc: anchor_loc(dict_value(object, "loc")?)?,
            pad: dict_value(object, "pad")?,
            borderpad,
            prop: dict_value::<Option<FontProps>>(object, "prop")?.map(|prop| prop.0),
            frameon: dict_value(object, "frameon")?,
            bbox_to_anchor: bbox_to_anchor(dict_value(object, "bbox_to_anchor")?)?,
            bbox_transform: dict_value(object, "bbox_transform")?,
        }))
    }
}

/// A loose frame as an anchored box makes its own, shown when `frameon`
/// (the `patch` the Python box makes when it is made).
fn new_frame(frameon: bool) -> PyResult<Patch<FancyBboxPatch>> {
    Ok(AnchoredBox::new_frame(frameon))
}

/// Adds the box `child` anchored as `options` say and framed by `frame`,
/// the loose `FancyBboxPatch` the Python box made (its `patch`, which
/// moves in), to `owner`, an axes or a (sub)figure, taking the common
/// properties of `plain`.
fn add_anchored(
    owner: RawHandle,
    plain: RawHandle,
    child: PyOffsetBox,
    options: PyAnchoredOptions,
    frame: RawHandle,
) -> PyResult<Added> {
    let spec = AnchoredBox::new(child.joining(&owner)?, options.0)
        .with_frame(BoxItem::joining(&frame, &owner)?)
        .map_err(|err| PyErr::type_error(err.to_string()))?;
    let anchored = match Axes::from_raw_handle(&owner) {
        Some(axes) => axes.add_anchored_box(spec),
        None => owner
            .downcast::<Figure>()
            .ok_or_else(|| PyErr::type_error("an offset box joins an axes or a figure"))?
            .add_anchored_box(spec),
    };
    carry_over(&plain, anchored.raw());
    Ok((anchored.raw().clone(), anchored.artists(), anchored.patch()))
}

/// Anchors the added anchored box `anchored` as `options` say from now on
/// (matplotlib reads them again at every draw).
fn set_anchored_options(anchored: RawHandle, options: PyAnchoredOptions) -> PyResult<()> {
    anchored
        .downcast::<AnchoredOffsetbox>()
        .ok_or_else(|| PyErr::type_error("not an anchored offset box"))?
        .set_options(&options.0);
    Ok(())
}

/// Places the child of the added anchored box `anchored` at the display
/// point `offset`, or back at its anchor (`None`): where the
/// `get_offset` of a box of the program's puts it.
fn set_anchored_offset(anchored: RawHandle, offset: Option<(f64, f64)>) -> PyResult<()> {
    anchored
        .downcast::<AnchoredOffsetbox>()
        .ok_or_else(|| PyErr::type_error("not an anchored offset box"))?
        .set_child_offset(offset.map(Point::from));
    Ok(())
}

/// The options of an `AnnotationBbox` as a dict: `xybox`, `xycoords`,
/// `boxcoords`, `frameon`, `pad`, `annotation_clip`, `box_alignment`,
/// `boxstyle` (a description), `bboxprops` (patch properties),
/// `arrowprops` (with an arrow style) and `fontsize` (in points).
struct PyAnnotationBboxOptions(AnnotationBboxOptions);

impl FromPy<'_> for PyAnnotationBboxOptions {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let boxstyle = dict_value::<Option<PyBoxStyle>>(object, "boxstyle")?.map(|style| style.0);
        let arrowprops = match dict_value::<Option<ArrowKwargs>>(object, "arrowprops")? {
            None => None,
            Some(ArrowKwargs(ArrowProps::Fancy(fancy))) => Some(fancy),
            Some(ArrowKwargs(ArrowProps::Legacy(_))) => {
                return Err(PyErr::value_error(
                    "the arrow of an AnnotationBbox needs an arrowstyle",
                ));
            }
        };
        Ok(Self(AnnotationBboxOptions {
            xybox: Some(Point::from(dict_value::<(f64, f64)>(object, "xybox")?)),
            xycoords: dict_value::<PyCoords>(object, "xycoords")?.0,
            boxcoords: Some(dict_value::<PyCoords>(object, "boxcoords")?.0),
            frameon: dict_value(object, "frameon")?,
            pad: dict_value(object, "pad")?,
            annotation_clip: dict_value(object, "annotation_clip")?,
            box_alignment: dict_value(object, "box_alignment")?,
            boxstyle,
            bboxprops: dict_value::<PatchKwargs>(object, "bboxprops")?.0,
            arrowprops,
            fontsize: Some(FontSize::Points(dict_value(object, "fontsize")?)),
        }))
    }
}

/// Adds the box `child` annotating `xy` as `options` say to `owner`, an
/// axes or a (sub)figure, taking the common properties of `plain`.
fn add_annotation_bbox(
    owner: RawHandle,
    plain: RawHandle,
    child: PyOffsetBox,
    xy: (f64, f64),
    options: PyAnnotationBboxOptions,
) -> PyResult<Added> {
    let added = match Axes::from_raw_handle(&owner) {
        Some(axes) => axes.add_annotation_bbox(child.joining(&owner)?, xy, options.0),
        None => owner
            .downcast::<Figure>()
            .ok_or_else(|| PyErr::type_error("an offset box joins an axes or a figure"))?
            .add_annotation_bbox(child.joining(&owner)?, xy, options.0),
    }
    .map_err(|err| PyErr::value_error(err.to_string()))?;
    carry_over(&plain, added.raw());
    Ok((added.raw().clone(), added.artists(), added.patch()))
}

/// Adds the box `child` with its origin at the display point `offset` to
/// `owner`, an axes or a (sub)figure, taking the common properties of
/// `plain`; returns the box and the artists of its tree.
fn add_placed(
    owner: RawHandle,
    plain: RawHandle,
    child: PyOffsetBox,
    offset: (f64, f64),
) -> PyResult<(RawHandle, Vec<RawHandle>)> {
    let child = child.joining(&owner)?;
    let placed = match Axes::from_raw_handle(&owner) {
        Some(axes) => axes.add_offset_box(child, offset),
        None => owner
            .downcast::<Figure>()
            .ok_or_else(|| PyErr::type_error("an offset box joins an axes or a figure"))?
            .add_offset_box(child, offset),
    };
    carry_over(&plain, placed.raw());
    Ok((placed.raw().clone(), placed.artists()))
}

/// Adds the box `child` with its origin at the display point `offset` as
/// a box the program's box `program` draws itself (a child of an
/// `OffsetBox` subclass of the program's, or the box such a subclass of
/// a package class is), taking the common properties of `plain` when
/// given; returns the box and the artists of its tree.
fn add_owned(
    program: ProgramOffsetBox,
    plain: Option<RawHandle>,
    child: PyOffsetBox,
    offset: (f64, f64),
) -> PyResult<(RawHandle, Vec<RawHandle>)> {
    let placed = program.add_box(child.joining(program.raw())?, offset);
    if let Some(plain) = plain {
        carry_over(&plain, placed.raw());
    }
    Ok((placed.raw().clone(), placed.artists()))
}

/// A loose frame as a `PaddedBox` makes its own (its `patch`), shown when
/// `draw_frame` and styled with `patch_kwargs`.
fn new_padded_frame(
    draw_frame: bool,
    patch_kwargs: PatchKwargs,
) -> PyResult<Patch<FancyBboxPatch>> {
    Ok(PaddedBox::new_frame(draw_frame, patch_kwargs.0))
}

/// Moves the origin of the added plain box `placed` to the display point
/// `offset` (`OffsetBox.set_offset`).
fn set_placed_offset(placed: RawHandle, offset: (f64, f64)) -> PyResult<()> {
    placed
        .downcast::<PlacedOffsetBox>()
        .ok_or_else(|| PyErr::type_error("not an offset box added at an offset"))?
        .set_offset(offset);
    Ok(())
}

/// The added annotation box `annotation`.
pub(super) fn annotation_bbox(annotation: &RawHandle) -> PyResult<AnnotationBbox> {
    annotation
        .downcast::<AnnotationBbox>()
        .ok_or_else(|| PyErr::type_error("not an annotation box"))
}

/// Moves the annotated point `xy` and the box position `xybox` of the
/// added annotation box `annotation`, each kept when `None`.
fn set_annotation_points(
    annotation: RawHandle,
    xy: Option<(f64, f64)>,
    xybox: Option<(f64, f64)>,
) -> PyResult<()> {
    let annotation = annotation_bbox(&annotation)?;
    if let Some(xy) = xy {
        annotation.set_xy(xy);
    }
    if let Some(xybox) = xybox {
        annotation.set_xybox(xybox);
    }
    Ok(())
}

/// When the added annotation box `annotation` is left out for an
/// annotated point outside its axes (`get_annotation_clip`).
fn get_annotation_clip(annotation: RawHandle) -> PyResult<Option<bool>> {
    Ok(annotation_bbox(&annotation)?.annotation_clip())
}

/// Sets when the added annotation box `annotation` is left out
/// (`set_annotation_clip`).
fn set_annotation_clip(annotation: RawHandle, clip: Option<bool>) -> PyResult<()> {
    annotation_bbox(&annotation)?.set_annotation_clip(clip);
    Ok(())
}

/// Sets the font size, in points, that scales the frame and the arrow of
/// the added annotation box `annotation` (`set_fontsize`).
fn set_annotation_fontsize(annotation: RawHandle, points: f64) -> PyResult<()> {
    annotation_bbox(&annotation)?.set_fontsize(points);
    Ok(())
}

/// Places the frame and the parts of the added annotation box
/// `annotation` where the figure draws them (`update_positions`).
fn update_annotation_positions(annotation: RawHandle) -> PyResult<()> {
    annotation_bbox(&annotation)?.update_positions();
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new_frame", new_frame)?;
    module.add("add_anchored", add_anchored)?;
    module.add("add_annotation_bbox", add_annotation_bbox)?;
    module.add("add_placed", add_placed)?;
    module.add("set_placed_offset", set_placed_offset)?;
    module.add("set_annotation_points", set_annotation_points)?;
    module.add("update_annotation_positions", update_annotation_positions)?;
    module.add("get_annotation_clip", get_annotation_clip)?;
    module.add("set_annotation_clip", set_annotation_clip)?;
    module.add("set_annotation_fontsize", set_annotation_fontsize)?;
    module.add("set_anchored_options", set_anchored_options)?;
    module.add("set_anchored_offset", set_anchored_offset)?;
    module.add("add_owned", add_owned)?;
    module.add("new_padded_frame", new_padded_frame)
}
