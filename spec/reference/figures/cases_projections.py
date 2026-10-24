"""Projections the program registers, wherever axes are made: subplot
grids (``subplot_kw``), mosaics (``subplot_kw`` and ``per_subplot_kw``),
insets and twins, with the keywords that reach the projection's
``__init__``.

``RangeFrameAxes`` (``range_frame``) draws rectilinear axes whose data
keeps ``pad`` (a fraction of the axes, given when the axes is made) off
every edge, so that spines, ticks and gridlines span the data alone. It
also pans and zooms in its own way, which ``probes/test_projections.py``
drives: the right button of the pan/zoom tool changes ``pad`` instead of
zooming, and the zoom-to-rectangle tool zooms into a box along x alone.
"""

import numpy as np
from matplotlib.axes import Axes
from matplotlib.backend_bases import MouseButton
from matplotlib.projections import register_projection
from matplotlib.transforms import Affine2D, blended_transform_factory

from spec_support import case

G = "projections"

#: How many display pixels a drag with the right button moves to change
#: ``pad`` by one.
PAD_PIXELS = 1000.0

#: The largest ``pad`` a drag reaches.
MAX_PAD = 0.4


class RangeFrameAxes(Axes):
    """Rectilinear axes whose data keeps ``pad`` off every edge."""

    name = "range_frame"

    def __init__(self, *args, pad=0.05, **kwargs):
        self.pad = pad
        self._inset = Affine2D().scale(1 - 2 * pad).translate(pad, pad)
        self._pad_pan = None
        super().__init__(*args, **kwargs)

    def set_pad(self, pad):
        self.pad = pad
        self._inset.clear().scale(1 - 2 * pad).translate(pad, pad)
        self.stale = True

    def _set_lim_and_transforms(self):
        super()._set_lim_and_transforms()
        self.transData = self.transScale + self.transLimits + self._inset + self.transAxes
        self._xaxis_transform = blended_transform_factory(self.transData, self.transAxes)
        self._yaxis_transform = blended_transform_factory(self.transAxes, self.transData)

    def start_pan(self, x, y, button):
        if button == MouseButton.RIGHT:
            self._pad_pan = (self.pad, x)
        else:
            super().start_pan(x, y, button)

    def drag_pan(self, button, key, x, y):
        if self._pad_pan is None:
            super().drag_pan(button, key, x, y)
            return
        pad, start = self._pad_pan
        self.set_pad(min(max(pad + (x - start) / PAD_PIXELS, 0.0), MAX_PAD))

    def end_pan(self):
        if self._pad_pan is None:
            super().end_pan()
        self._pad_pan = None

    def _set_view_from_bbox(self, bbox, direction='in', mode=None, twinx=False, twiny=False):
        if len(bbox) != 4 or direction != 'in':
            super()._set_view_from_bbox(bbox, direction, mode, twinx, twiny)
            return
        if twinx or mode == 'y':
            return
        start_x, start_y, stop_x, stop_y = bbox
        (a, _), (b, _) = self.transData.inverted().transform(
            [(start_x, start_y), (stop_x, stop_y)])
        self.set_xlim(min(a, b), max(a, b))


register_projection(RangeFrameAxes)


@case(f"{G}/grid")
def _(fig):
    axs = fig.subplots(2, 2, sharex=True, sharey=True,
                       subplot_kw={"projection": "range_frame", "pad": 0.08})
    x = np.linspace(0, 2 * np.pi, 25)
    for i, ax in enumerate(axs.flat):
        ax.plot(x, np.sin(x + i), "o-", ms=3)
        ax.set_title(f"phase {i}")
        ax.grid(True)


@case(f"{G}/geo_grid")
def _(fig):
    axs = fig.subplots(1, 2, subplot_kw={"projection": "hammer"})
    lon = np.linspace(-np.pi, np.pi, 30)
    for i, ax in enumerate(axs.flat):
        ax.plot(lon, 0.5 * np.sin(lon + i), color=f"C{i}")
        ax.grid(True)


@case(f"{G}/mosaic")
def _(fig):
    axd = fig.subplot_mosaic(
        "AB;CC",
        subplot_kw={"projection": "range_frame"},
        per_subplot_kw={"B": {"projection": "range_frame", "pad": 0.2},
                        "C": {"projection": "hammer"}})
    x = np.linspace(-np.pi, np.pi, 30)
    axd["A"].plot(x, np.cos(x))
    axd["A"].set_title("pad 0.05")
    axd["B"].scatter(x, np.sin(x), s=8, color="C1")
    axd["B"].set_title("pad 0.2")
    axd["C"].plot(x, 0.5 * np.sin(2 * x), color="C2")
    axd["C"].grid(True)


@case(f"{G}/insets")
def _(fig):
    ax = fig.add_subplot(projection="range_frame", pad=0.1)
    x = np.linspace(0, 10, 60)
    ax.plot(x, np.sin(x))
    inset = ax.inset_axes([0.55, 0.6, 0.4, 0.35], projection="range_frame", pad=0.15)
    inset.plot(x, np.cos(x), color="C1")
    inset.set_title("pad 0.15", fontsize=8)
    polar = ax.inset_axes([0.05, 0.05, 0.3, 0.3], projection="polar")
    polar.plot(x, x / 10, color="C2")
    polar.set_rticks([0.5, 1])


@case(f"{G}/twins")
def _(fig):
    ax = fig.add_subplot(projection="range_frame", pad=0.1)
    x = np.linspace(0, 10, 40)
    ax.plot(x, x ** 2, color="C0")
    ax.set_ylabel("x squared")
    twin = ax.twinx(projection="range_frame", pad=0.1)
    twin.plot(x, np.sqrt(x), color="C1")
    twin.set_ylabel("square root")
    top = ax.secondary_xaxis("top", functions=(lambda v: 2 * v, lambda v: v / 2))
    top.set_xlabel("twice x")
