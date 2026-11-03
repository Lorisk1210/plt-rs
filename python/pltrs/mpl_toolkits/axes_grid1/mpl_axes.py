"""
Axes whose sides are reached through ``ax.axis["left"]`` (matplotlib's
``mpl_toolkits.axes_grid1.mpl_axes``, from matplotlib 3.11.2, under the
Matplotlib License): a `SimpleAxisArtist` acts on the spine of a side and
on what the x or y axis draws there, through pltrs (Rust)'s
``SimpleAxisArtist`` (``_native.axes_grid1``).
"""

import pltrs.axes as maxes
from pltrs.artist import Artist
from pltrs.axis import XAxis, YAxis
from pltrs._native import axes_grid1 as _axes_grid1


class SimpleChainedObjects:
    def __init__(self, objects):
        self._objects = objects

    def __getattr__(self, k):
        _a = SimpleChainedObjects([getattr(a, k) for a in self._objects])
        return _a

    def __call__(self, *args, **kwargs):
        for m in self._objects:
            m(*args, **kwargs)


class Axes(maxes.Axes):

    class AxisDict(dict):
        def __init__(self, axes):
            self.axes = axes
            super().__init__()

        def __getitem__(self, k):
            if isinstance(k, tuple):
                r = SimpleChainedObjects(
                    # super() within a list comprehension needs explicit args.
                    [super(Axes.AxisDict, self).__getitem__(k1) for k1 in k])
                return r
            elif isinstance(k, slice):
                if k.start is None and k.stop is None and k.step is None:
                    return SimpleChainedObjects(list(self.values()))
                else:
                    raise ValueError("Unsupported slice")
            else:
                return dict.__getitem__(self, k)

        def __call__(self, *v, **kwargs):
            return maxes.Axes.axis(self.axes, *v, **kwargs)

    @classmethod
    def _make_native(cls, figure, kwargs, *, spec=None, rect=None):
        return cls._constructed_native(figure, kwargs, spec=spec, rect=rect)

    @property
    def axis(self):
        return self._axislines

    def _init_axes_state(self):
        super()._init_axes_state()
        self._init_axislines()

    def clear(self):
        # docstring inherited
        super().clear()
        self._init_axislines()

    def _init_axislines(self):
        """Init axis artists."""
        self._axislines = self.AxisDict(self)
        self._axislines.update(
            bottom=SimpleAxisArtist(self.xaxis, 1, self.spines["bottom"]),
            top=SimpleAxisArtist(self.xaxis, 2, self.spines["top"]),
            left=SimpleAxisArtist(self.yaxis, 1, self.spines["left"]),
            right=SimpleAxisArtist(self.yaxis, 2, self.spines["right"]))


class SimpleAxisArtist(Artist):
    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    def __init__(self, axis, axisnum, spine):
        self._axis = axis
        self._axisnum = axisnum
        self.line = spine

        if isinstance(axis, XAxis):
            self._axis_direction = ["bottom", "top"][axisnum-1]
        elif isinstance(axis, YAxis):
            self._axis_direction = ["left", "right"][axisnum-1]
        else:
            raise ValueError(
                f"axis must be instance of XAxis or YAxis, but got {axis}")
        super().__init__()

    @property
    def major_ticks(self):
        tickline = "tick%dline" % self._axisnum
        return SimpleChainedObjects([getattr(tick, tickline)
                                     for tick in self._axis.get_major_ticks()])

    @property
    def major_ticklabels(self):
        label = "label%d" % self._axisnum
        return SimpleChainedObjects([getattr(tick, label)
                                     for tick in self._axis.get_major_ticks()])

    @property
    def label(self):
        return self._axis.label

    def set_visible(self, b):
        _axes_grid1.simple_axis_set_visible(self._axis.axes._native,
                                            self._axis_direction, bool(b))
        super().set_visible(b)

    def set_label(self, txt):
        _axes_grid1.simple_axis_set_label(self._axis.axes._native,
                                          self._axis_direction, str(txt))

    def toggle(self, all=None, ticks=None, ticklabels=None, label=None):
        _axes_grid1.simple_axis_toggle(
            self._axis.axes._native, self._axis_direction,
            tuple(None if value is None else bool(value)
                  for value in (all, ticks, ticklabels, label)))
