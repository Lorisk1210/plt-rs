"""
Helper classes to adjust the positions of multiple axes at drawing time.

matplotlib's ``mpl_toolkits.axes_grid1.axes_divider`` (from matplotlib
3.11.2, under the Matplotlib License): the classes keep matplotlib's
attributes and methods, and each divider holds a pltrs (Rust) divider of
its figure (``_native.axes_grid1``), kept in step with its sizes, which
places the axes of its locators anew at every layout pass. matplotlib keeps
a divider's rectangle and sizes unchecked and fails on what it cannot
compute with as it places the axes: while a divider has such a value, its
axes are placed by calling their locators, which divide as matplotlib's do
(``Divider._locate``).
"""

import functools

import numpy as np

import pltrs as mpl
from pltrs import _api
from pltrs.gridspec import SubplotSpec
import pltrs.transforms as mtransforms
from pltrs._native import axes_grid1 as _axes_grid1
from pltrs.axes import _program_locators
from pltrs._offsetbox_trees import Unusable, number
from . import axes_size as Size


def _native_sizes(sizes):
    """The native descriptions of *sizes*; `.Unusable` for no list of
    sizes, or a size pltrs (Rust) cannot take."""
    try:
        sizes = list(sizes)
    except TypeError:
        raise Unusable from None
    return [Size._described(size) for size in sizes]


def _native_position(pos):
    """The rectangle *pos* as pltrs (Rust) takes it, four floats;
    `.Unusable` for anything else."""
    try:
        position = tuple(map(number, pos))
    except TypeError:
        raise Unusable from None
    if len(position) != 4:
        raise Unusable
    return position


class Divider:
    """
    An Axes positioning class.

    The divider is initialized with lists of horizontal and vertical sizes
    (:mod:`mpl_toolkits.axes_grid1.axes_size`) based on which a given
    rectangular area will be divided.

    The `new_locator` method then creates a callable object
    that can be used as the *axes_locator* of the axes.
    """

    def __init__(self, fig, pos, horizontal, vertical,
                 aspect=None, anchor="C"):
        """
        Parameters
        ----------
        fig : Figure
        pos : tuple of 4 floats
            Position of the rectangle that will be divided.
        horizontal : list of :mod:`~mpl_toolkits.axes_grid1.axes_size`
            Sizes for horizontal division.
        vertical : list of :mod:`~mpl_toolkits.axes_grid1.axes_size`
            Sizes for vertical division.
        aspect : bool, optional
            Whether overall rectangular area is reduced so that the relative
            part of the horizontal and vertical scales have the same scale.
        anchor : (float, float) or {'C', 'SW', 'S', 'SE', 'E', 'NE', 'N', \
'NW', 'W'}, default: 'C'
            Placement of the reduced rectangle, when *aspect* is True.
        """
        self._fig = fig
        self._pos = pos
        self._horizontal = horizontal
        self._vertical = vertical
        self._anchor = anchor
        self.set_anchor(anchor)
        self._aspect = aspect
        self._xrefindex = 0
        self._yrefindex = 0
        self._locator = None

    @property
    def _native(self):
        """The native divider, made once the divider first divides: like
        matplotlib's, the constructor reads nothing of what it divides
        (yellowbrick lays a mocked Axes out)."""
        native = self.__dict__.get("_native_divider")
        if native is None:
            native = self.__dict__["_native_divider"] = self._make_native()
            self._sync()
        return native

    def _make_native(self):
        """The native divider of the rectangle this divider divides (the
        unit rectangle, undivided, for one pltrs (Rust) cannot take, see
        `_sync`)."""
        try:
            position = _native_position(self._pos)
        except Unusable:
            position = (0., 0., 1., 1.)
        return _axes_grid1.divider_new(self._fig._native, position, [], [])

    def _sync(self):
        """Give the native divider the rectangle, sizes, aspect and anchor;
        while it cannot take one of them, have the axes placed by calling
        their locators (`_divide_in_python`)."""
        native = self.__dict__.get("_native_divider")
        if native is None:
            return
        try:
            position = self._native_rectangle()
            horizontal = _native_sizes(self._horizontal)
            vertical = _native_sizes(self._vertical)
            for cell in self.__dict__.get("_placed", {}).values():
                self._check_cell(cell, horizontal, vertical)
        except Unusable:
            self._divide_in_python(True)
            return
        if position is not None:
            _axes_grid1.divider_set_position(native, position)
        _axes_grid1.divider_set_horizontal(native, horizontal)
        _axes_grid1.divider_set_vertical(native, vertical)
        if self._aspect is not None:
            _axes_grid1.divider_set_aspect(native, bool(self._aspect))
        _axes_grid1.divider_set_anchor(native, self._anchor)
        self._divide_in_python(False)

    def _check_cell(self, cell, horizontal, vertical):
        """Raise `.Unusable` for the *cell* of a locator (its ``(nx, ny,
        nx1, ny1)``) past the offsets of the *horizontal* and *vertical*
        sizes, where matplotlib's division fails to index them."""
        (nx, nx1), (ny, ny1) = self._cells(*cell)
        for indices, sizes in (((nx, nx1), horizontal), ((ny, ny1), vertical)):
            if not all(-len(sizes) - 1 <= index <= len(sizes) for index in indices):
                raise Unusable

    def _native_rectangle(self):
        """The rectangle pltrs (Rust) divides, as it takes it (`.Unusable`
        for one it cannot take); None for one it follows itself (a subplot
        box, an Axes) or a locator gives."""
        return None if self._locator is not None else _native_position(self._pos)

    def _divide_in_python(self, in_python):
        """Place the axes of the divider's locators by calling the locators
        (*in_python*), as matplotlib does at every draw, or natively."""
        if in_python == self.__dict__.get("_in_python", False):
            return
        self._in_python = in_python
        for axes, cell in self.__dict__.get("_placed", {}).items():
            if in_python:
                _program_locators.follow(axes, axes._figure)
            else:
                _program_locators.forget(axes, axes._figure)
                _axes_grid1.divider_place(self._native, axes._native, *self._cells(*cell))

    def get_horizontal_sizes(self, renderer):
        return np.array([s.get_size(renderer) for s in self.get_horizontal()])

    def get_vertical_sizes(self, renderer):
        return np.array([s.get_size(renderer) for s in self.get_vertical()])

    def set_position(self, pos):
        """
        Set the position of the rectangle.

        Parameters
        ----------
        pos : tuple of 4 floats
            position of the rectangle that will be divided
        """
        self._pos = pos
        self._sync()

    def get_position(self):
        """Return the position of the rectangle."""
        return self._pos

    def set_anchor(self, anchor):
        """
        Parameters
        ----------
        anchor : (float, float) or {'C', 'SW', 'S', 'SE', 'E', 'NE', 'N', \
'NW', 'W'}
            Either an (*x*, *y*) pair of relative coordinates (0 is left or
            bottom, 1 is right or top), 'C' (center), or a cardinal direction
            ('SW', southwest, is bottom left, etc.).

        See Also
        --------
        .Axes.set_anchor
        """
        if isinstance(anchor, str):
            _api.check_in_list(mtransforms.Bbox.coefs, anchor=anchor)
        elif not isinstance(anchor, (tuple, list)) or len(anchor) != 2:
            raise TypeError("anchor must be str or 2-tuple")
        self._anchor = anchor
        self._sync()

    def get_anchor(self):
        """Return the anchor."""
        return self._anchor

    def get_subplotspec(self):
        return None

    def set_horizontal(self, h):
        """
        Parameters
        ----------
        h : list of :mod:`~mpl_toolkits.axes_grid1.axes_size`
            sizes for horizontal division
        """
        self._horizontal = h
        self._sync()

    def get_horizontal(self):
        """Return horizontal sizes."""
        return self._horizontal

    def set_vertical(self, v):
        """
        Parameters
        ----------
        v : list of :mod:`~mpl_toolkits.axes_grid1.axes_size`
            sizes for vertical division
        """
        self._vertical = v
        self._sync()

    def get_vertical(self):
        """Return vertical sizes."""
        return self._vertical

    def set_aspect(self, aspect=False):
        """
        Parameters
        ----------
        aspect : bool
        """
        self._aspect = aspect
        self._sync()

    def get_aspect(self):
        """Return aspect."""
        return self._aspect

    def set_locator(self, _locator):
        root = self._fig.get_figure(root=True)
        _program_locators.forget(self, root)
        located, self._locator = self._locator, _locator
        if _locator is not None:
            _program_locators.follow(self, root)
        elif located is not None:
            _axes_grid1.divider_set_position(
                self._native, tuple(map(float, self.get_position())))

    def _locate_by_program(self, renderer):
        """Divide the rectangle the locator gives now, if the divider
        places an Axes to ask it with (see `.axes._program_locators`)."""
        placed = self.__dict__.get("_placed")
        if placed:
            _axes_grid1.divider_set_position(
                self._native, tuple(map(float, self.get_position_runtime(
                    next(iter(placed)), renderer))))

    def get_locator(self):
        return self._locator

    def get_position_runtime(self, ax, renderer):
        if self._locator is None:
            return self.get_position()
        else:
            return self._locator(ax, renderer).bounds

    def new_locator(self, nx, ny, nx1=None, ny1=None):
        """
        Return an axes locator callable for the specified cell.

        Parameters
        ----------
        nx, nx1 : int
            Integers specifying the column-position of the
            cell. When *nx1* is None, a single *nx*-th column is
            specified. Otherwise, location of columns spanning between *nx*
            to *nx1* (but excluding *nx1*-th column) is specified.
        ny, ny1 : int
            Same as *nx* and *nx1*, but for row positions.
        """
        if nx1 is None:
            nx1 = nx + 1
        if ny1 is None:
            ny1 = ny + 1
        # append_size("left") adds a new size at the beginning of the
        # horizontal size lists; this shift transforms e.g.
        # new_locator(nx=2, ...) into effectively new_locator(nx=3, ...).  To
        # take that into account, instead of recording nx, we record
        # nx-self._xrefindex, where _xrefindex is shifted by 1 by each
        # append_size("left"), and re-add self._xrefindex back to nx in
        # _locate, when the actual axes position is computed.  Ditto for y.
        xref = self._xrefindex
        yref = self._yrefindex
        args = (nx - xref, ny - yref, nx1 - xref, ny1 - yref)
        locator = functools.partial(self._locate, *args)
        locator.get_subplotspec = self.get_subplotspec
        locator._place = functools.partial(self._place, *args)
        return locator

    def _cells(self, nx, ny, nx1, ny1):
        """The columns and rows of a locator's cell, as the native divider
        counts them."""
        return ((nx + self._xrefindex, nx1 + self._xrefindex),
                (ny + self._yrefindex, ny1 + self._yrefindex))

    def _locate(self, nx, ny, nx1, ny1, axes, renderer):
        """
        Implementation of ``divider.new_locator().__call__``.

        The axes locator callable returned by ``new_locator()`` is created as
        a `functools.partial` of this method with *nx*, *ny*, *nx1*, and *ny1*
        specifying the requested cell.
        """
        native = self._native
        if self.__dict__.get("_in_python"):
            return self._locate_as_given(nx, ny, nx1, ny1, axes, renderer)
        return mtransforms.Bbox.from_bounds(
            *_axes_grid1.divider_locate(native, *self._cells(nx, ny, nx1, ny1)))

    @staticmethod
    def _calc_k(sizes, total):
        # sizes is a (n, 2) array of (rel_size, abs_size); this method finds
        # the k factor such that sum(rel_size * k + abs_size) == total.
        rel_sum, abs_sum = sizes.sum(0)
        return (total - abs_sum) / rel_sum if rel_sum else 0

    @staticmethod
    def _calc_offsets(sizes, k):
        # Apply k factors to (n, 2) sizes array of (rel_size, abs_size); return
        # the resulting cumulative offset positions.
        return np.cumsum([0, *(sizes @ [k, 1])])

    def _locate_as_given(self, nx, ny, nx1, ny1, axes, renderer):
        """matplotlib's division itself, with the rectangle and sizes as
        the program gave them (which pltrs (Rust) cannot take), failing
        where matplotlib's fails on them."""
        (nx, nx1), (ny, ny1) = self._cells(nx, ny, nx1, ny1)
        fig_w, fig_h = self._fig.bbox.size / self._fig.dpi
        x, y, w, h = self.get_position_runtime(axes, renderer)
        hsizes = self.get_horizontal_sizes(renderer)
        vsizes = self.get_vertical_sizes(renderer)
        k_h = self._calc_k(hsizes, fig_w * w)
        k_v = self._calc_k(vsizes, fig_h * h)
        if self.get_aspect():
            k = min(k_h, k_v)
            ox = self._calc_offsets(hsizes, k)
            oy = self._calc_offsets(vsizes, k)
            ww = (ox[-1] - ox[0]) / fig_w
            hh = (oy[-1] - oy[0]) / fig_h
            pb = mtransforms.Bbox.from_bounds(x, y, w, h)
            pb1 = mtransforms.Bbox.from_bounds(x, y, ww, hh)
            x0, y0 = pb1.anchored(self.get_anchor(), pb).p0
        else:
            ox = self._calc_offsets(hsizes, k_h)
            oy = self._calc_offsets(vsizes, k_v)
            x0, y0 = x, y
        x1, w1 = x0 + ox[nx] / fig_w, (ox[nx1] - ox[nx]) / fig_w
        y1, h1 = y0 + oy[ny] / fig_h, (oy[ny1] - oy[ny]) / fig_h
        return mtransforms.Bbox.from_bounds(x1, y1, w1, h1)

    def _place(self, nx, ny, nx1, ny1, axes):
        """Make pltrs (Rust) place *axes* in the cell at every layout
        (``axes.set_axes_locator(locator)``), or have it placed by calling
        the locator while the divider divides in Python (see `_sync`)."""
        native = self._native
        self.__dict__.setdefault("_placed", {})[axes] = (nx, ny, nx1, ny1)
        if not self.__dict__.get("_in_python"):
            _axes_grid1.divider_place(native, axes._native,
                                      *self._cells(nx, ny, nx1, ny1))
        # A cell past the sizes makes the divider divide in Python.
        self._sync()
        if self.__dict__.get("_in_python"):
            _program_locators.follow(axes, axes._figure)

    def append_size(self, position, size):
        _api.check_in_list(["left", "right", "bottom", "top"],
                           position=position)
        if position == "left":
            self._horizontal.insert(0, size)
            self._xrefindex += 1
        elif position == "right":
            self._horizontal.append(size)
        elif position == "bottom":
            self._vertical.insert(0, size)
            self._yrefindex += 1
        else:  # 'top'
            self._vertical.append(size)
        try:
            described = Size._described(size)
        except Unusable:
            # A stand-in keeps the native divider's cells in step; the axes
            # are placed by their locators meanwhile (see `_sync`).
            described = ("fixed", 0.)
        _axes_grid1.divider_append_size(self._native, position, described)
        self._sync()

    def add_auto_adjustable_area(self, use_axes, pad=0.1, adjust_dirs=None):
        """
        Add auto-adjustable padding around *use_axes* to take their decorations
        (title, labels, ticks, ticklabels) into account during layout.

        Parameters
        ----------
        use_axes : `~matplotlib.axes.Axes` or list of `~matplotlib.axes.Axes`
            The Axes whose decorations are taken into account.
        pad : float, default: 0.1
            Additional padding in inches.
        adjust_dirs : list of {"left", "right", "bottom", "top"}, optional
            The sides where padding is added; defaults to all four sides.
        """
        if adjust_dirs is None:
            adjust_dirs = ["left", "right", "bottom", "top"]
        for d in adjust_dirs:
            self.append_size(d, Size._AxesDecorationsSize(use_axes, d) + pad)


class SubplotDivider(Divider):
    """
    The Divider class whose rectangle area is specified as a subplot geometry.
    """

    _kind = "grid"

    def __init__(self, fig, *args, horizontal=None, vertical=None,
                 aspect=None, anchor='C'):
        """
        Parameters
        ----------
        fig : `~matplotlib.figure.Figure`

        *args : tuple (*nrows*, *ncols*, *index*) or int
            The array of subplots in the figure has dimensions ``(nrows,
            ncols)``, and *index* is the index of the subplot being created.
            *index* starts at 1 in the upper left corner and increases to the
            right.

            If *nrows*, *ncols*, and *index* are all single digit numbers, then
            *args* can be passed as a single 3-digit number (e.g. 234 for
            (2, 3, 4)).
        horizontal : list of :mod:`~mpl_toolkits.axes_grid1.axes_size`, optional
            Sizes for horizontal division.
        vertical : list of :mod:`~mpl_toolkits.axes_grid1.axes_size`, optional
            Sizes for vertical division.
        aspect : bool, optional
            Whether overall rectangular area is reduced so that the relative
            part of the horizontal and vertical scales have the same scale.
        anchor : (float, float) or {'C', 'SW', 'S', 'SE', 'E', 'NE', 'N', \
'NW', 'W'}, default: 'C'
            Placement of the reduced rectangle, when *aspect* is True.
        """
        self.figure = fig
        self._subplotspec = SubplotSpec._from_subplot_args(fig, args)
        super().__init__(fig, [0, 0, 1, 1],
                         horizontal=horizontal or [], vertical=vertical or [],
                         aspect=aspect, anchor=anchor)
        self.set_subplotspec(self._subplotspec)

    def _make_native(self):
        # The sizes come with `_sync`.
        return _axes_grid1.divider_subplot(
            self._fig._native, self._subplotspec._native_spec(), self._kind, [], [])

    def _native_rectangle(self):
        # The subplot box, which pltrs (Rust) follows, is divided.
        return None

    def set_position(self, pos):
        # The subplot box, not this position, is divided (`get_position`).
        self._pos = pos

    def get_position(self):
        """Return the bounds of the subplot box."""
        return self.get_subplotspec().get_position(self.figure).bounds

    def get_subplotspec(self):
        """Get the SubplotSpec instance."""
        return self._subplotspec

    def set_subplotspec(self, subplotspec):
        """Set the SubplotSpec instance."""
        self._subplotspec = subplotspec
        position = subplotspec.get_position(self.figure)
        _axes_grid1.divider_set_subplotspec(self._native,
                                            subplotspec._native_spec())
        self.set_position(position)


class AxesDivider(Divider):
    """
    Divider based on the preexisting axes.
    """

    def __init__(self, axes, xref=None, yref=None):
        """
        Parameters
        ----------
        axes : :class:`~matplotlib.axes.Axes`
        xref
        yref
        """
        self._axes = axes
        if xref is None:
            self._xref = Size.AxesX(axes)
        else:
            self._xref = xref
        if yref is None:
            self._yref = Size.AxesY(axes)
        else:
            self._yref = yref

        super().__init__(fig=axes.get_figure(), pos=None,
                         horizontal=[self._xref], vertical=[self._yref],
                         aspect=None, anchor="C")

    def _make_native(self):
        return _axes_grid1.divider_of_axes(self._axes._native)

    def _native_rectangle(self):
        # docstring inherited: the Axes' box, which pltrs (Rust) follows,
        # unless given one.
        return None if self._pos is None else super()._native_rectangle()

    def _get_new_axes(self, *, axes_class=None, **kwargs):
        axes = self._axes
        if axes_class is None:
            axes_class = type(axes)
        return axes_class(axes.get_figure(), axes.get_position(original=True),
                          **kwargs)

    def new_horizontal(self, size, pad=None, pack_start=False, **kwargs):
        """
        Helper method for ``append_axes("left")`` and ``append_axes("right")``.

        See the documentation of `append_axes` for more details.

        :meta private:
        """
        if pad is None:
            pad = mpl.rcParams["figure.subplot.wspace"] * self._xref
        pos = "left" if pack_start else "right"
        if pad:
            if not isinstance(pad, Size._Base):
                pad = Size.from_any(pad, fraction_ref=self._xref)
            self.append_size(pos, pad)
        if not isinstance(size, Size._Base):
            size = Size.from_any(size, fraction_ref=self._xref)
        self.append_size(pos, size)
        locator = self.new_locator(
            nx=0 if pack_start else len(self._horizontal) - 1,
            ny=self._yrefindex)
        ax = self._get_new_axes(**kwargs)
        ax.set_axes_locator(locator)
        return ax

    def new_vertical(self, size, pad=None, pack_start=False, **kwargs):
        """
        Helper method for ``append_axes("top")`` and ``append_axes("bottom")``.

        See the documentation of `append_axes` for more details.

        :meta private:
        """
        if pad is None:
            pad = mpl.rcParams["figure.subplot.hspace"] * self._yref
        pos = "bottom" if pack_start else "top"
        if pad:
            if not isinstance(pad, Size._Base):
                pad = Size.from_any(pad, fraction_ref=self._yref)
            self.append_size(pos, pad)
        if not isinstance(size, Size._Base):
            size = Size.from_any(size, fraction_ref=self._yref)
        self.append_size(pos, size)
        locator = self.new_locator(
            nx=self._xrefindex,
            ny=0 if pack_start else len(self._vertical) - 1)
        ax = self._get_new_axes(**kwargs)
        ax.set_axes_locator(locator)
        return ax

    def append_axes(self, position, size, pad=None, *, axes_class=None,
                    **kwargs):
        """
        Add a new axes on a given side of the main axes.

        Parameters
        ----------
        position : {"left", "right", "bottom", "top"}
            Where the new axes is positioned relative to the main axes.
        size : :mod:`~mpl_toolkits.axes_grid1.axes_size` or float or str
            The axes width or height.  float or str arguments are interpreted
            as ``axes_size.from_any(size, AxesX(<main_axes>))`` for left or
            right axes, and likewise with ``AxesY`` for bottom or top axes.
        pad : :mod:`~mpl_toolkits.axes_grid1.axes_size` or float or str
            Padding between the axes.  float or str arguments are interpreted
            as for *size*.  Defaults to :rc:`figure.subplot.wspace` times the
            main Axes width (left or right axes) or :rc:`figure.subplot.hspace`
            times the main Axes height (bottom or top axes).
        axes_class : subclass type of `~.axes.Axes`, optional
            The type of the new axes.  Defaults to the type of the main axes.
        **kwargs
            All extra keywords arguments are passed to the created axes.
        """
        create_axes, pack_start = _api.getitem_checked({
            "left": (self.new_horizontal, True),
            "right": (self.new_horizontal, False),
            "bottom": (self.new_vertical, True),
            "top": (self.new_vertical, False),
        }, position=position)
        ax = create_axes(
            size, pad, pack_start=pack_start, axes_class=axes_class, **kwargs)
        self._fig.add_axes(ax)
        return ax

    def get_aspect(self):
        if self._aspect is None:
            aspect = self._axes.get_aspect()
            if aspect == "auto":
                return False
            else:
                return True
        else:
            return self._aspect

    def get_position(self):
        if self._pos is None:
            bbox = self._axes.get_position(original=True)
            return bbox.bounds
        else:
            return self._pos

    def get_anchor(self):
        if self._anchor is None:
            return self._axes.get_anchor()
        else:
            return self._anchor

    def get_subplotspec(self):
        return self._axes.get_subplotspec()


class HBoxDivider(SubplotDivider):
    """
    A `.SubplotDivider` for laying out axes horizontally, while ensuring that
    they have equal heights.

    Examples
    --------
    .. plot:: gallery/axes_grid1/demo_axes_hbox_divider.py
    """

    _kind = "hbox"

    def new_locator(self, nx, nx1=None):
        """
        Create an axes locator callable for the specified cell.

        Parameters
        ----------
        nx, nx1 : int
            Integers specifying the column-position of the
            cell. When *nx1* is None, a single *nx*-th column is
            specified. Otherwise, location of columns spanning between *nx*
            to *nx1* (but excluding *nx1*-th column) is specified.
        """
        return super().new_locator(nx, 0, nx1, 0)


class VBoxDivider(SubplotDivider):
    """
    A `.SubplotDivider` for laying out axes vertically, while ensuring that
    they have equal widths.
    """

    _kind = "vbox"

    def new_locator(self, ny, ny1=None):
        """
        Create an axes locator callable for the specified cell.

        Parameters
        ----------
        ny, ny1 : int
            Integers specifying the row-position of the
            cell. When *ny1* is None, a single *ny*-th row is
            specified. Otherwise, location of rows spanning between *ny*
            to *ny1* (but excluding *ny1*-th row) is specified.
        """
        return super().new_locator(0, ny, 0, ny1)


def make_axes_locatable(axes):
    divider = AxesDivider(axes)
    locator = divider.new_locator(nx=0, ny=0)
    axes.set_axes_locator(locator)

    return divider


def make_axes_area_auto_adjustable(
        ax, use_axes=None, pad=0.1, adjust_dirs=None):
    """
    Add auto-adjustable padding around *ax* to take its decorations (title,
    labels, ticks, ticklabels) into account during layout, using
    `.Divider.add_auto_adjustable_area`.

    By default, padding is determined from the decorations of *ax*.
    Pass *use_axes* to consider the decorations of other Axes instead.
    """
    if adjust_dirs is None:
        adjust_dirs = ["left", "right", "bottom", "top"]
    divider = make_axes_locatable(ax)
    if use_axes is None:
        use_axes = ax
    divider.add_auto_adjustable_area(use_axes=use_axes, pad=pad,
                                     adjust_dirs=adjust_dirs)
