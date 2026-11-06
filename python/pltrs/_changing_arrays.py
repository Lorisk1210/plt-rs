"""
Arrays handed out whose changes in place reach what they were read from:
matplotlib keeps some of an artist's state in arrays programs change in
place (nilearn colours the faces around a region of a surface with
``collection._facecolor3d[faces] = color``), which pltrs (Rust) holds
itself. A `ChangingArray` gives its owner its values again after each of
its items is assigned or an augmented operator changed it.
"""

import functools

import numpy as np


def _changing(method):
    """*method* of `numpy.ndarray`, then the owner given the values."""
    @functools.wraps(method)
    def changed(self, *args):
        result = method(self, *args)
        if self._setter is not None:
            self._setter(np.asarray(self))
        return result
    return changed


class ChangingArray(np.ndarray):
    """
    The array of *values* whose changes in place call *setter* with its
    values. The arrays made of it (views, copies, results) change nothing.
    """

    def __new__(cls, values, setter):
        array = np.array(values).view(cls)
        array._setter = setter
        return array

    def __array_finalize__(self, obj):
        self._setter = None

    __setitem__ = _changing(np.ndarray.__setitem__)
    __iadd__ = _changing(np.ndarray.__iadd__)
    __isub__ = _changing(np.ndarray.__isub__)
    __imul__ = _changing(np.ndarray.__imul__)
    __itruediv__ = _changing(np.ndarray.__itruediv__)
