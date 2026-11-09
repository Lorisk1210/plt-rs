"""
Classes to support contour plotting and labelling for the Axes class
(matplotlib's ``contour``).

pltrs (Rust) makes the contour sets: a `ContourSet` wraps the native
contour set (``self._contour``) and, as the `.Collection` it is in
matplotlib, the collection drawing its contours (``self._native``).
"""

import functools
import math
from numbers import Integral

import numpy as np
from numpy import ma

from . import _docstring
from . import (_api, _blocking_input, _handles, _mirrors, _owned_paths, _values,
               artist as martist, cbook, collections as mcoll, colors as mcolors)
from ._native import artist as _artist, collections as _collections, contour as _contour
from .backend_bases import MouseButton
from .path import Path


def _contour_labeler_event_handler(cs, inline, inline_spacing, event):
    """matplotlib's handler of the events of ``clabel(manual=True)``."""
    canvas = cs.axes.get_figure(root=True).canvas
    is_button = event.name == "button_press_event"
    is_key = event.name == "key_press_event"
    # Quit (even if not in infinite mode; this is consistent with
    # MATLAB and sometimes quite useful, but will require the user to
    # test how many points were actually returned before using data).
    if (is_button and event.button == MouseButton.MIDDLE
            or is_key and event.key in ["escape", "enter"]):
        canvas.stop_event_loop()
    # Pop last click.
    elif (is_button and event.button == MouseButton.RIGHT
          or is_key and event.key in ["backspace", "delete"]):
        # Unfortunately, if one is doing inline labels, then there is currently
        # no way to fix the broken contour - once humpty-dumpty is broken, he
        # can't be put back together.  In inline mode, this does nothing.
        if not inline:
            cs.pop_label()
            canvas.draw()
    # Add new click.
    elif (is_button and event.button == MouseButton.LEFT
          # On macOS/gtk, some keys return None.
          or is_key and event.key is not None):
        if cs.axes.contains(event)[0]:
            cs.add_label_near(event.x, event.y, transform=False,
                              inline=inline, inline_spacing=inline_spacing)
            canvas.draw()


def _label_texts(fmt, levels):
    """
    The ``(level, text)`` pairs of the labels of *levels* written with
    *fmt*, a `.Formatter` (asked as matplotlib's ``get_text`` asks it, with
    every labelled level) or a ``%`` template object other than a string.
    """
    if callable(getattr(fmt, "format_ticks", None)):
        return [(float(lev), str(fmt.format_ticks([*levels, lev])[-1]))
                for lev in levels]
    return [(float(lev), str(fmt % lev)) for lev in levels]


class ContourLabeler:
    """Mixin to provide labelling capability to `.ContourSet`."""

    def clabel(self, levels=None, *,
               fontsize=None, inline=True, inline_spacing=5, fmt=None,
               colors=None, use_clabeltext=False, manual=False,
               rightside_up=True, zorder=None):
        """
        Label a contour plot.

        Adds labels to line contours in this `.ContourSet` (which inherits from
        this mixin class).

        Parameters
        ----------
        levels : array-like, optional
            A list of level values, that should be labeled. The list must be
            a subset of ``cs.levels``. If not given, all levels are labeled.

        fontsize : str or float, default: :rc:`font.size`
            Size in points or relative size e.g., 'small', 'x-large'.
            See `.Text.set_size` for accepted string values.

        colors : :mpltype:`color` or colors or None, default: None
            The label colors:

            - If *None*, the color of each label matches the color of
              the corresponding contour.

            - If one string color, e.g., *colors* = 'r' or *colors* =
              'red', all labels will be plotted in this color.

            - If a tuple of colors (string, float, RGB, etc), different labels
              will be plotted in different colors in the order specified.

        inline : bool, default: True
            If ``True`` the underlying contour is removed where the label is
            placed.

        inline_spacing : float, default: 5
            Space in pixels to leave on each side of label when placing inline.

            This spacing will be exact for labels at locations where the
            contour is straight, less so for labels on curved contours.

        fmt : `.Formatter` or str or callable or dict, optional
            How the levels are formatted:

            - If a `.Formatter`, it is used to format all levels at once, using
              its `.Formatter.format_ticks` method.
            - If a str, it is interpreted as a %-style format string.
            - If a callable, it is called with one level at a time and should
              return the corresponding label.
            - If a dict, it should directly map levels to labels.

            The default is to use a standard `.ScalarFormatter`.

        manual : bool or iterable, default: False
            If ``True``, contour labels will be placed manually using
            mouse clicks. Click the first button near a contour to
            add a label, click the second button (or potentially both
            mouse buttons at once) to finish adding labels. The third
            button can be used to remove the last label added, but
            only if labels are not inline. Alternatively, the keyboard
            can be used to select label locations (enter to end label
            placement, delete or backspace act like the third mouse button,
            and any other key will select a label location).

            *manual* can also be an iterable object of (x, y) tuples.
            Contour labels will be created as if mouse is clicked at each
            (x, y) position.

        rightside_up : bool, default: True
            If ``True``, label rotations will always be plus
            or minus 90 degrees from level.

        use_clabeltext : bool, default: False
            If ``True``, use `.Text.set_transform_rotates_text` to ensure that
            label rotation is updated whenever the Axes aspect changes.

        zorder : float or None, default: ``(2 + contour.get_zorder())``
            zorder of the contour labels.

        Returns
        -------
        labels
            A list of `.Text` instances for the labels.

            Note: The returned Text instances should not be individually
            removed or have their geometry modified, e.g. by changing text or font size.
            If you need such a modification, remove the entire
            `.ContourSet` and recreate it.
        """
        if self.filled:
            _api.warn_deprecated(
                "3.11",
                message="clabel() is not designed to be used with filled contours and "
                        "may result in inconsistent plots. Applying clabel() to filled "
                        "contours is thus deprecated since %(since)s. If you need "
                        "labels with filled contours, instead draw contour lines "
                        "using contour() in addition to contourf() and add the labels "
                        "to the contour lines."
            )
        if isinstance(fmt, dict):
            fmt = [(float(level), str(text)) for level, text in fmt.items()]
        elif fmt is not None and not isinstance(fmt, str) and (
                not callable(fmt) or callable(getattr(fmt, "format_ticks", None))):
            labelled = self.levels if levels is None else [
                lev for lev in self.levels if lev in list(levels)]
            fmt = _label_texts(fmt, labelled)
        # Labels at the positions given, or placed by clicks (none yet), or
        # wherever they fit (None).
        interactive = False
        if np.iterable(manual):
            positions = [(float(self.axes.convert_xunits(x)),
                          float(self.axes.convert_yunits(y))) for x, y in manual]
        elif manual:
            positions, interactive = [], True
        else:
            positions = None
        if isinstance(fontsize, str):
            from .font_manager import FontProperties
            fontsize = FontProperties(size=fontsize).get_size_in_points()
        if colors is not None:
            from .colors import to_rgba_array
            colors = [tuple(c) for c in to_rgba_array(colors)]
        _contour.clabel(
            self._contour,
            (None if levels is None else [float(lev) for lev in levels],
             None if fontsize is None else float(fontsize), bool(inline),
             float(inline_spacing), colors, positions),
            fmt, (bool(rightside_up), None if zorder is None else float(zorder),
                  bool(use_clabeltext)), self._native_splitter)
        self._labelled_all_levels = levels is None
        if not interactive:
            # matplotlib's: every label of the set.
            return cbook.silent_list('text.Text', self.labelTexts)
        print('Select label locations manually using first mouse button.')
        print('End manual selection with second mouse button.')
        if not inline:
            print('Remove last label by clicking third mouse button.')
        _blocking_input.blocking_input_loop(
            self.axes.get_figure(root=True),
            ["button_press_event", "key_press_event"],
            timeout=-1, handler=functools.partial(
                _contour_labeler_event_handler,
                self, inline, inline_spacing))
        return cbook.silent_list('text.Text', self.labelTexts)

    def _native_splitter(self, vertices, codes, idx, screen_pos, lw, spacing):
        """What pltrs (Rust) calls for each label it places: matplotlib's
        `_split_path_and_get_label_rotation` on the line as a `.Path`."""
        return self._split_path_and_get_label_rotation(
            Path(vertices, codes), idx, screen_pos, lw, spacing)

    def _split_path_and_get_label_rotation(self, path, idx, screen_pos, lw, spacing=5):
        """
        Prepare for insertion of a label at index *idx* of *path*.

        Parameters
        ----------
        path : Path
            The path where the label will be inserted, in data space.
        idx : int
            The vertex index after which the label will be inserted.
        screen_pos : (float, float)
            The position where the label will be inserted, in screen space.
        lw : float
            The label width, in screen space.
        spacing : float
            Extra spacing around the label, in screen space.

        Returns
        -------
        path : Path
            The path, broken so that the label can be drawn over it.
        angle : float
            The rotation of the label.

        Notes
        -----
        Both tasks are done together to avoid calculating path lengths
        multiple times, which is relatively costly. pltrs (Rust) computes
        them.
        """
        rotation, (vertices, codes) = _contour.split_path(
            self._contour, path, int(idx), tuple(map(float, screen_pos)),
            (float(lw), float(spacing)))
        return rotation, Path(vertices, codes)

    def add_label_near(self, x, y, inline=True, inline_spacing=5,
                       transform=None):
        """
        Add a label near the point ``(x, y)``.

        Parameters
        ----------
        x, y : float
            The approximate location of the label.
        inline : bool, default: True
            If *True* remove the segment of the contour beneath the label.
        inline_spacing : int, default: 5
            Space in pixels to leave on each side of label when placing
            inline. This spacing will be exact for labels at locations where
            the contour is straight, less so for labels on curved contours.
        transform : `.Transform` or `False`, default: ``self.axes.transData``
            A transform applied to ``(x, y)`` before labeling.  The default
            causes ``(x, y)`` to be interpreted as data coordinates.  `False`
            is a synonym for `.IdentityTransform`; i.e. ``(x, y)`` should be
            interpreted as display coordinates.
        """
        if transform is None:
            transform = self.axes.transData
        if transform:
            x, y = transform.transform((x, y))
        _contour.add_label_near(self._contour, (float(x), float(y)),
                                bool(inline), float(inline_spacing),
                                self._native_splitter)

    def pop_label(self, index=-1):
        """Defaults to removing last label, but any index can be supplied"""
        count = len(self.labelTexts)
        if not count:
            raise IndexError("pop from empty list")
        if not -count <= index < count:
            raise IndexError("pop index out of range")
        _contour.pop_label(self._contour, index % count)

    @property
    def labelTexts(self):
        """The label `.Text` instances."""
        from .text import Text
        return _handles.wrap_all(_contour.label_texts(self._contour), Text,
                                 self._figure)

    @property
    def labelXYs(self):
        """The display positions of the labels."""
        return _contour.label_positions(self._contour)

    @property
    def labelCValues(self):
        """The colour values (`cvalues`) of the levels of the labels."""
        index = {level: i for i, level in enumerate(self.levels)}
        cvalues = self.cvalues
        return [cvalues[index[level]]
                for level in _contour.label_text_levels(self._contour)]

    @property
    def labelLevelList(self):
        """The levels the last `clabel` labelled: the set's levels when it
        labelled them all, else a list of those it labelled, as matplotlib
        keeps them."""
        if self.__dict__.get("_labelled_all_levels"):
            return self.levels
        return [np.float64(level) for level in _contour.label_levels(self._contour)]

    def print_label(self, linecontour, labelwidth):
        """Return whether a contour is long enough to hold a label."""
        fits = _contour.print_label(
            np.asarray(linecontour, dtype=float).reshape(-1, 2), float(labelwidth))
        # matplotlib's answer is numpy's but where the number of points
        # settles it (``len(linecontour) > 10 * labelwidth``).
        return fits if len(linecontour) > 10 * labelwidth else np.bool_(fits)

    def too_close(self, x, y, lw):
        """Return whether a label is already near this location."""
        return _contour.too_close(self._contour, (float(x), float(y)), float(lw))

    def get_text(self, lev, fmt):
        """Get the text of the label."""
        if isinstance(lev, str):
            return lev
        elif isinstance(fmt, dict):
            return fmt.get(lev, '%1.3f')
        elif callable(getattr(fmt, "format_ticks", None)):
            return fmt.format_ticks([*self.labelLevelList, lev])[-1]
        elif callable(fmt):
            return fmt(lev)
        else:
            return fmt % lev

    def locate_label(self, linecontour, labelwidth):
        """
        Find good place to draw a label (relatively flat part of the contour).
        """
        x, y, index = _contour.locate_label(
            self._contour, np.asarray(linecontour, dtype=float).reshape(-1, 2),
            float(labelwidth))
        # matplotlib's: points of the line's own array, a numpy index.
        number = np.asarray(linecontour).dtype.type
        return number(x), number(y), np.int64(index)

    def add_label(self, x, y, rotation, lev, cvalue):
        """Add a contour label, respecting whether *use_clabeltext* was set."""
        # pltrs: the label of level *lev* at the display point (*x*, *y*), in
        # the colour and font the last `clabel` gave the labels of that level.
        levels = [float(level) for level in self.levels]
        if float(lev) not in levels:
            raise ValueError(f"{lev!r} is not a level of the contour set")
        _contour.add_label(self._contour, (float(x), float(y)), float(rotation),
                           levels.index(float(lev)))

    def labels(self, inline, inline_spacing):
        _contour.place_labels(self._contour, bool(inline), float(inline_spacing),
                              self._native_splitter)

    def remove(self):
        super().remove()

    def _remove_natively(self):
        _contour.remove(self._contour)


class _Bands(_mirrors.Mirroring):
    """The bands of a hatched filled `ContourSet`, which pltrs (Rust)
    draws with a collection each: the set's setters change the first
    (``self._native``), whose look this gives the others, as matplotlib's
    set is one collection drawing its bands one by one."""

    def push(self, artist):
        _contour.share_first_band_look(artist._contour)


_docstring.interpd.register(contour_set_attributes=r"""
Attributes
----------
levels : array
    The values of the contour levels.

layers : array
    Same as levels for line contours; half-way between
    levels for filled contours.  See ``ContourSet._process_colors``.
""")

_docstring.interpd.register(contour_doc="""
`.contour` and `.contourf` draw contour lines and filled contours,
respectively.  Except as noted, function signatures and return values
are the same for both versions.

Parameters
----------
X, Y : array-like, optional
    The coordinates of the values in *Z*.

    *X* and *Y* must both be 2D with the same shape as *Z* (e.g.
    created via `numpy.meshgrid`), or they must both be 1-D such
    that ``len(X) == N`` is the number of columns in *Z* and
    ``len(Y) == M`` is the number of rows in *Z*.

    *X* and *Y* must both be ordered monotonically.

    If not given, they are assumed to be integer indices, i.e.
    ``X = range(N)``, ``Y = range(M)``.

Z : (M, N) array-like
    The height values over which the contour is drawn.  Color-mapping is
    controlled by *cmap*, *norm*, *vmin*, and *vmax*.

levels : int or array-like, optional
    Determines the number and positions of the contour lines / regions.

    If an int *n*, use `~matplotlib.ticker.MaxNLocator`, which tries to
    automatically choose no more than *n+2* "nice" contour level boundaries
    between the minimum and maximum numeric values of *Z*. These boundaries
    define where lines are drawn (for `contour`) or where filled regions
    are separated (for `contourf`).

    If not given, a reasonable default is chosen; for linear scales,
    *n*=7 is the default.

    If array-like, draw contour lines at the specified levels.
    The values must be in increasing order.

    If not specified, a reasonable default is automatically chosen. For
    linear scales, this corresponds to *levels=7*. For logarithmic
    scales, `~matplotlib.ticker.LogLocator` is used instead.

Returns
-------
`~.contour.QuadContourSet`

Other Parameters
----------------
corner_mask : bool, default: :rc:`contour.corner_mask`
    Enable/disable corner masking, which only has an effect if *Z* is
    a masked array.  If ``False``, any quad touching a masked point is
    masked out.  If ``True``, only the triangular corners of quads
    nearest those points are always masked out, other triangular
    corners comprising three unmasked points are contoured as usual.

colors : :mpltype:`color` or list of :mpltype:`color`, optional
    The colors of the levels, i.e. the lines for `.contour` and the
    areas for `.contourf`.

    The sequence is cycled for the levels in ascending order. If the
    sequence is shorter than the number of levels, it's repeated.

    As a shortcut, a single color may be used in place of one-element lists, i.e.
    ``'red'`` instead of ``['red']`` to color all levels with the same color.

    .. versionchanged:: 3.10
        Previously a single color had to be expressed as a string, but now any
        valid color format may be passed.

    By default (value *None*), the colormap specified by *cmap*
    will be used.

alpha : float, default: 1
    The alpha blending value, between 0 (transparent) and 1 (opaque).

%(cmap_doc)s

    This parameter is ignored if *colors* is set.

%(norm_doc)s

    This parameter is ignored if *colors* is set.

%(vmin_vmax_doc)s

    If *vmin* or *vmax* are not given, the default color scaling is based on
    *levels*.

    This parameter is ignored if *colors* is set.

%(colorizer_doc)s

    This parameter is ignored if *colors* is set.

origin : {*None*, 'upper', 'lower', 'image'}, default: None
    Determines the orientation and exact position of *Z* by specifying
    the position of ``Z[0, 0]``.  This is only relevant, if *X*, *Y*
    are not given.

    - *None*: ``Z[0, 0]`` is at X=0, Y=0 in the lower left corner.
    - 'lower': ``Z[0, 0]`` is at X=0.5, Y=0.5 in the lower left corner.
    - 'upper': ``Z[0, 0]`` is at X=N+0.5, Y=0.5 in the upper left
      corner.
    - 'image': Use the value from :rc:`image.origin`.

extent : (x0, x1, y0, y1), optional
    If *origin* is not *None*, then *extent* is interpreted as in
    `.imshow`: it gives the outer pixel boundaries. In this case, the
    position of Z[0, 0] is the center of the pixel, not a corner. If
    *origin* is *None*, then (*x0*, *y0*) is the position of Z[0, 0],
    and (*x1*, *y1*) is the position of Z[-1, -1].

    This argument is ignored if *X* and *Y* are specified in the call
    to contour.

locator : ticker.Locator subclass, optional
    The locator is used to determine the contour levels if they
    are not given explicitly via *levels*.
    Defaults to `~.ticker.MaxNLocator`.

extend : {'neither', 'both', 'min', 'max'}, default: 'neither'
    Determines the ``contourf``-coloring of values that are outside the
    *levels* range.

    If 'neither', values outside the *levels* range are not colored.
    If 'min', 'max' or 'both', color the values below, above or below
    and above the *levels* range.

    Values below ``min(levels)`` and above ``max(levels)`` are mapped
    to the under/over values of the `.Colormap`. Note that most
    colormaps do not have dedicated colors for these by default, so
    that the over and under values are the edge values of the colormap.
    You may want to set these values explicitly using
    `.Colormap.set_under` and `.Colormap.set_over`.

    .. note::

        An existing `.QuadContourSet` does not get notified if
        properties of its colormap are changed. Therefore, an explicit
        call `~.ContourSet.changed()` is needed after modifying the
        colormap. The explicit call can be left out, if a colorbar is
        assigned to the `.QuadContourSet` because it internally calls
        `~.ContourSet.changed()`.

    Example::

        x = np.arange(1, 10)
        y = x.reshape(-1, 1)
        h = x * y

        cs = plt.contourf(h, levels=[10, 30, 50],
            colors=['#808080', '#A0A0A0', '#C0C0C0'], extend='both')
        cs.cmap.set_over('red')
        cs.cmap.set_under('blue')
        cs.changed()

xunits, yunits : registered units, optional
    Override axis units by specifying an instance of a
    :class:`matplotlib.units.ConversionInterface`.

antialiased : bool, optional
    Enable antialiasing, overriding the defaults.  For
    filled contours, the default is *False*.  For line contours,
    it is taken from :rc:`lines.antialiased`.

nchunk : int >= 0, optional
    If 0, no subdivision of the domain.  Specify a positive integer to
    divide the domain into subdomains of *nchunk* by *nchunk* quads.
    Chunking reduces the maximum length of polygons generated by the
    contouring algorithm which reduces the rendering workload passed
    on to the backend and also requires slightly less RAM.  It can
    however introduce rendering artifacts at chunk boundaries depending
    on the backend, the *antialiased* flag and value of *alpha*.

linewidths : float or array-like, default: :rc:`contour.linewidth`
    *Only applies to* `.contour`.

    The line width of the contour lines.

    If a number, all levels will be plotted with this linewidth.

    If a sequence, the levels in ascending order will be plotted with
    the linewidths in the order specified.

    If None, this falls back to :rc:`lines.linewidth`.

linestyles : {*None*, 'solid', 'dashed', 'dashdot', 'dotted'}, optional
    *Only applies to* `.contour`.

    If *linestyles* is *None*, the default is 'solid' unless the lines are
    monochrome. In that case, negative contours will instead take their
    linestyle from the *negative_linestyles* argument.

    *linestyles* can also be an iterable of the above strings specifying a set
    of linestyles to be used. If this iterable is shorter than the number of
    contour levels it will be repeated as necessary.

negative_linestyles : {*None*, 'solid', 'dashed', 'dashdot', 'dotted'}, \
                       optional
    *Only applies to* `.contour`.

    If *linestyles* is *None* and the lines are monochrome, this argument
    specifies the line style for negative contours.

    If *negative_linestyles* is *None*, the default is taken from
    :rc:`contour.negative_linestyle`.

    *negative_linestyles* can also be an iterable of the above strings
    specifying a set of linestyles to be used. If this iterable is shorter than
    the number of contour levels it will be repeated as necessary.

hatches : list[str], optional
    *Only applies to* `.contourf`.

    A list of cross hatch patterns to use on the filled areas.
    If None, no hatching will be added to the contour.

algorithm : {'mpl2005', 'mpl2014', 'serial', 'threaded'}, optional
    Which contouring algorithm to use to calculate the contour lines and
    polygons. The algorithms are implemented in
    `ContourPy <https://github.com/contourpy/contourpy>`_, consult the
    `ContourPy documentation <https://contourpy.readthedocs.io>`_ for
    further information.

    The default is taken from :rc:`contour.algorithm`.

clip_path : `~matplotlib.patches.Patch` or `.Path` or `.TransformedPath`
    Set the clip path.  See `~matplotlib.artist.Artist.set_clip_path`.

    .. versionadded:: 3.8

data : indexable object, optional
    DATA_PARAMETER_PLACEHOLDER

rasterized : bool, optional
    *Only applies to* `.contourf`.
    Rasterize the contour plot when drawing vector graphics.  This can
    speed up rendering and produce smaller files for large data sets.
    See also :doc:`/gallery/misc/rasterization_demo`.


Notes
-----
1. `.contourf` differs from the MATLAB version in that it does not draw
   the polygon edges. To draw edges, add line contours with calls to
   `.contour`.

2. `.contourf` fills intervals that are closed at the top; that is, for
   boundaries *z1* and *z2*, the filled region is::

      z1 < Z <= z2

   except for the lowest interval, which is closed on both sides (i.e.
   it includes the lowest value).

3. `.contour` and `.contourf` use a `marching squares
   <https://en.wikipedia.org/wiki/Marching_squares>`_ algorithm to
   compute contour locations.  More information can be found in
   `ContourPy documentation <https://contourpy.readthedocs.io>`_.
""" % _docstring.interpd.params)


@_docstring.interpd
@_mirrors.pushing
class ContourSet(ContourLabeler, mcoll.Collection):
    """
    Store a set of contour lines or filled regions.

    User-callable method: `~.Axes.clabel`

    Parameters
    ----------
    ax : `~matplotlib.axes.Axes`

    levels : [level0, level1, ..., leveln]
        A list of floating point numbers indicating the contour levels.

    allsegs : [level0segs, level1segs, ...]
        List of all the polygon segments for all the *levels*.
        For contour lines ``len(allsegs) == len(levels)``, and for
        filled contour regions ``len(allsegs) = len(levels)-1``. The lists
        should look like ::

            level0segs = [polygon0, polygon1, ...]
            polygon0 = [[x0, y0], [x1, y1], ...]

    allkinds : ``None`` or [level0kinds, level1kinds, ...]
        Optional list of all the polygon vertex kinds (code types), as
        described and used in Path. This is used to allow multiply-
        connected paths such as holes within filled polygons.
        If not ``None``, ``len(allkinds) == len(allsegs)``. The lists
        should look like ::

            level0kinds = [polygon0kinds, ...]
            polygon0kinds = [vertexcode0, vertexcode1, ...]

        If *allkinds* is not ``None``, usually all polygons for a
        particular contour level are grouped together so that
        ``level0segs = [polygon0]`` and ``level0kinds = [polygon0kinds]``.

    **kwargs
        Keyword arguments are as described in the docstring of
        `~.Axes.contour`.

    %(contour_set_attributes)s
    """

    def __init__(self, ax, *args,
                 levels=None, filled=False, linewidths=None, linestyles=None,
                 hatches=(None,), alpha=None, origin=None, extent=None,
                 cmap=None, colors=None, norm=None, vmin=None, vmax=None,
                 colorizer=None, extend='neither', antialiased=None, nchunk=0,
                 locator=None, transform=None, negative_linestyles=None,
                 **kwargs):
        """
        Draw contour lines or filled regions, depending on
        whether keyword arg *filled* is ``False`` (default) or ``True``.

        Call signature::

            ContourSet(ax, levels, allsegs, [allkinds], **kwargs)

        Parameters
        ----------
        ax : `~matplotlib.axes.Axes`
            The `~.axes.Axes` object to draw on.

        levels : [level0, level1, ..., leveln]
            A list of floating point numbers indicating the contour
            levels.

        allsegs : [level0segs, level1segs, ...]
            List of all the polygon segments for all the *levels*.
            For contour lines ``len(allsegs) == len(levels)``, and for
            filled contour regions ``len(allsegs) = len(levels)-1``. The lists
            should look like ::

                level0segs = [polygon0, polygon1, ...]
                polygon0 = [[x0, y0], [x1, y1], ...]

        allkinds : [level0kinds, level1kinds, ...], optional
            Optional list of all the polygon vertex kinds (code types), as
            described and used in Path. This is used to allow multiply-
            connected paths such as holes within filled polygons.
            If not ``None``, ``len(allkinds) == len(allsegs)``. The lists
            should look like ::

                level0kinds = [polygon0kinds, ...]
                polygon0kinds = [vertexcode0, vertexcode1, ...]

            If *allkinds* is not ``None``, usually all polygons for a
            particular contour level are grouped together so that
            ``level0segs = [polygon0]`` and ``level0kinds = [polygon0kinds]``.

        **kwargs
            Keyword arguments are as described in the docstring of
            `~.Axes.contour`.
        """
        from .axes._plotting.contours import _contour_look, _style
        kwargs = {**cbook._given_keywords(ContourSet.__init__, locals(),
                                          exclude=("levels", "filled")), **kwargs}
        # matplotlib's collection takes the alpha first, checking it.
        martist._check_array_alpha(kwargs.get("alpha"))
        # Kept and checked as matplotlib's set does, which only a set
        # contouring a grid places with.
        origin = kwargs.pop("origin", None)
        extent = kwargs.pop("extent", None)
        _api.check_in_list([None, 'lower', 'upper', 'image'], origin=origin)
        if extent is not None and len(extent) != 4:
            raise ValueError(
                "If given, 'extent' must be None or (x0, x1, y0, y1)")
        if origin == 'image':
            from . import rcParams
            origin = rcParams['image.origin']
        levels, allsegs, *rest = args
        allkinds = rest[0] if rest else None
        if allkinds is None:
            allkinds = [[None] * len(segs) for segs in allsegs]
        # Check lengths of levels and allsegs.
        if filled:
            if len(allsegs) != len(levels) - 1:
                raise ValueError('must be one less number of segments as '
                                 'levels')
        else:
            if len(allsegs) != len(levels):
                raise ValueError('must be same number of segments as levels')
        # Check length of allkinds.
        if len(allkinds) != len(allsegs):
            raise ValueError('allkinds has different length to allsegs')
        segments = [[Path(seg, kind) for seg, kind in zip(segs, kinds)]
                    for segs, kinds in zip(allsegs, allkinds)]
        look = _contour_look(ax, kwargs)
        native = _contour.contour_segments(
            ax._native, bool(filled), [float(level) for level in levels],
            segments, *look[:2])
        _handles.adopt(self, _contour.get_collections(native)[0], ax.figure)
        self._set_contour(native, ax.figure)
        _style(self, look, kwargs)
        self.origin, self.extent = origin, extent

    @classmethod
    def _from_contour(cls, contour, axes):
        """The object around the native contour set *contour* of *axes*."""
        collections = _contour.get_collections(contour)
        cs = _handles.wrap(collections[0], cls, axes.figure)
        cs._take_contour(contour, axes)
        return cs

    def _take_contour(self, contour, axes):
        """Make the set, wrapping its first collection already, the object
        of the native contour set *contour* of *axes*."""
        self._set_contour(contour, axes.figure)
        self._attach_native_mapping()

    def _set_contour(self, contour, figure):
        """Make the set wrap the native contour set *contour* of
        *figure*, whose object it then is too (the set is one child of its
        axes, as in matplotlib)."""
        self._contour = contour
        figure._figure._artists[_artist.id(contour)] = self
        if len(_contour.get_collections(contour)) > 1:
            _mirrors.attach(self, _Bands())

    def _native_clim(self):
        if not hasattr(self, "_contour"):
            return (None, None)
        return _contour.get_clim(self._contour)

    def _native_cmap(self):
        if not hasattr(self, "_contour"):
            return None
        return _contour.get_cmap(self._contour)

    def _native_array(self):
        if not hasattr(self, "_contour"):
            return None
        return cbook.safe_masked_invalid(_contour.get_cvalues(self._contour)[0])

    def _attach_native_mapping(self, cmap=None, norm=None, colorizer=None):
        # Explicit colors are indexed through a NoNorm (matplotlib's
        # ``_process_colors``).
        if hasattr(self, "_contour") and _contour.get_cvalues(self._contour)[1]:
            if colorizer is None:
                norm = mcolors.NoNorm()
            else:
                colorizer.norm = mcolors.NoNorm()
        super()._attach_native_mapping(cmap, norm, colorizer)

    @property
    def levels(self):
        """The contour levels."""
        return _contour.get_levels(self._contour)

    @property
    def _levels(self):
        """The levels with the extended ends added, -1e250 and 1e250 (1e-250
        on a log scale), as matplotlib's ``_process_levels`` keeps them."""
        levels = list(self.levels)
        lower, upper = ((1e-250, 1e250) if isinstance(self.norm, mcolors.LogNorm)
                        else (-1e250, 1e250))
        if self.extend in ('both', 'min'):
            levels.insert(0, lower)
        if self.extend in ('both', 'max'):
            levels.append(upper)
        return np.asarray(levels)

    @property
    def layers(self):
        """The values the levels or bands are coloured by."""
        return _contour.get_layers(self._contour)

    @property
    def cvalues(self):
        """The values the levels or bands are mapped to colours by: the
        layers, or indices into explicitly given colours."""
        return _contour.get_cvalues(self._contour)[0]

    @property
    def filled(self):
        """Whether the contours are filled."""
        return _contour.get_state(self._contour)[0]

    @property
    def zmin(self):
        """The smallest value of the data."""
        return np.float64(_contour.get_state(self._contour)[1])

    @property
    def zmax(self):
        """The largest value of the data."""
        return np.float64(_contour.get_state(self._contour)[2])

    @property
    def extend(self):
        """Which ends the colors extend: 'neither', 'both', 'min' or 'max'
        (as given; any other value extends neither end)."""
        return self.__dict__.get("_extend", _contour.get_state(self._contour)[3])

    @property
    def _contour_generator(self):
        """
        The contour generator matplotlib keeps, shared with the contour
        sets made from this one: contourpy's for a `QuadContourSet` (where
        contourpy is importable), the ``_tri`` extension's for a
        `.TriContourSet`.
        """
        return self._generator._matplotlib_generator()

    def get_paths(self):
        """Return the paths: one compound path per level or band."""
        native = _contour.get_paths(self._contour)
        # The same list while the paths stay, as matplotlib keeps its
        # ``_paths``.
        kept = self.__dict__.get("_paths_kept")
        if kept is None or len(kept) != len(native) or not all(
                np.array_equal(path.vertices, vertices)
                and np.array_equal(path.codes, codes)
                for path, (vertices, codes) in zip(kept, native)):
            kept = self._paths_kept = [Path(vertices, codes)
                                       for vertices, codes in native]
            _owned_paths.hand_out(kept, self._push_kept_paths)
        return kept

    def _push_kept_paths(self):
        """Hand pltrs (Rust) the paths handed out again, one of which the
        program changed (see `._owned_paths`)."""
        kept = self.__dict__.get("_paths_kept")
        if kept is not None:
            _collections.set_paths(self._native, list(kept))

    allsegs = property(lambda self: [
        [subp.vertices for subp in p._iter_connected_components()]
        for p in self.get_paths()])
    allkinds = property(lambda self: [
        [subp.codes for subp in p._iter_connected_components()]
        for p in self.get_paths()])

    def get_linewidth(self):
        # The line widths of the levels (of the collection, for filled
        # contours).
        if self.filled:
            return super().get_linewidth()
        # The widths given, which pltrs (Rust) holds one per level, as
        # matplotlib broadcasts them against the dash patterns of the line
        # styles (``_bcast_lwls``; one per level until a style is set).
        native = np.asarray(_contour.get_linewidths(self._contour))
        widths = np.atleast_1d(_values.given(self, "linewidth", native,
                                             same=_values.same_cycled))
        dashes = len(self._given_linestyles())
        if len(widths) == dashes or not dashes:
            return widths
        return list(widths) * (dashes // math.gcd(dashes, len(widths)))

    def set_linewidth(self, lw):
        # The line widths of the levels (of the collection's edges, for
        # filled contours).
        lw = np.atleast_1d(lw)
        if self.filled:
            super().set_linewidth(lw)
            return
        _contour.set_linewidths(self._contour, [float(w) for w in lw])
        _values.remember(self, "linewidth", lw)

    get_linewidth.__doc__ = mcoll.Collection.get_linewidth.__doc__
    set_linewidth.__doc__ = mcoll.Collection.set_linewidth.__doc__

    def get_hatch(self):
        # The hatching pattern of the set as a collection: none until one is
        # set (the bands' are `hatches`).
        return self.__dict__.get("_collection_hatch")

    get_hatch.__doc__ = mcoll.Collection.get_hatch.__doc__

    def set_hatch(self, hatch):
        # The hatching pattern of the set as a collection.
        super().set_hatch(hatch)
        self._collection_hatch = hatch

    set_hatch.__doc__ = mcoll.Collection.set_hatch.__doc__

    def get_facecolor(self):
        """
        Return the face colors.

        A hatched filled set, which matplotlib draws band by band, mapping
        each band's value on its own, keeps the face of the band it drew
        last once drawn, until its mapping changes.
        """
        faces = super().get_facecolor()
        values = self.get_array()
        hatched = self.filled and any(hatch is not None for hatch in self.hatches)
        if (not hatched or values is None or len(values) < 2 or len(faces) != 1
                or not np.allclose(faces, self.to_rgba(values[:1], self.get_alpha()))):
            return faces
        if _artist.draw_count(self._native) > self.__dict__.get("_mapped_at_draw", 0):
            values = values[-1:]
        return self.to_rgba(values, self.get_alpha())

    def get_edgecolor(self):
        # The edge colors: those of the levels for line contours until
        # others are set, none for filled contours.
        return super().get_edgecolor()

    get_edgecolor.__doc__ = mcoll.Collection.get_edgecolor.__doc__

    def set_edgecolor(self, c):
        # The edge color(s) of the contours: a color or a sequence of
        # colors, cycled over the paths of the levels or bands; ``'none'``,
        # ``'face'`` or None as for any `.Collection`.
        if c is None or isinstance(c, str) and c.lower() in ("none", "face"):
            super().set_edgecolor(c)
            return
        _contour.set_edgecolors(
            self._contour, [tuple(rgba) for rgba in mcolors.to_rgba_array(c)])

    set_edgecolor.__doc__ = mcoll.Collection.set_edgecolor.__doc__

    def get_transform(self):
        """Return the `.Transform` instance used by this ContourSet."""
        return super().get_transform()

    @martist.allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)

    def find_nearest_contour(self, x, y, indices=None, pixel=True):
        """
        Find the point in the contour plot that is closest to ``(x, y)``.

        This method does not support filled contours.

        Parameters
        ----------
        x, y : float
            The reference point.
        indices : list of int or None, default: None
            Indices of contour levels to consider.  If None (the default), all
            levels are considered.
        pixel : bool, default: True
            If *True*, measure distance in pixel (screen) space, which is
            useful for manual contour labeling; else, measure distance in axes
            space.

        Returns
        -------
        path : int
            The index of the path that is closest to ``(x, y)``.  Each path corresponds
            to one contour level.
        subpath : int
            The index within that closest path of the subpath that is closest to
            ``(x, y)``.  Each subpath corresponds to one unbroken contour line.
        index : int
            The index of the vertices within that subpath that are closest to
            ``(x, y)``.
        xmin, ymin : float
            The point in the contour plot that is closest to ``(x, y)``.
        d2 : float
            The squared distance from ``(xmin, ymin)`` to ``(x, y)``.
        """
        if indices is not None:
            indices = [int(i) for i in np.atleast_1d(indices)]
        nearest = _contour.find_nearest_contour(
            self._contour, (float(x), float(y)), indices, bool(pixel))
        if nearest is None:
            return (None, None, None, None, None, None)
        level, segment, index, (xmin, ymin), d2 = nearest
        # matplotlib's: numpy's numbers but for the level's index.
        return (level, np.int64(segment), np.int64(index), np.float64(xmin),
                np.float64(ymin), np.float64(d2))

    def legend_elements(self, variable_name='x', str_format=str):
        """
        Return a list of artists and labels suitable for passing through
        to `~.Axes.legend` which represent this ContourSet.

        The labels have the form "0 < x <= 1" stating the data ranges which
        the artists represent.

        Parameters
        ----------
        variable_name : str
            The string used inside the inequality used on the labels.
        str_format : function: float -> str
            Function used to format the numbers in the labels.

        Returns
        -------
        artists : list[`.Artist`]
            A list of the artists.
        labels : list[str]
            A list of the labels.
        """
        from .lines import Line2D
        from .patches import Rectangle
        # matplotlib reads the style of each level off the collection's
        # arrays, which fail where they hold fewer (one width set for all).
        if self.filled:
            for idx in range(len(self.get_paths())):
                self.get_facecolor()[idx]
        else:
            for idx in range(len(self.levels)):
                (self.get_edgecolor()[idx], self.get_linewidths()[idx],
                 self.get_linestyles()[idx])
        entries, labels = _contour.legend_elements(
            self._contour, str(variable_name), str_format)
        artists = []
        for kind, color, *style in entries:
            if kind == "line":
                linewidth, dashes = style
                artists.append(Line2D([], [], color=color, linewidth=linewidth,
                                      linestyle=dashes))
            else:
                (hatch,) = style
                artists.append(Rectangle((0, 0), 1, 1, facecolor=color, hatch=hatch))
        return artists, labels

    def changed(self):
        if hasattr(self, "_contour"):
            # matplotlib's ``changed`` scales the norm to the levels and
            # maps the contour values again, which fails for limits the
            # norm cannot take.
            self.norm.autoscale_None(self.levels)
            _contour.set_mapping(self._contour, self.cmap, self.norm,
                                 self._norm_replaced())
            self._A = self._native_array()
            self.to_rgba(self._A)
            self._mapped_at_draw = _artist.draw_count(self._native)
        super(mcoll.Collection, self).changed()

    def _mapping_changed(self):
        # docstring inherited: `changed` hands pltrs (Rust) the mapping of
        # the contour set itself (``_contour.set_mapping``), the explicit
        # colours' `.NoNorm` included, not that of a plain collection.
        pass

    alpha = property(lambda self: self.get_alpha())
    linestyles = property(lambda self: self.__dict__.get("_orig_linestyles"))


class _ContourGenerator:
    """
    What a contour set was generated from, kept as ``_generator``: the
    *field* pltrs (Rust) contours again, at other levels, when the contour
    set is passed to ``contour`` or ``contourf`` (the arguments of the
    native ``contour`` before the levels), and the generator matplotlib
    keeps for it (``_contour_generator``), made when first asked for.
    """

    def __init__(self, *field):
        self.field = field
        self._matplotlib = None

    def _make_matplotlib(self):
        """
        The contourpy generator matplotlib makes for the field, with the
        same arguments, or None where contourpy is not importable. pltrs
        (Rust) still computes and draws the contours.
        """
        try:
            import contourpy
        except ImportError:
            return None
        field, generation = self.field
        z, mask = field[:2]
        x, y, algorithm, corner_mask = _contour.generator_arguments(field, generation)
        z = ma.masked_invalid(z, copy=False)
        if mask is not None:
            z[mask] = ma.masked
        return contourpy.contour_generator(
            x, y, z, name=algorithm, corner_mask=corner_mask,
            line_type=contourpy.LineType.SeparateCode,
            fill_type=contourpy.FillType.OuterCode,
            chunk_size=generation[1])

    def _matplotlib_generator(self):
        """matplotlib's generator for the field, made the first time; this
        generator where none can be made."""
        if self._matplotlib is None:
            self._matplotlib = self._make_matplotlib() or self
        return self._matplotlib

    def __getstate__(self):
        # matplotlib's generators cannot be pickled: they are made again.
        return {**self.__dict__, "_matplotlib": None}


@_docstring.interpd
class QuadContourSet(ContourSet):
    """
    Create and store a set of contour lines or filled regions.

    This class is typically not instantiated directly by the user but by
    `~.Axes.contour` and `~.Axes.contourf`.

    %(contour_set_attributes)s
    """

    def __init__(self, ax, *args,
                 levels=None, filled=False, linewidths=None, linestyles=None,
                 hatches=(None,), alpha=None, origin=None, extent=None,
                 cmap=None, colors=None, norm=None, vmin=None, vmax=None,
                 colorizer=None, extend='neither', antialiased=None, nchunk=0,
                 locator=None, transform=None, negative_linestyles=None,
                 **kwargs):
        # matplotlib's contour sets contour their arguments as
        # ``contour``/``contourf`` do, and add themselves to *ax*.
        """
        Draw contour lines or filled regions, depending on
        whether keyword arg *filled* is ``False`` (default) or ``True``.

        Call signature::

            ContourSet(ax, levels, allsegs, [allkinds], **kwargs)

        Parameters
        ----------
        ax : `~matplotlib.axes.Axes`
            The `~.axes.Axes` object to draw on.

        levels : [level0, level1, ..., leveln]
            A list of floating point numbers indicating the contour
            levels.

        allsegs : [level0segs, level1segs, ...]
            List of all the polygon segments for all the *levels*.
            For contour lines ``len(allsegs) == len(levels)``, and for
            filled contour regions ``len(allsegs) = len(levels)-1``. The lists
            should look like ::

                level0segs = [polygon0, polygon1, ...]
                polygon0 = [[x0, y0], [x1, y1], ...]

        allkinds : [level0kinds, level1kinds, ...], optional
            Optional list of all the polygon vertex kinds (code types), as
            described and used in Path. This is used to allow multiply-
            connected paths such as holes within filled polygons.
            If not ``None``, ``len(allkinds) == len(allsegs)``. The lists
            should look like ::

                level0kinds = [polygon0kinds, ...]
                polygon0kinds = [vertexcode0, vertexcode1, ...]

            If *allkinds* is not ``None``, usually all polygons for a
            particular contour level are grouped together so that
            ``level0segs = [polygon0]`` and ``level0kinds = [polygon0kinds]``.

        **kwargs
            Keyword arguments are as described in the docstring of
            `~.Axes.contour`.
        """
        kwargs = {**cbook._given_keywords(QuadContourSet.__init__, locals(),
                                          exclude=("levels", "filled")), **kwargs}
        if levels is not None:
            kwargs["levels"] = levels
        ax._contour(filled, args, kwargs, made=self)


def _contour_args(fn, args, kwargs):
    """
    The field and levels of ``contour``/``contourf``'s positional
    arguments ``[X, Y,] Z [, levels]``, checked as matplotlib checks them:
    ``(z, mask, x, y, levels)``.
    """
    nargs = len(args)
    if 0 < nargs <= 2:
        z, *args = args
        z = ma.asarray(z)
        x = y = None
    elif 2 < nargs <= 4:
        x, y, z_orig, *args = args
        x = np.asarray(x, dtype=np.float64)
        y = np.asarray(y, dtype=np.float64)
        z = ma.asarray(z_orig)
        if z.ndim != 2:
            raise TypeError(f"Input z must be 2D, not {z.ndim}D")
        if z.shape[0] < 2 or z.shape[1] < 2:
            raise TypeError(f"Input z must be at least a (2, 2) shaped array, "
                            f"but has shape {z.shape}")
        Ny, Nx = z.shape
        if x.ndim != y.ndim:
            raise TypeError(f"Number of dimensions of x ({x.ndim}) and y "
                            f"({y.ndim}) do not match")
        if x.ndim == 1:
            if x.shape[0] != Nx:
                raise TypeError(f"Length of x ({x.shape[0]}) must match number of "
                                f"columns in z ({Nx})")
            if y.shape[0] != Ny:
                raise TypeError(f"Length of y ({y.shape[0]}) must match number of "
                                f"rows in z ({Ny})")
        elif x.ndim == 2:
            if x.shape != z.shape:
                raise TypeError(
                    f"Shapes of x {x.shape} and z {z.shape} do not match")
            if y.shape != z.shape:
                raise TypeError(
                    f"Shapes of y {y.shape} and z {z.shape} do not match")
        else:
            raise TypeError(f"Inputs x and y must be 1D or 2D, not {x.ndim}D")
    else:
        raise _api.nargs_error(fn, takes="from 1 to 4", given=nargs)
    if z.ndim != 2:
        raise TypeError(f"Input z must be 2D, not {z.ndim}D")
    if z.shape[0] < 2 or z.shape[1] < 2:
        raise TypeError(f"Input z must be at least a (2, 2) shaped array, "
                        f"but has shape {z.shape}")
    levels = kwargs.pop("levels", None)
    if levels is None and args:
        levels = args[0]
    if np.issubdtype(z.dtype, bool) and levels is None:
        levels = [0, .5, 1] if fn == "contourf" else [.5]
    z = ma.masked_invalid(z.astype(float), copy=False)
    if levels is not None and not isinstance(levels, Integral):
        levels = np.asarray(levels, np.float64)
        if fn == "contourf" and len(levels) < 2:
            raise ValueError("Filled contours require at least 2 levels.")
        if len(levels) > 1 and np.min(np.diff(levels)) <= 0.0:
            raise ValueError("Contour levels must be increasing")
    mask = ma.getmask(z)
    return (np.ma.filled(z, np.nan).astype(float),
            None if mask is ma.nomask or not mask.any() else np.asarray(mask, bool),
            x, y, levels)


__all__ = ["ContourLabeler", "ContourSet", "QuadContourSet"]


# What matplotlib's module also has from its imports, which programs
# import from it.
from contextlib import ExitStack  # noqa: E402, F401
from . import (  # noqa: E402, F401
    artist, cm, font_manager, patches as mpatches, ticker, transforms as mtransforms)
from .lines import Line2D  # noqa: E402, F401
from .text import Text  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
