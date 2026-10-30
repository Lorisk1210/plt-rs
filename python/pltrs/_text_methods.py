"""
The methods placing text on a figure or an axes (matplotlib's
``FigureBase.text``, ``Axes.text`` and ``Axes.annotate``), as mixins the
classes include.
"""

import datetime

import numpy as np

from . import _handles
from ._native import annotation as _annotation, text as _text
from .text import (
    Annotation, Text, _is_offset, _native_arrowprops, _native_coords)


def _text_kwargs(fontdict, kwargs):
    """The properties of a new text: *fontdict* overridden by *kwargs*."""
    return {**(fontdict if fontdict is not None else {}), **kwargs}


def _coordinate(value):
    """A coordinate of an annotated point as `_native.annotation` takes
    it: a number, a category name, or ``("dates", number)`` for a date."""
    if isinstance(value, (str, bytes)):
        return value.decode() if isinstance(value, bytes) else value
    if isinstance(value, (datetime.date, np.datetime64)):
        from . import dates
        return ("dates", float(dates.date2num(value)))
    return float(value)


def _point(xy):
    """The point *xy* of an annotation, coordinate by coordinate."""
    x, y = xy
    return (_coordinate(x), _coordinate(y))


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
        native = _text.axes_text(self._native, float(self.convert_xunits(x)),
                                 float(self.convert_yunits(y)),
                                 '' if s is None else str(s))
        text = _handles.wrap(native, Text, self.figure)
        text._init_properties(_text_kwargs(fontdict, kwargs))
        return text

    def annotate(self, text, xy, xytext=None, xycoords='data', textcoords=None,
                 arrowprops=None, annotation_clip=None, **kwargs):
        """
        Annotate the point *xy* with text *text*.

        In the simplest form, the text is placed at *xy*.

        Optionally, the text can be displayed in another position *xytext*.
        An arrow pointing from the text to the annotated point *xy* can then
        be added by defining *arrowprops*.

        Parameters
        ----------
        text : str
            The text of the annotation.

        xy : (float, float)
            The point *(x, y)* to annotate. The coordinate system is determined
            by *xycoords*.

        xytext : (float, float), default: *xy*
            The position *(x, y)* to place the text at. The coordinate system
            is determined by *textcoords*.

        xycoords : single or two-tuple of str or `.Artist` or `.OffsetFrom`, \
default: 'data'
            The coordinate system that *xy* is given in: a name such as
            'figure points', 'axes fraction' or 'offset points' (for
            *textcoords* only), an `.Artist` whose bbox *xy* is a fraction
            of, or a tuple of these for x and y.

        textcoords : single or two-tuple of str or `.Artist` or \
`.OffsetFrom`, default: value of *xycoords*
            The coordinate system that *xytext* is given in.

        arrowprops : dict, optional
            The properties used to draw a `.FancyArrowPatch` arrow between the
            positions *xy* and *xytext*.  Defaults to None, i.e. no arrow is
            drawn.

        annotation_clip : bool or None, default: None
            Whether to clip (i.e. not draw) the annotation when the annotation
            point *xy* is outside the Axes area.

        **kwargs
            Additional kwargs are passed to `.Text`.

        Returns
        -------
        `.Annotation`
        """
        if _is_offset(xycoords):
            raise ValueError("xycoords cannot be an offset coordinate")
        native = _annotation.axes_annotate(
            self._native, '' if text is None else str(text),
            (_point(xy),
             None if xytext is None else _point(xytext),
             _native_coords(xycoords, self.figure),
             None if textcoords is None
             else _native_coords(textcoords, self.figure, "textcoords"),
             None if arrowprops is None else _native_arrowprops(arrowprops),
             annotation_clip))
        annotation = _handles.wrap(native, Annotation, self.figure)
        annotation._init_annotation(xycoords, textcoords, arrowprops, kwargs)
        return annotation
