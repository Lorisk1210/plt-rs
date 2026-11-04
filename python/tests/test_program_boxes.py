"""
Offset boxes, meshes and Axes backgrounds of the program's classes:
`OffsetBox` subclasses that size and draw themselves (matplotlib's own
``DrawingArea`` and ``AuxTransformBox``, copied as a program would copy
them) placed by the packers, padded boxes, anchored boxes and annotation
boxes, or added on their own; the boxes of an added box changed once it
was drawn; `QuadMesh(coordinates, ...)` added with
``add_collection``; the array of an RGB(A) ``pcolormesh``; the patch an
`Axes` subclass makes (``_gen_axes_patch``), which is its background and
clips what it draws.
Each ``scenario_*`` returns what its test compares; the expected values
were recorded from matplotlib 3.11.2 running the same scenarios.
"""

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import path as mpath, transforms as mtransforms
from pltrs.axes import Axes
from pltrs.collections import QuadMesh
from pltrs.lines import Line2D
from pltrs.offsetbox import (AnchoredOffsetbox, AnnotationBbox, HPacker,
                             OffsetBox, PaddedBox, TextArea, VPacker)
from pltrs.patches import Circle, Ellipse, Rectangle, RegularPolygon
from pltrs.projections import register_projection
from pltrs.transforms import Bbox

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


class ProgramDrawingArea(OffsetBox):
    """matplotlib 3.11.2's ``DrawingArea``, as a class of the program's."""

    def __init__(self, width, height, xdescent=0., ydescent=0., clip=False):
        super().__init__()
        self.width = width
        self.height = height
        self.xdescent = xdescent
        self.ydescent = ydescent
        self._clip_children = clip
        self.offset_transform = mtransforms.Affine2D()
        self.dpi_transform = mtransforms.Affine2D()

    def get_transform(self):
        return self.dpi_transform + self.offset_transform

    def set_transform(self, t):
        pass

    def set_offset(self, xy):
        self._offset = xy
        self.offset_transform.clear()
        self.offset_transform.translate(xy[0], xy[1])
        self.stale = True

    def get_offset(self):
        return self._offset

    def get_bbox(self, renderer):
        dpi_cor = renderer.points_to_pixels(1.)
        return Bbox.from_bounds(
            -self.xdescent * dpi_cor, -self.ydescent * dpi_cor,
            self.width * dpi_cor, self.height * dpi_cor)

    def add_artist(self, a):
        self._children.append(a)
        if not a.is_transform_set():
            a.set_transform(self.get_transform())
        if self.axes is not None:
            a.axes = self.axes
        fig = self.get_figure(root=False)
        if fig is not None:
            a.set_figure(fig)

    def draw(self, renderer):
        dpi_cor = renderer.points_to_pixels(1.)
        self.dpi_transform.clear()
        self.dpi_transform.scale(dpi_cor)
        tpath = mtransforms.TransformedPath(
            mpath.Path([[0, 0], [0, self.height],
                        [self.width, self.height],
                        [self.width, 0]]),
            self.get_transform())
        for c in self._children:
            if self._clip_children and not (c.clipbox or c._clippath):
                c.set_clip_path(tpath)
            c.draw(renderer)
        self.stale = False


class ProgramAuxTransformBox(OffsetBox):
    """matplotlib 3.11.2's ``AuxTransformBox``, as a class of the
    program's."""

    def __init__(self, aux_transform):
        self.aux_transform = aux_transform
        super().__init__()
        self.offset_transform = mtransforms.Affine2D()
        self.ref_offset_transform = mtransforms.Affine2D()

    def add_artist(self, a):
        self._children.append(a)
        a.set_transform(self.get_transform())
        self.stale = True

    def get_transform(self):
        return (self.aux_transform
                + self.ref_offset_transform
                + self.offset_transform)

    def set_transform(self, t):
        pass

    def set_offset(self, xy):
        self._offset = xy
        self.offset_transform.clear()
        self.offset_transform.translate(xy[0], xy[1])
        self.stale = True

    def get_offset(self):
        return self._offset

    def get_bbox(self, renderer):
        _off = self.offset_transform.get_matrix()
        self.ref_offset_transform.clear()
        self.offset_transform.clear()
        bboxes = [c.get_window_extent(renderer) for c in self._children]
        ub = Bbox.union(bboxes)
        self.ref_offset_transform.translate(-ub.x0, -ub.y0)
        self.offset_transform.set_matrix(_off)
        return Bbox.from_bounds(0, 0, ub.width, ub.height)

    def draw(self, renderer):
        for c in self._children:
            c.draw(renderer)
        self.stale = False


def _rounded(values, digits=3):
    """*values* as a list of floats rounded to *digits*."""
    return [round(float(v), digits) for v in np.ravel(values)]


def _pixels(fig):
    """The figure drawn, as an integer RGB array."""
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba())[..., :3].astype(int)


def _count(img, rgb):
    """How many pixels of *img* are the colour *rgb*."""
    return int((np.abs(img - np.array(rgb)).sum(axis=-1) == 0).sum())


#: tab:blue, tab:red and tab:green as 8-bit RGB.
BLUE, RED, GREEN = (31, 119, 180), (214, 39, 40), (44, 160, 44)


def scenario_packed_drawing_area():
    """A drawing area of the program's under a text in a `VPacker`,
    anchored in the upper right corner, and one added on its own."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    area = ProgramDrawingArea(40, 20)
    area.add_artist(Circle((10, 10), 10, fc="tab:blue"))
    area.add_artist(Circle((30, 10), 5, fc="tab:red"))
    packer = VPacker(children=[TextArea("hello"), area], sep=3)
    box = AnchoredOffsetbox(child=packer, loc="upper right", pad=0.3)
    ax.add_artist(box)
    alone = ProgramDrawingArea(10, 10)
    alone.add_artist(Rectangle((0, 0), 10, 10, fc="tab:green"))
    alone.set_offset((20, 20))
    ax.add_artist(alone)
    img = _pixels(fig)
    renderer = fig.canvas.get_renderer()
    return {
        "offset": _rounded(area.get_offset()),
        "extent": _rounded(area.get_window_extent(renderer).bounds),
        "box": _rounded(box.get_window_extent(renderer).bounds),
        "child": _rounded(area.get_children()[0].get_window_extent(renderer).bounds),
        "colours": (_count(img, BLUE), _count(img, RED), _count(img, GREEN)),
        "alone": tuple(int(v) for v in img[300 - 1 - 25, 25]),
    }


def test_packed_drawing_area():
    assert scenario_packed_drawing_area() == {
        "offset": [293.333, 206.608],
        "extent": [293.333, 206.608, 55.556, 27.778],
        "box": [289.167, 202.441, 63.889, 54.614],
        "child": [293.333, 206.608, 27.778, 27.778],
        "colours": (553, 128, 146),
        "alone": (44, 160, 44),
    }


def scenario_aux_transform_boxes():
    """The gallery's anchored ellipse and size bar, with an aux-transform
    box of the program's in data coordinates on an Axes of equal aspect."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.set_aspect(1)
    aux = ProgramAuxTransformBox(ax.transData)
    aux.add_artist(Ellipse((0, 0), width=0.1, height=0.15, fc="tab:red"))
    ellipse = AnchoredOffsetbox(child=aux, loc="lower left", frameon=True)
    ax.add_artist(ellipse)
    sizebar = ProgramAuxTransformBox(ax.transData)
    sizebar.add_artist(Line2D([0, 0.1], [0, 0], color="tab:blue", lw=4))
    packer = VPacker(children=[sizebar, TextArea("0.1")], align="center", sep=5)
    bar = AnchoredOffsetbox(child=packer, loc="lower center", frameon=False,
                            pad=0.1, borderpad=0.5)
    ax.add_artist(bar)
    img = _pixels(fig)
    renderer = fig.canvas.get_renderer()
    return {
        "ellipse": _rounded(aux.get_bbox(renderer).bounds),
        "ellipse_offset": _rounded(aux.get_offset()),
        "bar_offset": _rounded(sizebar.get_offset()),
        "box": _rounded(bar.get_window_extent(renderer).bounds),
        "colours": (_count(img, RED), _count(img, BLUE)),
    }


def test_aux_transform_boxes():
    assert scenario_aux_transform_boxes() == {
        "ellipse": [0.0, 0.0, 23.1, 34.65],
        "ellipse_offset": [102.0, 45.5],
        "bar_offset": [193.45, 62.614],
        "box": [192.061, 39.944, 25.878, 24.059],
        "colours": (575, 112),
    }


def scenario_annotation_and_padded_boxes():
    """A clipped drawing area of the program's in an annotation box with
    an arrow, and one in a framed `PaddedBox` around an `HPacker`."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    area = ProgramDrawingArea(30, 30, 5, 5, clip=True)
    area.add_artist(Circle((15, 15), 22, fc="tab:green"))
    annotation = AnnotationBbox(area, (0.3, 0.6), xybox=(0.7, 0.3),
                                arrowprops=dict(arrowstyle="->"))
    ax.add_artist(annotation)
    small = ProgramDrawingArea(20, 10)
    small.add_artist(Rectangle((0, 0), 20, 10, fc="tab:red"))
    padded = PaddedBox(HPacker(children=[TextArea("a"), small], sep=3,
                               align="center"),
                       pad=5, draw_frame=True)
    padded.set_offset((40, 40))
    ax.add_artist(padded)
    img = _pixels(fig)
    renderer = fig.canvas.get_renderer()
    return {
        "offset": _rounded(area.get_offset()),
        "clipped_child": area.get_children()[0].get_clip_path() is not None,
        "small_offset": _rounded(small.get_offset()),
        "padded": _rounded(padded.get_window_extent(renderer).bounds),
        "colours": (_count(img, GREEN), _count(img, RED)),
    }


def test_annotation_and_padded_boxes():
    assert scenario_annotation_and_padded_boxes() == {
        "offset": [253.111, 88.411],
        "clipped_child": True,
        "small_offset": [53.167, 40.224],
        "padded": [33.056, 33.056, 54.833, 28.225],
        "colours": (1764, 392),
    }


def scenario_raising_box():
    """A box of the program's whose ``get_bbox`` raises, in a packer."""
    class Broken(OffsetBox):
        def get_bbox(self, renderer):
            raise RuntimeError("no size")

    fig, ax = plt.subplots()
    ax.add_artist(AnchoredOffsetbox(child=VPacker(children=[Broken()]), loc="center"))
    with pytest.raises(RuntimeError) as raised:
        fig.canvas.draw()
    return str(raised.value)


def test_raising_box():
    assert scenario_raising_box() == "no size"


def scenario_quad_mesh():
    """`QuadMesh(coordinates)` made directly, given values and added with
    ``add_collection``; a Gouraud-shaded one; a malformed grid."""
    x, y = np.meshgrid(np.arange(4), np.arange(3))
    coords = np.stack([x, y], axis=-1)
    mesh = QuadMesh(coords)
    made = (str(mesh.get_coordinates().dtype), mesh.get_coordinates().shape,
            mesh.get_array(), mesh._antialiased, mesh.get_pickradius(),
            mesh.get_mouseover())
    mesh.set_array(np.arange(6.))
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    ax.add_collection(mesh)
    added = (_rounded(ax.get_xlim()), _rounded(ax.get_ylim()),
             _rounded(mesh.get_datalim(ax.transData).bounds),
             _rounded(mesh.get_facecolor()), mesh.get_edgecolor().shape,
             _rounded(mesh.get_linewidth()))
    gouraud = QuadMesh(coords.astype(float), shading="gouraud",
                       antialiased=False)
    gouraud.set_array(np.arange(12.))
    with pytest.raises(ValueError) as malformed:
        QuadMesh(np.zeros((3, 4)))
    img = _pixels(fig)
    return {
        "made": made,
        "added": added,
        "gouraud": (gouraud._shading, len(gouraud.get_paths()),
                    gouraud._antialiased),
        "malformed": str(malformed.value),
        "drawn": (_count(img, (68, 1, 84)), tuple(int(v) for v in img[75, 100])),
    }


def test_quad_mesh():
    assert scenario_quad_mesh() == {
        "made": ("int64", (3, 4, 2), None, True, 0, False),
        "added": ([-0.15, 3.15], [-0.1, 2.1], [0.0, 0.0, 3.0, 2.0],
                  [0.122, 0.467, 0.706, 1.0], (0, 4), [1.0]),
        "gouraud": ("gouraud", 6, False),
        "malformed": "'coordinates' must be 3D with shape (M, N, 2), but your "
                     "input has shape (3, 4)",
        "drawn": (2444, (122, 209, 81)),
    }


def scenario_rgb_pcolormesh():
    """The array of an RGB and of an 8-bit RGB ``pcolormesh``, and the norm
    they scale."""
    fig, ax = plt.subplots()
    rgb = np.linspace(0, 1, 2 * 3 * 3).reshape(2, 3, 3) * 0.5 + 0.2
    mesh = ax.pcolormesh(rgb)
    array = mesh.get_array()
    small = ax.pcolormesh((np.arange(18).reshape(2, 3, 3) * 10).astype(np.uint8))
    fig.canvas.draw()
    return {
        "rgb": (type(array).__name__, array.shape, str(array.dtype),
                round(float(array.sum()), 6)),
        "norm": (round(mesh.norm.vmin, 6), round(mesh.norm.vmax, 6)),
        "uint8": (str(small.get_array().dtype), small.get_array().shape),
        "faces": _rounded(mesh.get_facecolor()[1]),
    }


def test_rgb_pcolormesh():
    assert scenario_rgb_pcolormesh() == {
        "rgb": ("MaskedArray", (2, 3, 3), "float64", 8.1),
        "norm": (0.2, 0.7),
        "uint8": ("uint8", (2, 3, 3)),
        "faces": [0.288, 0.318, 0.347, 1.0],
    }


def scenario_axes_patches():
    """Rectilinear Axes classes of the program's making a circle, a
    hexagon reaching beyond the Axes box and a smaller rectangle: the
    background takes their shape and clips what the Axes draws."""
    shapes = {
        "round_ext2": lambda: Circle((0.5, 0.5), 0.5),
        "hexagon_ext2": lambda: RegularPolygon((0.5, 0.5), 6, radius=0.6),
        "inset_ext2": lambda: Rectangle((0.1, 0.1), 0.8, 0.8),
    }
    for name, shape in shapes.items():
        register_projection(type(name, (Axes,), {
            "name": name, "_gen_axes_patch": lambda self, shape=shape: shape()}))
    fig = plt.figure(figsize=(6, 2), dpi=50)
    out = {}
    for i, name in enumerate(shapes):
        ax = fig.add_subplot(1, 3, i + 1, projection=name)
        ax.set_facecolor("tab:green")
        ax.imshow(np.zeros((2, 2)), cmap="gray", extent=(-1, 2, -1, 2))
        ax.set(xlim=(0, 1), ylim=(0, 1), xticks=[], yticks=[])
        out[name] = type(ax.patch).__name__
    img = _pixels(fig)
    boxes = [ax.get_window_extent() for ax in fig.axes]
    corners = [tuple(int(v) for v in img[100 - 1 - int(b.y0 + 2), int(b.x0 + 2)])
               for b in boxes]
    out["corners"] = corners
    out["black"] = _count(img, (0, 0, 0))
    top = boxes[1]
    out["hexagon_tip"] = tuple(int(v) for v in img[100 - 1 - int(top.y1 + 3),
                                                   int(top.x0 + top.width / 2)])
    return out


def test_axes_patches():
    assert scenario_axes_patches() == {
        "round_ext2": "Circle",
        "hexagon_ext2": "RegularPolygon",
        "inset_ext2": "Rectangle",
        "corners": [(255, 255, 255), (255, 255, 255), (255, 255, 255)],
        "black": 10597,
        "hexagon_tip": (44, 160, 44),
    }


def scenario_changed_after_drawing():
    """An offset image zoomed and a drawing area widened once their boxes
    were drawn: the next draw shows them so."""
    from pltrs.offsetbox import DrawingArea, OffsetImage
    fig, ax = plt.subplots(figsize=(3, 3), dpi=100)
    image = OffsetImage(np.zeros((4, 4)), zoom=2, cmap="gray")
    ax.add_artist(AnnotationBbox(image, (0.5, 0.5), frameon=False))
    area = DrawingArea(10, 10)
    area.add_artist(Rectangle((0, 0), 100, 10, fc="tab:red"))
    anchored = AnchoredOffsetbox(loc="lower left", child=area, frameon=False)
    ax.add_artist(anchored)
    before = _count(_pixels(fig), (0, 0, 0))
    image.set_zoom(5)
    area.width = 40
    img = _pixels(fig)
    renderer = fig.canvas.get_renderer()
    return {
        "black": (before, _count(img, (0, 0, 0))),
        "image": _rounded(image.get_window_extent().bounds),
        "area": _rounded(anchored.get_window_extent(renderer).bounds),
        "stale": image.stale,
    }


def test_changed_after_drawing():
    assert scenario_changed_after_drawing() == {
        "black": (1182, 1817),
        "image": [139.861, 134.611, 27.778, 27.778],
        "area": [44.444, 39.944, 66.667, 25.0],
        "stale": False,
    }
