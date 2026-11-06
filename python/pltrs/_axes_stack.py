"""
matplotlib's ``_AxesStack`` (``matplotlib.figure._AxesStack``), the
private register of a figure's Axes and of its current one, which released
libraries read (nilearn's tests: ``fig._axstack.as_list()``).

pltrs (Rust) keeps the Axes of a figure and its current one itself: the
stack of a figure (``Figure._axstack``, also a subfigure's, which shares
its figure's as matplotlib's does) answers from them, after matplotlib
3.11.2's under the Matplotlib License.
"""

from . import _handles
from ._native import figure as _figure


class _AxesStack:
    """
    Helper class to track Axes in a figure.

    Axes are tracked both in the order in which they have been added
    (``self._axes`` insertion/iteration order) and in the separate "gca" stack
    (which is the index to which they map in the ``self._axes`` dict).
    """

    def __init__(self, figure):
        # The (root) figure whose Axes pltrs (Rust) keeps.
        self._figure = figure

    def as_list(self):
        """List the Axes that have been added to the figure."""
        return self._figure.axes

    def remove(self, a):
        """Remove the Axes from the stack."""
        if a not in self.as_list():
            raise KeyError(a)
        a.remove()

    def bubble(self, a):
        """Move an Axes, which must already exist in the stack, to the top."""
        if a not in self.as_list():
            raise ValueError("Axes has not been added yet")
        _figure.sca(self._figure._native, a._native)

    def add(self, a):
        """Add an Axes to the stack, ignoring it if already present."""
        if a not in self.as_list():
            self._figure.add_axes(a)

    def current(self):
        """Return the active Axes, or None if the stack is empty."""
        from .axes import Axes
        return _handles.wrap(_figure.current_axes(self._figure._native), Axes,
                             self._figure)
