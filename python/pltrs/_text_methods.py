"""
The methods placing text on a figure or an axes (matplotlib's
``FigureBase.text`` and ``Axes.text``), as mixins the classes include.
"""

from . import _handles
from ._native import text as _text
from .text import Text


def _text_kwargs(fontdict, kwargs):
    """The properties of a new text: *fontdict* overridden by *kwargs*."""
    return {**(fontdict if fontdict is not None else {}), **kwargs}


class FigureText:
    """`text` on a figure."""

    def text(self, x, y, s, fontdict=None, **kwargs):
        """
        Add text to figure.

        Parameters
        ----------
        x, y : float
            The position to place the text. By default, this is in figure
            coordinates, floats in [0, 1]. The coordinate system can be changed
            using the *transform* keyword.

        s : str
            The text string.

        fontdict : dict, optional
            A dictionary to override the default text properties. If not given,
            the defaults are determined by :rc:`font.*`. Properties passed as
            *kwargs* override the corresponding ones given in *fontdict*.

        Returns
        -------
        `~.text.Text`

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.text.Text` properties
            Other miscellaneous text parameters.
        """
        native = _text.figure_text(self._native, float(x), float(y),
                                   '' if s is None else str(s))
        text = _handles.wrap(native, Text, self.figure)
        text._init_properties(_text_kwargs(fontdict, kwargs))
        return text


class AxesText:
    """`text` on an axes, and the texts placed on it."""

    @property
    def texts(self):
        """The texts placed in the Axes with `text` and `annotate`."""
        return _handles.wrap_all(_text.axes_texts(self._native), Text,
                                 self.figure)

    def text(self, x, y, s, fontdict=None, **kwargs):
        """
        Add text to the Axes.

        Add the text *s* to the Axes at location *x*, *y* in data coordinates,
        with a default ``horizontalalignment`` on the ``left`` and
        ``verticalalignment`` at the ``baseline``.

        Parameters
        ----------
        x, y : float
            The position to place the text. By default, this is in data
            coordinates. The coordinate system can be changed using the
            *transform* parameter.

        s : str
            The text.

        fontdict : dict, default: None
            A dictionary to override the default text properties. If fontdict
            is None, the defaults are determined by `.rcParams`.

        Returns
        -------
        `.Text`
            The created `.Text` instance.

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.text.Text` properties.
            Other miscellaneous text parameters.
        """
        native = _text.axes_text(self._native, float(x), float(y),
                                 '' if s is None else str(s))
        text = _handles.wrap(native, Text, self.figure)
        text._init_properties(_text_kwargs(fontdict, kwargs))
        return text
