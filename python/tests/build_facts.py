"""
What the builds of matplotlib and numpy on this system show of themselves
in the values the smoke tests compare, which differ by system:

* the line of ``ft2font.cpp`` matplotlib's ``FT_Set_Char_Size`` errors
  name (``__LINE__`` in a call spanning lines 248 to 250): its Linux
  wheels' GCC gives the line the call starts on, its macOS wheels' clang
  the line it ends on;
* whether matplotlib's C++ cast of a negative resolution to FreeType's
  unsigned one wraps around (x86-64, to a resolution FreeType refuses) or
  saturates to none (arm64, which FreeType takes as 72 dpi);
* whether numpy warns of the floating-point exceptions of its matrix
  products (``np.dot``, ``@``): not on Apple silicon, where its BLAS,
  Accelerate, multiplies on a coprocessor raising none;
* whether numpy warns where it floor-divides an infinity or takes its
  remainder: not on macOS, whose ``fmod`` raises no invalid exception for
  an infinite dividend.
"""

import platform
import sys
import warnings

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


#: Whether numpy's matrix products warn of invalid values.
PRODUCTS_WARN = _warns(lambda: np.dot(np.zeros((2, 2)), np.full(2, np.inf)))

#: Whether numpy's scalar floor division and remainder of an infinity warn.
FLOOR_DIVIDING_INFINITY_WARNS = _warns(lambda: np.float64(np.inf) // 1.0)


def as_numpy_warns(messages):
    """*messages* (numpy's warnings, as Linux' numpy issues them), without
    those of matrix products where numpy's products warn of nothing."""
    return [message for message in messages
            if PRODUCTS_WARN or not message.endswith((" in dot", " in matmul"))]


def as_numpy_counts(counts):
    """*counts* (how often each warning came, as Linux' numpy issues
    them), as this system's numpy counts them: without the warnings of
    matrix products where its products warn of nothing."""
    return {message: counts[message] for message in as_numpy_warns(counts)}
