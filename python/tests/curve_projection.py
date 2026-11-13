"""
A projection with an axis of its own, defined without a library: a
rectilinear `~.axes.Axes` with a third `.Axis` along a parabola, under a
name of its own, with tick, spine and transform classes of its own (as
mpltern's ternary Axes has three such axes). What the smoke tests of
programs' projections check held axes with (``test_proj1.py``,
``test_proj2.py``); it runs under matplotlib 3.11 as it runs here.
"""

import numpy as np

import pltrs.pyplot as plt
import pltrs.transforms as mtransforms
from pltrs import cbook
from pltrs.axes import Axes
from pltrs.axis import XAxis, XTick
from pltrs.path import Path
from pltrs.projections import register_projection
from pltrs.spines import Spine

#: The calls of the projection's own methods, in order.
CALLS = []


class CurveTransform(mtransforms.Transform):
    """``(s, p)`` to Axes fractions: *s* along a parabola, *p* across it."""

    input_dims = output_dims = 2

    def transform_non_affine(self, values):
        values = np.asarray(values, float)
        s, p = values[:, 0], values[:, 1]
        return np.column_stack([0.1 + 0.8 * s, 0.2 + 0.6 * s ** 2 + (p - 0.5) * 0.1])

    def inverted(self):
        return InvertedCurveTransform()


class InvertedCurveTransform(mtransforms.Transform):
    input_dims = output_dims = 2

    def transform_non_affine(self, values):
        values = np.asarray(values, float)
        s = (values[:, 0] - 0.1) / 0.8
        return np.column_stack([s, (values[:, 1] - 0.2 - 0.6 * s ** 2) / 0.1 + 0.5])

    def inverted(self):
        return CurveTransform()


class CurveLabelShift(mtransforms.Affine2DBase):
    """A shift of *pad_points* across the chord of the curve, computed
    from where the Axes is when it is asked (as mpltern's shift of a tick
    label away from its corner is)."""

    def __init__(self, axes, pad_points, sign):
        super().__init__()
        self.axes, self.pad_points, self.sign = axes, pad_points, sign

    def get_matrix(self):
        start, end = self.axes._caxis_transform.transform([(0, 0.5), (1, 0.5)])
        along = (end - start) / np.linalg.norm(end - start)
        dpi = self.axes.get_figure(root=True).dpi
        x, y = self.sign * np.array([-along[1], along[0]]) * self.pad_points / 72 * dpi
        return np.array([[1.0, 0.0, x], [0.0, 1.0, y], [0.0, 0.0, 1.0]])


class CurveTick(XTick):
    """A tick of the curve axis: its lines through the axis' transform,
    its mark and label turned along the curve."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        transform = self.axes.get_caxis_transform()
        for line in (self.tick1line, self.tick2line, self.gridline):
            line.set_transform(transform)

    def _get_text1_transform(self):
        return self.axes.get_caxis_text1_transform(self._pad)

    def _get_text2_transform(self):
        return self.axes.get_caxis_text2_transform(self._pad)

    def update_position(self, loc):
        CALLS.append(("update_position", float(loc)))
        super().update_position(loc)
        ahead = self.axes._caxis_transform.transform([(loc, 0.5), (loc + 1e-3, 0.5)])
        angle = np.arctan2(*(ahead[1] - ahead[0])[::-1])
        self.tick1line._marker._transform = mtransforms.Affine2D().rotate(angle)
        self.label1.set_rotation(np.rad2deg(angle))
        self.label1.set_rotation_mode("anchor")


class CurveAxis(XAxis):
    """The axis along the curve, with limits of its own."""

    axis_name = "c"

    def _init(self):
        super()._init()
        self.label.set_transform(self.axes.transAxes)
        self.label_position = "curve"

    def _get_tick(self, major):
        tick_kw = self._major_tick_kw if major else self._minor_tick_kw
        return CurveTick(self.axes, 0, major=major, **tick_kw)

    def _copy_tick_props(self, src, dest):
        super()._copy_tick_props(src, dest)
        CALLS.append(("_copy_tick_props",))

    def get_view_interval(self):
        return self.axes.viewCLim.intervalx

    def set_view_interval(self, vmin, vmax, ignore=False):
        self.axes.viewCLim.intervalx = vmin, vmax

    def get_data_interval(self):
        return self.axes.viewCLim.intervalx

    def get_minpos(self):
        return 1e-300

    def get_tick_space(self):
        return 5

    def _update_label_position(self, renderer):
        CALLS.append(("_update_label_position",))
        boxes, _ = self._get_ticklabel_bboxes(self._update_ticks(), renderer)
        top = max(box.y1 for box in boxes)
        _, y = self.axes.transAxes.inverted().transform((0, top))
        self.label.set_position((0.3, y + 0.05))

    def _update_offset_text_position(self, bboxes, bboxes2):
        CALLS.append(("_update_offset_text_position", len(bboxes), len(bboxes2)))
        self.offsetText.set_position((1, self.axes.bbox.ymax))


class CurveSpine(Spine):
    """A spine along the curve, on one side of it, over the axis' limits."""

    def __init__(self, axes, spine_type, path, **kwargs):
        super().__init__(axes, spine_type, path, **kwargs)
        self.set_transform(axes.get_caxis_transform())

    def _adjust_location(self):
        CALLS.append(("_adjust_location", self.spine_type))
        self._path.vertices[:, 0] = self.axes.viewCLim.intervalx
        self._path.vertices[:, 1] = 0.0 if self.spine_type == "clow" else 1.0

    def get_spine_transform(self):
        return self.get_transform()


class CurveAxes(Axes):
    """A rectilinear Axes with a third axis along a parabola."""

    name = "proj1_curve"
    _axis_names = ("x", "y", "c")
    _shared_axes = {name: cbook.Grouper() for name in _axis_names}

    def __init__(self, *args, **kwargs):
        # matplotlib's clear reads the Axes each axis is shared with.
        self._sharec = None
        super().__init__(*args, **kwargs)

    @property
    def callbacks(self):
        # matplotlib's registry knows the x, y and z limits only.
        if "_curve_callbacks" not in self.__dict__:
            self._curve_callbacks = cbook.CallbackRegistry(
                signals=[f"{name}lim_changed" for name in self._axis_names])
        return self._curve_callbacks

    @callbacks.setter
    def callbacks(self, registry):
        # A clear starts over.
        self.__dict__.pop("_curve_callbacks", None)

    def set_figure(self, fig):
        self.viewCLim = mtransforms.Bbox.unit()
        super().set_figure(fig)

    def _set_lim_and_transforms(self):
        super()._set_lim_and_transforms()
        limits = mtransforms.BboxTransformFrom(
            mtransforms.TransformedBbox(self.viewCLim, self.transScale))
        self._caxis_transform = limits + CurveTransform() + self.transAxes

    def _init_axis(self):
        super()._init_axis()
        self.caxis = CurveAxis(self)
        self.spines["clow"].register_axis(self.caxis)

    def _gen_axes_spines(self, locations=None, offset=0.0, units="inches"):
        spines = super()._gen_axes_spines()
        for name in ("clow", "chigh"):
            path = Path([(0.0, 0.0), (1.0, 0.0)], _interpolation_steps=16)
            spines[name] = CurveSpine(self, name, path)
        return spines

    def get_caxis_transform(self, which="grid"):
        return self._caxis_transform

    def get_caxis_text1_transform(self, pad):
        return self._caxis_transform + CurveLabelShift(self, pad, -1), "top", "center"

    def get_caxis_text2_transform(self, pad):
        return self._caxis_transform + CurveLabelShift(self, pad, +1), "bottom", "center"

    def clear(self):
        CALLS.append(("clear",))
        self.viewCLim.intervalx = 0.0, 1.0
        super().clear()

    def autoscale_view(self, tight=None, scalex=True, scaley=True, scalec=True):
        # matplotlib asks for every axis of ``_axis_names``.
        super().autoscale_view(tight=tight, scalex=scalex, scaley=scaley)

    def get_clim(self):
        return tuple(self.viewCLim.intervalx)

    def set_clim(self, low, high):
        return self.caxis._set_lim(low, high, auto=None)


register_projection(CurveAxes)


class LiftedTitleAxes(CurveAxes):
    """A `CurveAxes` placing its titles its own way: above the label of
    its third axis, which it measures for it (as mpltern's Axes lifts its
    titles above the tick labels of its three axes)."""

    name = "proj1_lifted_title"

    def _update_title_position(self, renderer):
        CALLS.append(("_update_title_position",))
        top = self.caxis.get_tightbbox(renderer).y1
        _, y = self.transAxes.inverted().transform((0, top))
        for title in (self.title, self._left_title, self._right_title):
            title.set_position((title.get_position()[0], y + 0.25))


register_projection(LiftedTitleAxes)


def curve_axes():
    """A figure of 500 by 400 pixels with one `CurveAxes`."""
    fig = plt.figure(figsize=(5, 4), dpi=100)
    return fig, fig.add_subplot(projection="proj1_curve")
