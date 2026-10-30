"""
What the plotting mixins share: wrapping the artists native functions
return, the native form of plotting values and of keyword arguments.
"""

import datetime

import numpy as np

from ... import _api, _handles, cbook, collections as mcoll, lines as mlines
from ..._native import plotting as _plotting


def wrap(axes, native, cls):
    """The object of class *cls* for the artist *native* of *axes*' figure."""
    return _handles.wrap(native, cls, axes.figure)


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
    list of category names, or ``("dates", numbers)``.
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
        return ("dates", np.atleast_1d(np.asarray(dates.date2num(array), float)).ravel())
    return np.atleast_1d(array.astype(float)).ravel()


def convert_units(axes, name, values):
    """*values* as numbers of the axis *name* of *axes*, which takes the
    units they bring (matplotlib's ``_process_unit_info``); numbers stay."""
    array = np.asanyarray(values)
    if array.dtype.kind in "fiub":
        return values
    return _plotting.convert_units(axes._native, name,
                                   axis_values(values)).reshape(array.shape)


def floats(values):
    """*values* as a 1D float array (masked values as NaN)."""
    if np.ma.isMaskedArray(values):
        return np.ma.filled(np.ma.asarray(values, dtype=float), np.nan).ravel()
    return np.atleast_1d(np.asarray(values, dtype=float)).ravel()


def line_kwargs(kwargs):
    """`.Line2D` keyword arguments, aliases resolved, in native form."""
    return mlines._native_kwargs(cbook.normalize_kwargs(kwargs, mlines.Line2D))


def collection_kwargs(kwargs, cls=mcoll.Collection):
    """Collection keyword arguments, aliases of *cls* resolved, in native form."""
    return mcoll._native_kwargs(cbook.normalize_kwargs(kwargs, cls))


def record_container(axes, container, native):
    """Record *container*, made with the native container *native*, in
    ``axes.containers``."""
    container._native = native
    axes.containers.append(container)
    container._remove_method = axes.containers.remove
    return container
