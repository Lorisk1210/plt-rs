"""
The SVG backend (matplotlib's ``backend_svg``): `FigureCanvasSVG`, whose
``print_svg`` and ``print_svgz`` write the figure as an SVG document,
plain or gzip-compressed. pltrs (Rust) writes the documents.

The checks of the metadata are matplotlib's (from matplotlib 3.11.2, under
the Matplotlib License).
"""

import datetime
from datetime import UTC  # noqa: F401  (matplotlib's backend_svg has it)

import numpy as np

from .. import __version__, _api, rcParams
from .._native import backends as _backends, text_overrides as _text_overrides
from ..backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, RendererBase
from ..font_manager import FontManager
from ..text import _measuring_with
from ._native_drawing import DocumentDrawingMixin
from .backend_mixed import MixedModeRenderer

#: The XML declaration and doctype every SVG document starts with.
svgProlog = _backends.svg_prolog()


@_api.deprecated("3.11")
class XMLWriter:
    """
    Parameters
    ----------
    file : writable text file-like object

    pltrs (Rust) writes the XML (``pltrs_render::XmlWriter``); the writer
    hands the file each piece as matplotlib's does.
    """

    def __init__(self, file):
        self.__write = file.write
        if hasattr(file, "flush"):
            self.flush = file.flush
        self.__native = _backends.xml_new()

    def __written(self, result=None):
        """Write what pltrs (Rust) wrote to the file; return *result*."""
        text = _backends.xml_take(self.__native)
        if text:
            self.__write(text)
        return result

    def start(self, tag, attrib={}, **extra):
        """
        Open a new element.  Attributes can be given as keyword
        arguments, or as a string/string dictionary. The method returns
        an opaque identifier that can be passed to the :meth:`close`
        method, to close all open elements up to and including this one.

        Parameters
        ----------
        tag
            Element tag.
        attrib
            Attribute dictionary.  Alternatively, attributes can be given as
            keyword arguments.

        Returns
        -------
        An element identifier.
        """
        attributes = [(k, v) for k, v in {**attrib, **extra}.items() if v]
        return self.__written(_backends.xml_start(self.__native, tag, attributes))

    def comment(self, comment):
        """
        Add a comment to the output stream.

        Parameters
        ----------
        comment : str
            Comment text.
        """
        _backends.xml_comment(self.__native, comment)
        self.__written()

    def data(self, text):
        """
        Add character data to the output stream.

        Parameters
        ----------
        text : str
            Character data.
        """
        _backends.xml_data(self.__native, text)

    def end(self, tag=None, indent=True):
        """
        Close the current element (opened by the most recent call to
        :meth:`start`).

        Parameters
        ----------
        tag
            Element tag.  If given, the tag must match the start tag.  If
            omitted, the current element is closed.
        indent : bool, default: True
        """
        tags = _backends.xml_tags(self.__native)
        if tag:
            assert tags, f"unbalanced end({tag})"
            assert _backends.xml_closes(self.__native, tag), \
                f"expected end({tags[-1]}), got {tag}"
        else:
            assert tags, "unbalanced end()"
        _backends.xml_end(self.__native, bool(indent))
        self.__written()

    def close(self, id):
        """
        Close open elements, up to (and including) the element identified
        by the given identifier.

        Parameters
        ----------
        id
            Element identifier, as returned by the :meth:`start` method.
        """
        _backends.xml_close(self.__native, id)
        self.__written()

    def element(self, tag, text=None, attrib={}, **extra):
        """
        Add an entire element.  This is the same as calling :meth:`start`,
        :meth:`data`, and :meth:`end` in sequence. The *text* argument can be
        omitted.
        """
        self.start(tag, attrib, **extra)
        if text:
            self.data(text)
        self.end(indent=False)

    def flush(self):
        """Flush the output stream."""
        pass  # replaced by the constructor


def _escape_cdata(s):
    """*s* with the characters XML character data cannot hold escaped."""
    s = s.replace("&", "&amp;")
    s = s.replace("<", "&lt;")
    s = s.replace(">", "&gt;")
    return s

#: The metadata keys an SVG document holds.
_METADATA_KEYS = frozenset([
    'Title', 'Coverage', 'Date', 'Description', 'Format', 'Identifier',
    'Language', 'Relation', 'Source', 'Type', 'Creator', 'Contributor',
    'Publisher', 'Rights', 'Keywords'])


def _check_is_str(info, key):
    if not isinstance(info, str):
        raise TypeError(f'Invalid type for {key} metadata. Expected str, not '
                        f'{type(info)}.')


def _check_is_iterable_of_str(infos, key):
    if np.iterable(infos):
        for info in infos:
            if not isinstance(info, str):
                raise TypeError(f'Invalid type for {key} metadata. Expected '
                                f'iterable of str, not {type(info)}.')
    else:
        raise TypeError(f'Invalid type for {key} metadata. Expected str or '
                        f'iterable of str, not {type(infos)}.')


def _date_text(date):
    """The ``Date`` metadata *date* as matplotlib writes it: ISO 8601,
    several dates joined by slashes."""
    if isinstance(date, str):
        return date
    if isinstance(date, (datetime.datetime, datetime.date)):
        return date.isoformat()
    if np.iterable(date):
        dates = []
        for d in date:
            if isinstance(d, str):
                dates.append(d)
            elif isinstance(d, (datetime.datetime, datetime.date)):
                dates.append(d.isoformat())
            else:
                raise TypeError(
                    f'Invalid type for Date metadata. '
                    f'Expected iterable of str, date, or datetime, '
                    f'not {type(d)}.')
        return '/'.join(dates)
    raise TypeError(f'Invalid type for Date metadata. '
                    f'Expected str, date, datetime, or iterable '
                    f'of the same, not {type(date)}.')


def _svg_metadata(metadata):
    """*metadata* checked as matplotlib's SVG writer checks it, with
    pltrs (Python) as default creator and the dates written out; the
    entries left to pltrs (Rust)'s defaults (format, type, date) are not
    added."""
    from .._native import backends as _backends
    metadata = {
        'Creator': f'{_backends.default_creator()}, matplotlib {__version__} API',
        **(metadata or {}),
    }
    unknown = [key for key in metadata if key not in _METADATA_KEYS]
    if unknown:
        raise ValueError('Unknown metadata key(s) passed to SVG writer: ' +
                         ','.join(unknown))
    for key, value in metadata.items():
        if value is None:
            continue
        if key == 'Date':
            metadata[key] = _date_text(value)
        elif key in ('Creator', 'Contributor', 'Publisher', 'Rights',
                     'Keywords'):
            if not isinstance(value, str):
                _check_is_iterable_of_str(value, key)
                metadata[key] = list(value)
        else:
            _check_is_str(value, key)
    return metadata


class RendererSVG(DocumentDrawingMixin, RendererBase):
    """
    The renderer of an SVG document of *width* × *height* points, its
    images at *image_dpi*, written to *svgwriter*.

    pltrs (Rust) writes the documents itself: this is the renderer the
    program's drawing receives while a figure is saved as SVG
    (`FigureCanvasSVG._program_renderer`), which draws into the document
    pltrs (Rust) writes. One the program makes around an *svgwriter* of
    its own draws into a document of its own, which `finalize` writes
    there.
    """

    def __init__(self, width, height, svgwriter, basename=None, image_dpi=72,
                 *, metadata=None):
        super().__init__()
        self.width = width
        self.height = height
        self.image_dpi = image_dpi
        self.basename = basename
        if svgwriter is not None:
            self._file = svgwriter
            self._page = _backends.svg_page_new(
                float(width), float(height), float(image_dpi),
                _svg_metadata(metadata))

    def finalize(self):
        """Write the document drawn to the *svgwriter* (the renderers of
        pltrs's saves have no document of their own: pltrs (Rust) writes
        it)."""
        if self._page is not None:
            self._file.write(_backends.page_finish(self._page))
            if hasattr(self._file, "flush"):
                self._file.flush()

    def get_canvas_width_height(self):
        # docstring inherited
        return self.width, self.height

    def get_text_width_height_descent(self, s, prop, ismath):
        # docstring inherited
        _api.check_in_list(["TeX", True, False], ismath=ismath)
        return _text_overrides.measure_svg(s, FontManager._props(prop), ismath,
                                           72.0)

    def option_image_nocomposite(self):
        # docstring inherited
        return not rcParams['image.composite_image']

    def option_scale_image(self):
        # docstring inherited
        return True

    def get_image_magnification(self):
        # docstring inherited
        return self.image_dpi / 72.0


class FigureCanvasSVG(FigureCanvasBase):
    filetypes = {'svg': 'Scalable Vector Graphics',
                 'svgz': 'Scalable Vector Graphics'}

    fixed_dpi = 72

    def print_svg(self, filename, *, bbox_inches_restore=None, metadata=None,
                  facecolor=None, edgecolor=None):
        """
        Write the figure as an SVG document.

        Parameters
        ----------
        filename : str or path-like or file-like
            Output target; if a string, a file will be opened for writing.
        metadata : dict[str, Any], optional
            Metadata in the SVG file defined as key-value pairs of strings,
            datetimes, or lists of strings.
        """
        self._print_svg("svg", filename, bbox_inches_restore, metadata,
                        facecolor, edgecolor)

    def print_svgz(self, filename, *, bbox_inches_restore=None, metadata=None,
                   facecolor=None, edgecolor=None):
        """Write the figure as a gzip-compressed SVG document."""
        self._print_svg("svgz", filename, bbox_inches_restore, metadata,
                        facecolor, edgecolor)

    def _print_svg(self, fmt, filename, bbox_inches_restore, metadata,
                   facecolor, edgecolor):
        """Write the figure as *fmt*, ``svg`` or ``svgz``, its texts measured
        through the metrics cache of a renderer of the print's own, as
        matplotlib's ``print_svg`` draws with a new `RendererSVG`."""
        width, height = self.figure.get_size_inches()
        renderer = RendererSVG(width * 72, height * 72, None, image_dpi=self.figure.dpi)
        with _measuring_with(renderer):
            self._print_native(fmt, filename, metadata=_svg_metadata(metadata),
                               bbox_inches_restore=bbox_inches_restore,
                               facecolor=facecolor, edgecolor=edgecolor)

    def get_default_filetype(self):
        return 'svg'

    def _program_renderer(self, dpi, bbox_inches_restore):
        """The renderer the program's drawing receives while the figure is
        saved at *dpi* (matplotlib's ``print_svg`` draws the figure with it):
        a `MixedModeRenderer` around a `RendererSVG`, the figure drawn at
        72 dpi meanwhile."""
        width, height = self.figure.get_size_inches()
        return MixedModeRenderer(
            self.figure, width, height, dpi,
            RendererSVG(width * 72, height * 72, None, image_dpi=dpi),
            bbox_inches_restore=bbox_inches_restore)

    def draw(self):
        self.figure.draw_without_rendering()
        return super().draw()


FigureManagerSVG = FigureManagerBase


@_Backend.export
class _BackendSVG(_Backend):
    backend_version = __version__
    FigureCanvas = FigureCanvasSVG
