"""
What matplotlib's quad contour sets hand contourpy, and the checks
contourpy makes of their chunk size (``nchunk``), made here, so that a contour set refuses the
same values with the same errors whether or not contourpy is installed:
matplotlib requires contourpy, pltrs (Rust) contours by itself.

Ported from contourpy 1.4.0 under its BSD 3-Clause License: the checks
of ``calc_chunk_sizes``, and the ``TypeError`` its pybind11 constructors
raise for a chunk size their integer parameters do not take.
"""

import numpy as np
from numpy import ma

from ._native import contour as _contour

#: The range of the ``py::ssize_t`` the constructors take chunk sizes as.
_SSIZE_T = range(-2 ** 63, 2 ** 63)

#: The arrays every generator's constructor takes, as pybind11 names them.
_ARRAYS = ", ".join(
    [f"{name}: typing.Annotated[numpy.typing.ArrayLike, numpy.float64]" for name in "xyz"]
    + ["mask: typing.Annotated[numpy.typing.ArrayLike, numpy.bool]"])

_SIZE = "typing.SupportsInt | typing.SupportsIndex = 0"

#: The keywords of each algorithm's generator: those its constructor's
#: signature lists, and those matplotlib passes, in contourpy's order
#: (beyond the two chunk sizes and *corner_mask*, their fixed values).
_KEYWORDS = {
    "mpl2005": ("Mpl2005ContourGenerator", f"x_chunk_size: {_SIZE}, y_chunk_size: {_SIZE}", ()),
    "mpl2014": ("Mpl2014ContourGenerator",
                f"corner_mask: bool, x_chunk_size: {_SIZE}, y_chunk_size: {_SIZE}",
                ("corner_mask",)),
    "serial": ("SerialContourGenerator",
               "corner_mask: bool, line_type: contourpy._contourpy.LineType, "
               "fill_type: contourpy._contourpy.FillType, quad_as_tri: bool, "
               f"z_interp: contourpy._contourpy.ZInterp, x_chunk_size: {_SIZE}, "
               f"y_chunk_size: {_SIZE}",
               ("line_type=<LineType.SeparateCode: 102>", "fill_type=<FillType.OuterCode: 201>",
                "corner_mask", "quad_as_tri=False", "z_interp=<ZInterp.Linear: 1>")),
}
_KEYWORDS["threaded"] = ("ThreadedContourGenerator",
                         f"{_KEYWORDS['serial'][1]}, thread_count: {_SIZE}",
                         (*_KEYWORDS["serial"][2], "thread_count=0"))


def _takes_as_integer(size):
    """Whether pybind11 takes *size* for a ``py::ssize_t`` parameter: no
    float, but an integer in range or a number ``int`` converts to one."""
    if isinstance(size, float):
        return False
    try:
        return int(size) in _SSIZE_T
    except Exception:  # noqa: BLE001 - pybind11 refuses whatever fails
        return False


def generator_arguments(field, generation):
    """The arguments matplotlib gives ``contourpy.contour_generator`` for
    the contoured *field* (with *generation*, see
    `.contour._ContourGenerator`): ``(x, y, z, algorithm, corner_mask)``,
    z masked where the data is invalid or masked."""
    z, mask = field[:2]
    x, y, algorithm, corner_mask = _contour.generator_arguments(field, generation)
    z = ma.masked_invalid(z, copy=False)
    if mask is not None:
        z[mask] = ma.masked
    return x, y, z, algorithm, corner_mask


def _refusal(field, generation, sizes, data):
    """The ``TypeError`` of the generator's constructor taking the chunk
    sizes *sizes* (``y, x``) for the contoured *field* of the values
    *data*."""
    x, y, z, algorithm, corner_mask = generator_arguments(field, generation)
    # The mask contourpy hands the constructor, with the values under it.
    z = ma.masked_invalid(ma.asarray(z, dtype=np.float64), copy=False)
    mask = ma.getmask(z) if ma.is_masked(z) else None
    name, signature, keywords = _KEYWORDS[algorithm]
    given = [f"x_chunk_size={sizes[1]!r}", f"y_chunk_size={sizes[0]!r}"] + [
        f"corner_mask={corner_mask!r}" if keyword == "corner_mask" else keyword
        for keyword in keywords]
    data = ma.getdata(ma.asarray(data, dtype=np.float64))
    arguments = ", ".join(repr(argument) for argument in (x, y, data, mask))
    return TypeError(
        "__init__(): incompatible constructor arguments. The following argument types are "
        f"supported:\n    1. contourpy._contourpy.{name}({_ARRAYS}, *, {signature})\n\n"
        f"Invoked with: {arguments}; kwargs: {', '.join(given)}")


def check(field, generation, chunk_size, data):
    """Fail as contourpy fails to make matplotlib's generator for *field*
    (with *generation*, see `.contour._ContourGenerator`), the values
    *data* as given, in chunks of *chunk_size*, a value pltrs (Rust) does
    not take as its number of chunks (`._contour_arguments.held_chunks`)."""
    y_size, x_size = chunk_size if isinstance(chunk_size, tuple) else (chunk_size, chunk_size)
    if x_size < 0 or y_size < 0:
        raise ValueError("chunk_size cannot be negative")
    if not (_takes_as_integer(x_size) and _takes_as_integer(y_size)):
        raise _refusal(field, generation, (y_size, x_size), data)
