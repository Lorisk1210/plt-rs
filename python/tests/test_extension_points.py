"""
The extension points programs use (pltrs (Python) runs the program's own
subclass or object, with pltrs (Rust) drawing): a subclass of `PolarAxes`
with a transform, a background and spines of its own (the radar chart
example's), regions clipping the gridlines (``grid_clip_box``,
``grid_clip_path``), the `Shadow` patches of a pie and of a text's box,
the arrow of an `AnnotationBbox`, bivariate and multivariate colour
mapping of collections and meshes, a projection whose transforms do not
end with ``transAxes``.
Each ``scenario_*`` returns what its test compares; the expected values
were recorded from matplotlib 3.11.2 running the same scenarios.
"""

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import collections, colorizer, colors
from pltrs import transforms as mtransforms
from pltrs.axes import Axes
from pltrs.offsetbox import AnnotationBbox, TextArea
from pltrs.patches import Circle, Rectangle, RegularPolygon, Shadow
from pltrs.path import Path
from pltrs.projections import register_projection
from pltrs.projections.polar import PolarAxes
from pltrs.spines import Spine
from pltrs.transforms import Affine2D, Bbox, TransformedBbox

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def scenario_polar_subclass():
    """A five-sided version of the radar chart example's ``RadarAxes``:
    its constructor, background, spines and transform all run."""
    calls = {"path": 0, "steps": [], "patch": 0, "spines": 0, "init": 0}

    class StarTransform(PolarAxes.PolarTransform):
        def transform_path_non_affine(self, path):
            calls["path"] += 1
            calls["steps"].append(path._interpolation_steps)
            if path._interpolation_steps > 1:
                path = path.interpolated(5)
            return Path(self.transform(path.vertices), path.codes)

    class StarAxes(PolarAxes):
        name = "star5"
        PolarTransform = StarTransform

        def __init__(self, *args, **kwargs):
            calls["init"] += 1
            super().__init__(*args, **kwargs)
            self.set_theta_zero_location("N")

        def _gen_axes_patch(self):
            calls["patch"] += 1
            return RegularPolygon((0.5, 0.5), 5, radius=.5, edgecolor="k")

        def _gen_axes_spines(self):
            calls["spines"] += 1
            spine = Spine(axes=self, spine_type="circle",
                          path=Path.unit_regular_polygon(5))
            spine.set_transform(Affine2D().scale(.5).translate(.5, .5)
                                + self.transAxes)
            return {"polar": spine}

    register_projection(StarAxes)
    fig = plt.figure(figsize=(3, 3), dpi=50)
    ax = fig.add_subplot(projection="star5")
    out = {
        "type": type(ax).__name__,
        "offset": float(ax.get_theta_offset()),
        "spines": list(ax.spines),
        "patch": type(ax.patch).__name__,
        "patch_ec": tuple(map(float, ax.patch.get_edgecolor())),
        "spine_type": ax.spines["polar"].spine_type,
    }
    theta = np.linspace(0, 2 * np.pi, 5, endpoint=False)
    ax.plot(theta, [1, 2, 3, 2, 1])
    ax.fill(theta, [1, 2, 3, 2, 1], alpha=0.25)
    ax.set_rgrids([1, 2])
    before = dict(calls)
    fig.canvas.draw()
    out["calls_before_draw"] = (before["init"], before["patch"],
                                before["spines"], before["path"])
    out["draw_paths"] = calls["path"] - before["path"]
    out["steps"] = sorted(set(calls["steps"]))
    img = np.asarray(fig.canvas.buffer_rgba())
    # Outside the pentagon stays white; its top vertex is framed.
    out["corner"] = tuple(int(v) for v in img[140, 10, :3])
    out["top"] = int((img[2:12, 70:80, :3].sum(axis=-1) < 300).sum())
    return out


def test_polar_subclass():
    assert scenario_polar_subclass() == {
        "type": "StarAxes",
        "offset": 1.5707963267948966,
        "spines": ["polar"],
        "patch": "RegularPolygon",
        "patch_ec": (0.0, 0.0, 0.0, 0.0),
        "spine_type": "circle",
        "calls_before_draw": (1, 2, 1, 0),
        "draw_paths": 12,
        "steps": [1, 180],
        "corner": (255, 255, 255),
        "top": 10,
    }


def scenario_polar_constructor():
    """`PolarAxes(fig, rect, ...)` with the keywords of its constructor,
    which a clear keeps."""
    fig = plt.figure()
    ax = PolarAxes(fig, [0.1, 0.1, 0.8, 0.8], theta_offset=1.0,
                   theta_direction=-1, rlabel_position=45)
    fig.add_axes(ax)

    def settings():
        return (float(ax.get_theta_offset()), float(ax.get_theta_direction()),
                float(ax.get_rlabel_position()))

    made = (type(ax).__name__, ax.name, *settings())
    ax.clear()
    return made, settings()


def test_polar_constructor():
    assert scenario_polar_constructor() == (
        ("PolarAxes", "polar", 1.0, -1.0, 45.0), (1.0, -1.0, 45.0))


def _red_grid(setup, polar=False):
    """The red gridline pixels of a 100-pixel square Axes after *setup*:
    all of them, those of the top left and of the bottom right quarter."""
    fig = plt.figure(figsize=(2, 2), dpi=50)
    ax = fig.add_axes([0, 0, 1, 1], polar=polar)
    ax.grid(True, color="red", lw=2)
    if not polar:
        ax.set_xticks([0.25, 0.5, 0.75])
        ax.set_yticks([0.25, 0.5, 0.75])
    for spine in ax.spines.values():
        spine.set_visible(False)
    setup(ax)
    fig.canvas.draw()
    img = np.asarray(fig.canvas.buffer_rgba())[..., :3].astype(int)
    red = (img[..., 0] > 200) & (img[..., 1] < 80) & (img[..., 2] < 80)
    return int(red.sum()), int(red[:50, :50].sum()), int(red[50:, 50:].sum())


def scenario_grid_clip():
    """The gridlines clipped by boxes and paths of the program's."""
    square = Path([[0, 0], [1, 0], [0, 1], [0, 0]], closed=True)
    return {
        "plain": _red_grid(lambda ax: None),
        "box_none": _red_grid(lambda ax: ax.tick_params(grid_clip_box=None)),
        "box_data": _red_grid(lambda ax: ax.tick_params(grid_clip_box=TransformedBbox(
            Bbox([[0, 0], [0.5, 0.5]]), ax.transData))),
        "box_display": _red_grid(lambda ax: ax.tick_params(
            grid_clip_box=Bbox([[50, 50], [100, 100]]))),
        "path_circle": _red_grid(lambda ax: ax.tick_params(
            grid_clip_path=Circle((0.5, 0.5), 0.25, transform=ax.transAxes))),
        "path_rect": _red_grid(lambda ax: ax.tick_params(
            grid_clip_path=Rectangle((0, 0), 0.5, 0.5, transform=ax.transAxes))),
        "path_tuple": _red_grid(lambda ax: ax.tick_params(
            grid_clip_path=(square, ax.transAxes))),
        "path_none": _red_grid(lambda ax: ax.tick_params(grid_clip_path=None)),
        "polar_plain": _red_grid(lambda ax: None, polar=True),
        "polar_path_none": _red_grid(
            lambda ax: ax.tick_params(grid_clip_path=None), polar=True),
        "polar_box": _red_grid(lambda ax: ax.tick_params(
            grid_clip_box=Bbox([[0, 0], [50, 50]])), polar=True),
    }


def test_grid_clip():
    assert scenario_grid_clip() == {
        "plain": (588, 98, 196),
        "box_none": (588, 98, 196),
        "box_data": (146, 0, 0),
        "box_display": (148, 0, 0),
        "path_circle": (113, 8, 49),
        "path_rect": (146, 0, 0),
        "path_tuple": (294, 49, 99),
        "path_none": (588, 98, 196),
        "polar_plain": (1125, 239, 323),
        "polar_path_none": (1337, 292, 376),
        "polar_box": (282, 0, 0),
    }


def _rounded(values):
    return np.round(np.asarray(values, float), 3).tolist()


def scenario_shadows():
    """The shadows of a pie's wedges and of a text's box."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    ax.pie([1, 2, 3], shadow=True)
    shadow = ax.patches[1]
    out = {
        "types": [type(p).__name__ for p in ax.patches],
        "str": str(shadow),
        "is_patch": shadow.patch is ax.patches[0],
        "fc": _rounded(shadow.get_facecolor()),
        "z": (float(shadow.get_zorder()), float(ax.patches[0].get_zorder())),
        "label": shadow.get_label(),
        "ext_before": _rounded(shadow.get_window_extent().bounds),
    }
    fig.canvas.draw()
    out["ext_after"] = _rounded(shadow.get_window_extent().bounds)
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    ax.pie([1, 2], shadow={'ox': -0.04, 'edgecolor': 'none', 'shade': 0.9})
    fig.canvas.draw()
    out["dict"] = (_rounded(ax.patches[1].get_facecolor()),
                   _rounded(ax.patches[1].get_edgecolor()),
                   _rounded(ax.patches[1].get_window_extent().bounds))
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    text = ax.text(0.5, 0.5, "hello", bbox=dict(boxstyle="round"))
    box = text.get_bbox_patch()
    out["box_before"] = (_rounded(box.get_window_extent().bounds),
                         box.is_transform_set())
    fig.canvas.draw()
    out["box_after"] = (_rounded(box.get_window_extent().bounds),
                        _rounded(box.get_path().get_extents().bounds))
    text_shadow = Shadow(box, 3, -3)
    ax.add_patch(text_shadow)
    out["text_shadow"] = _rounded(text_shadow.get_window_extent().bounds)
    fig.canvas.draw()
    out["text_shadow_drawn"] = _rounded(text_shadow.get_window_extent().bounds)
    return out


def test_shadows():
    assert scenario_shadows() == {
        "types": ["Wedge", "Shadow", "Wedge", "Shadow", "Wedge", "Shadow"],
        "str": "Shadow(Wedge(center=(0, 0), r=1, theta1=0, theta2=60, width=None))",
        "is_patch": True,
        "fc": [0.036, 0.14, 0.212, 0.5],
        "z": (0.9999999999999999, 1.0),
        "label": "_nolegend_",
        "ext_before": [147.6, 106.92, 89.28, 57.615],
        "ext_after": [146.269, 105.589, 66.528, 57.615],
        "dict": ([0.012, 0.047, 0.071, 0.5], [0.0, 0.0, 0.0, 0.0],
                 [111.675, 105.589, 99.792, 66.528]),
        "box_before": ([-0.3, -0.3, 1.6, 1.6], True),
        "box_after": ([144.6, 101.518, 30.0, 16.402], [-3.0, -3.0, 30.0, 16.402]),
        "text_shadow": [144.6, 101.518, 30.0, 16.402],
        "text_shadow_drawn": [147.6, 98.518, 30.0, 16.402],
    }


def scenario_annotation_bbox_arrow():
    """``AnnotationBbox.arrow_patch`` before and after the box joins an
    Axes, changed by the program."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    box = AnnotationBbox(TextArea("box"), (0.2, 0.3), xybox=(0.7, 0.8),
                         arrowprops=dict(arrowstyle="->", color="r", relpos=(0, 0)))
    before = box.arrow_patch
    out = {"before": (type(before).__name__, box.arrowprops,
                      _rounded(before.get_edgecolor()), box._arrow_relpos)}
    ax.add_artist(box)
    fig.canvas.draw()
    arrow = box.arrow_patch
    out["after"] = (arrow is before, _rounded(arrow.get_edgecolor()),
                    type(arrow.get_arrowstyle()).__name__,
                    _rounded(arrow.get_window_extent().bounds))
    out["children"] = [type(c).__name__ for c in box.get_children()]
    arrow.set_color("b")
    fig.canvas.draw()
    img = np.asarray(fig.canvas.buffer_rgba())[..., :3].astype(int)
    out["blue"] = int(((img[..., 2] > 200) & (img[..., 0] < 80)).sum() > 20)
    out["none"] = AnnotationBbox(TextArea("no"), (0.5, 0.5)).arrow_patch
    return out


def test_annotation_bbox_arrow():
    assert scenario_annotation_bbox_arrow() == {
        "before": ("FancyArrowPatch", {"arrowstyle": "->", "color": "r"},
                   [1.0, 0.0, 0.0, 1.0], (0, 0)),
        "after": (True, [1.0, 0.0, 0.0, 1.0], "CurveB",
                  [83.127, 75.543, 94.177, 71.45]),
        "children": ["TextArea", "FancyBboxPatch", "FancyArrowPatch"],
        "blue": 1,
        "none": None,
    }


def _drawn(fig):
    """A checksum and the count of dark pixels of *fig* drawn."""
    fig.canvas.draw()
    img = np.asarray(fig.canvas.buffer_rgba())[..., :3].astype(np.int64)
    return int(img.sum() % 1000003), int((img.sum(axis=-1) < 700).sum())


def scenario_multivariate():
    """Collections and a mesh mapping two or three variates to colours,
    with matplotlib's norms and one of the program's."""
    a, b = np.linspace(0, 1, 6), np.linspace(-1, 2, 6)
    verts = [[(i, 0), (i + 1, 0), (i + 1, 1)] for i in range(6)]

    class Squared(colors.Normalize):
        def __call__(self, value, clip=None):
            return super().__call__(value, clip) ** 2

    def polygons(**kwargs):
        fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
        polys = collections.PolyCollection(verts, **kwargs)
        ax.add_collection(polys)
        ax.set_xlim(0, 7)
        ax.set_ylim(0, 1)
        return fig, polys

    out = {}
    fig, polys = polygons(colorizer=colorizer.Colorizer("BiOrangeBlue"))
    polys.set_array((a, b))
    out["bivariate"] = (_drawn(fig), float(np.round(polys.get_facecolors().sum(), 4)))
    fig, polys = polygons(array=(a, b, a), cmap="3VarAddA")
    out["multivariate"] = (_drawn(fig), type(polys.norm).__name__)
    fig, polys = polygons(colorizer=colorizer.Colorizer(
        "BiOrangeBlue", norm=[Squared(), "log"]))
    polys.set_array((a, b + 2))
    out["program_norm"] = _drawn(fig)
    x = np.linspace(0, 1, 12)
    fields = np.empty((12, 12), dtype=[('a', float), ('b', float)])
    fields['a'] = np.outer(np.sin(3 * x), np.cos(2 * x))
    fields['b'] = np.outer(x, x) * 4 - 1
    fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
    mesh = ax.pcolormesh(fields, cmap="BiOrangeBlue")
    out["mesh"] = (_drawn(fig)[0], type(mesh.norm).__name__, mesh.get_array().dtype.names)
    fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
    ax.pcolormesh(fields, cmap="BiOrangeBlue", norm=["linear", "log"])
    out["mesh_norms"] = _drawn(fig)[0]
    return out


def test_multivariate():
    assert scenario_multivariate() == {
        "bivariate": ((121235, 2107), 15.0),
        "multivariate": ((189003, 2094), "MultiNorm"),
        "program_norm": (94225, 2112),
        "mesh": (954868, "MultiNorm", ("a", "b")),
        "mesh_norms": 179889,
    }


def test_multivariate_colormap_refused_by_scalar_methods():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="does not support 1 variates"):
        ax.scatter([1, 2], [1, 2], c=[1, 2], cmap="BiOrangeBlue")


def scenario_projection_without_trans_axes():
    """A projection class whose ``transData`` ends with a box transform of
    its own rather than ``transAxes``, and whose tick labels are moved by
    a scaling."""

    class OwnAxes(Axes):
        name = "own_box"

        def _set_lim_and_transforms(self):
            super()._set_lim_and_transforms()
            self.transData = self.transScale + (
                self.transLimits + mtransforms.BboxTransformTo(self.bbox))
            self._xaxis_transform = mtransforms.blended_transform_factory(
                self.transData, self.transAxes)
            self._yaxis_transform = mtransforms.blended_transform_factory(
                self.transAxes, self.transData)

        def get_xaxis_text1_transform(self, pad_points):
            return (self.get_xaxis_transform(which='tick1')
                    + mtransforms.Affine2D().scale(1.0, 0.9), "top", "center")

    register_projection(OwnAxes)
    fig = plt.figure(figsize=(3, 3), dpi=50)
    ax = fig.add_subplot(projection="own_box")
    ax.plot([0, 1, 2], [0, 2, 1])
    drawn = _drawn(fig)
    return drawn[0], [round(float(v), 2) for v in ax.transData.transform((1, 1))]


def test_projection_without_trans_axes():
    assert scenario_projection_without_trans_axes() == (782047, [76.88, 74.25])
