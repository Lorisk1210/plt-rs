"""
Axes made by their constructor (matplotlib's ``Axes(fig, rect)``) and
placed by axes locators (``set_axes_locator``), as a mixin of `_AxesBase`.

An Axes made by its constructor is made by pltrs (Rust) in its figure but
outside it, as a removed axes, since matplotlib's constructors leave the
adding to ``fig.add_axes(ax)`` or ``fig.add_subplot(ax)``. Toolkit Axes
classes (``mpl_toolkits``) also come through their constructor when
``fig.add_subplot(axes_class=...)`` makes them (`_constructed_native`).

The Python state of an Axes object around its native handle (its `.XAxis`
and `.YAxis` objects, ...) is set up by ``_init_axes_state``, both for an
Axes made by its constructor and for one pltrs (Rust) made (`_from_native`).
"""

from collections.abc import Iterable

import numpy as np

from .. import _api, _handles
from .._native import figure as _figure, insets as _insets
from ..transforms import Bbox


class ConstructionMixin:
    """`Axes(fig, rect)` and the axes locators of an Axes."""

    def __init__(self, fig, *args, facecolor=None, frameon=True,
                 sharex=None, sharey=None, label='', xscale=None,
                 yscale=None, box_aspect=None,
                 forward_navigation_events="auto", **kwargs):
        """
        Build an Axes in a figure.

        Parameters
        ----------
        fig : `~matplotlib.figure.Figure`
            The Axes is built in the `.Figure` *fig*.
        *args
            ``*args`` can be a single ``(left, bottom, width, height)``
            rectangle or a single `.Bbox`.  This specifies the rectangle (in
            figure coordinates) where the Axes is positioned.

            ``*args`` can also consist of three numbers or a single
            three-digit number; in the latter case, the digits are
            considered as independent numbers.  The numbers are interpreted
            as ``(nrows, ncols, index)``: ``(nrows, ncols)`` specifies the
            size of an array of subplots, and ``index`` is the 1-based index
            of the subplot being created.  Finally, ``*args`` can also
            directly be a `.SubplotSpec` instance.
        sharex, sharey : `~matplotlib.axes.Axes`, optional
            The x- or y-`~.matplotlib.axis` is shared with the x- or y-axis
            in the input `~.axes.Axes`.
        frameon : bool, default: True
            Whether the Axes frame is visible.
        box_aspect : float, optional
            Set a fixed aspect for the Axes box, i.e. the ratio of height to
            width. See `~.axes.Axes.set_box_aspect` for details.
        forward_navigation_events : bool or "auto", default: "auto"
            Control whether pan/zoom events are passed through to Axes below
            this one. "auto" is *True* for axes with an invisible patch and
            *False* otherwise.
        **kwargs
            Other optional keyword arguments are Axes properties.

        Returns
        -------
        `~.axes.Axes`
            The new `~.axes.Axes` object.
        """
        if 'rect' in kwargs:
            if args:
                raise TypeError(
                    "'rect' cannot be used together with positional arguments")
            rect = kwargs.pop('rect')
            _api.check_isinstance((Bbox, Iterable), rect=rect)
            args = (rect,)
        _handles.adopt(self, _removed_native(fig, args), fig.figure)
        self._init_axes_state()
        self._init_projection()
        # As matplotlib's `__init__`, which subclasses' `clear` (or `cla`)
        # overrides take part in.
        self.clear()
        for name, other in (("x", sharex), ("y", sharey)):
            if other is not None:
                getattr(self, f"share{name}")(other)
        self.set_label(label)
        if facecolor is not None:
            self.set_facecolor(facecolor)
        if not frameon:
            self.set_frame_on(frameon)
        if box_aspect is not None:
            self.set_box_aspect(box_aspect)
        if forward_navigation_events != "auto":
            self.set_forward_navigation_events(forward_navigation_events)
        if xscale:
            self.set_xscale(xscale)
        if yscale:
            self.set_yscale(yscale)
        self._internal_update(kwargs)

    @classmethod
    def _constructed_native(cls, figure, kwargs, *, spec=None, rect=None):
        """
        The native axes of a new *cls* made by its constructor, with
        *kwargs*, at the native grid block *spec* or at *rect*, and added to
        *figure*; return it and the keywords left (none). Toolkit classes
        use this as their ``_make_native``, through which
        ``fig.add_subplot(axes_class=...)`` and ``fig.add_axes`` make them.
        """
        from ..gridspec import SubplotSpec
        where = (rect if rect is not None
                 else SubplotSpec._from_native(spec, figure))
        ax = cls(figure, where, **kwargs)
        ax._add_to_figure(figure, False)
        return ax._native, {}

    def _init_axes_state(self):
        """Set up the Python state of the Axes around its native handle."""

    def get_window_extent(self, renderer=None):
        """
        Return the Axes bounding box in display space.

        This bounding box does not include the spines, ticks, ticklabels,
        or other labels.  For a bounding box including these elements use
        `~matplotlib.axes.Axes.get_tightbbox`.
        """
        return Bbox(self.bbox.get_points())

    def set_axes_locator(self, locator):
        """
        Set the Axes locator.

        Parameters
        ----------
        locator : Callable[[Axes, Renderer], Bbox]
        """
        from . import _program_locators
        _program_locators.forget(self, self._figure)
        _insets.clear_axes_locator(self._native)
        self._axes_locator = locator
        if locator is not None:
            place = getattr(locator, "_place", None)
            if place is None:
                _program_locators.follow(self, self._figure)
            else:
                place(self)

    def _locate_by_program(self, renderer):
        """Place the Axes where its locator of the program's says now
        (see `._program_locators`)."""
        bbox = self._axes_locator(self, renderer)
        _insets.set_located_position(
            self._native, None if bbox is None else tuple(map(float, bbox.bounds)))

    def get_axes_locator(self):
        """Return the axes_locator."""
        return self.__dict__.get("_axes_locator")


def _removed_native(fig, args):
    """The native axes `Axes(fig, *args)` makes: at a rectangle or on a
    subplot block, outside the figure."""
    if len(args) == 1 and (isinstance(args[0], Bbox) or np.iterable(args[0])):
        rect, = args
        bounds = (rect.bounds if isinstance(rect, Bbox)
                  else Bbox.from_bounds(*rect).bounds)
        if bounds[2] < 0 or bounds[3] < 0:
            raise ValueError('Width and height specified must be non-negative')
        return _figure.removed_axes(fig._native, tuple(map(float, bounds)))
    from ..gridspec import SubplotSpec
    spec = SubplotSpec._from_subplot_args(fig, args)
    return _figure.removed_subplot(fig._native, spec._native_spec())
