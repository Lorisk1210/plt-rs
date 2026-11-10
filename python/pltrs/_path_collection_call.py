"""
The call matplotlib's Agg renderer refuses when a collection's line widths
are no numbers: ``RendererAgg.draw_path_collection`` as ``Collection.draw``
makes it (from matplotlib 3.11.2, under the Matplotlib License), whose
pybind11 binding fails converting the widths and lists the call's
arguments (`._pybind.incompatible`).

matplotlib's draw first calls the renderer with no paths and the hatch
colours to learn whether it takes them; that call fails the same way and
is swallowed, and the draw calls it again with the paths. A collection of
one path, one transform and one colour, width and style of each, whose
path fits the figure, is drawn as markers instead (``draw_markers``),
after the graphics context takes the width.
"""

import numpy as np

from . import _pybind, transforms
from .path import Path

#: The signature of matplotlib's ``RendererAgg.draw_path_collection``,
#: ``{module}`` standing for the module of pltrs's renderer.
_ARRAY = "typing.Annotated[numpy.typing.ArrayLike, numpy.float64]"
_SIGNATURE = (
    "(self: {module}.RendererAgg, gc: GCAgg, master_transform: trans_affine, "
    f"paths: PathGenerator, transforms: {_ARRAY}, offsets: {_ARRAY}, "
    f"offset_trans: trans_affine, facecolors: {_ARRAY}, edgecolors: {_ARRAY}, "
    f"linewidths: {_ARRAY}, dashes: collections.abc.Sequence[Dashes], "
    "antialiaseds: typing.Annotated[numpy.typing.ArrayLike, numpy.uint8], "
    "ignored: object, offset_position: object, *, "
    f"hatchcolors: {_ARRAY} = array([], shape=(0, 4), dtype=float64)) -> None")


def prepared_points(collection):
    """The transform, offset transform, offsets and paths matplotlib's draw
    hands the renderer (``Collection._prepare_points``)."""
    transform = collection.get_transform()
    offset_trf = collection.get_offset_transform()
    offsets = collection.get_offsets()
    paths = collection.get_paths()
    if collection.have_units():
        paths = []
        for path in collection.get_paths():
            vertices = path.vertices
            xs = collection.convert_xunits(vertices[:, 0])
            ys = collection.convert_yunits(vertices[:, 1])
            paths.append(Path(np.column_stack([xs, ys]), path.codes))
        xs = collection.convert_xunits(offsets[:, 0])
        ys = collection.convert_yunits(offsets[:, 1])
        offsets = np.ma.column_stack([xs, ys])
    if not transform.is_affine:
        paths = [transform.transform_path_non_affine(path) for path in paths]
        transform = transform.get_affine()
    if not offset_trf.is_affine:
        offsets = offset_trf.transform_non_affine(offsets)
        offset_trf = offset_trf.get_affine()
    if isinstance(offsets, np.ma.MaskedArray):
        offsets = offsets.filled(np.nan)
    return transform, offset_trf, offsets, paths


def _drawn_as_markers(collection, transform, paths):
    """Whether matplotlib's draw draws *collection* as markers: one path,
    transform, colour, width and style of line (without dashes) and
    neither several URLs nor a hatch, the path fitting in the figure."""
    linewidths = collection.get_linewidth()
    if not (len(paths) == 1 and len(collection.get_transforms()) <= 1
            and len(collection.get_facecolor()) == 1
            and len(collection.get_edgecolor()) == 1
            and len(linewidths) == 1
            and all(dashes is None for _, dashes in collection.get_linestyle())
            and len(collection.get_antialiased()) == 1
            and len(collection.get_urls()) == 1
            and collection.get_hatch() is None):
        return False
    trans = collection.get_transforms()
    combined = (transforms.Affine2D(trans[0]) + transform) if len(trans) else transform
    extents = paths[0].get_extents(combined)
    figure_box = collection.get_figure(root=True).bbox
    return extents.width < figure_box.width and extents.height < figure_box.height


def refused(collection, renderer):
    """pybind11's error for the ``draw_path_collection`` call matplotlib's
    draw of *collection* makes with *renderer*, an Agg renderer; None where
    its draw draws markers instead."""
    transform, offset_trf, offsets, paths = prepared_points(collection)
    if _drawn_as_markers(collection, transform, paths):
        return None
    arguments = (
        renderer._renderer, renderer.new_gc(), transform.frozen(), paths,
        collection.get_transforms(), offsets, offset_trf,
        collection.get_facecolor(), collection.get_edgecolor(),
        collection.get_linewidth(), collection.get_linestyle(),
        collection.get_antialiased(), collection.get_urls(), "screen")
    module = type(renderer._renderer).__module__
    return _pybind.incompatible(
        "draw_path_collection", [_SIGNATURE.format(module=module)], arguments, {})
