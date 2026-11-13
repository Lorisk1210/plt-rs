"""
What the system, the versions of the dependencies and the interpreter
show of themselves in the values the smoke tests compare, asked of them
here, so that each test expects what matplotlib gives in the environment
it runs in.

The builds of matplotlib and numpy on this system:

* the line of ``ft2font.cpp`` matplotlib's ``FT_Set_Char_Size`` errors
  name (``__LINE__`` in a call spanning lines 248 to 250): its Linux
  wheels' GCC gives the line the call starts on, its macOS wheels' clang
  the line it ends on;
* whether matplotlib's C++ cast of a negative resolution to FreeType's
  unsigned one wraps around (x86-64, to a resolution FreeType refuses) or
  saturates to none (arm64, which FreeType takes as 72 dpi);
* whether numpy warns of the floating-point exceptions of its matrix
  products (``np.dot``, ``@``): not on Apple silicon, where its BLAS,
  Accelerate, multiplies on a coprocessor raising none, and in numpy 1
  only those of ``@``;
* whether numpy warns where it floor-divides an infinity or takes its
  remainder: not on macOS, whose ``fmod`` raises no invalid exception for
  an infinite dividend.

The versions of the dependencies:

* where numpy keeps the exception classes of its ufuncs
  (`NUMPY_EXCEPTIONS`), and whether it computes with a numpy scalar and a
  Python number in the scalar's precision (`SCALARS_KEEP_PRECISION`:
  numpy 2's promotion, NEP 50) or in double (numpy 1);
* whether python-dateutil's time zones answer as zoneinfo's of the same
  name (`DATEUTIL_ZONES_AS_ZONEINFO`): python-dateutil 4 reads a zone's
  file as zoneinfo does, 2.x gives a wall time the clocks skip the offset
  after the change and ignores the rule a file ends with;
* whether they keep their files' offsets (`DATEUTIL_KEEPS_OFFSETS`): 2.7
  rounds them, local mean times such as Berlin's 0:53:28, to whole
  minutes.

The interpreter (`raised`): the messages of the errors matplotlib's code
meets in Python's and numpy's own operations, which CPython and numpy
word differently from version to version (CPython 3.14 reworded the math
module's domain errors, float division by zero, unhashable keys,
unpacking too many values, attributes set on an object without a
``__dict__`` and ``in`` on an object that is no container; numpy 2 its
ufuncs' refusals of strings and its scalars' reprs).
"""

import datetime
import importlib
import io
import platform
import struct
import sys
import warnings
import zoneinfo

import dateutil.tz
import numpy as np

#: The line of ``ft2font.cpp`` matplotlib's ``FT_Set_Char_Size`` errors name.
SET_CHAR_SIZE_LINE = 250 if sys.platform == "darwin" else 248

#: Whether matplotlib's cast of a negative resolution wraps around.
CASTS_WRAP = platform.machine() in ("x86_64", "AMD64")


def _warns(compute):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        compute()
    return bool(caught)


#: Whether numpy's ``np.dot`` and ``np.matmul`` (``@``) warn of invalid
#: values, by name.
PRODUCTS_WARN = {product.__name__: _warns(
    lambda product=product: product(np.zeros((2, 2)), np.full(2, np.inf)))
    for product in (np.dot, np.matmul)}

#: Whether numpy's scalar floor division and remainder of an infinity warn.
FLOOR_DIVIDING_INFINITY_WARNS = _warns(lambda: np.float64(np.inf) // 1.0)


def _zones_agree(name, local_times):
    """Whether dateutil's and zoneinfo's zones *name* give the wall times
    *local_times* the same offset, name and daylight saving time, either
    fold; False where zoneinfo has no such zone."""
    try:
        zones = dateutil.tz.gettz(name), zoneinfo.ZoneInfo(name)
    except zoneinfo.ZoneInfoNotFoundError:
        return False

    def facts(local, zone, fold):
        moment = local.replace(tzinfo=zone, fold=fold)
        return moment.utcoffset(), moment.tzname(), moment.dst()

    return all(facts(local, zones[0], fold) == facts(local, zones[1], fold)
               for local in local_times for fold in (0, 1))


#: Whether python-dateutil's zones answer as zoneinfo's: for a wall time
#: the clocks skip, one after the zone file's last change and one of local
#: mean time.
DATEUTIL_ZONES_AS_ZONEINFO = _zones_agree("America/New_York", [
    datetime.datetime(2020, 3, 8, 2, 30), datetime.datetime(2100, 7, 1),
    datetime.datetime(1600, 7, 1)])


#: numpy's module of the exceptions of its ufuncs: ``numpy._core``'s from
#: numpy 2, ``numpy.core``'s before.
NUMPY_EXCEPTIONS = importlib.import_module(
    "numpy._core._exceptions" if hasattr(np, "_core") else "numpy.core._exceptions")


#: Whether numpy computes with a numpy scalar and a Python number in the
#: scalar's precision (numpy 2) rather than in double (numpy 1).
SCALARS_KEEP_PRECISION = (np.float32(1) / 3.0).dtype == np.float32


def _keeps_offsets():
    """Whether python-dateutil's ``tzfile`` keeps the offset of a zone
    file of a single zone 3,601 seconds east of UTC."""
    data = (b"TZif" + bytes(16) + struct.pack(">6l", 0, 0, 0, 0, 1, 4)
            + struct.pack(">lbb", 3601, 0, 0) + b"LMT\0")
    zone = dateutil.tz.tzfile(io.BytesIO(data))
    return zone.utcoffset(datetime.datetime(2000, 1, 1)) == datetime.timedelta(seconds=3601)


#: Whether python-dateutil's zones keep their files' offsets of seconds.
DATEUTIL_KEEPS_OFFSETS = _keeps_offsets()


def as_numpy_warns(messages):
    """*messages* (numpy's warnings, as Linux' numpy issues them), without
    those of the matrix products that warn of nothing here."""
    return [message for message in messages
            if PRODUCTS_WARN.get(message.rpartition(" in ")[2], True)]


def as_numpy_counts(counts):
    """*counts* (how often each warning came, as Linux' numpy 2 issues
    them), as this system's numpy counts them: without the warnings of
    matrix products where its products warn of nothing."""
    return {message: counts[message] for message in as_numpy_warns(counts)}


def raised(action):
    """The message of the exception *action* (a callable doing what
    matplotlib's code does, to a plain object) raises in this Python."""
    try:
        action()
    except Exception as error:  # noqa: BLE001 - any exception's message
        return str(error)
    raise AssertionError(f"{action!r} raised nothing")
