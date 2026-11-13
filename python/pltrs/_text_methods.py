"""
The methods placing text on a figure or an axes (matplotlib's
``FigureBase.text``, ``Axes.text`` and ``Axes.annotate``), as mixins the
classes include.
"""

import datetime
from numbers import Real

import numpy as np

from . import _docstring
from . import _handles, _lazy_imports, units as munits
from ._native import annotation as _annotation, text as _text
from ._annotation_kept import annotate_arguments, warn_textcoords
from ._program_classes import takes_from_program
from .text import Annotation, Text, _is_offset, _place

_program_adds = _lazy_imports.module(globals(), ".axes._program_adds")


def _text_kwargs(fontdict, kwargs):
    """The properties of a new text: *fontdict* overridden by *kwargs*."""
    return {**(fontdict if fontdict is not None else {}), **kwargs}



def _init_added(text, properties):
    """
    Apply *properties* to the *text* just added, as matplotlib's
    ``Text.__init__`` applies them before the text is added: a property
    failing leaves no text.
    """
    try:
        text._init_properties(properties)
    except Exception:
        text.remove()
        raise


def _in_data_units(axes, xy, coords):
    """
    The point *xy* of an annotation of *axes*, its values in units (not
    numbers, names or dates, which pltrs (Rust) converts) converted when
    *coords* is ``'data'``, as matplotlib's annotation converts them when
    drawn: by the axis' converter, else by the converter of the value's
    type, which the axis does not take (matplotlib's annotation sets none).
    Names go to the axis' converter when it is of a class of the program's
    (pandas' period converter reads ``'2018-07-27'`` as a day).
    """
    if coords != 'data':
        return xy

    def converted(value, axis):
        if isinstance(value, (str, bytes)):
            converter = axis.get_converter()
            if converter is None or not takes_from_program(type(converter), "convert"):
                return value
            return converter.convert(value, axis.units, axis)
        if isinstance(value, (Real, datetime.date, np.datetime64)):
            return value
        converter = axis.get_converter() or munits.registry.get_converter(value)
        return value if converter is None else converter.convert(value, axis.units, axis)

    x, y = xy
    return converted(x, axes.xaxis), converted(y, axes.yaxis)

class FigureText:
    """`text` on a figure."""

    @_docstring.interpd
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

            %(Text:kwdoc)s

        See Also
        --------
        .Axes.text
        .pyplot.text
        """
        native = _text.figure_text(self._native, 0.0, 0.0, '' if s is None else str(s))
        text = _handles.wrap(native, Text, self._figure)
        _place(text, (x, y))
        _init_added(text, _text_kwargs(fontdict, kwargs))
        return text


class AxesText:
    """`text` and `annotate` on an axes (``ax.texts`` lists the texts placed
    on it, see `.axes._children`)."""

    @_docstring.interpd
    def text(self, x, y, s, fontdict=None, **kwargs):
        """
        Add text to the Axes.

        Add the text *s* to the Axes at location *x*, *y* in data coordinates,
        with a default ``horizontalalignment`` on the ``left`` and
        ``verticalalignment`` at the ``baseline``. See
        :doc:`/gallery/text_labels_and_annotations/text_alignment`.

        Parameters
        ----------
        x, y : float
            The position to place the text. By default, this is in data
            coordinates. The coordinate system can be changed using the
            *transform* parameter.

        s : str
            The text.

        fontdict : dict, default: None

            .. admonition:: Discouraged

               The use of *fontdict* is discouraged. Parameters should be passed as
               individual keyword arguments or using dictionary-unpacking
               ``text(..., **fontdict)``.

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

            %(Text:kwdoc)s

        Examples
        --------
        Individual keyword arguments can be used to override any given
        parameter::

            >>> text(x, y, s, fontsize=12)

        The default transform specifies that text is in data coords,
        alternatively, you can specify text in axis coords ((0, 0) is
        lower-left and (1, 1) is upper-right).  The example below places
        text in the center of the Axes::

            >>> text(0.5, 0.5, 'matplotlib', horizontalalignment='center',
            ...      verticalalignment='center', transform=ax.transAxes)

        You can put a rectangular box around the text instance (e.g., to
        set a background color) by using the keyword *bbox*.  *bbox* is
        a dictionary of `~matplotlib.patches.Rectangle`
        properties.  For example::

            >>> text(x, y, s, bbox=dict(facecolor='red', alpha=0.5))
        """
        native = _text.axes_text(self._native, 0.0, 0.0, '' if s is None else str(s))
        text = _handles.wrap(native, Text, self._figure)
        _place(text, (x, y))
        _init_added(text, _text_kwargs(fontdict, kwargs))
        _program_adds.announce(self, text)
        return text

    @_docstring.interpd
    def annotate(self, text, xy, xytext=None, xycoords='data', textcoords=None,
                 arrowprops=None, annotation_clip=None, **kwargs):
        x, y = xy  # matplotlib unpacks both points as it makes the annotation
        if _is_offset(xycoords):
            raise ValueError("xycoords cannot be an offset coordinate")
        warn_textcoords(xytext, textcoords, xycoords)
        tx, ty = (x, y) if xytext is None else xytext
        in_units = _in_data_units(self, (x, y), xycoords)
        if xytext is not None:
            xytext = _in_data_units(self, xytext, xycoords if textcoords is None else textcoords)
        native = _annotation.axes_annotate(
            self._native, '' if text is None else str(text),
            annotate_arguments(in_units, xytext, xycoords, textcoords, arrowprops,
                               annotation_clip, self.figure))
        annotation = _handles.wrap(native, Annotation, self._figure)
        annotation._init_annotation((x, y), (tx, ty), xycoords, textcoords, arrowprops,
                                    kwargs)
        _program_adds.announce(self, annotation)
        return annotation
    annotate.__doc__ = Annotation.__init__.__doc__
