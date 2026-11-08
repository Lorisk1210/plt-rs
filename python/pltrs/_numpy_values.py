"""
Values as matplotlib's numpy arithmetic takes them.

matplotlib stores many parameters by assigning them into the float arrays
of its boxes and matrices (``viewLim.x0 = value``, ``mtx[0, 2] = value``),
often after a ufunc (``np.deg2rad(value)``): numpy then reads None as NaN
and a numeric string as its number, and refuses sequences and other
strings with its own errors. pltrs (Rust) holds the float such an element
would hold; these give it, raising what numpy raises.
"""

import numpy as np


def element(value):
    """*value* as an element of a float array holds it once assigned
    (``array[i] = value``)."""
    cell = np.empty(1)
    cell[0] = value
    return float(cell[0])


def degrees(value):
    """
    *value*, an angle in degrees matplotlib hands ``np.deg2rad`` and then
    assigns into a float array (``viewLim.x0 = np.deg2rad(value)``), as
    the float pltrs (Rust) holds in degrees (it converts to radians the
    same way): the ufunc's and the assignment's errors are numpy's.
    """
    element(np.deg2rad(value))
    return element(value)
