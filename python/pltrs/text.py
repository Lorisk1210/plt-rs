"""
Text in a figure (matplotlib's ``text``).
"""

from ._native import text as _text
from .artist import Artist


class Text(Artist):
    """Handle storing and drawing of text in window or data coordinates."""

    def get_text(self):
        """Return the text string."""
        return _text.get_text(self._native)

    def set_text(self, s):
        """Set the text string *s*."""
        _text.set_text(self._native, "" if s is None else str(s))
