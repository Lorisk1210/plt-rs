"""
What the plotting mixins share: wrapping the artists native functions
return, the native form of plotting values and of keyword arguments.
"""

import copy
import datetime
import inspect
import warnings

import numpy as np

from ... import _api, _handles, _lazy_imports, cbook, collections as mcoll, lines as mlines
from ..._native import plotting as _plotting

_program_adds = _lazy_imports.module(globals(), ".._program_adds")


def wrap(axes, native, cls):
    """The object of class *cls* for the artist *native* of *axes*' figure,
    which a plotting method just made: handed to the ``add_*`` method of
    *axes*' class of the program's that matplotlib adds it with
    (`.._program_adds`)."""
    artist = _handles.wrap(native, cls, axes._figure)
    _program_adds.announce(axes, artist)
    return artist


def wrap_all(axes, natives, cls):
    """`wrap` for each handle of *natives*, as a list."""
    return [wrap(axes, native, cls) for native in natives]


def _is_dates(values):
    """Whether *values* (an array) holds dates."""
    if np.issubdtype(values.dtype, np.datetime64):
        return True
    if values.dtype != object or not values.size:
        return False
    first = cbook._safe_first_finite(values.ravel())
    return isinstance(first, (datetime.date, np.datetime64))


def axis_values(values):
    """
    *values* as the native side takes plotting values: a float array, a
    list of category names, or ``("dates", numbers, zone)`` (the zone of the
    first date, `.dates._axis_zone`).
    """
    values = cbook._unpack_to_numpy(values)
    if isinstance(values, str):
        return [values]
    if np.ma.isMaskedArray(values) and values.dtype.kind in "biuf":
        return np.ma.filled(values.astype(float), np.nan).ravel()
    if np.iterable(values) and len(values) and isinstance(
            cbook._safe_first_finite(values), (str, bytes)):
        # Category names: matplotlib's string category converter refuses
        # anything else among them.
        names = np.atleast_1d(np.array(values, dtype=object)).ravel()
        for name in names:
            _api.check_isinstance((str, bytes), value=name)
        return [name.decode() if isinstance(name, bytes) else name for name in names]
    if np.ma.isMaskedArray(values):
        return np.ma.filled(values.astype(float), np.nan).ravel()
    array = np.asarray(values)
    if array.dtype.kind in "US" or (
            array.dtype == object and array.size
            and all(isinstance(v, str) for v in array.ravel())):
        return [str(v) for v in array.ravel()]
    if _is_dates(array):
        from ... import dates
        return ("dates", np.atleast_1d(np.asarray(dates.date2num(array), float)).ravel(),
                dates._axis_zone(array))
    return np.atleast_1d(array.astype(float)).ravel()


def convert_units(axes, name, values):
    """*values* as numbers of the axis *name* of *axes*, which takes the
    units they bring (matplotlib's ``_process_unit_info``); numbers stay."""
    array = np.asanyarray(values)
    if array.dtype.kind in "fiub":
        return values
    from ... import units as munits
    from ...axis import _is_native_converter
    axis = axes._axis_map.get(name)
    if axis is not None and not _is_native_converter(
            axis._given_converter or munits.registry.get_converter(values)):
        # A converter of the program's (nc-time-axis' of cftime dates),
        # which the axis takes as matplotlib's ``_process_unit_info`` has
        # it take one.
        axis.update_units(values)
        return axis.convert_units(values)
    native = axis_values(values)
    converted = _plotting.convert_units(axes._native, name, native)
    if array.ndim == 0:
        # A scalar converts to a scalar, as matplotlib's date and category
        # converters give it.
        return converted[0]
    return converted.reshape(array.shape)


def line_data(axes, x, y):
    """
    The data *x*, *y* of a line on *axes* as pltrs (Rust) takes it: 1D
    float arrays, masked values as NaN, the axes taking the units of the
    data first (matplotlib's ``_plot_args`` and ``Line2D.recache``).
    """
    for name, values in (("x", x), ("y", y)):
        axis = axes._axis_map[name]
        if axis is not None:
            axis.update_units(values)
    return converted_line_data(axes, x, y)


def converted_line_data(axes, x, y):
    """`line_data` of data whose units *axes* took already, as
    ``_plot_args`` makes the axes take them before it makes the lines
    (``_plot_datasets``)."""
    return tuple(cbook._to_unmasked_float_array(values).ravel()
                 for values in (axes.convert_xunits(x), axes.convert_yunits(y)))


def keep_data(line, x, y):
    """Have *line*, made from *x* and *y*, keep copies of them as its
    original data, as matplotlib's `.Line2D` does: ``get_xdata()`` gives
    them back as matplotlib's lines do (masked arrays and integers as they
    are), and data in units converts again when the units of the line's
    axes change (matplotlib's lines convert their data when drawn).
    Returns *line*, which may be None (no data line: an error bar's of
    ``fmt='none'``)."""
    if line is not None:
        line._xorig, line._yorig = copy.copy(x), copy.copy(y)
    return line


def floats(values):
    """*values* as a 1D float array (masked values as NaN)."""
    if np.ma.isMaskedArray(values):
        return np.ma.filled(np.ma.asarray(values, dtype=float), np.nan).ravel()
    return np.atleast_1d(np.asarray(values, dtype=float)).ravel()


def line_kwargs(kwargs):
    """`.Line2D` keyword arguments, aliases resolved, in native form."""
    return mlines._native_kwargs(cbook.normalize_kwargs(kwargs, mlines.Line2D))


#: The parameters of `.Line2D`'s constructor that take their rcParams'
#: default when None.
_LINE_DEFAULTS = frozenset(
    name for name, parameter in inspect.signature(mlines.Line2D.__init__).parameters.items()
    if parameter.default is None)


def made_line_kwargs(kwargs):
    """
    The keyword arguments of a `.Line2D` made with them (``Line2D(...,
    **kwargs)``, as ``axhline`` makes its line), in native form: an alias
    overrides the property it stands for (statsmodels gives ``lw`` beside
    a ``linewidth``), as matplotlib's constructor sets its parameters, then
    the remaining arguments; a parameter of the constructor given None
    under its own name takes its default (yellowbrick gives
    ``color=None``), and so does a label of None, which leaves the line
    unlabelled until its Axes names it ``_child<n>``.
    """
    to_property = mlines.Line2D._alias_to_prop
    kwargs = {key: value for key, value in kwargs.items()
              if not (value is None and (key in _LINE_DEFAULTS or key == "label"))}
    resolved = {key: value for key, value in kwargs.items() if key not in to_property}
    resolved.update((to_property[key], value) for key, value in kwargs.items()
                    if key in to_property)
    return mlines._native_kwargs(resolved)


def collection_kwargs(kwargs, cls=mcoll.Collection):
    """Collection keyword arguments, aliases of *cls* resolved, in native form."""
    return mcoll._native_kwargs(cbook.normalize_kwargs(kwargs, cls))


def check_format(fmt):
    """Fail as matplotlib's ``_process_plot_format`` fails for a format
    that is no string: unless it is a colour, where it measures it."""
    from ... import colors as mcolors
    if isinstance(fmt, str):
        return
    try:
        mcolors.to_rgba(fmt)
    except ValueError:
        _ = len(fmt)


def takes_properties(artist, props):
    """Whether *artist*, made for the test, takes the properties *props*:
    those pltrs (Rust) gives artists once it made all of them, which
    matplotlib's code gives each artist as it makes it (failing then)."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        try:
            artist.set(**props)
        except Exception:
            return False
    return True


def record_container(axes, container, native):
    """Record *container*, made with the native container *native*, in
    ``axes.containers``."""
    container._native = native
    return axes.add_container(container)


def _make_axes_method(func):
    """
    Patch the qualname for functions that are directly added to Axes.

    Some Axes functionality is defined in functions in other submodules.
    These are simply added as attributes to Axes. As a result, their
    ``__qualname__`` is e.g. only "table" and not "Axes.table". This
    function fixes that.

    Note that the function itself is patched, so that
    ``matplotlib.table.table.__qualname__` will also show "Axes.table".
    However, since these functions are not intended to be standalone,
    this is bearable.

    Kept from matplotlib 3.11.2 (Matplotlib License).
    """
    func.__qualname__ = f"Axes.{func.__name__}"
    return func
