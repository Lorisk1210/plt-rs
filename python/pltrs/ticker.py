"""
Tick locating and formatting (matplotlib's ``ticker``).

A `Formatter` is a Python callable ``formatter(x, pos)``: plt-rs calls it
each time it labels the ticks of the axis it was given to.
"""


class Formatter:
    """Create a string based on a tick value and location."""

    axis = None

    def __call__(self, x, pos=None):
        """
        Return the format for tick value *x* at position pos.
        ``pos=None`` indicates an unspecified location.
        """
        raise NotImplementedError('Derived must override')


class NullFormatter(Formatter):
    """Always return the empty string."""

    def __call__(self, x, pos=None):
        return ''


class FuncFormatter(Formatter):
    """
    Use a user-defined function for formatting.

    The function should take in two inputs (a tick value ``x`` and a
    position ``pos``), and return a string containing the corresponding
    tick label.
    """

    def __init__(self, func):
        self.func = func

    def __call__(self, x, pos=None):
        """
        Return the value of the user defined function.

        *x* and *pos* are passed through as-is.
        """
        return self.func(x, pos)
