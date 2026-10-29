"""
Legends (matplotlib's ``legend``).
"""

from . import _handles
from ._native import legend as _legend
from .artist import Artist
from .text import Text


class Legend(Artist):
    """Place a legend on the figure/axes."""

    def get_texts(self):
        """Return the list of `~.text.Text`\\s in the legend."""
        return _handles.wrap_all(_legend.get_texts(self._native), Text, self.figure)
