"""
The `Axes` class: plotting methods on a pltrs (Rust) axes (matplotlib's
``axes._axes`` and ``axes._base``).

Each plotting method turns matplotlib's signature (positional forms,
aliases, defaults) into the explicit arguments of a native function of
``_native.axes``, and wraps the artists it returns (see `.._handles`).
"""

import numpy as np

from .. import _handles, cbook
from .._native import axes as _axes
from ..artist import Artist
from ..axis import XAxis, YAxis
from ..collections import PathCollection
from ..container import BarContainer
from ..legend import Legend
from ..lines import Line2D
from ..patches import Rectangle
from ..text import Text
from ._imaging import ImagingMixin


def _floats(values):
    """*values* as a float array (scalars as one value)."""
    return np.atleast_1d(np.asarray(values, dtype=float))


def _plot_groups(args):
    """
    Split ``plot``'s positional arguments into ``(x, y, fmt)`` groups,
    as matplotlib's ``_process_plot_var_args`` does: each group is ``[x],
    y, [fmt]``.
    """
    args = list(args)
    while args:
        this, args = args[:2], args[2:]
        if args and isinstance(args[0], str):
            this += [args[0]]
            args = args[1:]
        if len(this) == 2 and isinstance(this[1], str):
            this = [None, this[0], this[1]]
        elif len(this) == 1:
            this = [None, this[0], None]
        elif len(this) == 2:
            this = [this[0], this[1], None]
        x, y, fmt = this
        y = np.asarray(y, dtype=float)
        x = np.arange(len(y), dtype=float) if x is None else np.asarray(x, dtype=float)
        yield x, y, fmt


class Axes(ImagingMixin, Artist):
    """
    An Axes object encapsulates all the elements of an individual (sub-)plot
    in a figure.

    See matplotlib's documentation of `matplotlib.axes.Axes`.
    """

    #: The name of the projection, under which `.projections` registers
    #: the class.
    name = "rectilinear"

    @classmethod
    def _from_native(cls, native, figure):
        ax = super()._from_native(native, figure)
        ax.xaxis = XAxis(ax)
        ax.yaxis = YAxis(ax)
        ax.child_axes = []
        return ax

    def plot(self, *args, scalex=True, scaley=True, data=None, **kwargs):
        """Plot y versus x as lines and/or markers."""
        if data is not None:
            raise NotImplementedError("pltrs: plot(data=...)")
        kwargs = cbook.normalize_kwargs(kwargs, Line2D)
        lines = []
        for x, y, fmt in _plot_groups(args):
            if x.shape[0] != y.shape[0]:
                raise ValueError(f"x and y must have same first dimension, but "
                                 f"have shapes {x.shape} and {y.shape}")
            columns = y.reshape(len(y), -1).T if y.ndim > 1 else [y]
            for column in columns:
                native = _axes.plot(self._native, x, column, fmt, kwargs)
                lines.append(_handles.wrap(native, Line2D, self.figure))
        return lines

    def scatter(self, x, y, s=None, c=None, *, marker=None, cmap=None,
                norm=None, vmin=None, vmax=None, alpha=None, linewidths=None,
                edgecolors=None, colorizer=None, plotnonfinite=False,
                data=None, **kwargs):
        """A scatter plot of *y* vs. *x* with varying marker size and/or color."""
        unsupported = {"marker": marker, "cmap": cmap, "norm": norm,
                       "vmin": vmin, "vmax": vmax, "alpha": alpha,
                       "linewidths": linewidths, "edgecolors": edgecolors,
                       "colorizer": colorizer, "data": data}
        for name, value in unsupported.items():
            if value is not None:
                raise NotImplementedError(f"pltrs: scatter({name}=...)")
        color = kwargs.pop("color", None)
        if c is not None and color is not None:
            raise ValueError("Supply a 'c' argument or a 'color' kwarg but not "
                             "both; they differ but their functionalities "
                             "overlap.")
        label = kwargs.pop("label", None)
        if kwargs:
            raise NotImplementedError(f"pltrs: scatter({next(iter(kwargs))}=...)")
        x, y = _floats(x), _floats(y)
        sizes = None if s is None else _floats(s)
        native = _axes.scatter(self._native, x, y, sizes,
                               c if c is not None else color,
                               None if label is None else str(label))
        return _handles.wrap(native, PathCollection, self.figure)

    def bar(self, x, height, width=0.8, bottom=None, *, align="center",
            data=None, **kwargs):
        """Make a bar plot."""
        if align != "center" or data is not None:
            raise NotImplementedError("pltrs: bar(align=..., data=...)")
        color = kwargs.pop("color", None)
        label = kwargs.pop("label", None)
        if kwargs:
            raise NotImplementedError(f"pltrs: bar({next(iter(kwargs))}=...)")
        x, height = _floats(x), _floats(height)
        x, height = np.broadcast_arrays(x, height)
        width = np.broadcast_to(_floats(width), x.shape)
        bottom = None if bottom is None else np.broadcast_to(_floats(bottom), x.shape)
        patches, values = _axes.bar(self._native, x, height, width, bottom,
                                    color, None if label is None else str(label))
        rectangles = _handles.wrap_all(patches, Rectangle, self.figure)
        return BarContainer(rectangles, datavalues=np.asarray(values),
                            orientation="vertical", label=label)

    def set_title(self, label, fontdict=None, loc=None, pad=None, *, y=None,
                  **kwargs):
        """Set a title for the Axes."""
        if loc not in (None, "center") or pad is not None or y is not None:
            raise NotImplementedError("pltrs: set_title(loc=..., pad=..., y=...)")
        title = _handles.wrap(_axes.set_title(self._native, str(label)),
                              Text, self.figure)
        title._internal_update({**(fontdict or {}), **kwargs})
        return title

    def get_title(self, loc="center"):
        """Get an Axes title."""
        return _axes.get_title(self._native, loc)

    def set_xlabel(self, xlabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """Set the label for the x-axis."""
        _axes.set_xlabel(self._native, str(xlabel))

    def get_xlabel(self):
        """Get the xlabel text string."""
        return _axes.get_xlabel(self._native)

    def set_ylabel(self, ylabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """Set the label for the y-axis."""
        _axes.set_ylabel(self._native, str(ylabel))

    def get_ylabel(self):
        """Get the ylabel text string."""
        return _axes.get_ylabel(self._native)

    @staticmethod
    def _check_no_units(vals, names):
        # Helper method to check that vals are not unitized
        from .. import units
        for val, name in zip(vals, names):
            if not units._is_natively_supported(val):
                raise ValueError(f"{name} must be a single scalar value, "
                                 f"but got {val}")

    def axhline(self, y=0, xmin=0, xmax=1, **kwargs):
        """
        Add a horizontal line spanning the whole or fraction of the Axes.

        Parameters
        ----------
        y : float, default: 0
            y position in :ref:`data coordinates <coordinate-systems>`.
        xmin : float, default: 0
            The start x-position in :ref:`axes coordinates <coordinate-systems>`.
            Should be between 0 and 1, 0 being the far left of the plot,
            1 the far right of the plot.
        xmax : float, default: 1
            The end x-position in :ref:`axes coordinates <coordinate-systems>`.
            Should be between 0 and 1, 0 being the far left of the plot,
            1 the far right of the plot.

        Returns
        -------
        `~matplotlib.lines.Line2D`
        """
        self._check_no_units([xmin, xmax], ['xmin', 'xmax'])
        if "transform" in kwargs:
            raise ValueError("'transform' is not allowed as a keyword "
                             "argument; axhline generates its own transform.")
        native = _axes.axline(self._native, "h", y, (xmin, xmax),
                              cbook.normalize_kwargs(kwargs, Line2D))
        return _handles.wrap(native, Line2D, self.figure)

    def axvline(self, x=0, ymin=0, ymax=1, **kwargs):
        """
        Add a vertical line spanning the whole or fraction of the Axes.

        Parameters
        ----------
        x : float, default: 0
            x position in :ref:`data coordinates <coordinate-systems>`.
        ymin : float, default: 0
            The start y-position in :ref:`axes coordinates <coordinate-systems>`.
            Should be between 0 and 1, 0 being the bottom of the plot, 1 the
            top of the plot.
        ymax : float, default: 1
            The end y-position in :ref:`axes coordinates <coordinate-systems>`.
            Should be between 0 and 1, 0 being the bottom of the plot, 1 the
            top of the plot.

        Returns
        -------
        `~matplotlib.lines.Line2D`
        """
        self._check_no_units([ymin, ymax], ['ymin', 'ymax'])
        if "transform" in kwargs:
            raise ValueError("'transform' is not allowed as a keyword "
                             "argument; axvline generates its own transform.")
        native = _axes.axline(self._native, "v", x, (ymin, ymax),
                              cbook.normalize_kwargs(kwargs, Line2D))
        return _handles.wrap(native, Line2D, self.figure)

    def axhspan(self, ymin, ymax, xmin=0, xmax=1, **kwargs):
        """
        Add a horizontal span (rectangle) across the Axes.

        The rectangle spans from *ymin* to *ymax* vertically, and, by default,
        the whole x-axis horizontally.

        Returns
        -------
        `~matplotlib.patches.Rectangle`
            Horizontal span (rectangle) from (xmin, ymin) to (xmax, ymax).
        """
        self._check_no_units([xmin, xmax], ['xmin', 'xmax'])
        native = _axes.axspan(self._native, "h", (ymin, ymax), (xmin, xmax))
        patch = _handles.wrap(native, Rectangle, self.figure)
        patch._internal_update(kwargs)
        return patch

    def axvspan(self, xmin, xmax, ymin=0, ymax=1, **kwargs):
        """
        Add a vertical span (rectangle) across the Axes.

        The rectangle spans from *xmin* to *xmax* horizontally, and, by
        default, the whole y-axis vertically.

        Returns
        -------
        `~matplotlib.patches.Rectangle`
            Vertical span (rectangle) from (xmin, ymin) to (xmax, ymax).
        """
        self._check_no_units([ymin, ymax], ['ymin', 'ymax'])
        native = _axes.axspan(self._native, "v", (xmin, xmax), (ymin, ymax))
        patch = _handles.wrap(native, Rectangle, self.figure)
        patch._internal_update(kwargs)
        return patch

    def _display_to_data(self, xy):
        """The data coordinates of the display point *xy*
        (``self.transData.inverted().transform(xy)``)."""
        x, y = xy
        return _axes.display_to_data(self._native, x, y)

    def get_xlim(self):
        """Return the x-axis view limits."""
        return _axes.get_xlim(self._native)

    def set_xlim(self, left=None, right=None):
        """Set the x-axis view limits."""
        if right is None and np.iterable(left):
            left, right = left
        current = self.get_xlim()
        left = current[0] if left is None else left
        right = current[1] if right is None else right
        _axes.set_xlim(self._native, float(left), float(right))
        return left, right

    def get_ylim(self):
        """Return the y-axis view limits."""
        return _axes.get_ylim(self._native)

    def set_ylim(self, bottom=None, top=None):
        """Set the y-axis view limits."""
        if top is None and np.iterable(bottom):
            bottom, top = bottom
        current = self.get_ylim()
        bottom = current[0] if bottom is None else bottom
        top = current[1] if top is None else top
        _axes.set_ylim(self._native, float(bottom), float(top))
        return bottom, top

    @property
    def lines(self):
        """The lines of the Axes."""
        return _handles.wrap_all(_axes.get_lines(self._native), Line2D, self.figure)

    def legend(self, *args, **kwargs):
        """Place a legend on the Axes."""
        if args or kwargs:
            raise NotImplementedError("pltrs: legend() with arguments")
        return _handles.wrap(_axes.legend(self._native), Legend, self.figure)

    def get_legend(self):
        """Return the `.Legend` instance, or None if no legend is defined."""
        return _handles.wrap(_axes.get_legend(self._native), Legend, self.figure)
