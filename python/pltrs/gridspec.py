r"""
:mod:`~matplotlib.gridspec` contains classes that help to layout multiple
`~.axes.Axes` in a grid-like pattern within a figure.

The `GridSpec` specifies the overall grid structure. Individual cells within
the grid are referenced by `SubplotSpec`\s.

The classes and their API are matplotlib 3.11.2's, under the Matplotlib
License. Each grid holds the native grid of pltrs (Rust) (``pltrs::gridspec``)
for its whole life, as pltrs (Rust) and its layout engines tell grids apart
by identity; the geometry (`GridSpecBase.get_grid_positions`,
`SubplotSpec.get_position`) is computed there.
"""

from numbers import Integral
import weakref

import numpy as np

from . import _api, _pylab_helpers, rcParams
from ._api import UNSET as _UNSET
from ._native import figure_layout as _figure_layout, gridspec as _gridspec
from ._program_classes import takes_from_program
from .transforms import Bbox

#: The margin and spacing names of a grid, in the order they cross to
#: pltrs (Rust).
_KEYS = ("left", "bottom", "right", "top", "wspace", "hspace")

#: The Python object of each native grid, found by identity (see `_wrap`).
_GRIDSPECS = weakref.WeakSet()


def _figure_params(figure):
    """The subplot parameters of *figure*, else of rcParams, as a tuple."""
    if figure is None:
        return tuple(rcParams[f"figure.subplot.{key}"] for key in _KEYS)
    return _gridspec.figure_params(figure._native)


def _wrap(native, figure=None):
    """The Python grid of the native grid *native*: the one made before, or
    a new one around it."""
    for gridspec in _GRIDSPECS:
        if _gridspec.same(gridspec._native, native):
            return gridspec
    parent = _gridspec.parent(native)
    if parent is None:
        return GridSpec._from_native(native, figure)
    return GridSpecFromSubplotSpec._from_native(
        native, SubplotSpec._from_native(parent, figure))


class GridSpecBase:
    """
    A base class of GridSpec that specifies the geometry of the grid
    that a subplot will be placed.
    """

    def __init__(self, nrows, ncols, height_ratios=None, width_ratios=None):
        """
        Parameters
        ----------
        nrows, ncols : int
            The number of rows and columns of the grid.
        width_ratios : array-like of length *ncols*, optional
            Defines the relative widths of the columns. Each column gets a
            relative width of ``width_ratios[i] / sum(width_ratios)``.
            If not given, all columns will have the same width.
        height_ratios : array-like of length *nrows*, optional
            Defines the relative heights of the rows. Each row gets a
            relative height of ``height_ratios[i] / sum(height_ratios)``.
            If not given, all rows will have the same height.
        """
        if not isinstance(nrows, Integral) or nrows <= 0:
            raise ValueError(
                f"Number of rows must be a positive integer, not {nrows!r}")
        if not isinstance(ncols, Integral) or ncols <= 0:
            raise ValueError(
                f"Number of columns must be a positive integer, not {ncols!r}")
        self._nrows, self._ncols = nrows, ncols
        self.set_height_ratios(height_ratios)
        self.set_width_ratios(width_ratios)

    def _adopt_native(self, native):
        """Hold the native grid *native* (and register this grid)."""
        self._native = native
        _GRIDSPECS.add(self)

    def __getattr__(self, name):
        # A grid of a class of the program's deriving from this one alone
        # (plotnine's ``p9GridSpec``) gets its native grid when first
        # needed, without margins or spacing of its own: its
        # ``get_subplot_params`` gives them (`_params_of`).
        if name == "_native" and "_nrows" in self.__dict__:
            native = _gridspec.new(
                self._nrows, self._ncols,
                [float(r) for r in self._col_width_ratios],
                [float(r) for r in self._row_height_ratios], (None,) * len(_KEYS))
            # Constrained layout only places the grids of pltrs' classes.
            _gridspec.set_in_layout(native, False)
            self._adopt_native(native)
            return native
        raise AttributeError(
            f"{type(self).__name__!r} object has no attribute {name!r}")

    def _params_of(self, figure):
        """The subplot parameters the positions of the grid's cells in
        *figure* follow, as a tuple: those of *figure* (else of rcParams),
        or those the grid's class of the program's gives
        (``get_subplot_params``), as matplotlib's `get_grid_positions`
        asks for them."""
        if not takes_from_program(type(self), "get_subplot_params"):
            return _figure_params(figure)
        params = self.get_subplot_params(figure)
        return tuple(getattr(params, key) for key in _KEYS)

    def __repr__(self):
        height_arg = (f', height_ratios={self._row_height_ratios!r}'
                      if len(set(self._row_height_ratios)) != 1 else '')
        width_arg = (f', width_ratios={self._col_width_ratios!r}'
                     if len(set(self._col_width_ratios)) != 1 else '')
        return '{clsname}({nrows}, {ncols}{optionals})'.format(
            clsname=self.__class__.__name__,
            nrows=self._nrows,
            ncols=self._ncols,
            optionals=height_arg + width_arg,
            )

    nrows = property(lambda self: self._nrows,
                     doc="The number of rows in the grid.")
    ncols = property(lambda self: self._ncols,
                     doc="The number of columns in the grid.")

    def get_geometry(self):
        """
        Return a tuple containing the number of rows and columns in the grid.
        """
        return self._nrows, self._ncols

    def new_subplotspec(self, loc, rowspan=1, colspan=1):
        """
        Create and return a `.SubplotSpec` instance.

        Parameters
        ----------
        loc : (int, int)
            The position of the subplot in the grid as
            ``(row_index, column_index)``.
        rowspan, colspan : int, default: 1
            The number of rows and columns the subplot should span in the grid.
        """
        loc1, loc2 = loc
        subplotspec = self[loc1:loc1+rowspan, loc2:loc2+colspan]
        return subplotspec

    def set_width_ratios(self, width_ratios):
        """
        Set the relative widths of the columns.

        *width_ratios* must be of length *ncols*. Each column gets a relative
        width of ``width_ratios[i] / sum(width_ratios)``.
        """
        if width_ratios is None:
            width_ratios = [1] * self._ncols
        elif len(width_ratios) != self._ncols:
            raise ValueError('Expected the given number of width ratios to '
                             'match the number of columns of the grid')
        self._col_width_ratios = width_ratios
        native = self.__dict__.get("_native")
        if native is not None:
            _gridspec.set_width_ratios(native, [float(r) for r in width_ratios])

    def set_height_ratios(self, height_ratios):
        """
        Set the relative heights of the rows.

        *height_ratios* must be of length *nrows*. Each row gets a relative
        height of ``height_ratios[i] / sum(height_ratios)``.
        """
        if height_ratios is None:
            height_ratios = [1] * self._nrows
        elif len(height_ratios) != self._nrows:
            raise ValueError('Expected the given number of height ratios to '
                             'match the number of rows of the grid')
        self._row_height_ratios = height_ratios
        native = self.__dict__.get("_native")
        if native is not None:
            _gridspec.set_height_ratios(native, [float(r) for r in height_ratios])

    def get_width_ratios(self):
        """
        Return the width ratios.

        This is *None* if no width ratios have been set explicitly.
        """
        return self._col_width_ratios

    def get_height_ratios(self):
        """
        Return the height ratios.

        This is *None* if no height ratios have been set explicitly.
        """
        return self._row_height_ratios

    def get_subplot_params(self, figure=None):
        # Must be implemented in subclasses
        pass

    def get_grid_positions(self, fig):
        """
        Return the positions of the grid cells in figure coordinates.

        Parameters
        ----------
        fig : `~matplotlib.figure.Figure`
            The figure the grid should be applied to. The subplot parameters
            (margins and spacing between subplots) are taken from *fig*.

        Returns
        -------
        bottoms, tops, lefts, rights : array
            The bottom, top, left, right positions of the grid cells in
            figure coordinates.
        """
        return _gridspec.grid_positions(self._native, self._params_of(fig))

    @staticmethod
    def _check_gridspec_exists(figure, nrows, ncols):
        """
        Check if the figure already has a gridspec with these dimensions,
        or create a new one
        """
        for ax in figure.get_axes():
            gs = ax.get_gridspec()
            if gs is not None:
                if hasattr(gs, 'get_topmost_subplotspec'):
                    gs = gs.get_topmost_subplotspec().get_gridspec()
                if gs.get_geometry() == (nrows, ncols):
                    return gs
        return GridSpec(nrows, ncols, figure=figure)

    def __getitem__(self, key):
        """Create and return a `.SubplotSpec` instance."""
        nrows, ncols = self.get_geometry()

        def _normalize(key, size, axis):  # Includes last index.
            orig_key = key
            if isinstance(key, slice):
                start, stop, _ = key.indices(size)
                if stop > start:
                    return start, stop - 1
                raise IndexError("GridSpec slice would result in no space "
                                 "allocated for subplot")
            else:
                if key < 0:
                    key = key + size
                if 0 <= key < size:
                    return key, key
                elif axis is not None:
                    raise IndexError(f"index {orig_key} is out of bounds for "
                                     f"axis {axis} with size {size}")
                else:  # flat index
                    raise IndexError(f"index {orig_key} is out of bounds for "
                                     f"GridSpec with size {size}")

        if isinstance(key, tuple):
            try:
                k1, k2 = key
            except ValueError as err:
                raise ValueError("Unrecognized subplot spec") from err
            num1, num2 = np.ravel_multi_index(
                [_normalize(k1, nrows, 0), _normalize(k2, ncols, 1)],
                (nrows, ncols))
        else:  # Single key
            num1, num2 = _normalize(key, nrows * ncols, None)

        return SubplotSpec(self, num1, num2)

    def subplots(self, *, sharex=False, sharey=False, squeeze=True,
                 subplot_kw=None):
        """
        Add all subplots specified by this `GridSpec` to its parent figure.

        See `.Figure.subplots` for detailed documentation.
        """
        figure = self.figure

        if figure is None:
            raise ValueError("GridSpec.subplots() only works for GridSpecs "
                             "created with a parent figure")

        if not isinstance(sharex, str):
            sharex = "all" if sharex else "none"
        if not isinstance(sharey, str):
            sharey = "all" if sharey else "none"

        _api.check_in_list(["all", "row", "col", "none", False, True],
                           sharex=sharex, sharey=sharey)
        if subplot_kw is None:
            subplot_kw = {}
        # don't mutate kwargs passed by user...
        subplot_kw = subplot_kw.copy()

        # Create array to hold all Axes.
        axarr = np.empty((self._nrows, self._ncols), dtype=object)
        for row in range(self._nrows):
            for col in range(self._ncols):
                shared_with = {"none": None, "all": axarr[0, 0],
                               "row": axarr[row, 0], "col": axarr[0, col]}
                subplot_kw["sharex"] = shared_with[sharex]
                subplot_kw["sharey"] = shared_with[sharey]
                axarr[row, col] = figure.add_subplot(
                    self[row, col], **subplot_kw)

        # turn off redundant tick labeling
        from ._native import axes_base as _axes_base
        if sharex in ["col", "all"]:
            for ax in axarr.flat:
                _axes_base.label_outer_shared(ax._native, "x")
        if sharey in ["row", "all"]:
            for ax in axarr.flat:
                _axes_base.label_outer_shared(ax._native, "y")

        if squeeze:
            # Discarding unneeded dimensions that equal 1.  If we only have one
            # subplot, just return it instead of a 1-element array.
            return axarr.item() if axarr.size == 1 else axarr.squeeze()
        else:
            # Returned axis array will be always 2-d, even if nrows=ncols=1.
            return axarr


class GridSpec(GridSpecBase):
    """
    A grid layout to place subplots within a figure.

    The location of the grid cells is determined in a similar way to
    `.SubplotParams` using *left*, *right*, *top*, *bottom*, *wspace*
    and *hspace*.

    Indexing a GridSpec instance returns a `.SubplotSpec`.
    """

    def __init__(self, nrows, ncols, figure=None,
                 left=None, bottom=None, right=None, top=None,
                 wspace=None, hspace=None,
                 width_ratios=None, height_ratios=None):
        """
        Parameters
        ----------
        nrows, ncols : int
            The number of rows and columns of the grid.

        figure : `.Figure`, optional
            Only used for constrained layout to create a proper layoutgrid.

        left, right, top, bottom : float, optional
            Extent of the subplots as a fraction of figure width or height.
            Left cannot be larger than right, and bottom cannot be larger than
            top. If not given, the values will be inferred from a figure or
            rcParams at draw time. See also `GridSpec.get_subplot_params`.

        wspace : float, optional
            The amount of width reserved for space between subplots,
            expressed as a fraction of the average axis width.
            If not given, the values will be inferred from a figure or
            rcParams when necessary. See also `GridSpec.get_subplot_params`.

        hspace : float, optional
            The amount of height reserved for space between subplots,
            expressed as a fraction of the average axis height.
            If not given, the values will be inferred from a figure or
            rcParams when necessary. See also `GridSpec.get_subplot_params`.

        width_ratios : array-like of length *ncols*, optional
            Defines the relative widths of the columns. Each column gets a
            relative width of ``width_ratios[i] / sum(width_ratios)``.
            If not given, all columns will have the same width.

        height_ratios : array-like of length *nrows*, optional
            Defines the relative heights of the rows. Each row gets a
            relative height of ``height_ratios[i] / sum(height_ratios)``.
            If not given, all rows will have the same height.

        """
        self.figure = figure
        super().__init__(nrows, ncols,
                         width_ratios=width_ratios,
                         height_ratios=height_ratios)
        self._adopt_native(_gridspec.new(
            nrows, ncols,
            [float(r) for r in self._col_width_ratios],
            [float(r) for r in self._row_height_ratios],
            (left, bottom, right, top, wspace, hspace)))
        if figure is None:
            # Constrained layout only places the grids of a figure.
            _gridspec.set_in_layout(self._native, False)

    @classmethod
    def _from_native(cls, native, figure):
        """The grid around the native grid *native* (one pltrs (Rust) made)."""
        gridspec = cls.__new__(cls)
        gridspec.figure = figure
        nrows, ncols = _gridspec.shape(native)
        width_ratios, height_ratios = _gridspec.ratios(native)
        GridSpecBase.__init__(gridspec, nrows, ncols, height_ratios, width_ratios)
        gridspec._adopt_native(native)
        return gridspec

    def _override(index):
        """The property of the margin or spacing *index* of `_KEYS`, which
        the native grid holds: the numpy float a tight layout computed, as
        matplotlib's `tight_layout` sets it, while it stays."""
        def get(self):
            value = _gridspec.overrides(self._native)[index]
            computed = self.__dict__.get("_tight_overrides", {}).get(index)
            return np.float64(value) if value is not None and value == computed else value

        def set(self, value):
            values = list(_gridspec.overrides(self._native))
            values[index] = value
            _gridspec.set_overrides(self._native, tuple(values))
            self.__dict__.get("_tight_overrides", {}).pop(index, None)

        return property(get, set, doc=f"The grid's own *{_KEYS[index]}*.")

    left, bottom, right, top, wspace, hspace = map(_override, range(6))
    del _override

    _AllowedKeys = list(_KEYS)

    def update(self, *, left=_UNSET, bottom=_UNSET, right=_UNSET, top=_UNSET,
               wspace=_UNSET, hspace=_UNSET):
        """
        Update the subplot parameters of the grid.

        Parameters that are not explicitly given are not changed. Setting a
        parameter to *None* resets it to :rc:`figure.subplot.*`.

        Parameters
        ----------
        left, right, top, bottom : float or None, optional
            Extent of the subplots as a fraction of figure width or height.
        wspace, hspace : float or None, optional
            Spacing between the subplots as a fraction of the average subplot
            width / height.
        """
        given = dict(left=left, bottom=bottom, right=right, top=top,
                     wspace=wspace, hspace=hspace)
        for key, value in given.items():
            if value is not _UNSET:
                setattr(self, key, value)
        overrides = _gridspec.overrides(self._native)
        for manager in _pylab_helpers.Gcf.figs.values():
            figure = manager.canvas.figure
            if any(ax.get_subplotspec() is not None
                   and ax.get_subplotspec().get_topmost_subplotspec()
                   .get_gridspec() is self for ax in figure.axes):
                _gridspec.update(self._native, figure._native, overrides)

    def get_subplot_params(self, figure=None):
        """
        Return the `.SubplotParams` for the GridSpec.

        In order of precedence the values are taken from

        - non-*None* attributes of the GridSpec
        - the provided *figure*
        - :rc:`figure.subplot.*`

        Note that the ``figure`` attribute of the GridSpec is always ignored.
        """
        return SubplotParams(*_gridspec.subplot_params(
            self._native, _figure_params(figure)))

    def locally_modified_subplot_params(self):
        """
        Return a list of the names of the subplot parameters explicitly set
        in the GridSpec.

        This is a subset of the attributes of `.SubplotParams`.
        """
        return [k for k in self._AllowedKeys if getattr(self, k)]

    def tight_layout(self, figure, renderer=None,
                     pad=1.08, h_pad=None, w_pad=None, rect=None):
        """
        Adjust subplot parameters to give specified padding.

        Parameters
        ----------
        figure : `.Figure`
            The figure.
        renderer :  `.RendererBase` subclass, optional
            The renderer to be used.
        pad : float
            Padding between the figure edge and the edges of subplots, as a
            fraction of the font-size.
        h_pad, w_pad : float, optional
            Padding (height/width) between edges of adjacent subplots.
            Defaults to *pad*.
        rect : tuple (left, bottom, right, top), default: None
            (left, bottom, right, top) rectangle in normalized figure
            coordinates that the whole subplots area (including labels) will
            fit into. Default (None) is the whole figure.
        """
        not_applied = _figure_layout.grid_tight_layout(
            self._native, figure._native, float(pad), h_pad, w_pad,
            None if rect is None else tuple(rect))
        if not_applied is not None:
            _api.warn_external(not_applied)
        else:
            self._tight_overrides = dict(enumerate(_gridspec.overrides(self._native)))


class GridSpecFromSubplotSpec(GridSpecBase):
    """
    GridSpec whose subplot layout parameters are inherited from the
    location specified by a given SubplotSpec.
    """

    def __init__(self, nrows, ncols,
                 subplot_spec,
                 wspace=None, hspace=None,
                 height_ratios=None, width_ratios=None):
        """
        Parameters
        ----------
        nrows, ncols : int
            Number of rows and number of columns of the grid.
        subplot_spec : SubplotSpec
            Spec from which the layout parameters are inherited.
        wspace, hspace : float, optional
            See `GridSpec` for more details. If not specified default values
            (from the figure or rcParams) are used.
        height_ratios : array-like of length *nrows*, optional
            See `GridSpecBase` for details.
        width_ratios : array-like of length *ncols*, optional
            See `GridSpecBase` for details.
        """
        self._wspace = wspace
        self._hspace = hspace
        if isinstance(subplot_spec, SubplotSpec):
            self._subplot_spec = subplot_spec
        else:
            raise TypeError(
                            "subplot_spec must be type SubplotSpec, "
                            "usually from GridSpec, or axes.get_subplotspec.")
        self.figure = self._subplot_spec.get_gridspec().figure
        super().__init__(nrows, ncols,
                         width_ratios=width_ratios,
                         height_ratios=height_ratios)
        self._adopt_native(_gridspec.nested(
            subplot_spec._native_spec(), (nrows, ncols),
            [float(r) for r in self._col_width_ratios],
            [float(r) for r in self._row_height_ratios],
            (wspace, hspace)))

    @classmethod
    def _from_native(cls, native, subplot_spec):
        """The grid around the native nested grid *native* of the block
        *subplot_spec*."""
        gridspec = cls.__new__(cls)
        wspace, hspace = _gridspec.overrides(native)[4:]
        gridspec._wspace, gridspec._hspace = wspace, hspace
        gridspec._subplot_spec = subplot_spec
        gridspec.figure = subplot_spec.get_gridspec().figure
        nrows, ncols = _gridspec.shape(native)
        width_ratios, height_ratios = _gridspec.ratios(native)
        GridSpecBase.__init__(gridspec, nrows, ncols, height_ratios, width_ratios)
        gridspec._adopt_native(native)
        return gridspec

    def get_subplot_params(self, figure=None):
        """Return a dictionary of subplot layout parameters."""
        hspace = (self._hspace if self._hspace is not None
                  else figure.subplotpars.hspace if figure is not None
                  else rcParams["figure.subplot.hspace"])
        wspace = (self._wspace if self._wspace is not None
                  else figure.subplotpars.wspace if figure is not None
                  else rcParams["figure.subplot.wspace"])

        figbox = self._subplot_spec.get_position(figure)
        left, bottom, right, top = figbox.extents

        return SubplotParams(left=left, right=right,
                             bottom=bottom, top=top,
                             wspace=wspace, hspace=hspace)

    def get_topmost_subplotspec(self):
        """
        Return the topmost `.SubplotSpec` instance associated with the subplot.
        """
        return self._subplot_spec.get_topmost_subplotspec()


class SubplotSpec:
    """
    The location of a subplot in a `GridSpec`.

    .. note::

        Likely, you will never instantiate a `SubplotSpec` yourself. Instead,
        you will typically obtain one from a `GridSpec` using item-access.

    Parameters
    ----------
    gridspec : `~matplotlib.gridspec.GridSpec`
        The GridSpec, which the subplot is referencing.
    num1, num2 : int
        The subplot will occupy the *num1*-th cell of the given
        *gridspec*.  If *num2* is provided, the subplot will span between
        *num1*-th cell and *num2*-th cell **inclusive**.

        The index starts from 0.
    """

    def __init__(self, gridspec, num1, num2=None):
        self._gridspec = gridspec
        self.num1 = num1
        self.num2 = num2

    @classmethod
    def _from_native(cls, native, figure=None):
        """The block of the native block *native*."""
        gridspec = _wrap(_gridspec.spec_gridspec(native), figure)
        (row0, row1), (col0, col1) = _gridspec.spec_cells(native)
        ncols = gridspec.ncols
        return cls(gridspec, row0 * ncols + col0, (row1 - 1) * ncols + col1 - 1)

    def _native_spec(self):
        """The native block of cells of this subplot spec: the rows and
        columns between its two cells, whichever comes first, as
        matplotlib's `get_position` places it."""
        gridspec = self._gridspec
        if takes_from_program(type(gridspec), "get_subplot_params"):
            # The margins and spacing the grid's class of the program's
            # gives now, which place what is added on the block as
            # matplotlib places it (`.Axes.set_subplotspec`).
            _gridspec.set_overrides(gridspec._native, gridspec._params_of(
                getattr(gridspec, "figure", None)))
        ncols = gridspec.ncols
        r1, r2 = sorted([self.num1 // ncols, self.num2 // ncols])
        cols = self.colspan
        return _gridspec.span(gridspec._native, (r1, r2 + 1),
                              (cols.start, cols.stop))

    def __repr__(self):
        return (f"{self.get_gridspec()}["
                f"{self.rowspan.start}:{self.rowspan.stop}, "
                f"{self.colspan.start}:{self.colspan.stop}]")

    @staticmethod
    def _from_subplot_args(figure, args):
        """
        Construct a `.SubplotSpec` from a parent `.Figure` and either

        - a `.SubplotSpec` -- returned as is;
        - one or three numbers -- a MATLAB-style subplot specifier.
        """
        if len(args) == 1:
            arg, = args
            if isinstance(arg, SubplotSpec):
                return arg
            elif not isinstance(arg, Integral):
                raise ValueError(
                    f"Single argument to subplot must be a three-digit "
                    f"integer, not {arg!r}")
            try:
                rows, cols, num = map(int, str(arg))
            except ValueError:
                raise ValueError(
                    f"Single argument to subplot must be a three-digit "
                    f"integer, not {arg!r}") from None
        elif len(args) == 3:
            rows, cols, num = args
        else:
            raise _api.nargs_error("subplot", takes="1 or 3", given=len(args))

        gs = GridSpec._check_gridspec_exists(figure, rows, cols)
        if gs is None:
            gs = GridSpec(rows, cols, figure=figure)
        if isinstance(num, tuple) and len(num) == 2:
            if not all(isinstance(n, Integral) for n in num):
                raise ValueError(
                    f"Subplot specifier tuple must contain integers, not {num}"
                )
            i, j = num
        else:
            if not isinstance(num, Integral) or num < 1 or num > rows*cols:
                raise ValueError(
                    f"num must be an integer with 1 <= num <= {rows*cols}, "
                    f"not {num!r}"
                )
            i = j = num
        return gs[i-1:j]

    # num2 is a property only to handle the case where it is None and someone
    # mutates num1.

    @property
    def num2(self):
        return self.num1 if self._num2 is None else self._num2

    @num2.setter
    def num2(self, value):
        self._num2 = value

    def get_gridspec(self):
        return self._gridspec

    def get_geometry(self):
        """
        Return the subplot geometry as tuple ``(n_rows, n_cols, start, stop)``.

        The indices *start* and *stop* define the range of the subplot within
        the `GridSpec`. *stop* is inclusive (i.e. for a single cell
        ``start == stop``).
        """
        rows, cols = self.get_gridspec().get_geometry()
        return rows, cols, self.num1, self.num2

    @property
    def rowspan(self):
        """The rows spanned by this subplot, as a `range` object."""
        ncols = self.get_gridspec().ncols
        return range(self.num1 // ncols, self.num2 // ncols + 1)

    @property
    def colspan(self):
        """The columns spanned by this subplot, as a `range` object."""
        ncols = self.get_gridspec().ncols
        # We explicitly support num2 referring to a column on num1's *left*, so
        # we must sort the column indices here so that the range makes sense.
        c1, c2 = sorted([self.num1 % ncols, self.num2 % ncols])
        return range(c1, c2 + 1)

    def is_first_row(self):
        return self.rowspan.start == 0

    def is_last_row(self):
        return self.rowspan.stop == self.get_gridspec().nrows

    def is_first_col(self):
        return self.colspan.start == 0

    def is_last_col(self):
        return self.colspan.stop == self.get_gridspec().ncols

    def get_position(self, figure):
        """
        Update the subplot position from ``figure.subplotpars``.
        """
        gridspec = self.get_gridspec()
        if takes_from_program(type(gridspec), "get_grid_positions"):
            # The cells where the grid's class of the program's puts them,
            # as matplotlib's `get_position` asks for them.
            rows, cols = np.unravel_index([self.num1, self.num2],
                                          gridspec.get_geometry())
            bottoms, tops, lefts, rights = gridspec.get_grid_positions(figure)
            return Bbox.from_extents(lefts[cols].min(), bottoms[rows].min(),
                                     rights[cols].max(), tops[rows].max())
        return Bbox.from_extents(*_gridspec.spec_position(
            self._native_spec(), gridspec._params_of(figure)))

    def get_topmost_subplotspec(self):
        """
        Return the topmost `SubplotSpec` instance associated with the subplot.
        """
        gridspec = self.get_gridspec()
        if hasattr(gridspec, "get_topmost_subplotspec"):
            return gridspec.get_topmost_subplotspec()
        else:
            return self

    def __eq__(self, other):
        """
        Two SubplotSpecs are considered equal if they refer to the same
        position(s) in the same `GridSpec`.
        """
        # other may not even have the attributes we are checking.
        return ((self._gridspec, self.num1, self.num2)
                == (getattr(other, "_gridspec", object()),
                    getattr(other, "num1", object()),
                    getattr(other, "num2", object())))

    def __hash__(self):
        return hash((self._gridspec, self.num1, self.num2))

    def subgridspec(self, nrows, ncols, **kwargs):
        """
        Create a GridSpec within this subplot.

        The created `.GridSpecFromSubplotSpec` will have this `SubplotSpec` as
        a parent.

        Parameters
        ----------
        nrows : int
            Number of rows in grid.

        ncols : int
            Number of columns in grid.

        Returns
        -------
        `.GridSpecFromSubplotSpec`

        Other Parameters
        ----------------
        **kwargs
            All other parameters are passed to `.GridSpecFromSubplotSpec`.

        See Also
        --------
        matplotlib.pyplot.subplots

        Examples
        --------
        Adding three subplots in the space occupied by a single subplot::

            fig = plt.figure()
            gs0 = fig.add_gridspec(3, 1)
            ax1 = fig.add_subplot(gs0[0])
            ax2 = fig.add_subplot(gs0[1])
            gssub = gs0[2].subgridspec(1, 3)
            for i in range(3):
                fig.add_subplot(gssub[0, i])
        """
        return GridSpecFromSubplotSpec(nrows, ncols, self, **kwargs)


class SubplotParams:
    """
    Parameters defining the positioning of a subplots grid in a figure.
    """

    def __init__(self, left=None, bottom=None, right=None, top=None,
                 wspace=None, hspace=None):
        """
        Defaults are given by :rc:`figure.subplot.[name]`.

        Parameters
        ----------
        left : float, optional
            The position of the left edge of the subplots,
            as a fraction of the figure width.
        right : float, optional
            The position of the right edge of the subplots,
            as a fraction of the figure width.
        bottom : float, optional
            The position of the bottom edge of the subplots,
            as a fraction of the figure height.
        top : float, optional
            The position of the top edge of the subplots,
            as a fraction of the figure height.
        wspace : float, optional
            The width of the padding between subplots,
            as a fraction of the average Axes width.
        hspace : float, optional
            The height of the padding between subplots,
            as a fraction of the average Axes height.
        """
        for key in _KEYS:
            setattr(self, key, rcParams[f"figure.subplot.{key}"])
        self.update(left, bottom, right, top, wspace, hspace)

    def update(self, left=None, bottom=None, right=None, top=None,
               wspace=None, hspace=None):
        """
        Update the dimensions of the passed parameters. *None* means unchanged.
        """
        if ((left if left is not None else self.left)
                >= (right if right is not None else self.right)):
            raise ValueError('left cannot be >= right')
        if ((bottom if bottom is not None else self.bottom)
                >= (top if top is not None else self.top)):
            raise ValueError('bottom cannot be >= top')
        if left is not None:
            self.left = left
        if right is not None:
            self.right = right
        if bottom is not None:
            self.bottom = bottom
        if top is not None:
            self.top = top
        if wspace is not None:
            self.wspace = wspace
        if hspace is not None:
            self.hspace = hspace
        given = {key: value for key, value in zip(
            _KEYS, (left, bottom, right, top, wspace, hspace)) if value is not None}
        self.__dict__.setdefault("_given", {}).update(given)
        figure_native = _FIGURE_OF_PARAMS.get(self)
        if figure_native is not None:
            # The parameters of a figure, which pltrs (Rust) holds.
            from ._native import figure as _figure
            _figure.subplots_adjust(figure_native,
                                    (left, bottom, right, top, wspace, hspace))

    def reset(self):
        """Restore the subplot positioning parameters to the default rcParams values"""
        self.update(**{key: rcParams[f'figure.subplot.{key}']
                       for key in self.to_dict()})

    def to_dict(self):
        """Return a copy of the subplot parameters as a dict."""
        return {key: getattr(self, key) for key in _KEYS}


#: The figure (its native handle) of each `SubplotParams` a figure gives
#: as its `.Figure.subplotpars`.
_FIGURE_OF_PARAMS = weakref.WeakKeyDictionary()


def _figure_subplotpars(figure_native, params=None):
    """
    The `SubplotParams` of the figure *figure_native*: *params*, the
    object the figure gave before (a new one for None), with the
    parameters pltrs (Rust) holds for the figure now. Its `~.SubplotParams.update`
    gives the figure its changes.
    """
    if params is None:
        params = SubplotParams.__new__(SubplotParams)
        _FIGURE_OF_PARAMS[params] = figure_native
    given = params.__dict__.get("_given", {})
    for key, value in zip(_KEYS, _gridspec.figure_params(figure_native)):
        # A parameter given (or the rc's) reads back as given; one a tight
        # layout computed is the numpy float matplotlib's computes.
        kept = given.get(key, rcParams[f"figure.subplot.{key}"])
        setattr(params, key, kept if kept == value else np.float64(value))
    return params


# What matplotlib's module also has from its imports, which programs
# import from it.
import copy  # noqa: E402, F401
import logging  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
